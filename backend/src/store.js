'use strict'
/**
 * 轻量级持久化存储：内存 Map + JSON 文件落盘。
 * 每个 collection 对应 backend/data/<name>.json。
 * 无第三方依赖，进程重启后自动从磁盘恢复。
 */
const fs = require('fs')
const path = require('path')

// 数据目录可被 OUS_DATA_DIR 覆盖（测试隔离用），默认 backend/data
const DATA_DIR = path.resolve(process.env.OUS_DATA_DIR || path.join(__dirname, '..', 'data'))

function ensureDir() {
  if (!fs.existsSync(DATA_DIR)) fs.mkdirSync(DATA_DIR, { recursive: true })
}

function genId(prefix) {
  return (
    (prefix || 'id') +
    '_' +
    Date.now().toString(36) +
    Math.random().toString(36).slice(2, 7)
  )
}

class Store {
  constructor() {
    this.collections = {} // name -> Map<id, obj>
    this.files = {} // name -> filePath
    ensureDir()
  }

  _file(name) {
    return path.join(DATA_DIR, name + '.json')
  }

  load(name, defaults = []) {
    const f = this._file(name)
    try {
      if (fs.existsSync(f)) {
        const arr = JSON.parse(fs.readFileSync(f, 'utf8'))
        this.collections[name] = new Map(arr.map((x) => [x.id, x]))
      } else {
        this.collections[name] = new Map()
        if (defaults.length) {
          defaults.forEach((x) => this.collections[name].set(x.id, x))
          this.persist(name)
        }
      }
    } catch (e) {
      this.collections[name] = new Map()
    }
    this.files[name] = f
    return this
  }

  ensure(name) {
    if (!this.collections[name]) this.load(name)
    return this
  }

  all(name) {
    this.ensure(name)
    return Array.from(this.collections[name].values())
  }

  get(name, id) {
    this.ensure(name)
    return this.collections[name].get(id)
  }

  find(name, pred) {
    return this.all(name).filter(pred)
  }

  insert(name, obj) {
    this.ensure(name)
    const col = this.collections[name]
    if (!obj.id) obj.id = genId(name)
    if (!obj.created_at) obj.created_at = new Date().toISOString()
    col.set(obj.id, obj)
    this.persist(name)
    return obj
  }

  update(name, id, patch) {
    this.ensure(name)
    const col = this.collections[name]
    if (!col || !col.has(id)) return null
    const merged = Object.assign({}, col.get(id), patch, { id, updated_at: new Date().toISOString() })
    col.set(id, merged)
    this.persist(name)
    return merged
  }

  remove(name, id) {
    this.ensure(name)
    const col = this.collections[name]
    if (!col || !col.has(id)) return false
    col.delete(id)
    this.persist(name)
    return true
  }

  // 幂等合并：用于导入迁移包
  upsert(name, obj) {
    this.ensure(name)
    const col = this.collections[name]
    if (col.has(obj.id)) {
      col.set(obj.id, Object.assign({}, col.get(obj.id), obj, { id: obj.id }))
    } else {
      col.set(obj.id, obj)
    }
    this.persist(name)
    return col.get(obj.id)
  }

  persist(name) {
    const col = this.collections[name]
    if (!col) return
    const arr = Array.from(col.values())
    try {
      fs.writeFileSync(this.files[name], JSON.stringify(arr, null, 2))
    } catch (e) {
      // 落盘失败不阻塞内存操作，仅记录
      console.error('[store] persist failed for', name, e.message)
    }
  }

  persistAll() {
    for (const n of Object.keys(this.collections)) this.persist(n)
  }
}

module.exports = { Store, genId, DATA_DIR, ensureDir }
