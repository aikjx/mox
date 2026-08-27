// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! MOX KG Streams Service
//!
//! Real-time graph stream processing:
//! - Event-driven node/edge updates
//! - Continuous query evaluation
//! - Windowed aggregation over graph streams
//! - Change data capture (CDC) for graph mutations

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StreamError {
    #[error("stream not found: {0}")]
    StreamNotFound(String),
    #[error("invalid event: {0}")]
    InvalidEvent(String),
    #[error("query failed: {0}")]
    QueryFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEvent {
    pub id: String,
    pub event_type: GraphEventType,
    pub entity_type: EntityType,
    pub entity_id: String,
    pub data: serde_json::Value,
    pub timestamp: String,
    pub sequence: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphEventType { Created, Updated, Deleted, Merged, Linked }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType { Node, Edge }

impl GraphEvent {
    pub fn new(event_type: GraphEventType, entity_type: EntityType, entity_id: &str, data: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::now_v7().to_string(),
            event_type, entity_type, entity_id: entity_id.into(), data,
            timestamp: chrono::Utc::now().to_rfc3339(),
            sequence: 0,
        }
    }
}

pub trait EventHandler: Send + Sync {
    fn name(&self) -> &str;
    fn handle(&self, event: &GraphEvent) -> Result<(), StreamError>;
}

/// Windowed event processor: collects events within time windows and processes them.
#[derive(Clone)]
pub struct WindowedProcessor {
    pub name: String,
    pub window_size: Duration,
    pub slide_interval: Duration,
    events: Arc<parking_lot::Mutex<VecDeque<(Instant, GraphEvent)>>>,
    handlers: Arc<parking_lot::RwLock<Vec<Arc<dyn EventHandler>>>>,
}

impl WindowedProcessor {
    pub fn new(name: &str, window_size: Duration, slide_interval: Duration) -> Self {
        Self {
            name: name.into(), window_size, slide_interval,
            events: Arc::new(parking_lot::Mutex::new(VecDeque::new())),
            handlers: Arc::new(parking_lot::RwLock::new(Vec::new())),
        }
    }

    pub fn add_handler(&self, handler: Arc<dyn EventHandler>) {
        self.handlers.write().push(handler);
    }

    pub fn ingest(&self, event: GraphEvent) {
        self.events.lock().push_back((Instant::now(), event));
    }

    pub fn ingest_batch(&self, events: Vec<GraphEvent>) {
        let mut q = self.events.lock();
        for e in events { q.push_back((Instant::now(), e)); }
    }

    /// Process events in the current window. Returns count processed.
    pub fn process_window(&self) -> Result<usize, StreamError> {
        let now = Instant::now();
        let window_events: Vec<GraphEvent> = {
            let mut q = self.events.lock();
            // Remove events outside window
            while let Some((t, _)) = q.front() {
                if now.duration_since(*t) > self.window_size { q.pop_front(); } else { break; }
            }
            q.iter().map(|(_, e)| e.clone()).collect()
        };

        let mut count = 0;
        let handlers = self.handlers.read().clone();
        for event in &window_events {
            for handler in &handlers {
                handler.handle(event)?;
            }
            count += 1;
        }
        Ok(count)
    }

    pub fn pending_count(&self) -> usize {
        self.events.lock().len()
    }

    pub fn purge_old(&self) -> usize {
        let now = Instant::now();
        let mut q = self.events.lock();
        let before = q.len();
        while let Some((t, _)) = q.front() {
            if now.duration_since(*t) > self.window_size { q.pop_front(); } else { break; }
        }
        before - q.len()
    }
}

/// Continuous query: evaluates a predicate over the event stream.
pub struct ContinuousQuery {
    pub name: String,
    pub predicate: Arc<dyn Fn(&GraphEvent) -> bool + Send + Sync>,
    matches: Arc<parking_lot::Mutex<Vec<GraphEvent>>>,
    max_matches: usize,
}

impl ContinuousQuery {
    pub fn new<F>(name: &str, predicate: F, max_matches: usize) -> Self
    where F: Fn(&GraphEvent) -> bool + Send + Sync + 'static {
        Self { name: name.into(), predicate: Arc::new(predicate), matches: Arc::new(parking_lot::Mutex::new(Vec::new())), max_matches }
    }

    pub fn evaluate(&self, event: &GraphEvent) -> bool {
        if (self.predicate)(event) {
            let mut m = self.matches.lock();
            m.push(event.clone());
            if m.len() > self.max_matches { m.remove(0); }
            true
        } else { false }
    }

    pub fn matches(&self) -> Vec<GraphEvent> { self.matches.lock().clone() }
    pub fn match_count(&self) -> usize { self.matches.lock().len() }
}

impl EventHandler for ContinuousQuery {
    fn name(&self) -> &str { &self.name }
    fn handle(&self, event: &GraphEvent) -> Result<(), StreamError> {
        self.evaluate(event);
        Ok(())
    }
}

/// Stream statistics: counts by event type, entity type, throughput.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStats {
    pub total_events: u64,
    pub by_type: HashMap<String, u64>,
    pub by_entity: HashMap<String, u64>,
    pub events_per_second: f64,
    pub last_event_at: Option<String>,
}

#[derive(Clone)]
pub struct GraphStream {
    pub name: String,
    processor: WindowedProcessor,
    queries: Arc<parking_lot::RwLock<Vec<Arc<ContinuousQuery>>>>,
    stats: Arc<parking_lot::Mutex<StreamStatsInternal>>,
}

struct StreamStatsInternal {
    total: u64,
    by_type: HashMap<String, u64>,
    by_entity: HashMap<String, u64>,
    start_time: Instant,
    last_event: Option<String>,
}

impl GraphStream {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            processor: WindowedProcessor::new(name, Duration::from_secs(60), Duration::from_secs(10)),
            queries: Arc::new(parking_lot::RwLock::new(Vec::new())),
            stats: Arc::new(parking_lot::Mutex::new(StreamStatsInternal {
                total: 0, by_type: HashMap::new(), by_entity: HashMap::new(),
                start_time: Instant::now(), last_event: None,
            })),
        }
    }

    pub fn publish(&self, event: GraphEvent) {
        // Update stats
        {
            let mut s = self.stats.lock();
            s.total += 1;
            *s.by_type.entry(format!("{:?}", event.event_type)).or_insert(0) += 1;
            *s.by_entity.entry(format!("{:?}", event.entity_type)).or_insert(0) += 1;
            s.last_event = Some(event.timestamp.clone());
        }
        // Evaluate continuous queries
        for q in self.queries.read().iter() {
            q.evaluate(&event);
        }
        // Ingest to window processor
        self.processor.ingest(event);
    }

    pub fn publish_batch(&self, events: Vec<GraphEvent>) {
        for e in events { self.publish(e); }
    }

    pub fn add_query(&self, query: Arc<ContinuousQuery>) {
        self.queries.write().push(query);
    }

    pub fn add_handler(&self, handler: Arc<dyn EventHandler>) {
        self.processor.add_handler(handler);
    }

    pub fn process_window(&self) -> Result<usize, StreamError> {
        self.processor.process_window()
    }

    pub fn stats(&self) -> StreamStats {
        let s = self.stats.lock();
        let elapsed = s.start_time.elapsed().as_secs_f64();
        StreamStats {
            total_events: s.total,
            by_type: s.by_type.clone(),
            by_entity: s.by_entity.clone(),
            events_per_second: if elapsed > 0.0 { s.total as f64 / elapsed } else { 0.0 },
            last_event_at: s.last_event.clone(),
        }
    }

    pub fn pending_events(&self) -> usize { self.processor.pending_count() }
}

#[derive(Clone)]
pub struct StreamManager {
    streams: Arc<parking_lot::RwLock<HashMap<String, GraphStream>>>,
}

impl StreamManager {
    pub fn new() -> Self { Self { streams: Arc::new(parking_lot::RwLock::new(HashMap::new())) } }

    pub fn create_stream(&self, name: &str) -> GraphStream {
        let stream = GraphStream::new(name);
        self.streams.write().insert(name.into(), stream.clone());
        stream
    }

    pub fn get_stream(&self, name: &str) -> Option<GraphStream> {
        self.streams.read().get(name).cloned()
    }

    pub fn publish(&self, stream: &str, event: GraphEvent) -> Result<(), StreamError> {
        let s = self.streams.read().get(stream).cloned().ok_or_else(|| StreamError::StreamNotFound(stream.into()))?;
        s.publish(event);
        Ok(())
    }

    pub fn list_streams(&self) -> Vec<String> {
        self.streams.read().keys().cloned().collect()
    }
}

impl Default for StreamManager { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_publish_stats() {
        let stream = GraphStream::new("test");
        stream.publish(GraphEvent::new(GraphEventType::Created, EntityType::Node, "n1", serde_json::json!({})));
        stream.publish(GraphEvent::new(GraphEventType::Updated, EntityType::Node, "n1", serde_json::json!({})));
        stream.publish(GraphEvent::new(GraphEventType::Created, EntityType::Edge, "e1", serde_json::json!({})));
        let stats = stream.stats();
        assert_eq!(stats.total_events, 3);
        assert!(stats.events_per_second >= 0.0);
    }

    #[test]
    fn continuous_query() {
        let query = Arc::new(ContinuousQuery::new("node_created", |e| e.event_type == GraphEventType::Created && e.entity_type == EntityType::Node, 100));
        let stream = GraphStream::new("test");
        stream.add_query(query.clone());
        stream.publish(GraphEvent::new(GraphEventType::Created, EntityType::Node, "n1", serde_json::json!({})));
        stream.publish(GraphEvent::new(GraphEventType::Updated, EntityType::Node, "n1", serde_json::json!({})));
        assert_eq!(query.match_count(), 1);
    }

    #[test]
    fn windowed_processor() {
        let handler = Arc::new(ContinuousQuery::new("all", |_| true, 1000));
        let processor = WindowedProcessor::new("test", Duration::from_secs(60), Duration::from_secs(1));
        processor.add_handler(handler.clone());
        processor.ingest(GraphEvent::new(GraphEventType::Created, EntityType::Node, "n1", serde_json::json!({})));
        processor.ingest(GraphEvent::new(GraphEventType::Created, EntityType::Node, "n2", serde_json::json!({})));
        let count = processor.process_window().unwrap();
        assert_eq!(count, 2);
        assert_eq!(handler.match_count(), 2);
    }

    #[test]
    fn stream_manager() {
        let mgr = StreamManager::new();
        mgr.create_stream("s1");
        assert!(mgr.get_stream("s1").is_some());
        assert!(mgr.get_stream("nonexistent").is_none());
        assert_eq!(mgr.list_streams().len(), 1);
    }

    #[test]
    fn publish_to_manager() {
        let mgr = StreamManager::new();
        mgr.create_stream("s1");
        mgr.publish("s1", GraphEvent::new(GraphEventType::Created, EntityType::Node, "n1", serde_json::json!({}))).unwrap();
        assert!(mgr.publish("nonexistent", GraphEvent::new(GraphEventType::Created, EntityType::Node, "n1", serde_json::json!({}))).is_err());
    }

    #[test]
    fn event_creation() {
        let e = GraphEvent::new(GraphEventType::Deleted, EntityType::Edge, "e1", serde_json::json!({"reason": "test"}));
        assert_eq!(e.entity_type, EntityType::Edge);
        assert_eq!(e.event_type, GraphEventType::Deleted);
        assert!(!e.id.is_empty());
    }
}
