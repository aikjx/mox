//! GraphQL Schema管理 — GraphQL Schema Registry

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// GraphQL端点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLEndpoint {
    /// 端点路径（如 /graphql）
    pub path: String,
    /// Schema名称
    pub schema_name: String,
    /// 是否启用GraphiQL
    #[serde(default = "default_true")]
    pub graphiql_enabled: bool,
    /// 是否启用订阅（WebSocket）
    #[serde(default)]
    pub subscriptions_enabled: bool,
    /// 元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_true() -> bool { true }

/// GraphQL Schema（简化版，实际应使用async-graphql）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLSchema {
    /// Schema名称
    pub name: String,
    /// Schema版本
    pub version: String,
    /// Query类型定义
    pub query_type: String,
    /// Mutation类型定义
    #[serde(default)]
    pub mutation_type: Option<String>,
    /// Subscription类型定义
    #[serde(default)]
    pub subscription_type: Option<String>,
    /// 类型定义列表
    #[serde(default)]
    pub types: Vec<GraphQLType>,
    /// 端点配置
    pub endpoint: GraphQLEndpoint,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// GraphQL类型定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLType {
    /// 类型名
    pub name: String,
    /// 类型种类（OBJECT/INTERFACE/ENUM/SCALAR/INPUT_OBJECT/UNION）
    pub kind: String,
    /// 字段列表
    #[serde(default)]
    pub fields: Vec<GraphQLField>,
    /// 描述
    #[serde(default)]
    pub description: String,
}

/// GraphQL字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLField {
    pub name: String,
    pub return_type: String,
    #[serde(default)]
    pub args: Vec<GraphQLArgument>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub deprecated: bool,
}

/// GraphQL参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLArgument {
    pub name: String,
    pub arg_type: String,
    #[serde(default)]
    pub default_value: Option<serde_json::Value>,
    #[serde(default)]
    pub description: String,
}

/// GraphQL Schema注册表
pub struct GraphQLSchemaRegistry {
    schemas: RwLock<HashMap<String, Arc<GraphQLSchema>>>,
}

impl GraphQLSchemaRegistry {
    pub fn new() -> Self {
        Self { schemas: RwLock::new(HashMap::new()) }
    }

    /// 注册Schema
    pub fn register(&self, schema: GraphQLSchema) {
        let name = schema.name.clone();
        tracing::info!("register GraphQL schema: {} at {}", name, schema.endpoint.path);
        self.schemas.write().insert(name, Arc::new(schema));
    }

    /// 注销Schema
    pub fn unregister(&self, name: &str) -> Option<Arc<GraphQLSchema>> {
        self.schemas.write().remove(name)
    }

    /// 获取Schema
    pub fn get(&self, name: &str) -> Option<Arc<GraphQLSchema>> {
        self.schemas.read().get(name).cloned()
    }

    /// 按路径查找Schema
    pub fn get_by_path(&self, path: &str) -> Option<Arc<GraphQLSchema>> {
        self.schemas.read()
            .values()
            .find(|s| s.endpoint.path == path)
            .cloned()
    }

    /// 列出所有Schema
    pub fn list(&self) -> Vec<Arc<GraphQLSchema>> {
        self.schemas.read().values().cloned().collect()
    }

    /// 生成Schema introspection响应
    pub fn introspection_response(&self, schema_name: &str) -> Option<serde_json::Value> {
        self.get(schema_name).map(|s| serde_json::json!({
            "__schema": {
                "queryType": { "name": s.query_type },
                "mutationType": s.mutation_type.as_ref().map(|t| serde_json::json!({"name": t})),
                "subscriptionType": s.subscription_type.as_ref().map(|t| serde_json::json!({"name": t})),
                "types": s.types.iter().map(|t| serde_json::json!({
                    "kind": t.kind,
                    "name": t.name,
                    "description": t.description,
                    "fields": t.fields.iter().map(|f| serde_json::json!({
                        "name": f.name,
                        "type": { "name": f.return_type },
                        "args": f.args.iter().map(|a| serde_json::json!({
                            "name": a.name,
                            "type": { "name": a.arg_type },
                        })).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            }
        }))
    }

    /// Schema数量
    pub fn len(&self) -> usize { self.schemas.read().len() }
    pub fn is_empty(&self) -> bool { self.schemas.read().is_empty() }
}

impl Default for GraphQLSchemaRegistry {
    fn default() -> Self { Self::new() }
}
