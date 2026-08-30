// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::client::CloudClient;
use crate::error::{CloudError, Result};
use crate::types::{
    BucketInfo, MultipartUploadInfo, ObjectInfo, PartEtag,
};
use crate::utils::{fxhash, rand_u64};

impl CloudClient {
    // ========== Bucket (5) ==========

    pub async fn create_bucket(&self, name: &str) -> Result<BucketInfo> {
        let mut s = self.lock()?;
        let info = BucketInfo {
            name: name.to_string(),
            creation_date: 0,
            acl: "private".to_string(),
        };
        s.buckets.insert(name.to_string(), info.clone());
        Ok(info)
    }

    pub async fn delete_bucket(&self, name: &str) -> Result<()> {
        let mut s = self.lock()?;
        s.buckets.remove(name);
        Ok(())
    }

    pub async fn list_buckets(&self) -> Result<Vec<BucketInfo>> {
        let s = self.lock()?;
        let mut out: Vec<BucketInfo> = s.buckets.values().cloned().collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    pub async fn head_bucket(&self, name: &str) -> Result<BucketInfo> {
        let s = self.lock()?;
        s.buckets
            .get(name)
            .cloned()
            .ok_or_else(|| CloudError::NotFound(format!("bucket {name}")))
    }

    pub async fn set_bucket_acl(&self, name: &str, acl: &str) -> Result<()> {
        let mut s = self.lock()?;
        let b = s
            .buckets
            .get_mut(name)
            .ok_or_else(|| CloudError::NotFound(format!("bucket {name}")))?;
        b.acl = acl.to_string();
        Ok(())
    }

    // ========== Object (6) ==========

    pub async fn put_object(&self, bucket: &str, key: &str, data: Vec<u8>) -> Result<String> {
        let mut s = self.lock()?;
        // create bucket implicitly
        s.buckets
            .entry(bucket.to_string())
            .or_insert_with(|| BucketInfo {
                name: bucket.to_string(),
                creation_date: 0,
                acl: "private".to_string(),
            });
        let etag = format!("{:016x}", fxhash(&data));
        s.objects.insert((bucket.to_string(), key.to_string()), data);
        Ok(etag)
    }

    pub async fn get_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>> {
        let s = self.lock()?;
        s.objects
            .get(&(bucket.to_string(), key.to_string()))
            .cloned()
            .ok_or_else(|| CloudError::NotFound(format!("{bucket}/{key}")))
    }

    pub async fn delete_object(&self, bucket: &str, key: &str) -> Result<()> {
        let mut s = self.lock()?;
        // WORM compliance check
        if let Some(w) = s.worms.get(&(bucket.to_string(), key.to_string())) {
            if w.mode == "compliance" && w.retain_until > 0 {
                return Err(CloudError::WormLocked(format!("{bucket}/{key}")));
            }
        }
        s.objects.remove(&(bucket.to_string(), key.to_string()));
        Ok(())
    }

    pub async fn list_prefix(
        &self,
        bucket: &str,
        prefix: &str,
        max_keys: Option<u32>,
    ) -> Result<Vec<ObjectInfo>> {
        let s = self.lock()?;
        let limit = max_keys.unwrap_or(1000) as usize;
        let mut items: Vec<ObjectInfo> = s
            .objects
            .iter()
            .filter(|((b, k), _)| b == bucket && k.starts_with(prefix))
            .take(limit)
            .map(|((_, k), v)| ObjectInfo {
                key: k.clone(),
                size: v.len(),
                etag: format!("{:016x}", fxhash(v)),
                last_modified: 0,
            })
            .collect();
        items.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(items)
    }

    pub async fn copy_object(
        &self,
        src_bucket: &str,
        src_key: &str,
        dst_bucket: &str,
        dst_key: &str,
    ) -> Result<String> {
        let data = {
            let s = self.lock()?;
            s.objects
                .get(&(src_bucket.to_string(), src_key.to_string()))
                .cloned()
                .ok_or_else(|| CloudError::NotFound(format!("{src_bucket}/{src_key}")))?
        };
        self.put_object(dst_bucket, dst_key, data).await
    }

    // ========== Multipart (5) ==========

    pub async fn create_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<String> {
        let mut s = self.lock()?;
        let upload_id = format!("mpu-{}-{}-{}", bucket, key, rand_u64());
        s.multiparts.insert(
            upload_id.clone(),
            crate::types::MultipartUpload {
                upload_id: upload_id.clone(),
                bucket: bucket.to_string(),
                key: key.to_string(),
                parts: std::collections::BTreeMap::new(),
            },
        );
        Ok(upload_id)
    }

    pub async fn upload_part(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        part_number: u16,
        data: Vec<u8>,
    ) -> Result<PartEtag> {
        let mut s = self.lock()?;
        let mpu = s
            .multiparts
            .get_mut(upload_id)
            .ok_or_else(|| CloudError::NotFound(format!("upload_id {upload_id}")))?;
        debug_assert_eq!(mpu.bucket, bucket);
        debug_assert_eq!(mpu.key, key);
        let etag = format!("{:016x}", fxhash(&data));
        mpu.parts.insert(part_number, (etag.clone(), data));
        Ok(PartEtag { part_number, etag })
    }

    pub async fn complete_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        parts: Vec<PartEtag>,
    ) -> Result<String> {
        let (bytes, _parts_saved) = {
            let mut s = self.lock()?;
            let mpu = s
                .multiparts
                .remove(upload_id)
                .ok_or_else(|| CloudError::NotFound(format!("upload_id {upload_id}")))?;
            let mut bytes = Vec::new();
            for pe in &parts {
                if let Some((_etag, data)) = mpu.parts.get(&pe.part_number) {
                    bytes.extend_from_slice(data);
                }
            }
            (bytes, mpu.parts.len())
        };
        self.put_object(bucket, key, bytes).await
    }

    pub async fn abort_multipart_upload(&self, upload_id: &str) -> Result<()> {
        let mut s = self.lock()?;
        s.multiparts.remove(upload_id);
        Ok(())
    }

    pub async fn list_multipart_uploads(&self) -> Result<Vec<MultipartUploadInfo>> {
        let s = self.lock()?;
        let mut out: Vec<MultipartUploadInfo> = s
            .multiparts
            .values()
            .map(|m| MultipartUploadInfo {
                upload_id: m.upload_id.clone(),
                bucket: m.bucket.clone(),
                key: m.key.clone(),
                parts_count: m.parts.len(),
            })
            .collect();
        out.sort_by(|a, b| a.upload_id.cmp(&b.upload_id));
        Ok(out)
    }
}
