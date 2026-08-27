//! 字段规格与槽位分配器
//!
//! 将业务字段映射到通用 biz_data 表的预定义扩展槽位列，
//! 支持 string/int/decimal/bool/timestamp 五种类型，按优先级评分分配最优槽位。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 字段规格定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSpec {
    pub field_code: String,
    pub field_type: String,
    pub is_required: bool,
    pub is_indexed: bool,
    pub is_searchable: bool,
    pub is_sortable: bool,
    pub is_filterable: bool,
    pub options_inline: Option<Vec<String>>,
}

/// 槽位分配结果
#[derive(Debug, Clone)]
pub struct SlotAllocation {
    pub slot_name: String,
    pub slot_type: SlotType,
    pub priority_score: u32,
}

/// 槽位类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotType {
    Str,
    Int,
    Dec,
    Bool,
    Ts,
}

impl SlotType {
    pub fn prefix(&self) -> &'static str {
        match self {
            SlotType::Str => "ext_str_",
            SlotType::Int => "ext_int_",
            SlotType::Dec => "ext_dec_",
            SlotType::Bool => "ext_bool_",
            SlotType::Ts => "ext_ts_",
        }
    }

    /// 每种类型的槽位数量
    pub fn count(&self) -> usize {
        match self {
            SlotType::Str => 16,
            SlotType::Int => 8,
            SlotType::Dec => 8,
            SlotType::Bool => 4,
            SlotType::Ts => 4,
        }
    }

    /// 从 field_type 字符串映射到 SlotType
    pub fn from_field_type(field_type: &str) -> Self {
        match field_type.to_lowercase().as_str() {
            "int" | "integer" | "long" | "bigint" => SlotType::Int,
            "decimal" | "float" | "double" | "number" | "numeric" => SlotType::Dec,
            "bool" | "boolean" => SlotType::Bool,
            "timestamp" | "datetime" | "date" | "time" => SlotType::Ts,
            _ => SlotType::Str,
        }
    }
}

/// 计算字段优先级评分（用于槽位分配排序）
/// 权重: required=32, indexed=16, searchable=8, sortable=4, filterable=2
pub fn field_priority_score(f: &FieldSpec) -> u32 {
    let mut score = 0u32;
    if f.is_required { score += 32; }
    if f.is_indexed { score += 16; }
    if f.is_searchable { score += 8; }
    if f.is_sortable { score += 4; }
    if f.is_filterable { score += 2; }
    score
}

/// 字段槽位分配器
pub struct FieldSlotAllocator;

impl FieldSlotAllocator {
    /// 为指定实体类型的字段列表分配槽位
    /// 返回 field_code -> SlotAllocation 的映射
    pub fn allocate(_entity_type: &str, fields: &[FieldSpec]) -> HashMap<String, SlotAllocation> {
        let mut result = HashMap::new();

        // 按类型分组
        let mut by_type: HashMap<SlotType, Vec<&FieldSpec>> = HashMap::new();
        for f in fields {
            let st = SlotType::from_field_type(&f.field_type);
            by_type.entry(st).or_default().push(f);
        }

        // 对每种类型，按优先级降序分配槽位
        for (slot_type, type_fields) in by_type {
            let mut sorted: Vec<&&FieldSpec> = type_fields.iter().collect();
            sorted.sort_by(|a, b| field_priority_score(b).cmp(&field_priority_score(a)));

            let max_slots = slot_type.count();
            for (idx, f) in sorted.iter().enumerate() {
                if idx >= max_slots {
                    // 超出槽位数量的字段放入 data_json（不分配列槽位）
                    // 这里仍记录但标记为 overflow
                    result.insert(f.field_code.clone(), SlotAllocation {
                        slot_name: format!("{}_overflow_{}", slot_type.prefix(), idx - max_slots),
                        slot_type,
                        priority_score: field_priority_score(f),
                    });
                } else {
                    result.insert(f.field_code.clone(), SlotAllocation {
                        slot_name: format!("{}{}", slot_type.prefix(), idx),
                        slot_type,
                        priority_score: field_priority_score(f),
                    });
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_type_mapping() {
        assert_eq!(SlotType::from_field_type("string"), SlotType::Str);
        assert_eq!(SlotType::from_field_type("int"), SlotType::Int);
        assert_eq!(SlotType::from_field_type("decimal"), SlotType::Dec);
        assert_eq!(SlotType::from_field_type("bool"), SlotType::Bool);
        assert_eq!(SlotType::from_field_type("timestamp"), SlotType::Ts);
    }

    #[test]
    fn test_priority_score() {
        let f = FieldSpec {
            field_code: "x".into(), field_type: "string".into(),
            is_required: true, is_indexed: true, is_searchable: false,
            is_sortable: false, is_filterable: false, options_inline: None,
        };
        assert_eq!(field_priority_score(&f), 48); // 32 + 16
    }

    #[test]
    fn test_allocate_basic() {
        let fields = vec![
            FieldSpec { field_code: "title".into(), field_type: "string".into(), is_required: true, is_indexed: true, is_searchable: true, is_sortable: true, is_filterable: true, options_inline: None },
            FieldSpec { field_code: "amount".into(), field_type: "decimal".into(), is_required: false, is_indexed: false, is_searchable: false, is_sortable: true, is_filterable: true, options_inline: None },
        ];
        let map = FieldSlotAllocator::allocate("project", &fields);
        assert!(map.contains_key("title"));
        assert!(map.contains_key("amount"));
        assert!(map.get("title").unwrap().slot_name.starts_with("ext_str_"));
        assert!(map.get("amount").unwrap().slot_name.starts_with("ext_dec_"));
    }
}
