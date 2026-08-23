// xuanji-common-meta: T2 Green version
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AisLayer {
    L2Gateway,
    L3Orchestration,
    L4Services,
    L5Domain,
    L6Kernel,
    L6KernelExt,
    L7Infrastructure,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrateMeta {
    pub id: &'static str,
    pub name: &'static str,
    pub version: &'static str,
    pub layer: AisLayer,
    pub owner: &'static str,
}

impl CrateMeta {
    pub fn engine_name(&self) -> String {
        format!("xuanji::{}", self.name.replace('-', "_"))
    }
}

pub const CRATE_ID: &str = "34a20231-1a80-5426-b392-40d7a2ddd9f7";
pub const ENGINE_NAME: &str = "xuanji::xuanji_common_meta";
pub const CRATE_META: CrateMeta = CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: AisLayer::L5Domain,
    owner: "xuanji-core",
};

pub fn all_crate_metas() -> Vec<CrateMeta> {
    vec![
        CrateMeta {
            id: "00374bdd-cc60-55bf-8970-a879afbfe443",
            name: "ai-agent",
            version: "3.0.0-ai-powered",
            layer: AisLayer::L4Services,
            owner: "xuanji-core",
        },
        CrateMeta {
            id: "62b2cca1-d98f-5e41-b26e-8d2a43966117",
            name: "business-catalog",
            version: "0.1.0",
            layer: AisLayer::L4Services,
            owner: "xuanji-core",
        },
        CrateMeta {
            id: "2fcd3eac-e894-5876-b007-fb33c56c0d65",
            name: "flow-ai",
            version: "3.0.0-ai-powered",
            layer: AisLayer::L4Services,
            owner: "xuanji-core",
        },
        CrateMeta {
            id: "fbd31c6a-41cd-5274-be2f-2a28066eaf0a",
            name: "graph-algorithms",
            version: "3.0.0-ai-powered",
            layer: AisLayer::L4Services,
            owner: "xuanji-core",
        },
        CrateMeta {
            id: "9bfaf43b-385a-5a44-9fb2-65b4003ee80d",
            name: "hermes-flow-bridge",
            version: "0.1.0",
            layer: AisLayer::L4Services,
            owner: "xuanji-core",
        },
        CrateMeta {
            id: "cb909f06-c0df-55ec-b397-543623a8c349",
            name: "kg-hub",
            version: "3.0.0-ai-powered",
            layer: AisLayer::L4Services,
            owner: "xuanji-core",
        },
        CrateMeta {
            id: "acf14283-3931-5528-adce-2c0cd3815363",
            name: "operator-core",
            version: "3.0.0-ai-powered",
            layer: AisLayer::L6Kernel,
            owner: "xuanji-core",
        },
        CrateMeta {
            id: "5a1df407-b217-5340-a5ae-5f4535d1e6de",
            name: "operator-wasm",
            version: "3.0.0-ai-powered",
            layer: AisLayer::L4Services,
            owner: "xuanji-core",
        },
        CrateMeta {
            id: "e56676c7-ec1f-5415-9587-ba8249d0178a",
            name: "optimizer",
            version: "3.0.0-ai-powered",
            layer: AisLayer::L4Services,
            owner: "xuanji-core",
        },
        CrateMeta {
            id: "8c8d2382-6f9f-5218-894e-a07a43aa9554",
            name: "primiflow-core",
            version: "3.0.0-ai-powered",
            layer: AisLayer::L4Services,
            owner: "xuanji-core",
        },
        CrateMeta {
            id: "75238345-b48b-534b-818b-8d9abe083a41",
            name: "primiflow-fusion",
            version: "0.1.0",
            layer: AisLayer::L4Services,
            owner: "xuanji-core",
        },
        CrateMeta {
            id: "4d2e50c1-9d64-525d-86cf-2d7d610a27b9",
            name: "template-market",
            version: "3.0.0-ai-powered",
            layer: AisLayer::L4Services,
            owner: "xuanji-core",
        },
        CrateMeta {
            id: "50bb6200-04c5-5e4c-8354-4c6e1b230024",
            name: "xuanji-expert",
            version: "0.1.0",
            layer: AisLayer::L4Services,
            owner: "xuanji-core",
        },
        CrateMeta {
            id: "b81eec75-22ff-5155-ac49-19edf6f6b5ab",
            name: "xuanji-system",
            version: "0.1.0",
            layer: AisLayer::L7Infrastructure,
            owner: "xuanji-core",
        },
        CrateMeta {
            id: "a6f7ad5c-dbc8-5c27-837f-d8332fd6f27b",
            name: "runtime",
            version: "3.0.0-ai-powered",
            layer: AisLayer::L3Orchestration,
            owner: "xuanji-core",
        },
        CrateMeta {
            id: "34a20231-1a80-5426-b392-40d7a2ddd9f7",
            name: "xuanji-common-meta",
            version: "3.0.0-ai-powered",
            layer: AisLayer::L5Domain,
            owner: "xuanji-core",
        },
    ]
}

pub fn lookup_meta_by_engine(name: &str) -> Option<CrateMeta> {
    all_crate_metas()
        .into_iter()
        .find(|m| m.engine_name() == name)
}
