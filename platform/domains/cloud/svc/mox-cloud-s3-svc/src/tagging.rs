// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Object/Bucket Tagging：XML 解析 + 写入；Get/PutObjectTagging / Get/PutBucketTagging 响应。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tag {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tagging {
    pub tags: Vec<Tag>,
}

impl Tagging {
    pub fn from_map(m: &BTreeMap<String, String>) -> Self {
        let tags = m.iter().map(|(k, v)| Tag { key: k.clone(), value: v.clone() }).collect();
        Tagging { tags }
    }

    pub fn to_map(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        for t in &self.tags {
            m.insert(t.key.clone(), t.value.clone());
        }
        m
    }

    /// AWS S3 标准 Tagging XML：
    /// <Tagging><TagSet><Tag><Key>k</Key><Value>v</Value></Tag>...</TagSet></Tagging>
    pub fn to_xml(&self) -> String {
        let mut inner = String::new();
        for t in &self.tags {
            inner.push_str(&format!(
                "      <Tag>\n        <Key>{}</Key>\n        <Value>{}</Value>\n      </Tag>\n",
                xml_escape(&t.key),
                xml_escape(&t.value)
            ));
        }
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <Tagging xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\n\
               <TagSet>\n{}\
               </TagSet>\n\
             </Tagging>",
            inner
        )
    }

    /// 从 XML 解析 Tagging。
    pub fn from_xml(xml: &str) -> Result<Self, String> {
        let reader = xml::EventReader::from_str(xml);
        let mut tags: Vec<Tag> = Vec::new();
        let mut cur_tag: Option<Tag> = None;
        let mut text = String::new();
        for e in reader {
            use xml::reader::XmlEvent;
            match e.map_err(|x| x.to_string())? {
                XmlEvent::StartElement { name, .. } => {
                    text.clear();
                    if name.local_name == "Tag" {
                        cur_tag = Some(Tag::default());
                    }
                },
                XmlEvent::Characters(s) => text.push_str(&s),
                XmlEvent::EndElement { name, .. } => {
                    let tag_name = name.local_name.as_str();
                    match tag_name {
                        "Key" => {
                            if let Some(t) = cur_tag.as_mut() {
                                t.key = text.trim().to_string();
                            }
                        },
                        "Value" => {
                            if let Some(t) = cur_tag.as_mut() {
                                t.value = text.trim().to_string();
                            }
                        },
                        "Tag" => {
                            if let Some(t) = cur_tag.take() {
                                tags.push(t);
                            }
                        },
                        _ => {},
                    }
                    text.clear();
                },
                _ => {},
            }
        }
        Ok(Tagging { tags })
    }
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}
