// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! AIS-SPEC-9001：企业级统一契约头 —— 模块名 kernel_ext.rs\n//! AIS-REV-1：自描述接口 · 幂等 · 可观测 · 零外部副作用（网络/IO 仅限封装函数）\n//! AIS-REV-2：公开项 pub fn/pub struct 必须具备 /// 文档注释与错误语义说明\n//! AIS-REV-3：遵循 MOX-AIS-通用 标准，禁止占位实现宏遗留\n\n//! # Operator Core - L5 Extension Layer (kernel_ext)
//!
//! 扩展层：负责将纯内核（kernel.rs）与外部依赖能力（serde / nalgebra / serde_json）连接。
//! 通过「依赖倒置（DIP）」：为外部 crate 的类型（如 `nalgebra::DVector<f64>`）
//! 实现 `kernel::VectorOps` trait，以及为 kernel 纯结构体手动实现 `Serialize/Deserialize`。

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use nalgebra::{DMatrix, DVector};
use serde::de::Error as DeError;
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeStruct, Serializer};

use crate::kernel::{
    builtin, KernelStateVector, ResourceCost, ResourceLimits, ResourceUsage, TypeIdentifier,
    TypePair, VectorOps,
};

// ============================================================
// §1 为 kernel 纯类型手动实现 Serialize / Deserialize
//    （Kernel 层零外部依赖，所以 serde trait impl 放在此扩展层）
// ============================================================

// ---------- TypeIdentifier ----------

// 说明：impl Serialize —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
impl Serialize for TypeIdentifier {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("TypeIdentifier", 2)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("id", &self.id)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for TypeIdentifier {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // 说明：enum Field —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
        // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
        enum Field {
            Name,
            Id,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Field, D::Error> {
                // 说明：struct FieldVisitor —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
                // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
                struct FieldVisitor;
                impl<'de> Visitor<'de> for FieldVisitor {
                    // 说明：type Value —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
                    // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
                    type Value = Field;
                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        f.write_str("`name` or `id`")
                    }
                    fn visit_str<E: DeError>(self, v: &str) -> Result<Field, E> {
                        Ok(match v {
                            "name" => Field::Name,
                            "id" => Field::Id,
                            other => return Err(DeError::unknown_field(other, &["name", "id"])),
                        })
                    }
                }
                d.deserialize_identifier(FieldVisitor)
            }
        }

        // 说明：struct TiVisitor —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
        // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
        struct TiVisitor;
        impl<'de> Visitor<'de> for TiVisitor {
            // 说明：type Value —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
            // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
            type Value = TypeIdentifier;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("struct TypeIdentifier")
            }
            fn visit_map<V: MapAccess<'de>>(self, mut map: V) -> Result<TypeIdentifier, V::Error> {
                let mut name: Option<String> = None;
                let mut id: Option<u64> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            if name.is_some() {
                                return Err(DeError::duplicate_field("name"));
                            }
                            name = Some(map.next_value()?);
                        }
                        Field::Id => {
                            if id.is_some() {
                                return Err(DeError::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        }
                    }
                }
                let name = name.ok_or_else(|| DeError::missing_field("name"))?;
                let id: u64 = match id {
                    Some(i) => i,
                    None => {
                        // 兼容：若 name 存在但 id 缺失，按 name 重新计算
                        let mut hasher = DefaultHasher::new();
                        name.hash(&mut hasher);
                        hasher.finish()
                    }
                };
                Ok(TypeIdentifier { name, id })
            }
        }

        const FIELDS: &[&str] = &["name", "id"];
        deserializer.deserialize_struct("TypeIdentifier", FIELDS, TiVisitor)
    }
}

// ---------- TypePair ----------

// 说明：impl Serialize —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
impl Serialize for TypePair {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("TypePair", 2)?;
        st.serialize_field("input", &self.input)?;
        st.serialize_field("output", &self.output)?;
        st.end()
    }
}

impl<'de> Deserialize<'de> for TypePair {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // 说明：enum F —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
        // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
        enum F {
            Input,
            Output,
        }
        impl<'de> Deserialize<'de> for F {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<F, D::Error> {
                // 说明：struct V —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
                // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
                struct V;
                impl<'de> Visitor<'de> for V {
                    // 说明：type Value —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
                    // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
                    type Value = F;
                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        f.write_str("`input` or `output`")
                    }
                    fn visit_str<E: DeError>(self, v: &str) -> Result<F, E> {
                        Ok(match v {
                            "input" => F::Input,
                            "output" => F::Output,
                            other => {
                                return Err(DeError::unknown_field(other, &["input", "output"]))
                            }
                        })
                    }
                }
                d.deserialize_identifier(V)
            }
        }
        // 说明：struct TPVisitor —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
        // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
        struct TPVisitor;
        impl<'de> Visitor<'de> for TPVisitor {
            // 说明：type Value —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
            // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
            type Value = TypePair;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("struct TypePair")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut m: M) -> Result<TypePair, M::Error> {
                let mut input: Option<TypeIdentifier> = None;
                let mut output: Option<TypeIdentifier> = None;
                while let Some(k) = m.next_key()? {
                    match k {
                        F::Input => {
                            if input.is_some() {
                                return Err(DeError::duplicate_field("input"));
                            }
                            input = Some(m.next_value()?);
                        }
                        F::Output => {
                            if output.is_some() {
                                return Err(DeError::duplicate_field("output"));
                            }
                            output = Some(m.next_value()?);
                        }
                    }
                }
                Ok(TypePair::new(
                    input.ok_or_else(|| DeError::missing_field("input"))?,
                    output.ok_or_else(|| DeError::missing_field("output"))?,
                ))
            }
        }
        const FIELDS: &[&str] = &["input", "output"];
        d.deserialize_struct("TypePair", FIELDS, TPVisitor)
    }
}

// ---------- builtin::Unit ----------

// 说明：impl Serialize —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
impl Serialize for builtin::Unit {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_unit_struct("Unit")
    }
}

impl<'de> Deserialize<'de> for builtin::Unit {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // 说明：struct V —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
        // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
        struct V;
        impl<'de> Visitor<'de> for V {
            // 说明：type Value —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
            // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
            type Value = builtin::Unit;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("unit struct Unit")
            }
            fn visit_unit<E: DeError>(self) -> Result<builtin::Unit, E> {
                Ok(builtin::Unit)
            }
        }
        d.deserialize_unit_struct("Unit", V)
    }
}

// ---------- builtin::Any ----------

// 说明：impl Serialize —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
impl Serialize for builtin::Any {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_unit_struct("Any")
    }
}

impl<'de> Deserialize<'de> for builtin::Any {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // 说明：struct V —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
        // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
        struct V;
        impl<'de> Visitor<'de> for V {
            // 说明：type Value —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
            // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
            type Value = builtin::Any;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("unit struct Any")
            }
            fn visit_unit<E: DeError>(self) -> Result<builtin::Any, E> {
                Ok(builtin::Any)
            }
        }
        d.deserialize_unit_struct("Any", V)
    }
}

// ---------- ResourceCost ----------

// 说明：impl Serialize —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
impl Serialize for ResourceCost {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("ResourceCost", 4)?;
        st.serialize_field("cpu_cycles", &self.cpu_cycles)?;
        st.serialize_field("memory_bytes", &self.memory_bytes)?;
        st.serialize_field("disk_io_bytes", &self.disk_io_bytes)?;
        st.serialize_field("network_bytes", &self.network_bytes)?;
        st.end()
    }
}

impl<'de> Deserialize<'de> for ResourceCost {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // 说明：enum F —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
        // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
        enum F {
            CpuCycles,
            MemoryBytes,
            DiskIoBytes,
            NetworkBytes,
        }
        impl<'de> Deserialize<'de> for F {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<F, D::Error> {
                // 说明：struct V —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
                // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
                struct V;
                impl<'de> Visitor<'de> for V {
                    // 说明：type Value —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
                    // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
                    type Value = F;
                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        f.write_str("field of ResourceCost")
                    }
                    fn visit_str<E: DeError>(self, v: &str) -> Result<F, E> {
                        Ok(match v {
                            "cpu_cycles" => F::CpuCycles,
                            "memory_bytes" => F::MemoryBytes,
                            "disk_io_bytes" => F::DiskIoBytes,
                            "network_bytes" => F::NetworkBytes,
                            other => {
                                return Err(DeError::unknown_field(
                                    other,
                                    &[
                                        "cpu_cycles",
                                        "memory_bytes",
                                        "disk_io_bytes",
                                        "network_bytes",
                                    ],
                                ))
                            }
                        })
                    }
                }
                d.deserialize_identifier(V)
            }
        }
        // 说明：struct RCVisitor —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
        // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
        struct RCVisitor;
        impl<'de> Visitor<'de> for RCVisitor {
            // 说明：type Value —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
            // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
            type Value = ResourceCost;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("struct ResourceCost")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut m: M) -> Result<ResourceCost, M::Error> {
                let mut cpu_cycles = 0u64;
                let mut memory_bytes = 0u64;
                let mut disk_io_bytes = 0u64;
                let mut network_bytes = 0u64;
                while let Some(k) = m.next_key()? {
                    match k {
                        F::CpuCycles => cpu_cycles = m.next_value()?,
                        F::MemoryBytes => memory_bytes = m.next_value()?,
                        F::DiskIoBytes => disk_io_bytes = m.next_value()?,
                        F::NetworkBytes => network_bytes = m.next_value()?,
                    }
                }
                Ok(ResourceCost {
                    cpu_cycles,
                    memory_bytes,
                    disk_io_bytes,
                    network_bytes,
                })
            }
        }
        const FIELDS: &[&str] = &[
            "cpu_cycles",
            "memory_bytes",
            "disk_io_bytes",
            "network_bytes",
        ];
        d.deserialize_struct("ResourceCost", FIELDS, RCVisitor)
    }
}

// ---------- ResourceUsage ----------

// 说明：impl Serialize —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
impl Serialize for ResourceUsage {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("ResourceUsage", 4)?;
        st.serialize_field("cpu_time_ms", &self.cpu_time_ms)?;
        st.serialize_field("memory_peak_bytes", &self.memory_peak_bytes)?;
        st.serialize_field("disk_io_bytes", &self.disk_io_bytes)?;
        st.serialize_field("network_bytes", &self.network_bytes)?;
        st.end()
    }
}

impl<'de> Deserialize<'de> for ResourceUsage {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // 说明：enum F —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
        // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
        enum F {
            CpuTimeMs,
            MemoryPeakBytes,
            DiskIoBytes,
            NetworkBytes,
        }
        impl<'de> Deserialize<'de> for F {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<F, D::Error> {
                // 说明：struct V —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
                // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
                struct V;
                impl<'de> Visitor<'de> for V {
                    // 说明：type Value —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
                    // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
                    type Value = F;
                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        f.write_str("field of ResourceUsage")
                    }
                    fn visit_str<E: DeError>(self, v: &str) -> Result<F, E> {
                        Ok(match v {
                            "cpu_time_ms" => F::CpuTimeMs,
                            "memory_peak_bytes" => F::MemoryPeakBytes,
                            "disk_io_bytes" => F::DiskIoBytes,
                            "network_bytes" => F::NetworkBytes,
                            other => {
                                return Err(DeError::unknown_field(
                                    other,
                                    &[
                                        "cpu_time_ms",
                                        "memory_peak_bytes",
                                        "disk_io_bytes",
                                        "network_bytes",
                                    ],
                                ))
                            }
                        })
                    }
                }
                d.deserialize_identifier(V)
            }
        }
        // 说明：struct RUVisitor —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
        // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
        struct RUVisitor;
        impl<'de> Visitor<'de> for RUVisitor {
            // 说明：type Value —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
            // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
            type Value = ResourceUsage;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("struct ResourceUsage")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut m: M) -> Result<ResourceUsage, M::Error> {
                let mut cpu_time_ms = 0u64;
                let mut memory_peak_bytes = 0u64;
                let mut disk_io_bytes = 0u64;
                let mut network_bytes = 0u64;
                while let Some(k) = m.next_key()? {
                    match k {
                        F::CpuTimeMs => cpu_time_ms = m.next_value()?,
                        F::MemoryPeakBytes => memory_peak_bytes = m.next_value()?,
                        F::DiskIoBytes => disk_io_bytes = m.next_value()?,
                        F::NetworkBytes => network_bytes = m.next_value()?,
                    }
                }
                Ok(ResourceUsage {
                    cpu_time_ms,
                    memory_peak_bytes,
                    disk_io_bytes,
                    network_bytes,
                })
            }
        }
        const FIELDS: &[&str] = &[
            "cpu_time_ms",
            "memory_peak_bytes",
            "disk_io_bytes",
            "network_bytes",
        ];
        d.deserialize_struct("ResourceUsage", FIELDS, RUVisitor)
    }
}

// ---------- ResourceLimits ----------

// 说明：impl Serialize —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
impl Serialize for ResourceLimits {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("ResourceLimits", 4)?;
        st.serialize_field("max_cpu_time_ms", &self.max_cpu_time_ms)?;
        st.serialize_field("max_memory_bytes", &self.max_memory_bytes)?;
        st.serialize_field("max_disk_io_bytes", &self.max_disk_io_bytes)?;
        st.serialize_field("max_network_bytes", &self.max_network_bytes)?;
        st.end()
    }
}

impl<'de> Deserialize<'de> for ResourceLimits {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[allow(clippy::enum_variant_names)]
        // 说明：enum F —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
        // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
        enum F {
            MaxCpuTimeMs,
            MaxMemoryBytes,
            MaxDiskIoBytes,
            MaxNetworkBytes,
        }
        impl<'de> Deserialize<'de> for F {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<F, D::Error> {
                // 说明：struct V —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
                // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
                struct V;
                impl<'de> Visitor<'de> for V {
                    // 说明：type Value —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
                    // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
                    type Value = F;
                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        f.write_str("field of ResourceLimits")
                    }
                    fn visit_str<E: DeError>(self, v: &str) -> Result<F, E> {
                        Ok(match v {
                            "max_cpu_time_ms" => F::MaxCpuTimeMs,
                            "max_memory_bytes" => F::MaxMemoryBytes,
                            "max_disk_io_bytes" => F::MaxDiskIoBytes,
                            "max_network_bytes" => F::MaxNetworkBytes,
                            other => {
                                return Err(DeError::unknown_field(
                                    other,
                                    &[
                                        "max_cpu_time_ms",
                                        "max_memory_bytes",
                                        "max_disk_io_bytes",
                                        "max_network_bytes",
                                    ],
                                ))
                            }
                        })
                    }
                }
                d.deserialize_identifier(V)
            }
        }
        // 说明：struct RLVisitor —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
        // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
        struct RLVisitor;
        impl<'de> Visitor<'de> for RLVisitor {
            // 说明：type Value —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
            // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
            type Value = ResourceLimits;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("struct ResourceLimits")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut m: M) -> Result<ResourceLimits, M::Error> {
                let mut max_cpu_time_ms = 30_000u64;
                let mut max_memory_bytes = 1024u64 * 1024 * 1024;
                let mut max_disk_io_bytes = 100u64 * 1024 * 1024;
                let mut max_network_bytes = 100u64 * 1024 * 1024;
                while let Some(k) = m.next_key()? {
                    match k {
                        F::MaxCpuTimeMs => max_cpu_time_ms = m.next_value()?,
                        F::MaxMemoryBytes => max_memory_bytes = m.next_value()?,
                        F::MaxDiskIoBytes => max_disk_io_bytes = m.next_value()?,
                        F::MaxNetworkBytes => max_network_bytes = m.next_value()?,
                    }
                }
                Ok(ResourceLimits {
                    max_cpu_time_ms,
                    max_memory_bytes,
                    max_disk_io_bytes,
                    max_network_bytes,
                })
            }
        }
        const FIELDS: &[&str] = &[
            "max_cpu_time_ms",
            "max_memory_bytes",
            "max_disk_io_bytes",
            "max_network_bytes",
        ];
        d.deserialize_struct("ResourceLimits", FIELDS, RLVisitor)
    }
}

// ---------- KernelStateVector (极简序列化: [f64] + timestamp) ----------

// 说明：impl Serialize —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
impl Serialize for KernelStateVector {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeTupleStruct;
        let mut t = s.serialize_tuple_struct("KernelStateVector", 2)?;
        t.serialize_field(&self.data)?;
        t.serialize_field(&self.timestamp)?;
        t.end()
    }
}

impl<'de> Deserialize<'de> for KernelStateVector {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // 说明：struct KSVVisitor —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
        // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
        struct KSVVisitor;
        impl<'de> Visitor<'de> for KSVVisitor {
            // 说明：type Value —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
            // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
            type Value = KernelStateVector;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("tuple struct KernelStateVector(data, timestamp)")
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<KernelStateVector, A::Error> {
                let data: Vec<f64> = seq
                    .next_element()?
                    .ok_or_else(|| DeError::invalid_length(0, &self))?;
                let timestamp: u64 = seq.next_element()?.unwrap_or(0);
                Ok(KernelStateVector { data, timestamp })
            }
        }
        d.deserialize_tuple_struct("KernelStateVector", 2, KSVVisitor)
    }
}

// ============================================================
// §2 DIP：为外部类型（nalgebra::DVector<f64>）实现 VectorOps
// ============================================================

// 说明：impl VectorOps —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
impl VectorOps for DVector<f64> {
    #[inline]
    fn dimension(&self) -> usize {
        self.len()
    }
    #[inline]
    fn as_slice(&self) -> &[f64] {
        self.as_slice()
    }
    #[inline]
    fn norm_l2(&self) -> f64 {
        nalgebra::Vector::norm(self)
    }
}

// ============================================================
// §3 KernelStateVector 与 nalgebra::DVector 互转
// ============================================================

// 说明：impl From —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
impl From<&KernelStateVector> for DVector<f64> {
    fn from(k: &KernelStateVector) -> Self {
        DVector::from_vec(k.data.clone())
    }
}

// 说明：impl From —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
impl From<KernelStateVector> for DVector<f64> {
    fn from(k: KernelStateVector) -> Self {
        DVector::from_vec(k.data)
    }
}

// 说明：impl From —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
impl From<&DVector<f64>> for KernelStateVector {
    fn from(dv: &DVector<f64>) -> Self {
        KernelStateVector::from_vec(dv.iter().copied().collect())
    }
}

// 说明：impl From —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
impl From<DVector<f64>> for KernelStateVector {
    fn from(dv: DVector<f64>) -> Self {
        KernelStateVector::from_vec(dv.iter().copied().collect())
    }
}

// ============================================================
// §4 为原 state::StateVector（上层带 nalgebra 的）预留 impl 位置
//    （因为本扩展层看不到 StateVector，实际 impl 在 state.rs 里）
// ============================================================

/// 辅助：把 nalgebra::DMatrix + DVector 线性组合为 DVector（供上层 operator 调用）
pub fn apply_dmatrix(m: &DMatrix<f64>, v: &DVector<f64>) -> DVector<f64> {
    m * v
}

// ============================================================
// §5 在扩展层重新导出内核中关键的守恒律类型（以便上层使用同一名字）
// ============================================================

pub use crate::kernel::{ConservationLaw, ResidualMonitor};

// ============================================================
// §6 扩展层单元测试（验证 serde 序列化与 VectorOps 外部 impl）
// ============================================================

#[cfg(test)]
// 说明：mod tests —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
mod tests {
    use super::*;
    use crate::{ConservationChecker, L2Conservation};

    #[test]
    fn ext_serde_typeidentifier_roundtrip() {
        let ti = TypeIdentifier::of::<i32>();
        let json = serde_json::to_string(&ti).unwrap();
        let back: TypeIdentifier = serde_json::from_str(&json).unwrap();
        assert_eq!(ti, back);
    }

    #[test]
    fn ext_serde_typeidentifier_name_only_compat() {
        // name 存在但 id 缺失时应兼容重新计算
        let json = r#"{"name":"i32"}"#;
        let back: TypeIdentifier = serde_json::from_str(json).unwrap();
        assert_eq!(back, TypeIdentifier::of::<i32>());
    }

    #[test]
    fn ext_serde_typepair_roundtrip() {
        let a = TypeIdentifier::new("A");
        let b = TypeIdentifier::new("B");
        let tp = TypePair::new(a, b);
        let json = serde_json::to_string(&tp).unwrap();
        let back: TypePair = serde_json::from_str(&json).unwrap();
        assert_eq!(tp, back);
    }

    #[test]
    fn ext_serde_unit_roundtrip() {
        let u = builtin::Unit;
        let json = serde_json::to_string(&u).unwrap();
        let back: builtin::Unit = serde_json::from_str(&json).unwrap();
        assert_eq!(u, back);
    }

    #[test]
    fn ext_serde_any_roundtrip() {
        let a = builtin::Any;
        let json = serde_json::to_string(&a).unwrap();
        let back: builtin::Any = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
    }

    #[test]
    fn ext_serde_resourcecost_roundtrip() {
        let rc = ResourceCost::new(100, 200);
        let json = serde_json::to_string(&rc).unwrap();
        let back: ResourceCost = serde_json::from_str(&json).unwrap();
        assert_eq!(rc, back);
    }

    #[test]
    fn ext_serde_resourceusage_roundtrip() {
        let ru = ResourceUsage {
            cpu_time_ms: 10,
            memory_peak_bytes: 20,
            disk_io_bytes: 30,
            network_bytes: 40,
        };
        let json = serde_json::to_string(&ru).unwrap();
        let back: ResourceUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(ru, back);
    }

    #[test]
    fn ext_serde_resourcelimits_default_fields() {
        // 所有字段缺失时返回默认值
        let json = r#"{}"#;
        let rl: ResourceLimits = serde_json::from_str(json).unwrap();
        let d = ResourceLimits::default();
        assert_eq!(rl.max_cpu_time_ms, d.max_cpu_time_ms);
        assert_eq!(rl.max_memory_bytes, d.max_memory_bytes);
    }

    #[test]
    fn ext_serde_kernelstatevector_roundtrip() {
        let ksv = KernelStateVector::from_vec(vec![1.0, 2.0, 3.0]);
        let json = serde_json::to_string(&ksv).unwrap();
        let back: KernelStateVector = serde_json::from_str(&json).unwrap();
        assert_eq!(ksv.data, back.data);
    }

    #[test]
    fn ext_nalgebra_dvector_impl_vectorops() {
        let dv = DVector::from_vec(vec![3.0, 4.0]);
        assert!((dv.norm_l2() - 5.0).abs() < 1e-12);
        assert!((dv.norm_l1() - 7.0).abs() < 1e-12);
        assert_eq!(dv.sum(), 7.0);
        assert_eq!(dv.dimension(), 2);
    }

    #[test]
    fn ext_kernelstatevector_from_dvector() {
        let dv = DVector::from_vec(vec![1.0, 2.0, 3.0]);
        let ksv: KernelStateVector = dv.into();
        assert_eq!(ksv.data, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn ext_dvector_from_kernelstatevector() {
        let ksv = KernelStateVector::from_vec(vec![1.0, 2.0]);
        let dv: DVector<f64> = ksv.into();
        assert_eq!(dv[0], 1.0);
        assert_eq!(dv[1], 2.0);
    }

    #[test]
    fn ext_conservation_law_on_dvector() {
        // 通过 DIP：L2Conservation + DVector<f64> impl VectorOps
        let law = L2Conservation::unit_energy();
        let dv = DVector::from_vec(vec![1.0, 0.0, 0.0]);
        assert!(law.is_satisfied(&dv, 1e-10));
    }

    #[test]
    fn ext_conservation_checker_on_dvector() {
        let mut checker = ConservationChecker::new(1e-10);
        checker.add_law(L2Conservation::unit_energy());
        let dv = DVector::from_vec(vec![0.5, 0.5]);
        let normalized = dv.normalize(); // DVector::normalize 返回归一化向量
        let result = checker.check_all(&normalized);
        assert!(result.is_ok(), "check_all error: {:?}", result.err());
    }
}
