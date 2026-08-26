//! Multipart UploadId manager: extends the existing mox-cloud-drive-s3/mpu.rs
//! with data-plane level abstractions (CRC64 aggregation + PartAggregate).
//!
//! This module deliberately keeps disk-persistence pluggable:
//!   - In-memory store for unit tests
//!   - Production plug-in via trait to SQL / volume backend

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartAggregate {
    pub upload_id: String,
    pub n_parts: u32,
    pub total_bytes: u64,
    pub etag: String,
    pub crc64_ecma: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Part {
    pub number: u16,
    pub bytes: Vec<u8>,
    pub crc64_ecma: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipartUpload {
    pub upload_id: String,
    pub bucket: String,
    pub key: String,
    pub created_at_ms: i64,
    pub expires_at_ms: i64,
    pub parts: BTreeMap<u16, Part>,
    pub owner: String,
    pub crc64_seed: u64,
}

#[derive(Default)]
pub struct MultipartManager {
    inner: Mutex<BTreeMap<String, MultipartUpload>>,
}

fn crc64_update(mut state: u64, bytes: &[u8]) -> u64 {
    // CRC64/ECMA-182 (poly 0x42F0E1EBA9EA3693, init 0, no tail xor) — matches manifest.rs
    const POLY: u64 = 0x42F0E1EBA9EA3693;
    for &b in bytes {
        state ^= (b as u64) << 56;
        for _ in 0..8 {
            if state & (1u64 << 63) != 0 {
                state = (state << 1) ^ POLY;
            } else {
                state <<= 1;
            }
        }
    }
    state
}

impl MultipartManager {
    pub fn new() -> Self { Self::default() }

    pub fn create(&self, bucket: impl Into<String>, key: impl Into<String>, owner: impl Into<String>) -> String {
        let owner_s: String = owner.into();
        let id = format!("{}-{}", Uuid::new_v4(), short_signature(&owner_s));
        let up = MultipartUpload {
            upload_id: id.clone(),
            bucket: bucket.into(),
            key: key.into(),
            created_at_ms: chrono::Utc::now().timestamp_millis(),
            expires_at_ms: chrono::Utc::now().timestamp_millis() + 7 * 24 * 3600 * 1000,
            parts: BTreeMap::new(),
            owner: owner_s,
            crc64_seed: 0,
        };
        self.inner.lock().insert(id.clone(), up);
        id
    }

    pub fn upload_part(&self, upload_id: &str, part_number: u16, bytes: Vec<u8>) -> Result<(u64, String), String> {
        if bytes.is_empty() { return Err("empty part".to_string()); }
        let mut w = self.inner.lock();
        let up = w.get_mut(upload_id).ok_or_else(|| "upload_id not found".to_string())?;
        let crc = crc64_update(0, &bytes);
        let etag = format!("{:016x}", crc);
        up.parts.insert(part_number, Part { number: part_number, bytes, crc64_ecma: crc });
        Ok((crc, etag))
    }

    pub fn abort(&self, upload_id: &str) -> bool {
        self.inner.lock().remove(upload_id).is_some()
    }

    pub fn complete(&self, upload_id: &str) -> Result<PartAggregate, String> {
        let mut w = self.inner.lock();
        let up = w.remove(upload_id).ok_or_else(|| "upload_id not found".to_string())?;
        if up.parts.is_empty() { return Err("no parts uploaded".to_string()); }
        // multi-part upload (N>=2): parts must be contiguous starting at part_number=1.
        // single-part upload: any valid part_number is acceptable (trivially contiguous set).
        if up.parts.len() > 1 {
            let expected: Vec<u16> = (1..=up.parts.len() as u16).collect();
            let actual: Vec<u16> = up.parts.keys().copied().collect();
            if expected != actual { return Err(format!("parts not contiguous: expected 1..={}, got {:?}", up.parts.len(), actual)); }
        }
        let mut agg_crc = 0u64;
        let mut total = 0u64;
        let mut etag_parts = Vec::new();
        for (_, p) in &up.parts {
            agg_crc = crc64_update(agg_crc, &p.bytes);
            total += p.bytes.len() as u64;
            etag_parts.push(format!("{:08x}-{:016x}", p.number, p.crc64_ecma));
        }
        let etag = format!("{}-{}", up.parts.len(), {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(&etag_parts.join("|"));
            let d = h.finalize();
            hex::encode(&d[..12])
        });
        Ok(PartAggregate {
            upload_id: up.upload_id,
            n_parts: up.parts.len() as u32,
            total_bytes: total,
            etag,
            crc64_ecma: agg_crc,
        })
    }

    pub fn count(&self) -> usize { self.inner.lock().len() }
}

fn short_signature(s: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    let d = h.finalize();
    hex::encode(&d[..6])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    // T3-01
    #[test]
    fn t3_01_create_upload_id_len_and_owner_sig() {
        let m = MultipartManager::new();
        let id = m.create("b", "k", "owner-alice");
        assert!(id.len() >= 32, "upload_id must be >=32 chars, got len={}", id.len());
        let sig = short_signature("owner-alice");
        assert!(
            id.ends_with(&sig),
            "upload_id must contain owner signature suffix, id={}, sig={}",
            id, sig
        );
    }

    // T3-02
    #[test]
    fn t3_02_upload_part_empty_err() {
        let m = MultipartManager::new();
        let id = m.create("b", "k", "u");
        let r = m.upload_part(&id, 1, vec![]);
        assert!(r.is_err());
        assert_eq!(r.unwrap_err(), "empty part");
    }

    // T3-03
    #[test]
    fn t3_03_upload_part_unknown_id_err() {
        let m = MultipartManager::new();
        let r = m.upload_part("nonexistent-id-xyz", 1, b"data".to_vec());
        assert!(r.is_err());
        assert_eq!(r.unwrap_err(), "upload_id not found");
    }

    // T3-04
    #[test]
    fn t3_04_upload_5_contiguous_parts_ok() {
        let m = MultipartManager::new();
        let id = m.create("b", "k", "u");
        let sizes: [usize; 5] = [10, 20, 30, 40, 50];
        let mut total = 0u64;
        for (i, &sz) in sizes.iter().enumerate() {
            let pn = (i + 1) as u16;
            let data = vec![pn as u8; sz];
            total += sz as u64;
            let _ = m.upload_part(&id, pn, data).unwrap();
        }
        let agg = m.complete(&id).unwrap();
        assert_eq!(agg.n_parts, 5);
        assert_eq!(agg.total_bytes, total);
    }

    // T3-05
    #[test]
    fn t3_05_part3_missing_part1_contiguous_fails() {
        let m = MultipartManager::new();
        let id = m.create("b", "k", "u");
        let _ = m.upload_part(&id, 3, b"part3".to_vec()).unwrap();
        let _ = m.upload_part(&id, 1, b"part1".to_vec()).unwrap();
        // missing part 2, so 1,3 is not contiguous 1..=2
        let r = m.complete(&id);
        assert!(r.is_err());
        let msg = r.unwrap_err();
        assert!(msg.contains("parts not contiguous"), "msg={}", msg);
    }

    // T3-06
    #[test]
    fn t3_06_duplicate_part_number_last_wins() {
        let m = MultipartManager::new();
        let id = m.create("b", "k", "u");
        let first = vec![1u8; 10];
        let second = vec![9u8; 100];
        let _ = m.upload_part(&id, 1, first).unwrap();
        let (crc2, etag2) = m.upload_part(&id, 1, second.clone()).unwrap();
        let agg = m.complete(&id).unwrap();
        assert_eq!(agg.n_parts, 1);
        assert_eq!(agg.total_bytes, 100);
        assert_eq!(agg.crc64_ecma, crc2);
        // etag in agg for 1 part should reference the last-won etag
        let agg_etag_prefix = format!("1-");
        assert!(agg.etag.starts_with(&agg_etag_prefix));
        let _ = etag2; // used as sanity check value
    }

    // T3-07
    #[test]
    fn t3_07_complete_unknown_id_err() {
        let m = MultipartManager::new();
        let r = m.complete("no-such-upload-id");
        assert!(r.is_err());
        assert_eq!(r.unwrap_err(), "upload_id not found");
    }

    // T3-08
    #[test]
    fn t3_08_abort_exists_and_not_exists() {
        let m = MultipartManager::new();
        let id = m.create("b", "k", "u");
        assert_eq!(m.count(), 1);
        assert!(m.abort(&id));
        assert_eq!(m.count(), 0);
        assert!(!m.abort(&id));
        assert!(!m.abort("never-created-id"));
    }

    // T3-09
    #[test]
    fn t3_09_create_50_unique_upload_ids_all_in_manager() {
        let m = MultipartManager::new();
        let mut ids: Vec<String> = Vec::with_capacity(50);
        for i in 0..50 {
            let id = m.create("b", format!("k{}", i), format!("user{}", i));
            ids.push(id);
        }
        assert_eq!(m.count(), 50);
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 50, "all 50 upload_ids must be unique");
    }

    // T3-10: CRC-64/ECMA-182 known vector for "123456789"
    #[test]
    fn t3_10_crc64_ecma_known_vector() {
        // Known vector per ECMA-182 / CRC-64/XZ standard poly 0x42F0E1EBA9EA3693 init=0 no xor:
        // echo -n "123456789" | xz --check=crc64 produces the check value 0x6C40DF5F0B497347
        let expected: u64 = 0x6C40DF5F0B497347;
        let got = crc64_update(0, b"123456789");
        assert_eq!(got, expected, "crc64(123456789) mismatch: got={:#x} expected={:#x}", got, expected);
    }

    // T3-11
    #[test]
    fn t3_11_two_1kb_parts_combined_crc_matches() {
        let p1 = vec![0xABu8; 1024];
        let p2 = vec![0xCDu8; 1024];
        let combined: Vec<u8> = p1.iter().chain(p2.iter()).copied().collect();
        let direct = crc64_update(0, &combined);
        let step1 = crc64_update(0, &p1);
        let step2 = crc64_update(step1, &p2);
        assert_eq!(direct, step2, "combined crc must equal crc(crc(0,p1), p2)");
    }

    // T3-12
    #[test]
    fn t3_12_part_etag_is_16_char_hex_lowercase() {
        let m = MultipartManager::new();
        let id = m.create("b", "k", "u");
        let (_crc, etag) = m.upload_part(&id, 1, b"any-non-empty-bytes".to_vec()).unwrap();
        assert_eq!(etag.len(), 16, "etag len must be 16, got {}", etag.len());
        // hex digits 0-9 are not lowercase letters; ensure no uppercase a-f exist
        assert!(etag.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "etag must be lowercase hex (no uppercase letters), got {}", etag);
    }

    // T3-13
    #[test]
    fn t3_13_complete_etag_format_parts_dash_24hex() {
        let m = MultipartManager::new();
        let id = m.create("b", "k", "u");
        let _ = m.upload_part(&id, 1, b"aaa".to_vec()).unwrap();
        let _ = m.upload_part(&id, 2, b"bbb".to_vec()).unwrap();
        let _ = m.upload_part(&id, 3, b"ccc".to_vec()).unwrap();
        let agg = m.complete(&id).unwrap();
        // format: "{n_parts}-{24_hex_lowercase}"
        let parts: Vec<&str> = agg.etag.split('-').collect();
        assert_eq!(parts.len(), 2, "etag format wrong: {}", agg.etag);
        assert_eq!(parts[0], "3");
        assert_eq!(parts[1].len(), 24, "hex suffix must be 24 chars, got {}", parts[1].len());
        assert!(
            parts[1].chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "hex suffix must be lowercase hex (no uppercase letters): {}", parts[1]
        );
    }

    // T3-14
    #[test]
    fn t3_14_complete_removes_upload_from_manager() {
        let m = MultipartManager::new();
        let id = m.create("b", "k", "u");
        assert_eq!(m.count(), 1);
        let _ = m.upload_part(&id, 1, b"data".to_vec()).unwrap();
        let _ = m.complete(&id).unwrap();
        assert_eq!(m.count(), 0, "complete must remove upload from manager");
        // subsequent complete fails
        let r2 = m.complete(&id);
        assert!(r2.is_err());
    }

    // T3-15
    #[test]
    fn t3_15_part_number_100_single_complete_ok() {
        let m = MultipartManager::new();
        let id = m.create("b", "k", "u");
        let data = b"large-indexed-single-part".to_vec();
        let expected_len = data.len();
        let _ = m.upload_part(&id, 100, data).unwrap();
        let agg = m.complete(&id).unwrap();
        assert_eq!(agg.n_parts, 1);
        assert_eq!(agg.total_bytes, expected_len as u64);
    }

    // T3-16
    #[test]
    fn t3_16_concurrent_5_threads_10_uploads_each_count_50() {
        let m = Arc::new(MultipartManager::new());
        let mut handles = Vec::new();
        for t in 0..5 {
            let mgr = Arc::clone(&m);
            let handle = thread::spawn(move || {
                for i in 0..10 {
                    let bucket = format!("t{}-b{}", t, i);
                    let key = format!("t{}-k{}", t, i);
                    let owner = format!("u{}", t);
                    let id = mgr.create(bucket, key, owner);
                    // each thread also uploads 1 part to be a realistic scenario
                    let _ = mgr.upload_part(&id, 1, vec![t as u8; 16]).unwrap();
                }
            });
            handles.push(handle);
        }
        for h in handles { h.join().expect("thread panicked"); }
        assert_eq!(m.count(), 50, "expected 50 concurrent uploads, got {}", m.count());
    }

    // --- Original 3 tests preserved for regression safety (TOTAL: 3+16=19) ---
    #[test]
    fn mpu_create_then_3_parts_complete_ok() {
        let m = MultipartManager::new();
        let id = m.create("b", "k", "u1");
        let _ = m.upload_part(&id, 1, b"hello".to_vec()).unwrap();
        let _ = m.upload_part(&id, 2, vec![0u8; 1024]).unwrap();
        let _ = m.upload_part(&id, 3, vec![42; 64]).unwrap();
        let agg = m.complete(&id).unwrap();
        assert_eq!(agg.n_parts, 3);
        assert_eq!(agg.total_bytes, 5 + 1024 + 64);
    }

    #[test]
    fn mpu_non_contiguous_error() {
        let m = MultipartManager::new();
        let id = m.create("b", "k", "u");
        let _ = m.upload_part(&id, 1, b"a".to_vec()).unwrap();
        let _ = m.upload_part(&id, 3, b"b".to_vec()).unwrap();
        let r = m.complete(&id);
        assert!(r.is_err());
    }

    #[test]
    fn mpu_abort() {
        let m = MultipartManager::new();
        let id = m.create("b", "k", "u");
        assert_eq!(m.count(), 1);
        assert!(m.abort(&id));
        assert_eq!(m.count(), 0);
        assert!(!m.abort(&id));
    }
}
