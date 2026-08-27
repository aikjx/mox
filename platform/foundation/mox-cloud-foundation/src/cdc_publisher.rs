// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use async_trait::async_trait;
use std::collections::{BTreeMap, VecDeque};
use std::error::Error;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CdcEvent {
    pub event_id: String,
    pub topic: String,
    pub event_type: String,
    pub payload_json: String,
    pub timestamp: u64,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CdcSubscription {
    pub subscription_id: String,
    pub topic: String,
    pub consumer_group: String,
    pub last_offset: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConsumerLag {
    pub topic: String,
    pub consumer_group: String,
    pub latest_offset: u64,
    pub committed_offset: u64,
    pub lag: u64,
}

#[derive(Debug, Clone, Default)]
struct TopicState {
    events: VecDeque<CdcEvent>,
    next_offset: u64,
    committed: BTreeMap<String, u64>,
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
fn esc_json(s: &str) -> String {
    let mut r = String::with_capacity(s.len() + 2);
    r.push('\"');
    for c in s.chars() {
        match c {
            '\"' => r.push_str("\\\""),
            '\\' => r.push_str("\\\\"),
            '\n' => r.push_str("\\n"),
            '\r' => r.push_str("\\r"),
            '\t' => r.push_str("\\t"),
            c if (c as u32) < 0x20 => r.push_str(&format!("\\u{:04x}", c as u32)),
            c => r.push(c),
        }
    }
    r.push('\"');
    r
}
fn json_props(props: &BTreeMap<String, String>) -> String {
    let mut s = String::from("{");
    let mut first = true;
    for (k, v) in props {
        if !first {
            s.push(',');
        }
        first = false;
        s.push_str(&esc_json(k));
        s.push(':');
        s.push_str(&esc_json(v));
    }
    s.push('}');
    s
}
fn json_tags(tags: &[String]) -> String {
    let mut s = String::from("[");
    for (i, t) in tags.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&esc_json(t));
    }
    s.push(']');
    s
}

#[async_trait]
pub trait CdcPublisherProvider: Send + Sync {
    async fn emit_vertex_created(
        &self,
        sp: &str,
        vid: &str,
        tags: Vec<String>,
        props: BTreeMap<String, String>,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;
    async fn emit_vertex_updated(
        &self,
        sp: &str,
        vid: &str,
        changed_props: BTreeMap<String, String>,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;
    async fn emit_vertex_deleted(
        &self,
        sp: &str,
        vid: &str,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;
    async fn emit_edge_created(
        &self,
        sp: &str,
        s: &str,
        d: &str,
        et: &str,
        rk: i64,
        props: BTreeMap<String, String>,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;
    async fn emit_edge_deleted(
        &self,
        sp: &str,
        s: &str,
        d: &str,
        et: &str,
        rk: i64,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;
    async fn subscribe(
        &self,
        topic: &str,
        cg: &str,
    ) -> Result<CdcSubscription, Box<dyn Error + Send + Sync>>;
    async fn list_topics(&self) -> Result<Vec<String>, Box<dyn Error + Send + Sync>>;
    async fn get_consumer_lag(
        &self,
        topic: &str,
        cg: &str,
    ) -> Result<ConsumerLag, Box<dyn Error + Send + Sync>>;
    async fn commit_offset(
        &self,
        sub_id: &str,
        new_offset: u64,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}

pub struct MockCdcPublisherProvider {
    t: parking_lot::Mutex<BTreeMap<String, TopicState>>,
    subs: parking_lot::Mutex<BTreeMap<String, CdcSubscription>>,
    ev: parking_lot::Mutex<u64>,
    sid: parking_lot::Mutex<u64>,
}
impl Default for MockCdcPublisherProvider {
    fn default() -> Self {
        Self {
            t: parking_lot::Mutex::new(BTreeMap::new()),
            subs: parking_lot::Mutex::new(BTreeMap::new()),
            ev: parking_lot::Mutex::new(1),
            sid: parking_lot::Mutex::new(1),
        }
    }
}

impl MockCdcPublisherProvider {
    fn emit(&self, topic: &str, event_type: &str, payload_json: &str, source: &str) -> String {
        let mut ts = self.t.lock();
        let mut ev = self.ev.lock();
        let eid = format!("evt-{}", *ev);
        *ev += 1;
        let evt = CdcEvent {
            event_id: eid.clone(),
            topic: topic.into(),
            event_type: event_type.into(),
            payload_json: payload_json.into(),
            timestamp: now_ms(),
            source: source.into(),
        };
        let st = ts.entry(topic.into()).or_default();
        st.events.push_back(evt);
        st.next_offset += 1;
        eid
    }
}

#[async_trait]
impl CdcPublisherProvider for MockCdcPublisherProvider {
    async fn emit_vertex_created(
        &self,
        sp: &str,
        vid: &str,
        tags: Vec<String>,
        props: BTreeMap<String, String>,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let j = format!(
            "{{\"space\":{},\"vid\":{},\"tags\":{},\"properties\":{}}}",
            esc_json(sp),
            esc_json(vid),
            json_tags(&tags),
            json_props(&props)
        );
        Ok(self.emit(&format!("vertex.{}", sp), "vertex_created", &j, sp))
    }
    async fn emit_vertex_updated(
        &self,
        sp: &str,
        vid: &str,
        props: BTreeMap<String, String>,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let j = format!(
            "{{\"space\":{},\"vid\":{},\"changed_properties\":{}}}",
            esc_json(sp),
            esc_json(vid),
            json_props(&props)
        );
        Ok(self.emit(&format!("vertex.{}", sp), "vertex_updated", &j, sp))
    }
    async fn emit_vertex_deleted(
        &self,
        sp: &str,
        vid: &str,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let j = format!("{{\"space\":{},\"vid\":{}}}", esc_json(sp), esc_json(vid));
        Ok(self.emit(&format!("vertex.{}", sp), "vertex_deleted", &j, sp))
    }
    async fn emit_edge_created(
        &self,
        sp: &str,
        s: &str,
        d: &str,
        et: &str,
        rk: i64,
        props: BTreeMap<String, String>,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let j = format!(
            "{{\"space\":{},\"src\":{},\"dst\":{},\"edge_type\":{},\"rank\":{},\"properties\":{}}}",
            esc_json(sp),
            esc_json(s),
            esc_json(d),
            esc_json(et),
            rk,
            json_props(&props)
        );
        Ok(self.emit(&format!("edge.{}", sp), "edge_created", &j, sp))
    }
    async fn emit_edge_deleted(
        &self,
        sp: &str,
        s: &str,
        d: &str,
        et: &str,
        rk: i64,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let j = format!(
            "{{\"space\":{},\"src\":{},\"dst\":{},\"edge_type\":{},\"rank\":{}}}",
            esc_json(sp),
            esc_json(s),
            esc_json(d),
            esc_json(et),
            rk
        );
        Ok(self.emit(&format!("edge.{}", sp), "edge_deleted", &j, sp))
    }
    async fn subscribe(
        &self,
        topic: &str,
        cg: &str,
    ) -> Result<CdcSubscription, Box<dyn Error + Send + Sync>> {
        let mut sid_c = self.sid.lock();
        let sub_id = format!("sub-{}", *sid_c);
        *sid_c += 1;
        let last = {
            let ts = self.t.lock();
            ts.get(topic)
                .map(|t| t.committed.get(cg).copied().unwrap_or(0))
                .unwrap_or(0)
        };
        let sub = CdcSubscription {
            subscription_id: sub_id.clone(),
            topic: topic.into(),
            consumer_group: cg.into(),
            last_offset: last,
        };
        self.subs.lock().insert(sub_id.clone(), sub.clone());
        Ok(sub)
    }
    async fn list_topics(&self) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        Ok(self.t.lock().keys().cloned().collect())
    }
    async fn get_consumer_lag(
        &self,
        topic: &str,
        cg: &str,
    ) -> Result<ConsumerLag, Box<dyn Error + Send + Sync>> {
        let ts = self.t.lock();
        let st = ts.get(topic).ok_or("no topic")?;
        let committed = st.committed.get(cg).copied().unwrap_or(0);
        let latest = st.next_offset;
        Ok(ConsumerLag {
            topic: topic.into(),
            consumer_group: cg.into(),
            latest_offset: latest,
            committed_offset: committed,
            lag: latest.saturating_sub(committed),
        })
    }
    async fn commit_offset(
        &self,
        sub_id: &str,
        new_offset: u64,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut subs = self.subs.lock();
        let sub = subs.get_mut(sub_id).ok_or("no sub")?;
        let topic = sub.topic.clone();
        let cg = sub.consumer_group.clone();
        sub.last_offset = new_offset;
        let mut ts = self.t.lock();
        let st = ts.entry(topic).or_default();
        st.committed.insert(cg, new_offset);
        Ok(())
    }
}
