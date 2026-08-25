//! Tag parser: extracts TagSet from S3-style headers, normalizes keys/values,
//! and injects default derived tags (content_type / size_bucket / mime_category).

use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

/// A single tag: normalized key + string value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub k: String,
    pub v: String,
}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        self.k == other.k && self.v == other.v
    }
}
impl Eq for Tag {}

impl Hash for Tag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.k.hash(state);
        self.v.hash(state);
    }
}

impl Tag {
    pub fn new(k: impl Into<String>, v: impl Into<String>) -> Self {
        Self { k: k.into(), v: v.into() }
    }
}

/// A set (Vec-backed) of tags.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagSet(pub Vec<Tag>);

impl TagSet {
    /// Construct from S3-style headers.
    ///
    /// Header key conventions supported:
    /// - `x-amz-meta-<name>` / `x-amz-tag-<name>`: tag key extracted from the suffix.
    /// - `tagging-<name>`: same.
    /// - `X-Amz-Tagging`: url-encoded `k1=v1&k2=v2` form (per S3 PutObject Tagging spec).
    /// - Otherwise, the raw header (k, v) becomes a tag if not a known internal header
    ///   like `Content-Length`, `Host`, `Authorization`, `Date`, `ETag` etc.
    pub fn from_s3_headers(
        headers: &[(String, String)],
        apply_defaults: bool,
        obj_content_type: Option<&str>,
        obj_size: u64,
    ) -> TagSet {
        let mut tags: Vec<Tag> = Vec::with_capacity(headers.len().saturating_add(4));

        for (raw_k, raw_v) in headers {
            let k = raw_k.trim();
            if k.is_empty() {
                continue;
            }
            // skip a handful of internal protocol headers if they appear
            let lower = k.to_ascii_lowercase();
            if matches!(
                lower.as_str(),
                "content-length"
                    | "host"
                    | "authorization"
                    | "date"
                    | "etag"
                    | "user-agent"
                    | "accept"
                    | "connection"
            ) {
                continue;
            }

            if lower == "x-amz-tagging" {
                extend_from_querystring(raw_v.as_str(), &mut tags);
                continue;
            }

            // x-amz-meta-FOO  -> FOO
            let key = if let Some(rest) = lower.strip_prefix("x-amz-meta-") {
                rest.to_string()
            } else if let Some(rest) = lower.strip_prefix("x-amz-tag-") {
                rest.to_string()
            } else if let Some(rest) = lower.strip_prefix("tagging-") {
                rest.to_string()
            } else if lower == "content-type" {
                // Content-Type header: synthesize content_type tag
                tags.push(Tag::new("content_type", raw_v.trim()));
                continue;
            } else {
                // raw header name becomes the tag key verbatim (will be normalized later)
                k.to_string()
            };
            tags.push(Tag::new(key, raw_v.trim()));
        }

        // dedup by (k, v) before defaults
        let mut seen: HashSet<(String, String)> = HashSet::new();
        tags.retain(|t| seen.insert((t.k.clone(), t.v.clone())));

        if apply_defaults {
            // Only add defaults if not already present by key.
            let has: HashSet<String> = tags.iter().map(|t| t.k.clone()).collect();

            // content_type (from header arg if any)
            if !has.contains("content_type") {
                let v = obj_content_type.map(|s| s.trim().to_string()).unwrap_or_default();
                tags.push(Tag::new("content_type", v));
            }

            // size_bucket
            if !has.contains("size_bucket") {
                tags.push(Tag::new("size_bucket", bucketize_size(obj_size)));
            }

            // mime_category
            if !has.contains("mime_category") {
                let ct = tags
                    .iter()
                    .find(|t| t.k == "content_type")
                    .map(|t| t.v.as_str())
                    .or(obj_content_type);
                tags.push(Tag::new("mime_category", mime_category(ct)));
            }
        }

        TagSet(tags)
    }

    /// Normalize keys: non-alphanumeric and non-underscore -> `_`; lowercase.
    /// Drop tags whose (trimmed) key is empty. Empty values become `"(empty)"`.
    /// Keys longer than 64 chars are truncated.
    /// Total tag count capped at 50; if truncated returns `Some("truncated")`.
    /// The truncated tags are silently dropped but the caller can log via the alarm.
    pub fn normalize(&mut self) -> Option<String> {
        let mut out: Vec<Tag> = Vec::with_capacity(self.0.len());
        for t in self.0.drain(..) {
            let k = normalize_key(&t.k);
            if k.is_empty() {
                continue;
            }
            let v = if t.v.is_empty() { String::from("(empty)") } else { t.v };
            out.push(Tag { k, v });
        }
        // dedup by k + v
        let mut seen: HashSet<(String, String)> = HashSet::new();
        out.retain(|t| seen.insert((t.k.clone(), t.v.clone())));
        // re-dedup by k (last wins? keep first.)
        let mut kseen: HashSet<String> = HashSet::new();
        let mut out2: Vec<Tag> = Vec::with_capacity(out.len());
        for t in out {
            if kseen.insert(t.k.clone()) {
                out2.push(t);
            }
        }
        self.0 = out2;

        if self.0.len() > 50 {
            self.0.truncate(50);
            Some(String::from("truncated"))
        } else {
            None
        }
    }

    /// Number of tags.
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn into_inner(self) -> Vec<Tag> {
        self.0
    }
}

fn normalize_key(k: &str) -> String {
    let mut s = String::with_capacity(k.len());
    let mut prev_under = false;
    for ch in k.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            s.push(ch.to_ascii_lowercase());
            prev_under = ch == '_';
        } else if ch == '-' || ch == '.' || ch == '/' || ch == ':' || ch == ' ' {
            if !prev_under {
                s.push('_');
                prev_under = true;
            }
        } else {
            if !prev_under {
                s.push('_');
                prev_under = true;
            }
        }
    }
    // A key that collapses to all underscores (e.g. "!!!", "___?__") is
    // semantically empty -> drop it.
    if s.chars().all(|c| c == '_') {
        s.clear();
    }
    // Truncate to 64 characters AFTER transformation
    if s.chars().count() > 64 {
        let mut t: String = s.chars().take(64).collect();
        // trim trailing underscores for aesthetics
        while t.ends_with('_') {
            t.pop();
        }
        t
    } else {
        s
    }
}

fn bucketize_size(size: u64) -> &'static str {
    match size {
        0..=1_023 => "0..1KB",
        1_024..=1_048_575 => "1KB..1MB",
        1_048_576..=1_073_741_823 => "1MB..1GB",
        _ => "1GB+",
    }
}

fn mime_category(content_type: Option<&str>) -> &'static str {
    let ct = match content_type {
        None | Some("") => return "other",
        Some(s) => s.trim().to_ascii_lowercase(),
    };
    if ct.starts_with("application/") {
        "application"
    } else if ct.starts_with("text/") {
        "text"
    } else if ct.starts_with("image/") {
        "image"
    } else if ct.starts_with("audio/") {
        "audio"
    } else if ct.starts_with("video/") {
        "video"
    } else {
        "other"
    }
}

fn extend_from_querystring(q: &str, out: &mut Vec<Tag>) {
    for pair in q.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = match pair.split_once('=') {
            Some((k, v)) => (
                percent_decode_lossy(k),
                percent_decode_lossy(v),
            ),
            None => (percent_decode_lossy(pair), String::new()),
        };
        out.push(Tag::new(k, v));
    }
}

// Inline percent-decoder using percent-encoding.
fn percent_decode_lossy(s: &str) -> String {
    percent_encoding::percent_decode_str(s)
        .decode_utf8_lossy()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_key_common() {
        assert_eq!(normalize_key("Content-Type"), "content_type");
        assert_eq!(normalize_key("X-Amz-Meta-Project"), "x_amz_meta_project");
        assert_eq!(normalize_key("foo/bar.baz:qux"), "foo_bar_baz_qux");
        assert_eq!(normalize_key("   "), "");
    }

    #[test]
    fn bucketize_smoke() {
        assert_eq!(bucketize_size(0), "0..1KB");
        assert_eq!(bucketize_size(1023), "0..1KB");
        assert_eq!(bucketize_size(1024), "1KB..1MB");
        assert_eq!(bucketize_size(1 << 20), "1MB..1GB");
        assert_eq!(bucketize_size(1 << 30), "1GB+");
    }

    #[test]
    fn mime_category_cases() {
        assert_eq!(mime_category(Some("application/pdf")), "application");
        assert_eq!(mime_category(Some("text/html; charset=utf-8")), "text");
        assert_eq!(mime_category(Some("image/png")), "image");
        assert_eq!(mime_category(Some("audio/mpeg")), "audio");
        assert_eq!(mime_category(Some("video/mp4")), "video");
        assert_eq!(mime_category(Some("x-custom/foo")), "other");
        assert_eq!(mime_category(None), "other");
    }

    #[test]
    fn default_tags_applied() {
        let headers: &[(String, String)] = &[];
        let t = TagSet::from_s3_headers(headers, true, Some("application/pdf"), 2_000_000);
        let by_k: std::collections::HashMap<String, String> =
            t.0.into_iter().map(|t| (t.k, t.v)).collect();
        assert_eq!(by_k.get("content_type").unwrap(), "application/pdf");
        assert_eq!(by_k.get("size_bucket").unwrap(), "1MB..1GB");
        assert_eq!(by_k.get("mime_category").unwrap(), "application");
    }

    #[test]
    fn default_tags_not_applied_when_off() {
        let headers: &[(String, String)] = &[(String::from("x-amz-meta-foo"), String::from("bar"))];
        let t = TagSet::from_s3_headers(headers, false, Some("application/pdf"), 2_000_000);
        let by_k: std::collections::HashMap<String, String> =
            t.0.into_iter().map(|t| (t.k, t.v)).collect();
        assert!(!by_k.contains_key("size_bucket"));
        assert!(!by_k.contains_key("mime_category"));
        assert_eq!(by_k.get("foo").unwrap(), "bar");
    }

    #[test]
    fn normalize_truncation() {
        let mut ts = TagSet((0..55u16).map(|i| Tag::new(format!("k{:03}", i), "v")).collect());
        let alarm = ts.normalize();
        assert_eq!(ts.0.len(), 50);
        assert_eq!(alarm.as_deref(), Some("truncated"));
    }
}
