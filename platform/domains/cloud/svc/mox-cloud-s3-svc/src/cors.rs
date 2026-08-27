// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! CORS 规则实现：AllowedOrigins/Methods/Headers/ExposeHeaders/MaxAgeSeconds + OPTIONS 预检。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorsRule {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub expose_headers: Vec<String>,
    pub max_age_seconds: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorsConfiguration {
    pub rules: Vec<CorsRule>,
}

impl CorsConfiguration {
    pub fn to_xml(&self) -> String {
        let mut inner = String::new();
        for r in &self.rules {
            inner.push_str("  <CORSRule>\n");
            for o in &r.allowed_origins {
                inner.push_str(&format!("    <AllowedOrigin>{}</AllowedOrigin>\n", o));
            }
            for m in &r.allowed_methods {
                inner.push_str(&format!("    <AllowedMethod>{}</AllowedMethod>\n", m));
            }
            for h in &r.allowed_headers {
                inner.push_str(&format!("    <AllowedHeader>{}</AllowedHeader>\n", h));
            }
            for e in &r.expose_headers {
                inner.push_str(&format!("    <ExposeHeader>{}</ExposeHeader>\n", e));
            }
            if r.max_age_seconds > 0 {
                inner.push_str(&format!(
                    "    <MaxAgeSeconds>{}</MaxAgeSeconds>\n",
                    r.max_age_seconds
                ));
            }
            inner.push_str("  </CORSRule>\n");
        }
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<CORSConfiguration xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\n{}</CORSConfiguration>",
            inner
        )
    }

    pub fn from_xml(xml: &str) -> Result<Self, String> {
        let reader = xml::EventReader::from_str(xml);
        let mut rules: Vec<CorsRule> = Vec::new();
        let mut current: Option<CorsRule> = None;
        let mut current_text = String::new();
        for e in reader {
            use xml::reader::XmlEvent;
            match e.map_err(|x| x.to_string())? {
                XmlEvent::StartElement { name, .. } => {
                    current_text.clear();
                    if name.local_name == "CORSRule" {
                        current = Some(CorsRule::default());
                    }
                }
                XmlEvent::Characters(s) => {
                    current_text.push_str(&s);
                }
                XmlEvent::EndElement { name, .. } => {
                    let tag = name.local_name.as_str();
                    if let Some(rule) = current.as_mut() {
                        match tag {
                            "AllowedOrigin" => {
                                rule.allowed_origins.push(current_text.trim().to_string())
                            }
                            "AllowedMethod" => {
                                rule.allowed_methods.push(current_text.trim().to_string())
                            }
                            "AllowedHeader" => {
                                rule.allowed_headers.push(current_text.trim().to_string())
                            }
                            "ExposeHeader" => {
                                rule.expose_headers.push(current_text.trim().to_string())
                            }
                            "MaxAgeSeconds" => {
                                rule.max_age_seconds = current_text.trim().parse().unwrap_or(0)
                            }
                            "CORSRule" => {
                                rules.push(current.take().unwrap());
                            }
                            _ => {}
                        }
                    }
                    current_text.clear();
                }
                _ => {}
            }
        }
        Ok(CorsConfiguration { rules })
    }

    /// 匹配预检请求，返回匹配到的 rule。
    pub fn match_preflight(&self, origin: &str, method: &str) -> Option<&CorsRule> {
        self.rules.iter().find(|r| {
            let origin_ok = r.allowed_origins.iter().any(|o| o == "*" || o == origin);
            let method_ok = r
                .allowed_methods
                .iter()
                .any(|m| m == "*" || m.eq_ignore_ascii_case(method));
            origin_ok && method_ok
        })
    }
}
