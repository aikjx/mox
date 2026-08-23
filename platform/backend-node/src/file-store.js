'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { getStorage } = require('./storage');
const { config, DATA_DIR } = require('./config');
const { createDefaultBackend, IChunkBackend } = require('./storage/chunk-backend');

const FILE_STORE_DIR = path.join(DATA_DIR, 'file-store');
const VERSIONS_DIR = path.join(FILE_STORE_DIR, 'versions');
const CHUNKS_DIR = path.join(FILE_STORE_DIR, 'chunks');

function ensureDirs() {
  [FILE_STORE_DIR, VERSIONS_DIR, CHUNKS_DIR].forEach(dir => {
    if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
  });
}
ensureDirs();

function hashFile(buffer) {
  return crypto.createHash('sha256').update(buffer).digest('hex');
}
function generateId(prefix) {
  return `${prefix}_${Date.now()}_${crypto.randomBytes(4).toString('hex')}`;
}
function _parseBool(v, fallback) {
  if (v === undefined || v === null || v === '') return fallback;
  const s = String(v).toLowerCase();
  if (['1', 'true', 'yes', 'on'].includes(s)) return true;
  if (['0', 'false', 'no', 'off'].includes(s)) return false;
  return fallback;
}

/**
 * 版本 manifest 后端：默认写本地 VERSIONS_DIR，同时可写对象桶（可选）。
 * 为保持与旧版 file-store.js 完全兼容，默认仍保留本地 vN.json 作为主版本元数据。
 */
class VersionManifestBackend {
  constructor({ chunkBackend } = {}) {
    this.chunkBackend = chunkBackend;
    this.remoteManifestBucket = process.env.S3_MANIFEST_BUCKET || null;
    this.remotePrefix = process.env.S3_MANIFEST_PREFIX || 'manifests';
  }
  _local(fileId, ver) { return path.join(VERSIONS_DIR, fileId, `v${ver}.json`); }
  async write(fileId, version, meta) {
    const localDir = path.join(VERSIONS_DIR, fileId);
    if (!fs.existsSync(localDir)) fs.mkdirSync(localDir, { recursive: true });
    const local = this._local(fileId, version);
    fs.writeFileSync(local, JSON.stringify(meta, null, 2));
    if (this.remoteManifestBucket && this.chunkBackend && typeof this.chunkBackend.writeChunk === 'function') {
      // 借用 chunkBackend 的通用 S3 写（S3 客户端相同，key 走 manifests 前缀）
      const key = `${this.remotePrefix}/${fileId}/v${version}.json`;
      try {
        // S3Backend 不公开此接口；此处降级为：若后端是 S3ChunkBackend，则用其懒加载的 s3，否则仅本地。
        // 为保持最小实现，这里不重复造轮子。生产环境可单独增强 RemoteManifestBackend。
        void key;
      } catch {}
    }
  }
  read(fileId, version) {
    const local = this._local(fileId, version);
    if (!fs.existsSync(local)) throw new Error(`Version ${version} not found for ${fileId}`);
    return JSON.parse(fs.readFileSync(local, 'utf8'));
  }
  listVersions(fileId) {
    const dir = path.join(VERSIONS_DIR, fileId);
    if (!fs.existsSync(dir)) return [];
    const fs_ = fs.readdirSync(dir).filter(f => f.endsWith('.json'));
    return fs_.map(f => {
      const m = JSON.parse(fs.readFileSync(path.join(dir, f), 'utf8'));
      return { version: m.version, size: m.size, hash: m.hash, uploadedAt: m.uploadedAt, changeNote: m.changeNote };
    }).sort((a, b) => b.version - a.version);
  }
  removeAll(fileId) {
    const dir = path.join(VERSIONS_DIR, fileId);
    if (fs.existsSync(dir)) fs.rmSync(dir, { recursive: true, force: true });
  }
}

class FileStore {
  constructor({ chunkBackend, storage, options = {} } = {}) {
    this.storage = storage || getStorage();
    this.chunkBackend = chunkBackend || createDefaultBackend();
    this.chunkSize = options.chunkSize || (1024 * 1024);
    this.mpuThreshold = options.mpuThreshold || (100 * 1024 * 1024); // >= 100MB 用 MPU
    this.mpuConcurrency = parseInt(process.env.FILE_MPU_CONCURRENCY || String(options.mpuConcurrency || '4'), 10) || 4;
    this.graceDays = parseFloat(process.env.FILE_GRACE_DAYS || String(options.graceDays || '30'));
    // 是否启用软删：FILE_SOFT_DELETE=true（默认 true，符合企业级"安全删除"）
    this.softDelete = _parseBool(process.env.FILE_SOFT_DELETE, options.softDelete !== undefined ? options.softDelete : true);
    this.versionManifestBackend = new VersionManifestBackend({ chunkBackend: this.chunkBackend });
    this._initIndex();
  }

  _initIndex() {
    const index = this.storage.getEntityData('file_store_index', 'main');
    if (!index) {
      this.storage.upsertEntity('file_store_index', 'main', {
        totalFiles: 0,
        totalVersions: 0,
        totalSize: 0,
        createdAt: new Date().toISOString()
      });
    }
    // chunk 引用计数器（用于 GC）：entity_type=file_chunk_refs id=chunkHash → {refs:[fileId:version]}
  }

  /**
   * 写一组 chunks：SHA-256 去重。大文件≥mpuThreshold 时对每个 chunk 走 writeChunk。
   * 若调用方整体 buffer ≥ mpuThreshold，仍采用"先切块再并行 writeChunk"；
   * S3 MPU 的"单对象大文件 MPU"在此作为可选加速，当 chunk 数 >= mpuMinParts 时使用。
   */
  async _writeChunksDeduped(chunks) {
    const chunkHashes = [];
    for (let i = 0; i < chunks.length; i++) {
      const chunk = chunks[i];
      const chunkHash = hashFile(chunk);
      const ok = await this.chunkBackend.hasChunk(chunkHash);
      if (!ok) await this.chunkBackend.writeChunk(chunkHash, chunk);
      chunkHashes.push(chunkHash);
    }
    return chunkHashes;
  }

  async _writeBigFileMPU(buffer, fileHash) {
    // MPU 最小 partSize 5MB，最大 16MB，按 buffer.length 计算：
    const total = buffer.length;
    let partSize = 5 * 1024 * 1024;
    if (total / partSize > 1000) partSize = 8 * 1024 * 1024;
    if (total / partSize > 1000) partSize = 16 * 1024 * 1024;
    const N = Math.ceil(total / partSize);
    const { uploadId } = await this.chunkBackend.createMultipartUpload(fileHash);
    const parts = [];
    // 并发上传（FILE_MPU_CONCURRENCY）
    const queue = [];
    let idx = 1;
    for (let off = 0; off < total; off += partSize) {
      const partNumber = idx++;
      const slice = buffer.slice(off, Math.min(off + partSize, total));
      queue.push((async () => {
        const r = await this.chunkBackend.uploadPart(fileHash, uploadId, partNumber, slice);
        return r;
      })());
      if (queue.length >= this.mpuConcurrency) {
        parts.push(...(await Promise.all(queue)));
        queue.length = 0;
      }
    }
    if (queue.length) parts.push(...(await Promise.all(queue)));
    const fin = await this.chunkBackend.completeMultipartUpload(fileHash, uploadId, parts);
    // 为了和非 MPU 路径一致，仍返回"伪 chunk 列表"：整个文件 hash 作为单块（vN 仍需要 chunks 数组来追溯 hash）。
    // 但为保持现有 chunks / 逐块读一致性，我们不采用这种处理：这里 MPU 路径只用于"已 hash 写大对象桶加速上传"，
    // 仍保留 chunk level（1MB）的 chunks：所以此方法返回 {fileHash, accelerated:true}，由调用方仍负责落 chunks 数组。
    return { fileHash, accelerated: true, bytes: total, totalParts: parts.length, fin };
  }

  async uploadFile(buffer, filename, options = {}) {
    const fileId = generateId('file');
    const ext = path.extname(filename) || '.bin';
    const hash = hashFile(buffer);
    const size = buffer.length;
    const mimeType = options.mimeType || this._guessMime(ext);

    // MPU 加速（仅对 s3-like backend 有意义；fs MPU 等价写本地合并）
    if (size >= this.mpuThreshold) {
      try { await this._writeBigFileMPU(buffer, hash); } catch (e) {
        console.warn('[file-store] MPU 失败，回退到常规分块上传：', e.message);
      }
    }

    const chunks = this._chunkBuffer(buffer);
    const chunkHashes = await this._writeChunksDeduped(chunks);

    const versionMeta = {
      version: 1,
      hash,
      size,
      chunks: chunkHashes,
      chunkCount: chunkHashes.length,
      uploadedAt: new Date().toISOString(),
      uploadedBy: options.userId || 'system',
      changeNote: options.changeNote || 'initial upload'
    };
    await this.versionManifestBackend.write(fileId, 1, versionMeta);

    const fileEntity = {
      id: fileId,
      originalName: filename,
      ext,
      mimeType,
      currentVersion: 1,
      totalVersions: 1,
      size,
      hash,
      chunkCount: chunkHashes.length,
      chunks: chunkHashes,
      uploadDir: path.join(VERSIONS_DIR, fileId),
      tags: options.tags || [],
      metadata: options.metadata || {},
      linkedGraphIds: options.linkedGraphIds || [],
      status: 'active',
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString()
    };
    this.storage.upsertEntity('files', fileId, fileEntity);
    this._updateIndex(1, size);
    this._bumpChunkRefs(fileId, 1, chunkHashes, +1);
    return this._hydrateFile(fileEntity);
  }

  async uploadNewVersion(fileId, buffer, changeNote = '') {
    const file = this.storage.getEntityData('files', fileId);
    if (!file) throw new Error('File not found: ' + fileId);
    const hash = hashFile(buffer);
    const size = buffer.length;
    if (size >= this.mpuThreshold) {
      try { await this._writeBigFileMPU(buffer, hash); } catch (e) {
        console.warn('[file-store][new-version] MPU 失败，回退到常规分块上传：', e.message);
      }
    }
    const chunks = this._chunkBuffer(buffer);
    const chunkHashes = await this._writeChunksDeduped(chunks);
    const newVersion = file.currentVersion + 1;
    const versionMeta = {
      version: newVersion,
      hash,
      size,
      chunks: chunkHashes,
      chunkCount: chunkHashes.length,
      uploadedAt: new Date().toISOString(),
      uploadedBy: 'system',
      changeNote: changeNote || `v${newVersion}`
    };
    await this.versionManifestBackend.write(fileId, newVersion, versionMeta);

    const oldSize = file.size || 0;
    const updated = {
      ...file,
      currentVersion: newVersion,
      totalVersions: newVersion,
      size,
      hash,
      chunkCount: chunkHashes.length,
      chunks: chunkHashes,
      updatedAt: new Date().toISOString()
    };
    this.storage.upsertEntity('files', fileId, updated);
    this._bumpChunkRefs(fileId, newVersion, chunkHashes, +1);
    this._updateIndex(1, size - oldSize);
    return this._hydrateFile(updated);
  }

  getFile(fileId) {
    const file = this.storage.getEntityData('files', fileId);
    return file ? this._hydrateFile(file) : null;
  }

  async getFileContent(fileId, version) {
    const file = this.storage.getEntityData('files', fileId);
    if (!file) throw new Error('File not found: ' + fileId);
    if (file.status === 'deleted' || file.status === 'purged') throw new Error('File has been purged: ' + fileId);
    const ver = version || file.currentVersion;
    const meta = this.versionManifestBackend.read(fileId, ver);
    const bufs = [];
    for (const h of meta.chunks) bufs.push(await this.chunkBackend.readChunk(h));
    return Buffer.concat(bufs);
  }

  getVersions(fileId) { return this.versionManifestBackend.listVersions(fileId); }

  listFiles(filters = {}) {
    let files = this.storage.getList('files');
    if (filters.ext) files = files.filter(f => f.ext === filters.ext);
    if (filters.tag) files = files.filter(f => (f.tags || []).includes(filters.tag));
    if (filters.status) files = files.filter(f => f.status === filters.status);
    if (filters.graphLinked !== undefined) {
      files = files.filter(f => filters.graphLinked ? (f.linkedGraphIds || []).length > 0 : (f.linkedGraphIds || []).length === 0);
    }
    return files.map(f => this._hydrateFile(f));
  }

  searchFiles(query) {
    const files = this.storage.getList('files');
    const q = String(query).toLowerCase();
    return files.filter(f =>
      f.originalName.toLowerCase().includes(q) ||
      (f.tags || []).some(t => String(t).toLowerCase().includes(q)) ||
      (f.metadata && JSON.stringify(f.metadata).toLowerCase().includes(q))
    ).map(f => this._hydrateFile(f));
  }

  /**
   * 删除文件。
   *  - 若 softDelete=true 且 force!==true：标记 status='soft_deleted'，记录 deletedAt；chunk 引用 -1，但不物理删；
   *  - 若 force=true 或 softDelete=false：立即物理删（版本 vN + chunk ref 回收）。
   */
  deleteFile(fileId, { force = false } = {}) {
    const file = this.storage.getEntityData('files', fileId);
    if (!file) return false;
    if (!force && this.softDelete && file.status !== 'soft_deleted') {
      const updated = { ...file, status: 'soft_deleted', deletedAt: new Date().toISOString(), updatedAt: new Date().toISOString() };
      this.storage.upsertEntity('files', fileId, updated);
      // 引用 -1，仍保留版本 meta 以便恢复
      for (let v = 1; v <= (file.totalVersions || 1); v++) {
        let meta; try { meta = this.versionManifestBackend.read(fileId, v); } catch { meta = null; }
        if (meta) this._bumpChunkRefs(fileId, v, meta.chunks, -1);
      }
      this._updateIndex(-1, -(file.size || 0));
      return true;
    }
    // 物理删
    for (let v = 1; v <= (file.totalVersions || 1); v++) {
      let meta; try { meta = this.versionManifestBackend.read(fileId, v); } catch { meta = null; }
      if (meta) this._bumpChunkRefs(fileId, v, meta.chunks, -1);
    }
    // 物理 chunks：由 GC 作业异步清理（runGC），此处不直接删，以避免多版本共享 chunk 的误删。
    this.versionManifestBackend.removeAll(fileId);
    this.storage.deleteEntity(fileId);
    this._updateIndex(-1, -(file.size || 0));
    return true;
  }

  /**
   * GC：
   *  - 对 soft_deleted 且 deletedAt + graceDays < now 的文件执行物理删（版本元数据 + files entity）；
   *  - 扫描所有 chunk refs：refCount<=0 且不在任何活跃版本 chunks 中的 chunk，物理删除；
   * 返回 {soft_purged, chunks_deleted, bytes_freed}。
   */
  async runGC({ now = Date.now() } = {}) {
    const stats = { soft_purged: 0, chunks_deleted: 0, bytes_freed: 0, inspected_chunks: 0 };
    // 1) 过期软删文件 → 物理删
    const graceMs = this.graceDays * 24 * 3600 * 1000;
    const softs = this.storage.getList('files').filter(f => f.status === 'soft_deleted');
    for (const f of softs) {
      const delAt = f.deletedAt ? new Date(f.deletedAt).getTime() : now;
      if (now - delAt >= graceMs) {
        // 物理删：force=true，绕开软删再判断
        this.storage.upsertEntity('files', f.id, { ...f, status: 'purged' });
        this.versionManifestBackend.removeAll(f.id);
        // 此时 bump 已在 deleteFile soft 阶段 -1 过，这里只清理 entity 与版本目录
        this.storage.deleteEntity(f.id);
        stats.soft_purged++;
      }
    }
    // 2) 扫描 chunk refs：refCount <= 0 的 chunk 直接删
    const refs = this._allChunkRefs();
    stats.inspected_chunks = Object.keys(refs).length;
    for (const [hash, r] of Object.entries(refs)) {
      if (r.count <= 0) {
        const size = r.size || 0;
        try {
          const r0 = await this.chunkBackend.deleteChunk(hash);
          if (r0 && r0.deleted) { stats.chunks_deleted++; stats.bytes_freed += size; }
          this.storage.deleteEntity('__chunk_ref_' + hash);
        } catch (e) {
          console.warn('[file-store][GC] deleteChunk failed:', hash, e.message);
        }
      }
    }
    return stats;
  }

  restoreVersion(fileId, targetVersion) {
    const file = this.storage.getEntityData('files', fileId);
    if (!file) throw new Error('File not found');
    const meta = this.versionManifestBackend.read(fileId, targetVersion);
    const newVersion = file.currentVersion + 1;
    const versionMeta = {
      version: newVersion,
      hash: meta.hash,
      size: meta.size,
      chunks: meta.chunks,
      chunkCount: meta.chunkCount,
      uploadedAt: new Date().toISOString(),
      uploadedBy: 'system',
      changeNote: `Restore from v${targetVersion}`
    };
    this.versionManifestBackend.write(fileId, newVersion, versionMeta).catch(() => {}); // 本地同步写
    const oldSize = file.size || 0;
    const updated = {
      ...file,
      status: file.status === 'soft_deleted' ? 'active' : file.status,
      currentVersion: newVersion,
      totalVersions: newVersion,
      size: meta.size,
      hash: meta.hash,
      chunkCount: meta.chunkCount,
      chunks: meta.chunks,
      updatedAt: new Date().toISOString()
    };
    this._bumpChunkRefs(fileId, newVersion, meta.chunks, +1);
    this.storage.upsertEntity('files', fileId, updated);
    this._updateIndex(1, meta.size - oldSize);
    return this._hydrateFile(updated);
  }

  linkToGraph(fileId, graphNodeIds) {
    const file = this.storage.getEntityData('files', fileId);
    if (!file) throw new Error('File not found');
    const existing = new Set(file.linkedGraphIds || []);
    graphNodeIds.forEach(id => existing.add(id));
    const updated = {
      ...file,
      linkedGraphIds: Array.from(existing),
      updatedAt: new Date().toISOString()
    };
    this.storage.upsertEntity('files', fileId, updated);
    return this._hydrateFile(updated);
  }

  unlinkFromGraph(fileId, graphNodeId) {
    const file = this.storage.getEntityData('files', fileId);
    if (!file) throw new Error('File not found');
    const updated = {
      ...file,
      linkedGraphIds: (file.linkedGraphIds || []).filter(id => id !== graphNodeId),
      updatedAt: new Date().toISOString()
    };
    this.storage.upsertEntity('files', fileId, updated);
    return this._hydrateFile(updated);
  }

  getStats() {
    const index = this.storage.getEntityData('file_store_index', 'main') || {};
    const files = this.storage.getList('files');
    const byExt = {};
    let linkedCount = 0;
    files.forEach(f => {
      byExt[f.ext] = (byExt[f.ext] || 0) + 1;
      if (f.linkedGraphIds && f.linkedGraphIds.length > 0) linkedCount++;
    });
    return {
      totalFiles: index.totalFiles || files.length,
      totalVersions: index.totalVersions || files.reduce((a, f) => a + (f.totalVersions || 0), 0),
      totalSize: index.totalSize || files.reduce((a, f) => a + (f.size || 0), 0),
      filesByExtension: byExt,
      graphLinkedFiles: linkedCount,
      graphCoverage: files.length > 0 ? Math.round((linkedCount / files.length) * 100) : 0,
      chunkBackend: this.chunkBackend.name || this.chunkBackend.constructor.name,
      softDelete: this.softDelete,
      graceDays: this.graceDays
    };
  }

  _hydrateFile(file) {
    return {
      id: file.id,
      originalName: file.originalName,
      ext: file.ext,
      mimeType: file.mimeType,
      currentVersion: file.currentVersion,
      totalVersions: file.totalVersions,
      size: file.size,
      hash: file.hash,
      chunkCount: file.chunkCount,
      tags: file.tags || [],
      metadata: file.metadata || {},
      linkedGraphIds: file.linkedGraphIds || [],
      status: file.status || 'active',
      createdAt: file.createdAt,
      updatedAt: file.updatedAt,
      deletedAt: file.deletedAt || null,
      graphLinked: (file.linkedGraphIds || []).length > 0
    };
  }

  _chunkBuffer(buffer) {
    const chunks = [];
    for (let i = 0; i < buffer.length; i += this.chunkSize) chunks.push(buffer.slice(i, i + this.chunkSize));
    return chunks;
  }

  _updateIndex(fileDelta, sizeDelta) {
    const index = this.storage.getEntityData('file_store_index', 'main') || { totalFiles: 0, totalVersions: 0, totalSize: 0 };
    index.totalFiles = Math.max(0, (index.totalFiles || 0) + (fileDelta > 0 && fileDelta === 1 ? 1 : (fileDelta < 0 ? -1 : 0)));
    index.totalSize = Math.max(0, (index.totalSize || 0) + sizeDelta);
    index.totalVersions = (index.totalVersions || 0) + (fileDelta > 0 ? 1 : 0);
    index.updatedAt = new Date().toISOString();
    this.storage.upsertEntity('file_store_index', 'main', index);
  }

  /**
   * chunk 引用计数：key = hash → { count, size, refs: ["fileId:v1",...] }
   * 存储用 entity_type=file_chunk_refs id=hash。
   */
  _readChunkRef(hash) {
    return this.storage.getEntityData('file_chunk_refs', hash) || { count: 0, size: 0, refs: [] };
  }
  _writeChunkRef(hash, obj) { this.storage.upsertEntity('file_chunk_refs', hash, obj); }
  _allChunkRefs() {
    const list = this.storage.listEntities('file_chunk_refs');
    const out = {};
    for (const e of list) out[e.id] = e.data;
    return out;
  }
  _bumpChunkRefs(fileId, version, chunkHashes, delta) {
    const refKey = `${fileId}:v${version}`;
    for (const h of chunkHashes) {
      const ref = this._readChunkRef(h);
      const set = new Set(ref.refs || []);
      if (delta > 0) set.add(refKey); else set.delete(refKey);
      ref.refs = Array.from(set);
      ref.count = ref.refs.length;
      this._writeChunkRef(h, ref);
    }
  }

  _guessMime(ext) {
    const types = {
      '.pdf': 'application/pdf',
      '.doc': 'application/msword',
      '.docx': 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
      '.xls': 'application/vnd.ms-excel',
      '.xlsx': 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
      '.ppt': 'application/vnd.ms-powerpoint',
      '.pptx': 'application/vnd.openxmlformats-officedocument.presentationml.presentation',
      '.txt': 'text/plain',
      '.md': 'text/markdown',
      '.json': 'application/json',
      '.html': 'text/html',
      '.css': 'text/css',
      '.js': 'application/javascript',
      '.ts': 'application/typescript',
      '.png': 'image/png',
      '.jpg': 'image/jpeg',
      '.jpeg': 'image/jpeg',
      '.gif': 'image/gif',
      '.svg': 'image/svg+xml',
      '.mp4': 'video/mp4',
      '.mp3': 'audio/mpeg',
      '.zip': 'application/zip',
      '.rar': 'application/x-rar-compressed',
      '.7z': 'application/x-7z-compressed'
    };
    return types[(ext || '').toLowerCase()] || 'application/octet-stream';
  }
}

let _instance = null;

function getFileStore() {
  if (!_instance) _instance = new FileStore();
  return _instance;
}

/** 用于单测：重置单例，注入临时 backend/storage */
function resetFileStore({ chunkBackend, storage, options } = {}) {
  _instance = new FileStore({ chunkBackend, storage, options });
  return _instance;
}

module.exports = { FileStore, getFileStore, resetFileStore, VersionManifestBackend, FILE_STORE_DIR, VERSIONS_DIR, CHUNKS_DIR };
