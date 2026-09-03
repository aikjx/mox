// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Filesystem layout helpers for EC shards and manifests.
//!
//! Each object OID is stored under a two-hex-char prefix (first two chars of
//! the oid) similar to how S3 disk layouts shard directories:
//!
//! ```text
//! mountpath / bucket_prefix / oid[0:2] / oid / ec / shard_{i}.slice
//! mountpath / bucket_prefix / oid[0:2] / oid / ec / manifest.json
//! ```
//!
//! Parsing (`parse_shard_path`) returns the extracted `(oid, shard_id, bucket)`
//! tuple when the path matches this schema.

use std::path::{Path, PathBuf};

use crate::error::{VolumeError, VolumeResult};

/// Builds the directory that contains the EC shards/manifest for an object.
pub fn ec_object_dir(mountpath: &Path, bucket_prefix: &str, oid: &str) -> PathBuf {
    let safe_prefix = oid.get(..2).unwrap_or("00");
    mountpath.join(bucket_prefix).join(safe_prefix).join(oid).join("ec")
}

/// Builds a shard file path: `ec/shard_{i}.slice`.
pub fn shard_path(mountpath: &Path, bucket_prefix: &str, oid: &str, shard_id: usize) -> PathBuf {
    ec_object_dir(mountpath, bucket_prefix, oid).join(format!("shard_{shard_id}.slice"))
}

/// Builds the manifest path for an object.
pub fn manifest_path(mountpath: &Path, bucket_prefix: &str, oid: &str) -> PathBuf {
    ec_object_dir(mountpath, bucket_prefix, oid).join("manifest.json")
}

/// Given a shard path produced by [`shard_path`], recover the
/// `(bucket_prefix, oid, shard_id)` triple.
///
/// Returns `VolumeError::Internal` on any mismatch so callers can use it for
/// glob + parse flows.
pub fn parse_shard_path(
    mountpath: &Path,
    shard_path: &Path,
) -> VolumeResult<(String, String, usize)> {
    let rel = shard_path.strip_prefix(mountpath).map_err(|_| {
        VolumeError::Internal(format!(
            "shard path {:?} is not under mountpath {:?}",
            shard_path, mountpath
        ))
    })?;

    // Expect: bucket/prefix[:any]/<2-char>/<oid>/ec/shard_N.slice
    let components: Vec<_> = rel.components().collect();
    if components.len() < 5 {
        return Err(VolumeError::Internal(format!("too few components in shard path: {:?}", rel)));
    }

    let n = components.len();
    let ec_dir = components[n - 2].as_os_str().to_string_lossy();
    if ec_dir != "ec" {
        return Err(VolumeError::Internal(format!("expected ec parent dir, got {}", ec_dir)));
    }
    let file_name = components[n - 1].as_os_str().to_string_lossy();
    let shard_id = parse_shard_file_name(&file_name).ok_or_else(|| {
        VolumeError::Internal(format!("unrecognized shard filename: {file_name}"))
    })?;
    let oid = components[n - 3].as_os_str().to_string_lossy().into_owned();
    // All components *before* the (two-char prefix, oid, ec, filename) block
    // constitute the bucket / bucket-prefix path.
    let bucket = components[..n - 4]
        .iter()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");
    Ok((bucket, oid, shard_id))
}

fn parse_shard_file_name(name: &str) -> Option<usize> {
    let rest = name.strip_prefix("shard_")?.strip_suffix(".slice")?;
    rest.parse::<usize>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_roundtrip() {
        let mount = Path::new("/mnt/vol1");
        let bucket = "mybucket";
        let oid = "abcd1234";
        for i in 0..10 {
            let p = shard_path(mount, bucket, oid, i);
            let (b, o, s) = parse_shard_path(mount, &p).unwrap();
            assert_eq!(b, bucket);
            assert_eq!(o, oid);
            assert_eq!(s, i);
        }
        let man = manifest_path(mount, bucket, oid);
        assert!(man.ends_with(format!("{oid}/ec/manifest.json")));
    }

    #[test]
    fn short_oid_prefix() {
        // oid[:2] must be safe even when oid is only 1 char long.  The final
        // component of ec_object_dir is always "ec" and the parent is named
        // after the oid, regardless of length or platform separator.
        let p = ec_object_dir(Path::new("/x"), "b", "z");
        let components: Vec<_> =
            p.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
        assert!(components.len() >= 3);
        assert_eq!(*components.last().unwrap(), "ec");
        assert_eq!(components[components.len() - 2], "z"); // oid dir
    }
}
