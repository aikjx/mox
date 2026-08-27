// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use crate::port::FieldSpec;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotCategory {
    String,
    Text,
    Json,
    Int,
    Decimal,
    Date,
    DateTime,
    Bool,
    Overflow,
}

#[derive(Debug, Clone)]
pub struct SlotInfo {
    pub slot_name: String,
    pub category: SlotCategory,
    pub priority_score: u32,
}

pub struct FieldSlotAllocator;

impl FieldSlotAllocator {
    pub fn allocate(_entity_code: &str, fields: &[FieldSpec]) -> HashMap<String, SlotInfo> {
        let mut scored: Vec<(u32, usize, FieldSpec)> = fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let mut score = 0u32;
                if f.is_indexed {
                    score += 32;
                }
                if f.is_searchable {
                    score += 16;
                }
                if f.is_required {
                    score += 8;
                }
                if f.is_filterable {
                    score += 4;
                }
                if f.is_sortable {
                    score += 2;
                }
                (score, i, f.clone())
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

        let mut result = HashMap::new();
        let mut str_used = 0usize;
        let mut text_used = 0usize;
        let mut json_used = 0usize;
        let mut int_used = 0usize;
        let mut dec_used = 0usize;
        let mut date_used = 0usize;
        let mut dt_used = 0usize;
        let mut bool_used = 0usize;

        for (score, _idx, f) in scored {
            let ft = f.field_type.to_ascii_lowercase();
            let (category, slot_opt): (SlotCategory, Option<String>) = match ft.as_str() {
                "string" | "str" | "varchar" | "text"
                    if ft == "string" || ft == "str" || ft == "varchar" =>
                {
                    (
                        SlotCategory::String,
                        if str_used < 12 {
                            str_used += 1;
                            Some(format!("ext_str_{:02}", str_used))
                        } else {
                            None
                        },
                    )
                }
                "text" | "clob" => (
                    SlotCategory::Text,
                    if text_used < 2 {
                        text_used += 1;
                        Some(format!("ext_text_{:02}", text_used))
                    } else {
                        None
                    },
                ),
                "json" | "jsonb" | "object" => (
                    SlotCategory::Json,
                    if json_used < 4 {
                        json_used += 1;
                        Some(format!("ext_json_{:02}", json_used))
                    } else {
                        None
                    },
                ),
                "int" | "integer" | "long" | "bigint" => (
                    SlotCategory::Int,
                    if int_used < 5 {
                        int_used += 1;
                        Some(format!("ext_int_{:02}", int_used))
                    } else {
                        None
                    },
                ),
                "decimal" | "number" | "float" | "double" | "numeric" => (
                    SlotCategory::Decimal,
                    if dec_used < 5 {
                        dec_used += 1;
                        Some(format!("ext_dec_{:02}", dec_used))
                    } else {
                        None
                    },
                ),
                "date" => (
                    SlotCategory::Date,
                    if date_used < 1 {
                        date_used += 1;
                        Some("ext_date_01".to_string())
                    } else {
                        None
                    },
                ),
                "datetime" | "timestamp" | "time" => (
                    SlotCategory::DateTime,
                    if dt_used < 1 {
                        dt_used += 1;
                        Some("ext_datetime_01".to_string())
                    } else {
                        None
                    },
                ),
                "bool" | "boolean" => (
                    SlotCategory::Bool,
                    if bool_used < 3 {
                        bool_used += 1;
                        Some(format!("ext_bool_{:02}", bool_used))
                    } else {
                        None
                    },
                ),
                "enum" => (
                    SlotCategory::String,
                    if str_used < 12 {
                        str_used += 1;
                        Some(format!("ext_str_{:02}", str_used))
                    } else {
                        None
                    },
                ),
                _ => (SlotCategory::Overflow, None),
            };

            let (slot_name, final_cat) = match slot_opt {
                Some(s) => (s, category),
                None => ("dynamic_data".to_string(), SlotCategory::Overflow),
            };

            result.insert(
                f.field_code.clone(),
                SlotInfo {
                    slot_name,
                    category: final_cat,
                    priority_score: score,
                },
            );
        }

        result
    }
}
