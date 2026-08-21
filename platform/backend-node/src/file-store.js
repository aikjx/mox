'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { getStorage } = require('./storage');
const { config, DATA_DIR } = require('./config');

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

class FileStore {
  constructor() {
    this.storage = getStorage();
    this.chunkSize = 1024 * 1024;
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
  }

  async uploadFile(buffer, filename, options = {}) {
    const fileId = generateId('file');
    const ext = path.extname(filename) || '.bin';
    const hash = hashFile(buffer);
    const size = buffer.length;
    const mimeType = options.mimeType || this._guessMime(ext);
    const chunks = this._chunkBuffer(buffer);
    const chunkHashes = [];

    for (let i = 0; i < chunks.length; i++) {
      const chunkHash = hashFile(chunks[i]);
      const chunkPath = path.join(CHUNKS_DIR, chunkHash);
      if (!fs.existsSync(chunkPath)) {
        fs.writeFileSync(chunkPath, chunks[i]);
      }
      chunkHashes.push(chunkHash);
    }

    const versionDir = path.join(VERSIONS_DIR, fileId);
    if (!fs.existsSync(versionDir)) fs.mkdirSync(versionDir, { recursive: true });

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

    const versionPath = path.join(versionDir, 'v1.json');
    fs.writeFileSync(versionPath, JSON.stringify(versionMeta, null, 2));

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
      uploadDir: versionDir,
      tags: options.tags || [],
      metadata: options.metadata || {},
      linkedGraphIds: [],
      status: 'active',
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString()
    };

    this.storage.upsertEntity('files', fileId, fileEntity);
    this._updateIndex(1, size);

    return this._hydrateFile(fileEntity);
  }

  async uploadNewVersion(fileId, buffer, changeNote = '') {
    const file = this.storage.getEntityData('files', fileId);
    if (!file) throw new Error('File not found: ' + fileId);

    const hash = hashFile(buffer);
    const size = buffer.length;
    const chunks = this._chunkBuffer(buffer);
    const chunkHashes = [];

    for (let i = 0; i < chunks.length; i++) {
      const chunkHash = hashFile(chunks[i]);
      const chunkPath = path.join(CHUNKS_DIR, chunkHash);
      if (!fs.existsSync(chunkPath)) {
        fs.writeFileSync(chunkPath, chunks[i]);
      }
      chunkHashes.push(chunkHash);
    }

    const newVersion = file.currentVersion + 1;
    const versionDir = path.join(VERSIONS_DIR, fileId);
    if (!fs.existsSync(versionDir)) fs.mkdirSync(versionDir, { recursive: true });

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

    fs.writeFileSync(path.join(versionDir, `v${newVersion}.json`), JSON.stringify(versionMeta, null, 2));

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
    this._updateIndex(1, size);

    return this._hydrateFile(updated);
  }

  getFile(fileId) {
    const file = this.storage.getEntityData('files', fileId);
    return file ? this._hydrateFile(file) : null;
  }

  getFileContent(fileId, version) {
    const file = this.storage.getEntityData('files', fileId);
    if (!file) throw new Error('File not found: ' + fileId);

    const ver = version || file.currentVersion;
    const versionDir = path.join(VERSIONS_DIR, fileId);
    const versionPath = path.join(versionDir, `v${ver}.json`);

    if (!fs.existsSync(versionPath)) throw new Error(`Version ${ver} not found`);

    const meta = JSON.parse(fs.readFileSync(versionPath, 'utf8'));
    const buffers = meta.chunks.map(h => fs.readFileSync(path.join(CHUNKS_DIR, h)));
    return Buffer.concat(buffers);
  }

  getVersions(fileId) {
    const file = this.storage.getEntityData('files', fileId);
    if (!file) throw new Error('File not found: ' + fileId);

    const versionDir = path.join(VERSIONS_DIR, fileId);
    if (!fs.existsSync(versionDir)) return [];

    const files = fs.readdirSync(versionDir).filter(f => f.endsWith('.json'));
    return files.map(f => {
      const meta = JSON.parse(fs.readFileSync(path.join(versionDir, f), 'utf8'));
      return { version: meta.version, size: meta.size, hash: meta.hash, uploadedAt: meta.uploadedAt, changeNote: meta.changeNote };
    }).sort((a, b) => b.version - a.version);
  }

  listFiles(filters = {}) {
    let files = this.storage.getList('files');
    if (filters.ext) files = files.filter(f => f.ext === filters.ext);
    if (filters.tag) files = files.filter(f => f.tags.includes(filters.tag));
    if (filters.status) files = files.filter(f => f.status === filters.status);
    if (filters.graphLinked !== undefined) {
      files = files.filter(f => filters.graphLinked ? f.linkedGraphIds.length > 0 : f.linkedGraphIds.length === 0);
    }
    return files.map(f => this._hydrateFile(f));
  }

  searchFiles(query) {
    const files = this.storage.getList('files');
    const q = query.toLowerCase();
    return files.filter(f =>
      f.originalName.toLowerCase().includes(q) ||
      (f.tags || []).some(t => t.toLowerCase().includes(q)) ||
      (f.metadata && JSON.stringify(f.metadata).toLowerCase().includes(q))
    ).map(f => this._hydrateFile(f));
  }

  deleteFile(fileId) {
    const file = this.storage.getEntityData('files', fileId);
    if (!file) return false;

    const versionDir = path.join(VERSIONS_DIR, fileId);
    if (fs.existsSync(versionDir)) {
      const versions = fs.readdirSync(versionDir);
      for (const v of versions) {
        const meta = JSON.parse(fs.readFileSync(path.join(versionDir, v), 'utf8'));
        meta.chunks.forEach(h => {
          const cp = path.join(CHUNKS_DIR, h);
          if (fs.existsSync(cp)) {
            try { fs.unlinkSync(cp); } catch {}
          }
        });
      }
      fs.rmSync(versionDir, { recursive: true, force: true });
    }

    this.storage.deleteEntity(fileId);
    this._updateIndex(-1, -(file.size || 0));
    return true;
  }

  restoreVersion(fileId, targetVersion) {
    const file = this.storage.getEntityData('files', fileId);
    if (!file) throw new Error('File not found');

    const versionDir = path.join(VERSIONS_DIR, fileId);
    const vPath = path.join(versionDir, `v${targetVersion}.json`);
    if (!fs.existsSync(vPath)) throw new Error(`Version ${targetVersion} not found`);

    const meta = JSON.parse(fs.readFileSync(vPath, 'utf8'));
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

    fs.writeFileSync(path.join(versionDir, `v${newVersion}.json`), JSON.stringify(versionMeta, null, 2));

    const updated = {
      ...file,
      currentVersion: newVersion,
      totalVersions: newVersion,
      size: meta.size,
      hash: meta.hash,
      chunkCount: meta.chunkCount,
      chunks: meta.chunks,
      updatedAt: new Date().toISOString()
    };

    this.storage.upsertEntity('files', fileId, updated);
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
      chunkDirectory: CHUNKS_DIR
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
      status: file.status,
      createdAt: file.createdAt,
      updatedAt: file.updatedAt,
      graphLinked: (file.linkedGraphIds || []).length > 0
    };
  }

  _chunkBuffer(buffer) {
    const chunks = [];
    for (let i = 0; i < buffer.length; i += this.chunkSize) {
      chunks.push(buffer.slice(i, i + this.chunkSize));
    }
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

  _guessMime(ext) {
    const types = {
      '.pdf': 'application/pdf',
      '.doc': 'application/msword',
      '.docx': 'application/vnd.openxmlformats',
      '.xls': 'application/vnd.ms-excel',
      '.xlsx': 'application/vnd.openxmlformats',
      '.ppt': 'application/vnd.ms-powerpoint',
      '.pptx': 'application/vnd.openxmlformats',
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
      '.rar': 'application/x-rar',
      '.7z': 'application/x-7z'
    };
    return types[ext.toLowerCase()] || 'application/octet-stream';
  }
}

let _instance = null;

function getFileStore() {
  if (!_instance) _instance = new FileStore();
  return _instance;
}

module.exports = { FileStore, getFileStore, FILE_STORE_DIR, VERSIONS_DIR, CHUNKS_DIR };