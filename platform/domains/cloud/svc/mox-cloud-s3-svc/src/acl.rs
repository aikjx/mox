// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! Canned ACL 实现：S3 标准 6 种预置 ACL。

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CannedAcl {
    #[default]
    Private,
    PublicRead,
    PublicReadWrite,
    AuthenticatedRead,
    BucketOwnerRead,
    BucketOwnerFullControl,
}

impl fmt::Display for CannedAcl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            CannedAcl::Private => "private",
            CannedAcl::PublicRead => "public-read",
            CannedAcl::PublicReadWrite => "public-read-write",
            CannedAcl::AuthenticatedRead => "authenticated-read",
            CannedAcl::BucketOwnerRead => "bucket-owner-read",
            CannedAcl::BucketOwnerFullControl => "bucket-owner-full-control",
        };
        f.write_str(s)
    }
}

impl CannedAcl {
    pub fn from_header(val: &str) -> Option<Self> {
        match val {
            "private" => Some(CannedAcl::Private),
            "public-read" => Some(CannedAcl::PublicRead),
            "public-read-write" => Some(CannedAcl::PublicReadWrite),
            "authenticated-read" => Some(CannedAcl::AuthenticatedRead),
            "bucket-owner-read" => Some(CannedAcl::BucketOwnerRead),
            "bucket-owner-full-control" => Some(CannedAcl::BucketOwnerFullControl),
            _ => None,
        }
    }

    /// 是否允许公开匿名读。
    pub fn allows_public_read(&self) -> bool {
        matches!(self, CannedAcl::PublicRead | CannedAcl::PublicReadWrite)
    }

    /// 是否允许公开匿名写。
    pub fn allows_public_write(&self) -> bool {
        matches!(self, CannedAcl::PublicReadWrite)
    }

    /// 生成 AWS S3 AccessControlPolicy XML。
    pub fn to_acl_xml(&self, owner_id: &str, owner_display: &str) -> String {
        // Grant 列表基于 canned ACL。
        let (grantee_read, grantee_write) = match self {
            CannedAcl::Private => (owner_id, ""),
            CannedAcl::PublicRead => ("*OPEN*", ""),
            CannedAcl::PublicReadWrite => ("*OPEN*", "*OPEN*"),
            CannedAcl::AuthenticatedRead => ("*AUTH*", ""),
            CannedAcl::BucketOwnerRead => (owner_id, ""),
            CannedAcl::BucketOwnerFullControl => (owner_id, owner_id),
        };
        let mut grants = String::new();
        grants.push_str("    <Grant>\n");
        grants.push_str(&format!(
            "      <Grantee xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:type=\"CanonicalUser\">\n        <ID>{}</ID>\n        <DisplayName>{}</DisplayName>\n      </Grantee>\n",
            owner_id, owner_display
        ));
        grants.push_str("      <Permission>FULL_CONTROL</Permission>\n    </Grant>\n");
        if grantee_read != owner_id && !grantee_read.is_empty() {
            grants.push_str("    <Grant>\n");
            if grantee_read == "*OPEN*" {
                grants.push_str("      <Grantee xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:type=\"Group\">\n        <URI>http://acs.amazonaws.com/groups/global/AllUsers</URI>\n      </Grantee>\n");
            } else if grantee_read == "*AUTH*" {
                grants.push_str("      <Grantee xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:type=\"Group\">\n        <URI>http://acs.amazonaws.com/groups/global/AuthenticatedUsers</URI>\n      </Grantee>\n");
            } else {
                grants.push_str(&format!(
                    "      <Grantee xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:type=\"CanonicalUser\">\n        <ID>{}</ID>\n        <DisplayName>{}</DisplayName>\n      </Grantee>\n",
                    grantee_read, owner_display
                ));
            }
            grants.push_str("      <Permission>READ</Permission>\n    </Grant>\n");
        }
        if !grantee_write.is_empty() && grantee_write != owner_id {
            grants.push_str("    <Grant>\n");
            if grantee_write == "*OPEN*" {
                grants.push_str("      <Grantee xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:type=\"Group\">\n        <URI>http://acs.amazonaws.com/groups/global/AllUsers</URI>\n      </Grantee>\n");
            } else {
                grants.push_str(&format!(
                    "      <Grantee xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:type=\"CanonicalUser\">\n        <ID>{}</ID>\n        <DisplayName>{}</DisplayName>\n      </Grantee>\n",
                    grantee_write, owner_display
                ));
            }
            grants.push_str("      <Permission>WRITE</Permission>\n    </Grant>\n");
        }
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <AccessControlPolicy xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\n\
               <Owner>\n\
                 <ID>{}</ID>\n\
                 <DisplayName>{}</DisplayName>\n\
               </Owner>\n\
               <AccessControlList>\n{}\
               </AccessControlList>\n\
             </AccessControlPolicy>",
            owner_id, owner_display, grants
        )
    }
}
