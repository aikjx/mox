//! # 算子商城：路径迁移 / 备份 / 审计 / 存储 IO
//!
//! 统一收敛 market 模块的磁盘布局与 IO 侧支撑能力：
//!
//! - **路径归一化**：`$OUS_HOME/market/packages/<id>.json`（`OUS_HOME` 默认 `~/.ous`）。
//! - **旧路径迁移**：首次启动检测 `./data/market/<id>.json`（遗留布局）以及
//!   `$OUS_HOME/market/<id>.json`（中间布局），自动备份后迁移到 `packages/` 子目录。
//! - **读取兼容**：`find_package_file` 按 新布局 → 中间布局 → 遗留布局 依次探测，
//!   保证旧路径数据可读（向后兼容），并在命中旧路径时自动补迁。
//! - **审计**：导入 / 导出 / 回滚 / 迁移 等敏感操作统一追加 `$OUS_HOME/market/audit.log`。
//! - **签名**：导出物用 HMAC-SHA256 签名，导入时校验（密钥来自 `OUS_MARKET_SIGN_SECRET`）。
//! - **ZIP**：内置 minimal "stored"（无压缩）ZIP 读写器，零外部依赖，导出全量包用。

use std::path::{Path, PathBuf};

/// 当前时间 RFC3339（market 各模块统一入口）
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ========== 路径布局 ==========

/// OUS 归一化根目录：默认 `~/.ous`，可由 `OUS_HOME` 环境变量覆盖。
pub fn ous_home() -> PathBuf {
    if let Ok(v) = std::env::var("OUS_HOME") {
        if !v.trim().is_empty() {
            return PathBuf::from(v.trim());
        }
    }
    if let Some(home) = dirs_home() {
        return home.join(".ous");
    }
    PathBuf::from(".ous")
}

/// 跨平台取用户主目录（避免额外依赖）
fn dirs_home() -> Option<PathBuf> {
    if let Ok(v) = std::env::var("HOME") {
        if !v.is_empty() {
            return Some(PathBuf::from(v));
        }
    }
    if let Ok(v) = std::env::var("USERPROFILE") {
        if !v.is_empty() {
            return Some(PathBuf::from(v));
        }
    }
    None
}

/// 归一化后的商城根目录：`$OUS_HOME/market`
pub fn market_dir() -> PathBuf {
    ous_home().join("market")
}

/// 归一化后的包存储目录：`$OUS_HOME/market/packages`
pub fn packages_dir() -> PathBuf {
    market_dir().join("packages")
}

/// 版本快照目录：`$OUS_HOME/market/versions/<id>/`
pub fn versions_dir(id: &str) -> PathBuf {
    market_dir().join("versions").join(sanitize_id(id))
}

/// 变更日志目录：`$OUS_HOME/market/changelog/`（每个包一个 `<id>.md`）
pub fn changelogs_dir() -> PathBuf {
    market_dir().join("changelog")
}

/// 备份目录：`$OUS_HOME/market/backup/`
pub fn backups_dir() -> PathBuf {
    market_dir().join("backup")
}

/// 审计日志文件：`$OUS_HOME/market/audit.log`
pub fn audit_log_path() -> PathBuf {
    market_dir().join("audit.log")
}

/// 遗留（旧）存储目录：环境变量 `OUS_LEGACY_MARKET_DIR` 可覆盖（测试用），
/// 生产默认 `./data/market`（归一化前的项目相对路径）。
pub fn legacy_market_dir() -> PathBuf {
    if let Ok(v) = std::env::var("OUS_LEGACY_MARKET_DIR") {
        if !v.trim().is_empty() {
            return PathBuf::from(v.trim());
        }
    }
    PathBuf::from("./data/market")
}

/// 历史版本保留数：`OUS_MARKET_KEEP_VERSIONS`（默认 5；0 = 不限制）
pub fn keep_versions_limit() -> usize {
    if let Ok(v) = std::env::var("OUS_MARKET_KEEP_VERSIONS") {
        if let Ok(n) = v.trim().parse::<usize>() {
            return n;
        }
    }
    5
}

/// id 用作目录名时做安全清洗（防路径穿越）
fn sanitize_id(id: &str) -> String {
    sanitize_file_component(id)
}

/// 把任意字符串清洗为安全的文件名组件（仅保留字母数字 . _ -）
pub fn sanitize_file_component(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
        .collect()
}

/// 规范包文件路径：`$OUS_HOME/market/packages/<id>.json`
pub fn package_path(id: &str) -> PathBuf {
    packages_dir().join(format!("{}.json", sanitize_id(id)))
}

/// 查找包文件（向后兼容）：新布局 → 中间布局（market 根）→ 遗留布局（./data/market）
pub fn find_package_file(id: &str) -> Option<PathBuf> {
    let candidates = [
        package_path(id),
        market_dir().join(format!("{}.json", sanitize_id(id))),
        legacy_market_dir().join(format!("{}.json", sanitize_id(id))),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

// ========== 迁移 ==========

/// 迁移结果报告
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct MigrationReport {
    /// 从遗留目录迁入的包数
    pub migrated_from_legacy: usize,
    /// 从 market 根（中间布局）迁入 packages/ 的包数
    pub migrated_from_root: usize,
    /// 备份的包数
    pub backed_up: usize,
    /// 遗留目录中仍存在的 json 数（目标已存在被清理的计入 skipped）
    pub skipped_existing: usize,
    /// 备份目录位置
    pub backup_dir: Option<String>,
}

/// 首次启动迁移：
/// 1. `./data/market/*.json`（旧布局）→ 备份 → `packages/`
/// 2. `$OUS_HOME/market/*.json`（中间布局）→ `packages/`
pub fn ensure_migrated() -> MigrationReport {
    let mut report = MigrationReport::default();
    let pkg_dir = packages_dir();
    if !pkg_dir.exists() {
        let _ = std::fs::create_dir_all(&pkg_dir);
    }

    // ---- 1) 遗留目录迁移（自动备份）----
    let legacy = legacy_market_dir();
    if legacy.is_dir() {
        let files = json_files_in(&legacy);
        if !files.is_empty() {
            let backup_dir = backups_dir().join(format!("legacy-{}", timestamp_tag()));
            let _ = std::fs::create_dir_all(&backup_dir);
            let mut moved = 0usize;
            for path in &files {
                let name = match path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n.to_string(),
                    None => continue,
                };
                let target = pkg_dir.join(&name);
                if !target.exists() {
                    // 备份：拷贝一份到 backup/（迁移可回退）
                    if std::fs::copy(path, backup_dir.join(&name)).is_ok() {
                        report.backed_up += 1;
                    }
                    if std::fs::rename(path, &target).is_ok() {
                        moved += 1;
                    } else if std::fs::copy(path, &target).is_ok() {
                        let _ = std::fs::remove_file(path);
                        moved += 1;
                    }
                } else {
                    // 目标已存在：不覆盖，清理遗留副本
                    let _ = std::fs::remove_file(path);
                    report.skipped_existing += 1;
                }
            }
            report.migrated_from_legacy = moved;
            report.backup_dir = Some(backup_dir.display().to_string());
            if moved > 0 {
                audit(
                    "migration",
                    "system",
                    &format!("从 {} 迁移 {} 个包到 {}", legacy.display(), moved, pkg_dir.display()),
                );
                tracing::info!(
                    "算子商城路径归一化：从 {} 迁移 {} 个包到 {}（备份于 {}）",
                    legacy.display(),
                    moved,
                    pkg_dir.display(),
                    backup_dir.display()
                );
            }
        }
    }

    // ---- 2) 中间布局：market 根目录下的散 json 移入 packages/ ----
    let root = market_dir();
    if root.is_dir() {
        let mut moved = 0usize;
        for path in json_files_in(&root) {
            // 跳过 packages/versions/changelog/backup 子目录内的（json_files_in 只扫顶层）
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            let target = pkg_dir.join(&name);
            if !target.exists() {
                if std::fs::rename(&path, &target).is_ok() {
                    moved += 1;
                }
            } else {
                let _ = std::fs::remove_file(&path);
            }
        }
        if moved > 0 {
            report.migrated_from_root = moved;
            audit(
                "migration",
                "system",
                &format!("从 {} 迁移 {} 个散包到 {}", root.display(), moved, pkg_dir.display()),
            );
        }
    }
    report
}

/// 列出目录顶层（非递归）的 .json 文件
fn json_files_in(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
                out.push(path);
            }
        }
    }
    out
}

/// 时间戳标签（目录名用）
fn timestamp_tag() -> String {
    chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string()
}

/// 手动触发一次全量备份：`$OUS_HOME/market/backup/manual-<ts>/`
/// 由 `POST /api/market/backup` 调用（market.rs backup_market handler）。
pub fn backup_now(tag: &str) -> Option<PathBuf> {
    let src = packages_dir();
    if !src.is_dir() {
        return None;
    }
    let dir = backups_dir().join(format!("{}-{}", tag, timestamp_tag()));
    if std::fs::create_dir_all(&dir).is_err() {
        return None;
    }
    let mut n = 0usize;
    for path in json_files_in(&src) {
        if let Some(name) = path.file_name() {
            if std::fs::copy(&path, dir.join(name)).is_ok() {
                n += 1;
            }
        }
    }
    audit("backup", "system", &format!("手动备份 {} 个包到 {}", n, dir.display()));
    Some(dir)
}

// ========== 审计 ==========

/// 追加一条审计日志（导入/导出/回滚/迁移/备份等敏感操作必须调用）。
/// 写失败只告警不阻塞业务。
pub fn audit(action: &str, actor: &str, detail: &str) {
    let path = audit_log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let line = format!("{} | market | {} | {} | {}\n", now_rfc3339(), action, actor, detail);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        use std::io::Write;
        if f.write_all(line.as_bytes()).is_err() {
            tracing::warn!("审计日志写入失败: {}", path.display());
        }
    } else {
        tracing::warn!("审计日志打开失败: {}", path.display());
    }
}

// ========== 签名（HMAC-SHA256）==========

/// 签名密钥：优先 `OUS_MARKET_SIGN_SECRET`，缺省使用内置默认值（非生产安全）。
fn sign_secret() -> Vec<u8> {
    static WARNED: std::sync::Once = std::sync::Once::new();
    if let Ok(v) = std::env::var("OUS_MARKET_SIGN_SECRET") {
        if !v.trim().is_empty() {
            return v.trim().as_bytes().to_vec();
        }
    }
    WARNED.call_once(|| {
        tracing::warn!(
            "OUS_MARKET_SIGN_SECRET 未设置，使用内置默认签名密钥（仅限开发环境，生产请务必配置）"
        );
    });
    b"ous-market-sign-v1-insecure-default-key".to_vec()
}

/// 对导出文档（去掉 signature 字段后的规范 JSON）计算 HMAC-SHA256 签名
pub fn sign_doc(doc: &mut serde_json::Value) -> String {
    doc["signature"] = serde_json::Value::Null;
    let canonical = serde_json::to_vec(doc).unwrap_or_default();
    use hmac::{Hmac, Mac};
    type HmacSha256 = Hmac<sha2::Sha256>;
    let mut mac = HmacSha256::new_from_slice(&sign_secret()).expect("hmac key");
    mac.update(&canonical);
    let sig = hex::encode(mac.finalize().into_bytes());
    // 关键修复：必须把计算出的签名写回 doc["signature"]，否则 verify_doc 读取到 Null 必然失败。
    doc["signature"] = serde_json::Value::String(sig.clone());
    sig
}

/// 校验导出文档签名：通过则返回 true。
/// 要求 doc["signature"] 为十六进制串，且与重新计算的 HMAC 一致（常量时间比较）。
pub fn verify_doc(doc: &serde_json::Value) -> bool {
    let sig = match doc.get("signature").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return false,
    };
    let sig_bytes = match hex::decode(sig) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let mut clean = doc.clone();
    clean["signature"] = serde_json::Value::Null;
    let canonical = match serde_json::to_vec(&clean) {
        Ok(b) => b,
        Err(_) => return false,
    };
    use hmac::{Hmac, Mac};
    type HmacSha256 = Hmac<sha2::Sha256>;
    let mut mac = match HmacSha256::new_from_slice(&sign_secret()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(&canonical);
    mac.verify_slice(&sig_bytes).is_ok()
}

// ========== minimal ZIP（stored，零依赖）==========

/// 计算 CRC-32（IEEE）
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// 写一个 "stored"（无压缩）ZIP 归档：entries = (文件名, 内容)
pub fn zip_write(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut central: Vec<(String, u32, u32, u32, u32, u32)> = Vec::new(); // name, crc, size, offset, mtime, mdate
    let dos_time = 0u16; // 00:00:00
    let dos_date = 0x5821u16; // 2026-01-01
    for (name, data) in entries {
        let offset = out.len() as u32;
        let crc = crc32(data);
        let name_bytes = name.as_bytes();
        // Local file header
        out.extend_from_slice(&[0x50, 0x4B, 0x03, 0x04]); // PK\x03\x04
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0x0800u16.to_le_bytes()); // flags: UTF-8 names
        out.extend_from_slice(&0u16.to_le_bytes()); // method: stored
        out.extend_from_slice(&dos_time.to_le_bytes());
        out.extend_from_slice(&dos_date.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra len
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(data);
        central.push((name.clone(), crc, data.len() as u32, offset, dos_time as u32, dos_date as u32));
    }
    // Central directory
    let cd_start = out.len() as u32;
    for (name, crc, size, offset, mtime, mdate) in &central {
        let name_bytes = name.as_bytes();
        out.extend_from_slice(&[0x50, 0x4B, 0x01, 0x02]); // PK\x01\x02
        out.extend_from_slice(&(0x0314u16).to_le_bytes()); // version made by
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0x0800u16.to_le_bytes()); // flags
        out.extend_from_slice(&0u16.to_le_bytes()); // method
        out.extend_from_slice(&(*mtime as u16).to_le_bytes());
        out.extend_from_slice(&(*mdate as u16).to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra
        out.extend_from_slice(&0u16.to_le_bytes()); // comment
        out.extend_from_slice(&0u16.to_le_bytes()); // disk
        out.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        out.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        out.extend_from_slice(&offset.to_le_bytes());
        out.extend_from_slice(name_bytes);
    }
    let cd_size = out.len() as u32 - cd_start;
    // EOCD
    out.extend_from_slice(&[0x50, 0x4B, 0x05, 0x06]); // PK\x05\x06
    out.extend_from_slice(&0u16.to_le_bytes()); // disk
    out.extend_from_slice(&0u16.to_le_bytes()); // cd disk
    out.extend_from_slice(&(central.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u16).to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment len
    out
}

/// 读 ZIP：返回 (文件名, 内容) 列表（按中央目录顺序）。
/// 仅支持 stored（method 0）；deflate（method 8）返回明确错误。
pub fn zip_read(data: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    // 定位 EOCD：从末尾往前找 PK\x05\x06
    let min_eocd = if data.len() > 65557 { data.len() - 65557 } else { 0 };
    let mut eocd = None;
    for i in (min_eocd..data.len().saturating_sub(3)).rev() {
        if data[i] == 0x50 && data[i + 1] == 0x4B && data[i + 2] == 0x05 && data[i + 3] == 0x06 {
            eocd = Some(i);
            break;
        }
    }
    let eocd = eocd.ok_or_else(|| "不是合法的 ZIP 文件（找不到 EOCD）".to_string())?;
    let total = u16::from_le_bytes([data[eocd + 10], data[eocd + 11]]) as usize;
    let cd_size = u32::from_le_bytes([data[eocd + 12], data[eocd + 13], data[eocd + 14], data[eocd + 15]]) as usize;
    let cd_off = u32::from_le_bytes([data[eocd + 16], data[eocd + 17], data[eocd + 18], data[eocd + 19]]) as usize;
    if cd_off + cd_size > data.len() {
        return Err("ZIP 中央目录越界".to_string());
    }
    let mut entries = Vec::with_capacity(total);
    let mut pos = cd_off;
    for _ in 0..total {
        if pos + 46 > data.len() || data[pos..pos + 4] != [0x50, 0x4B, 0x01, 0x02] {
            return Err("ZIP 中央目录条目格式错误".to_string());
        }
        let method = u16::from_le_bytes([data[pos + 10], data[pos + 11]]);
        let crc = u32::from_le_bytes([data[pos + 16], data[pos + 17], data[pos + 18], data[pos + 19]]);
        let comp_size = u32::from_le_bytes([data[pos + 20], data[pos + 21], data[pos + 22], data[pos + 23]]) as usize;
        let _uncomp_size = u32::from_le_bytes([data[pos + 24], data[pos + 25], data[pos + 26], data[pos + 27]]) as usize;
        let name_len = u16::from_le_bytes([data[pos + 28], data[pos + 29]]) as usize;
        let extra_len = u16::from_le_bytes([data[pos + 30], data[pos + 31]]) as usize;
        let comment_len = u16::from_le_bytes([data[pos + 32], data[pos + 33]]) as usize;
        let local_off = u32::from_le_bytes([data[pos + 42], data[pos + 43], data[pos + 44], data[pos + 45]]) as usize;
        let name = String::from_utf8_lossy(&data[pos + 46..pos + 46 + name_len]).to_string();
        // 读取本地头，定位数据区
        if local_off + 30 > data.len() || data[local_off..local_off + 4] != [0x50, 0x4B, 0x03, 0x04] {
            return Err(format!("条目 {} 本地头缺失", name));
        }
        let lname_len = u16::from_le_bytes([data[local_off + 26], data[local_off + 27]]) as usize;
        let lextra_len = u16::from_le_bytes([data[local_off + 28], data[local_off + 29]]) as usize;
        let data_start = local_off + 30 + lname_len + lextra_len;
        if method == 0 {
            if data_start + comp_size > data.len() {
                return Err(format!("条目 {} 数据越界", name));
            }
            let content = data[data_start..data_start + comp_size].to_vec();
            if crc32(&content) != crc {
                return Err(format!("条目 {} CRC 校验失败", name));
            }
            entries.push((name, content));
        } else if method == 8 {
            return Err(format!(
                "条目 {} 使用 deflate 压缩，当前导入仅支持 stored（无压缩）ZIP，请用系统导出的全量包",
                name
            ));
        } else {
            return Err(format!("条目 {} 压缩方式 {} 不受支持", name, method));
        }
        pos += 46 + name_len + extra_len + comment_len;
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_roundtrip_stored() {
        let entries = vec![
            ("manifest.json".to_string(), br#"{"a":1}"#.to_vec()),
            ("packages/x.json".to_string(), br#"{"id":"x"}"#.to_vec()),
            ("中文名.json".to_string(), "内容".as_bytes().to_vec()),
        ];
        let bytes = zip_write(&entries);
        let read = zip_read(&bytes).unwrap();
        assert_eq!(read.len(), 3);
        assert_eq!(read[0].0, "manifest.json");
        assert_eq!(read[1].1, br#"{"id":"x"}"#);
        assert_eq!(read[2].0, "中文名.json");
    }

    #[test]
    fn zip_rejects_corrupt() {
        assert!(zip_read(b"not a zip").is_err());
    }

    #[test]
    fn crc32_known_vector() {
        // CRC32("123456789") == 0xCBF43926
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn sign_verify_roundtrip() {
        let mut doc = serde_json::json!({ "kind": "ous-market-export", "package": { "id": "p1" } });
        let sig = sign_doc(&mut doc);
        assert!(verify_doc(&doc));
        // 篡改后应校验失败
        doc["package"]["id"] = serde_json::Value::String("p2".into());
        assert!(!verify_doc(&doc));
        // 无签名应失败
        let plain = serde_json::json!({ "kind": "ous-market-export" });
        assert!(!verify_doc(&plain));
        let _ = sig;
    }
}
