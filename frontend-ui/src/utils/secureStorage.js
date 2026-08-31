/**
 * 安全存储工具 - secureStorage
 *
 * 功能：
 * 1. 对敏感数据进行 XOR 混淆 + Base64 编码后存储
 * 2. 支持存储项过期机制（token 过期时间）
 * 3. 内存缓存层（减少 localStorage 读取次数）
 * 4. 向后兼容：能读取旧的明文 localStorage key
 * 5. 提供 cookie 存储备选方案（SameSite=Lax）
 *
 * 注意：XOR 混淆不是真正的加密，仅提高直接读取 localStorage 的门槛。
 * 生产环境建议配合 HttpOnly Cookie + Secure + SameSite 方案使用。
 */

// ===== 配置常量 =====

// XOR 混淆密钥（偏移量），生产环境建议从环境变量读取
const XOR_KEY =
  (typeof import.meta !== 'undefined' &&
    import.meta.env &&
    import.meta.env.VITE_STORAGE_KEY) ||
  'mox_secure_storage_v1_default_key'

// 存储前缀，用于标识安全存储的数据
const SECURE_PREFIX = '__mox_sec__'

// 旧版明文 localStorage key 列表（用于向后兼容读取）
const LEGACY_TOKEN_KEYS = ['mox-token', 'ous_api_token', 'ous_token']

// ===== 内存缓存层 =====

const memoryCache = new Map()

// ===== 核心工具函数 =====

/**
 * XOR 混淆 + Base64 编码
 * @param {string} plain 明文
 * @returns {string} 混淆后的 Base64 字符串
 */
function _xorEncrypt(plain) {
  try {
    if (!plain) return ''
    const key = XOR_KEY
    let result = ''
    for (let i = 0; i < plain.length; i++) {
      result += String.fromCharCode(
        plain.charCodeAt(i) ^ key.charCodeAt(i % key.length)
      )
    }
    // 使用 btoa 前先转 UTF-8 字节序列，兼容中文
    const utf8Bytes = unescape(encodeURIComponent(result))
    return btoa(utf8Bytes)
  } catch (e) {
    console.warn('[secureStorage] 加密失败:', e)
    return ''
  }
}

/**
 * Base64 解码 + XOR 还原
 * @param {string} encoded Base64 字符串
 * @returns {string} 明文
 */
function _xorDecrypt(encoded) {
  try {
    if (!encoded) return ''
    const utf8Bytes = atob(encoded)
    const result = decodeURIComponent(escape(utf8Bytes))
    const key = XOR_KEY
    let plain = ''
    for (let i = 0; i < result.length; i++) {
      plain += String.fromCharCode(
        result.charCodeAt(i) ^ key.charCodeAt(i % key.length)
      )
    }
    return plain
  } catch (e) {
    console.warn('[secureStorage] 解密失败，数据可能已损坏:', e)
    return ''
  }
}

/**
 * 打包存储数据（含过期时间和签名标记）
 * @param {string} value 存储值
 * @param {number} ttl 过期时间（秒），0 表示不过期
 * @returns {string} 打包后的 JSON 字符串
 */
function _packValue(value, ttl = 0) {
  const payload = {
    v: value,           // value
    t: Date.now(),      // timestamp
    e: ttl > 0 ? Date.now() + ttl * 1000 : 0  // expireAt, 0 = 永不过期
  }
  return JSON.stringify(payload)
}

/**
 * 解包存储数据，校验过期
 * @param {string} packed JSON 字符串
 * @returns {{ value: string, expired: boolean, valid: boolean }}
 */
function _unpackValue(packed) {
  try {
    const payload = JSON.parse(packed)
    if (!payload || typeof payload !== 'object' || !('v' in payload)) {
      return { value: '', expired: false, valid: false }
    }
    const expired = payload.e > 0 && Date.now() > payload.e
    return { value: payload.v, expired, valid: true }
  } catch {
    return { value: '', expired: false, valid: false }
  }
}

// ===== 主接口 =====

/**
 * 安全存储：写入数据
 * @param {string} key 存储键名
 * @param {string} value 存储值（明文）
 * @param {object} options 配置项
 * @param {number} options.ttl 过期时间（秒），0 表示不过期，默认 0
 * @param {boolean} options.useCookie 是否同时写入 cookie（用于跨标签页同步等场景）
 * @param {number} options.cookieDays cookie 有效期（天），默认会话级
 */
export function secureSetItem(key, value, options = {}) {
  try {
    const { ttl = 0, useCookie = false, cookieDays = 0 } = options

    // 打包 + 加密
    const packed = _packValue(String(value), ttl)
    const encrypted = _xorEncrypt(packed)
    const storageKey = SECURE_PREFIX + key

    // 写入 localStorage
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(storageKey, encrypted)
    }

    // 写入内存缓存
    memoryCache.set(key, {
      value: String(value),
      expireAt: ttl > 0 ? Date.now() + ttl * 1000 : 0
    })

    // 可选：写入 cookie
    if (useCookie && typeof document !== 'undefined') {
      _setCookie(key, String(value), cookieDays)
    }

    return true
  } catch (e) {
    console.error('[secureStorage] setItem 失败:', e)
    return false
  }
}

/**
 * 安全存储：读取数据
 * @param {string} key 存储键名
 * @param {object} options 配置项
 * @param {boolean} options.tryLegacy 是否尝试读取旧版明文 key（向后兼容）
 * @param {boolean} options.tryCookie 是否尝试从 cookie 读取
 * @returns {string} 存储值，不存在或过期返回空字符串
 */
export function secureGetItem(key, options = {}) {
  const { tryLegacy = true, tryCookie = false } = options

  // 1. 优先从内存缓存读取
  const cached = memoryCache.get(key)
  if (cached) {
    if (cached.expireAt > 0 && Date.now() > cached.expireAt) {
      memoryCache.delete(key)
    } else {
      return cached.value
    }
  }

  const storageKey = SECURE_PREFIX + key

  // 2. 从安全存储读取
  if (typeof localStorage !== 'undefined') {
    const raw = localStorage.getItem(storageKey)
    if (raw) {
      const decrypted = _xorDecrypt(raw)
      const { value, expired, valid } = _unpackValue(decrypted)
      if (valid && !expired) {
        // 回填内存缓存
        memoryCache.set(key, {
          value,
          expireAt: 0
        })
        return value
      }
      if (expired) {
        // 已过期，清除
        localStorage.removeItem(storageKey)
      }
    }
  }

  // 3. 向后兼容：尝试读取旧版明文 key
  if (tryLegacy && typeof localStorage !== 'undefined') {
    // 检查 key 是否在旧版 key 列表中
    if (LEGACY_TOKEN_KEYS.includes(key)) {
      for (const legacyKey of LEGACY_TOKEN_KEYS) {
        const legacyValue = localStorage.getItem(legacyKey)
        if (legacyValue) {
          // 迁移到安全存储
          try {
            secureSetItem(key, legacyValue)
          } catch {}
          // 不删除旧 key，保持完全向后兼容
          return legacyValue
        }
      }
    } else {
      // 非 token 类 key，也尝试直接读旧值
      const legacyValue = localStorage.getItem(key)
      if (legacyValue) {
        return legacyValue
      }
    }
  }

  // 4. 可选：从 cookie 读取
  if (tryCookie && typeof document !== 'undefined') {
    const cookieValue = _getCookie(key)
    if (cookieValue) {
      return cookieValue
    }
  }

  return ''
}

/**
 * 安全存储：删除数据
 * @param {string} key 存储键名
 * @param {object} options 配置项
 * @param {boolean} options.clearCookie 是否同时清除 cookie
 * @param {boolean} options.clearLegacy 是否同时清除旧版明文 key
 */
export function secureRemoveItem(key, options = {}) {
  const { clearCookie = false, clearLegacy = true } = options

  try {
    const storageKey = SECURE_PREFIX + key

    // 清除内存缓存
    memoryCache.delete(key)

    // 清除安全存储
    if (typeof localStorage !== 'undefined') {
      localStorage.removeItem(storageKey)
    }

    // 清除旧版明文 key
    if (clearLegacy && typeof localStorage !== 'undefined') {
      if (LEGACY_TOKEN_KEYS.includes(key)) {
        for (const legacyKey of LEGACY_TOKEN_KEYS) {
          localStorage.removeItem(legacyKey)
        }
      } else {
        localStorage.removeItem(key)
      }
    }

    // 清除 cookie
    if (clearCookie && typeof document !== 'undefined') {
      _removeCookie(key)
    }

    return true
  } catch (e) {
    console.error('[secureStorage] removeItem 失败:', e)
    return false
  }
}

/**
 * 检查存储项是否已过期
 * @param {string} key 存储键名
 * @returns {boolean|null} 过期返回 true，未过期返回 false，不存在返回 null
 */
export function isExpired(key) {
  const storageKey = SECURE_PREFIX + key

  // 检查内存缓存
  const cached = memoryCache.get(key)
  if (cached) {
    if (cached.expireAt > 0 && Date.now() > cached.expireAt) {
      return true
    }
    return false
  }

  // 检查 localStorage
  if (typeof localStorage !== 'undefined') {
    const raw = localStorage.getItem(storageKey)
    if (raw) {
      const decrypted = _xorDecrypt(raw)
      const { expired, valid } = _unpackValue(decrypted)
      if (valid) return expired
    }
  }

  return null
}

/**
 * 获取存储项剩余有效时间（毫秒）
 * @param {string} key 存储键名
 * @returns {number} 剩余毫秒数，-1 表示永久有效，-2 表示不存在或已过期
 */
export function getRemainingTime(key) {
  const storageKey = SECURE_PREFIX + key

  // 检查内存缓存
  const cached = memoryCache.get(key)
  if (cached) {
    if (cached.expireAt === 0) return -1
    const remaining = cached.expireAt - Date.now()
    return remaining > 0 ? remaining : -2
  }

  // 检查 localStorage
  if (typeof localStorage !== 'undefined') {
    const raw = localStorage.getItem(storageKey)
    if (raw) {
      const decrypted = _xorDecrypt(raw)
      const { value, expired, valid } = _unpackValue(decrypted)
      if (!valid || expired) return -2
      const payload = JSON.parse(_xorDecrypt(raw))
      if (payload.e === 0) return -1
      const remaining = payload.e - Date.now()
      return remaining > 0 ? remaining : -2
    }
  }

  return -2
}

/**
 * 清空所有安全存储项（仅清除带安全前缀的）
 */
export function secureClear() {
  try {
    if (typeof localStorage === 'undefined') return

    const keysToRemove = []
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i)
      if (key && key.startsWith(SECURE_PREFIX)) {
        keysToRemove.push(key)
      }
    }
    keysToRemove.forEach(k => localStorage.removeItem(k))
    memoryCache.clear()
  } catch (e) {
    console.error('[secureStorage] clear 失败:', e)
  }
}

// ===== Cookie 备选方案 =====

/**
 * 设置 Cookie（SameSite=Lax，Secure 自动适配 https）
 *
 * 说明：
 * - HttpOnly Cookie 无法通过前端 JS 设置，必须由服务端下发
 * - 前端 Cookie 方案可用于非敏感数据的跨标签页同步
 * - 生产环境 Token 强烈建议使用 HttpOnly + Secure + SameSite=Strict Cookie
 *
 * @param {string} name cookie 名称
 * @param {string} value cookie 值
 * @param {number} days 有效期（天），0 表示会话级
 */
function _setCookie(name, value, days = 0) {
  try {
    let cookieStr = `${encodeURIComponent(name)}=${encodeURIComponent(value)}; path=/; SameSite=Lax`

    // HTTPS 环境下自动添加 Secure
    if (typeof window !== 'undefined' && window.location.protocol === 'https:') {
      cookieStr += '; Secure'
    }

    if (days > 0) {
      const expires = new Date()
      expires.setTime(expires.getTime() + days * 24 * 60 * 60 * 1000)
      cookieStr += `; expires=${expires.toUTCString()}`
    }

    document.cookie = cookieStr
  } catch (e) {
    console.warn('[secureStorage] setCookie 失败:', e)
  }
}

/**
 * 读取 Cookie
 * @param {string} name cookie 名称
 * @returns {string}
 */
function _getCookie(name) {
  try {
    const nameEQ = encodeURIComponent(name) + '='
    const ca = document.cookie.split(';')
    for (let i = 0; i < ca.length; i++) {
      let c = ca[i].trim()
      if (c.indexOf(nameEQ) === 0) {
        return decodeURIComponent(c.substring(nameEQ.length))
      }
    }
  } catch (e) {
    console.warn('[secureStorage] getCookie 失败:', e)
  }
  return ''
}

/**
 * 删除 Cookie
 * @param {string} name cookie 名称
 */
function _removeCookie(name) {
  try {
    document.cookie = `${encodeURIComponent(name)}=; path=/; expires=Thu, 01 Jan 1970 00:00:00 GMT; SameSite=Lax`
  } catch (e) {
    console.warn('[secureStorage] removeCookie 失败:', e)
  }
}

// ===== 便捷方法：Token 专用 =====

// 统一的 token 存储 key
const TOKEN_KEY = 'mox-token'

/**
 * 存储 Token（带过期时间）
 * @param {string} token token 值
 * @param {number} expiresIn 有效期（秒），默认 24 小时
 */
export function setToken(token, expiresIn = 86400) {
  return secureSetItem(TOKEN_KEY, token, { ttl: expiresIn })
}

/**
 * 读取 Token（自动兼容旧版 key）
 * @returns {string}
 */
export function getToken() {
  return secureGetItem(TOKEN_KEY, { tryLegacy: true })
}

/**
 * 清除 Token
 */
export function removeToken() {
  return secureRemoveItem(TOKEN_KEY, { clearLegacy: true })
}

/**
 * 检查 Token 是否有效（存在且未过期）
 * @returns {boolean}
 */
export function hasValidToken() {
  const token = getToken()
  if (!token) return false
  const expired = isExpired(TOKEN_KEY)
  // 如果无法判断过期状态（旧版 token），认为有效
  return expired !== true
}

// ===== 导出默认对象 =====

export default {
  setItem: secureSetItem,
  getItem: secureGetItem,
  removeItem: secureRemoveItem,
  isExpired,
  getRemainingTime,
  clear: secureClear,
  setToken,
  getToken,
  removeToken,
  hasValidToken,
}
