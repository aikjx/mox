//! Step 4：复用模板路由（本地缓存版最短路径点亮）。
//!
//! 设计：把历史「工具序列模板」存为键，新调用若匹配则标记 `source="flow-template:<id>"`，
//! Hermes 上游读到该注解即可走轻量执行、跳过完整 ReAct（由 Hermes 侧读取注解实现；
//! bridge 只负责标注）。这是 flow-ai `TopologyGraph::route` 的轻量同步投影，避免在同步中间件里跑图算法。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 一个复用的流程图模板：用规范化工具序列作键。
#[derive(Debug, Clone)]
pub struct FlowTemplate {
    pub id: String,
    pub tool_seq: Vec<String>,
}

#[derive(Clone, Default)]
pub struct Router {
    /// 工具序列（join 成字符串）→ 模板 id
    index: Arc<Mutex<HashMap<String, String>>>,
    templates: Arc<Mutex<HashMap<String, FlowTemplate>>>,
}

impl Router {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一个可复用模板（来自 xuanji-expert 关系网的最短路径挖掘结果）。
    pub fn register(&self, tpl: FlowTemplate) {
        let key = tpl.tool_seq.join("|");
        let mut idx = self.index.lock().unwrap();
        let mut tpls = self.templates.lock().unwrap();
        idx.insert(key, tpl.id.clone());
        tpls.insert(tpl.id.clone(), tpl);
    }

    /// 尝试匹配当前回合工具序列，返回命中模板 id（并点亮复用路径）。
    /// 与 `TopologyGraph::route` 语义一致：fast-path 命中即跳过完整推理。
    pub fn match_template(&self, recent_tools: &[String]) -> Option<String> {
        if recent_tools.is_empty() {
            return None;
        }
        let key = recent_tools.join("|");
        self.index.lock().unwrap().get(&key).cloned()
    }

    /// 前缀匹配：某模板序列是 `recent_tools` 的**前缀** → 返回模板 id。
    /// 即 `recent_tools[..tpl.tool_seq.len()] == tpl.tool_seq`。
    /// agent 据此可回放「已知前缀」部分、跳过其 LLM 决策，仅对尾部未知步骤调 LLM。
    pub fn match_prefix(&self, recent_tools: &[String]) -> Option<String> {
        if recent_tools.is_empty() {
            return None;
        }
        let tpls = self.templates.lock().unwrap();
        for tpl in tpls.values() {
            if tpl.tool_seq.len() <= recent_tools.len()
                && tpl.tool_seq[..] == recent_tools[..tpl.tool_seq.len()]
            {
                return Some(tpl.id.clone());
            }
        }
        None
    }

    /// 取已注册模板（agent 拿到模板 id 后回放其工具序列）。
    pub fn get_template(&self, id: &str) -> Option<FlowTemplate> {
        self.templates.lock().unwrap().get(id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_and_matches_template() {
        let r = Router::new();
        r.register(FlowTemplate {
            id: "gov-pii".into(),
            tool_seq: vec!["db.read".into(), "guard.desensitize".into(), "web1".into()],
        });
        let hit = r.match_template(&["db.read".into(), "guard.desensitize".into(), "web1".into()]);
        assert_eq!(hit, Some("gov-pii".into()));
    }

    #[test]
    fn no_match_for_unknown_seq() {
        let r = Router::new();
        assert_eq!(r.match_template(&["unknown.tool".into()]), None);
    }
}
