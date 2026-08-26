'use strict';

/**
 * MOX Enterprise · 数据加密工具
 * ==============================
 * 静态加密（At-Rest）+ 传输加密（In-Transit）工具集
 *
 * 算法选择：
 *  - 对称加密：AES-256-GCM（认证加密，防篡改）
 *  - 非对称加密：RSA-4096 / ECDSA P-384（密钥交换/签名）
 *  - 哈希：SHA-256（内容寻址）/ SHA-512（高安全场景）
 *  - 密钥派生：PBKDF2（100万次迭代）/ Argon2id
 *  - HMAC：HMAC-SHA256（消息认证）
 *
 * 密钥管理：
 *  - 生产环境应接入 KMS（AWS KMS / 阿里云 KMS / HashiCorp Vault）
 *  - 本模块提供本地密钥环 + KMS 接口抽象
 */

const crypto = require('crypto');

// ─── 常量 ───
const ALGORITHMS = {
  AES_256_GCM: 'aes-256-gcm',
  AES_256_CBC: 'aes-256-cbc',
  RSA_OAEP: 'rsa-oaep-sha256',
  ECDH_P384: 'secp384r1',
};

const KEY_SIZES = {
  AES_256: 32,  // 256 bits
  IV_GCM: 12,   // 96 bits (GCM 推荐)
  IV_CBC: 16,   // 128 bits
  AUTH_TAG: 16, // 128 bits
  SALT: 16,
  PBKDF2_ITERATIONS: 1000000, // 100 万次迭代
};

// ─── 密钥环（内存中，生产环境应从 KMS 加载） ───
class KeyRing {
  constructor() {
    this.keys = new Map(); // keyId -> { key: Buffer, algorithm, createdAt, expiresAt }
    this.activeKeyId = null;
  }

  /**
   * 添加密钥
   */
  addKey(keyId, key, algorithm = ALGORITHMS.AES_256_GCM, options = {}) {
    if (!Buffer.isBuffer(key)) key = Buffer.from(key, 'hex');
    this.keys.set(keyId, {
      key,
      algorithm,
      createdAt: new Date(),
      expiresAt: options.expiresAt || null,
      metadata: options.metadata || {},
    });
    if (!this.activeKeyId || options.setActive) {
      this.activeKeyId = keyId;
    }
    return this;
  }

  /**
   * 获取密钥
   */
  getKey(keyId) {
    const entry = this.keys.get(keyId);
    if (!entry) throw new Error(`密钥不存在: ${keyId}`);
    if (entry.expiresAt && new Date(entry.expiresAt) < new Date()) {
      throw new Error(`密钥已过期: ${keyId}`);
    }
    return entry;
  }

  /**
   * 获取当前活跃密钥
   */
  getActiveKey() {
    if (!this.activeKeyId) throw new Error('未设置活跃密钥');
    return { keyId: this.activeKeyId, ...this.getKey(this.activeKeyId) };
  }

  /**
   * 轮换密钥（旧密钥保留用于解密，新密钥用于加密）
   */
  rotateKey(newKeyId, newKey) {
    this.addKey(newKeyId, newKey, ALGORITHMS.AES_256_GCM, { setActive: true });
    return this;
  }

  /**
   * 列出所有密钥 ID
   */
  listKeys() {
    return Array.from(this.keys.entries()).map(([keyId, entry]) => ({
      keyId,
      algorithm: entry.algorithm,
      createdAt: entry.createdAt,
      expiresAt: entry.expiresAt,
      isActive: keyId === this.activeKeyId,
    }));
  }
}

// ─── 全局密钥环单例 ───
const globalKeyRing = new KeyRing();

/**
 * 从环境变量初始化密钥环
 */
function initKeyRingFromEnv() {
  const masterKey = process.env.MOX_MASTER_KEY;
  if (masterKey) {
    globalKeyRing.addKey('master-001', masterKey, ALGORITHMS.AES_256_GCM, { setActive: true });
  }
  // 支持多个密钥（用于轮换）
  for (let i = 1; i <= 5; i++) {
    const key = process.env[`MOX_ENCRYPTION_KEY_${i}`];
    if (key) {
      globalKeyRing.addKey(`key-${String(i).padStart(3, '0')}`, key);
    }
  }
  return globalKeyRing;
}

// ─── AES-256-GCM 加密/解密 ───

/**
 * 加密数据（AES-256-GCM）
 * 返回格式：{ keyId, iv, authTag, ciphertext } 全部 hex 编码
 */
function encrypt(plaintext, options = {}) {
  const keyRing = options.keyRing || globalKeyRing;
  const { keyId, key, algorithm } = keyRing.getActiveKey();

  if (!Buffer.isBuffer(plaintext)) {
    plaintext = Buffer.from(String(plaintext), 'utf8');
  }

  const iv = crypto.randomBytes(KEY_SIZES.IV_GCM);
  const cipher = crypto.createCipheriv(algorithm, key, iv);

  let ciphertext = cipher.update(plaintext);
  ciphertext = Buffer.concat([ciphertext, cipher.final()]);
  const authTag = cipher.getAuthTag();

  return {
    keyId,
    algorithm,
    iv: iv.toString('hex'),
    authTag: authTag.toString('hex'),
    ciphertext: ciphertext.toString('hex'),
    // 紧凑格式：keyId:iv:authTag:ciphertext（便于存储）
    compact: `${keyId}:${iv.toString('hex')}:${authTag.toString('hex')}:${ciphertext.toString('hex')}`,
  };
}

/**
 * 解密数据（AES-256-GCM）
 * 支持 compact 格式或独立字段
 */
function decrypt(encrypted, options = {}) {
  const keyRing = options.keyRing || globalKeyRing;

  let keyId, iv, authTag, ciphertext;

  if (typeof encrypted === 'string') {
    // compact 格式
    const parts = encrypted.split(':');
    if (parts.length !== 4) throw new Error('无效的加密数据格式');
    [keyId, iv, authTag, ciphertext] = parts;
  } else {
    ({ keyId, iv, authTag, ciphertext } = encrypted);
  }

  const { key, algorithm } = keyRing.getKey(keyId);

  const decipher = crypto.createDecipheriv(
    algorithm,
    key,
    Buffer.from(iv, 'hex')
  );
  decipher.setAuthTag(Buffer.from(authTag, 'hex'));

  let plaintext = decipher.update(Buffer.from(ciphertext, 'hex'));
  plaintext = Buffer.concat([plaintext, decipher.final()]);

  return options.encoding === 'utf8' ? plaintext.toString('utf8') : plaintext;
}

// ─── 流式加密（用于大文件 / chunk） ───

/**
 * 创建加密流（AES-256-GCM）
 * 用于流式处理大文件，不需要一次性加载到内存
 */
function createEncryptStream(options = {}) {
  const keyRing = options.keyRing || globalKeyRing;
  const { keyId, key, algorithm } = keyRing.getActiveKey();
  const iv = crypto.randomBytes(KEY_SIZES.IV_GCM);
  const cipher = crypto.createCipheriv(algorithm, key, iv);

  // 在流开头写入 header（keyId + iv），便于解密时读取
  const header = Buffer.from(JSON.stringify({ keyId, iv: iv.toString('hex') }) + '\n');

  return {
    keyId,
    iv,
    cipher,
    header,
    // 包装成 Transform 流的使用方式
    getStream() {
      const { Transform } = require('stream');
      let headerSent = false;
      return new Transform({
        transform(chunk, encoding, callback) {
          if (!headerSent) {
            this.push(header);
            headerSent = true;
          }
          this.push(cipher.update(chunk));
          callback();
        },
        flush(callback) {
          this.push(cipher.final());
          this.push(cipher.getAuthTag());
          callback();
        },
      });
    },
  };
}

// ─── 哈希函数 ───

/**
 * SHA-256 哈希（内容寻址用）
 */
function sha256(data) {
  if (!Buffer.isBuffer(data)) data = Buffer.from(String(data));
  return crypto.createHash('sha256').update(data).digest('hex');
}

/**
 * SHA-512 哈希（高安全场景）
 */
function sha512(data) {
  if (!Buffer.isBuffer(data)) data = Buffer.from(String(data));
  return crypto.createHash('sha512').update(data).digest('hex');
}

/**
 * 流式哈希（大文件）
 */
function createHashStream(algorithm = 'sha256') {
  return crypto.createHash(algorithm);
}

// ─── HMAC 消息认证 ───

/**
 * HMAC-SHA256 签名
 */
function hmacSign(data, secret) {
  if (!Buffer.isBuffer(data)) data = Buffer.from(String(data));
  return crypto.createHmac('sha256', secret).update(data).digest('hex');
}

/**
 * HMAC 验证（常量时间比较，防时序攻击）
 */
function hmacVerify(data, secret, signature) {
  const expected = hmacSign(data, secret);
  return crypto.timingSafeEqual(
    Buffer.from(expected, 'hex'),
    Buffer.from(signature, 'hex')
  );
}

// ─── 密钥派生（PBKDF2） ───

/**
 * 从密码派生密钥（PBKDF2-SHA256，100万次迭代）
 */
function deriveKey(password, salt, iterations = KEY_SIZES.PBKDF2_ITERATIONS, keyLength = 32) {
  if (!Buffer.isBuffer(salt)) salt = Buffer.from(salt, 'hex');
  return crypto.pbkdf2Sync(password, salt, iterations, keyLength, 'sha256');
}

/**
 * 生成随机 salt
 */
function generateSalt(size = KEY_SIZES.SALT) {
  return crypto.randomBytes(size).toString('hex');
}

// ─── RSA 非对称加密 ───

/**
 * 生成 RSA 密钥对（4096 位）
 */
function generateRSAKeyPair() {
  return crypto.generateKeyPairSync('rsa', {
    modulusLength: 4096,
    publicKeyEncoding: { type: 'spki', format: 'pem' },
    privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
  });
}

/**
 * RSA-OAEP 加密（用于加密对称密钥，不用于大数据）
 */
function rsaEncrypt(publicKey, data) {
  if (!Buffer.isBuffer(data)) data = Buffer.from(String(data));
  return crypto.publicEncrypt(
    { key: publicKey, padding: crypto.constants.RSA_PKCS1_OAEP_PADDING, oaepHash: 'sha256' },
    data
  ).toString('base64');
}

/**
 * RSA-OAEP 解密
 */
function rsaDecrypt(privateKey, encrypted) {
  return crypto.privateDecrypt(
    { key: privateKey, padding: crypto.constants.RSA_PKCS1_OAEP_PADDING, oaepHash: 'sha256' },
    Buffer.from(encrypted, 'base64')
  );
}

// ─── ECDSA 签名/验签 ───

/**
 * 生成 ECDSA 密钥对（P-384）
 */
function generateECDSAKeyPair() {
  return crypto.generateKeyPairSync('ec', {
    namedCurve: 'secp384r1',
    publicKeyEncoding: { type: 'spki', format: 'pem' },
    privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
  });
}

/**
 * ECDSA 签名
 */
function ecdsaSign(privateKey, data) {
  if (!Buffer.isBuffer(data)) data = Buffer.from(String(data));
  const sign = crypto.createSign('SHA384');
  sign.update(data);
  return sign.sign(privateKey, 'hex');
}

/**
 * ECDSA 验签
 */
function ecdsaVerify(publicKey, data, signature) {
  if (!Buffer.isBuffer(data)) data = Buffer.from(String(data));
  const verify = crypto.createVerify('SHA384');
  verify.update(data);
  return verify.verify(publicKey, signature, 'hex');
}

// ─── 安全随机数 ───

/**
 * 生成安全随机令牌
 */
function generateToken(bytes = 32) {
  return crypto.randomBytes(bytes).toString('hex');
}

/**
 * 生成 API Key（带前缀，便于识别）
 */
function generateApiKey(prefix = 'mox') {
  return `${prefix}_${crypto.randomBytes(24).toString('base64url')}`;
}

// ─── 常量时间字符串比较（防时序攻击） ───
function safeEqual(a, b) {
  const bufA = Buffer.from(String(a));
  const bufB = Buffer.from(String(b));
  if (bufA.length !== bufB.length) return false;
  return crypto.timingSafeEqual(bufA, bufB);
}

module.exports = {
  ALGORITHMS,
  KEY_SIZES,
  KeyRing,
  globalKeyRing,
  initKeyRingFromEnv,
  encrypt,
  decrypt,
  createEncryptStream,
  sha256,
  sha512,
  createHashStream,
  hmacSign,
  hmacVerify,
  deriveKey,
  generateSalt,
  generateRSAKeyPair,
  rsaEncrypt,
  rsaDecrypt,
  generateECDSAKeyPair,
  ecdsaSign,
  ecdsaVerify,
  generateToken,
  generateApiKey,
  safeEqual,
};
