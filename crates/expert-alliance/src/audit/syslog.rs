//! Syslog Sink — RFC 5424 格式，写入系统日志或 SIEM

use super::{AuditError, AuditSink, FlushPolicy};
use super::event::ExtAuditEvent;
use std::sync::RwLock;
use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

pub struct SyslogSink {
    address: String,
    protocol: SyslogProtocol,
    app_name: String,
    proc_id: String,
    flush_policy: FlushPolicy,
    buffer: RwLock<Vec<String>>,
    last_flush_ms: RwLock<u64>,
    reconnect_interval_sec: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyslogProtocol { TCP, UDP, TLS }

impl SyslogSink {
    pub fn new(address: &str, protocol: &str) -> Self {
        let proto = match protocol.to_lowercase().as_str() {
            "tcp" => SyslogProtocol::TCP,
            "udp" => SyslogProtocol::UDP,
            "tls" => SyslogProtocol::TLS,
            _ => SyslogProtocol::TCP,
        };
        Self {
            address: address.into(),
            protocol: proto,
            app_name: "expert-alliance".into(),
            proc_id: std::process::id().to_string(),
            flush_policy: FlushPolicy::default(),
            buffer: RwLock::new(Vec::new()),
            last_flush_ms: RwLock::new(now_ms()),
            reconnect_interval_sec: 5,
        }
    }

    pub fn with_app_name(mut self, name: &str) -> Self { self.app_name = name.into(); self }

    pub fn with_flush_policy(mut self, policy: FlushPolicy) -> Self { self.flush_policy = policy; self }

    fn serialize(&self, event: &ExtAuditEvent) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        let pri = (16u8 * 8 + event.outcome.to_severity() as u8) as usize;
        write!(s, "<{pri}>1 ").unwrap();
        write!(s, "{} ", event.timestamp.to_rfc3339()).unwrap();
        // HOSTNAME：取环境变量 HOSTNAME 或 fallback
        if let Ok(h) = std::env::var("HOSTNAME") {
            write!(s, "{} ", h).unwrap();
        } else {
            write!(s, "- ").unwrap();
        }
        write!(s, "{} ", self.app_name).unwrap();
        write!(s, "[{}] ", self.proc_id).unwrap();
        write!(s, "{} ", event.action.to_string()).unwrap();
        write!(s, "[audit@32473").unwrap();
        write!(s, " event_id=\"{}\"", event.event_id).unwrap();
        write!(s, " tenant=\"{}\"", event.tenant_id).unwrap();
        write!(s, " actor=\"{}\"", event.actor.id).unwrap();
        write!(s, " role=\"{}\"", event.actor.role).unwrap();
        if let Some(ref session) = event.session_id { write!(s, " session=\"{session}\"").unwrap(); }
        if let Some(ref ip) = event.client_ip { write!(s, " client_ip=\"{ip}\"").unwrap(); }
        write!(s, " outcome=\"{:?}\"", event.outcome).unwrap();
        write!(s, " resource_type=\"{}\"", event.resource.resource_type).unwrap();
        write!(s, " resource_id=\"{}\"", event.resource.resource_id).unwrap();
        if !event.extra.is_empty() {
            if let Ok(json) = serde_json::to_string(&event.extra) {
                write!(s, " extra={json}").unwrap();
            }
        }
        write!(s, " content_hash=\"{}\"", event.content_hash).unwrap();
        write!(s, " chain_hash=\"{}\"", event.chain_hash).unwrap();
        write!(s, "] ").unwrap();
        write!(s, "action={} resource={}:{} outcome={:?}",
            event.action, event.resource.resource_type, event.resource.resource_id, event.outcome).unwrap();
        s
    }

    fn connect_tcp(&self) -> std::io::Result<TcpStream> {
        let addr: std::net::SocketAddr = self
            .address
            .parse::<std::net::SocketAddr>()
            .map_err(|e: std::net::AddrParseError| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;
        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        Ok(stream)
    }

    fn write_frame(&self, frame: &str) -> Result<(), AuditError> {
        match self.protocol {
            SyslogProtocol::TCP => {
                let mut retry = 0;
                loop {
                    match self.connect_tcp() {
                        Ok(mut stream) => {
                            stream.write_all(frame.as_bytes()).map_err(|e| AuditError::WriteFailed(e.to_string()))?;
                            stream.write_all(b"\n").ok();
                            return Ok(());
                        }
                        Err(_e) if retry < 3 => {
                            retry += 1;
                            std::thread::sleep(Duration::from_secs(self.reconnect_interval_sec * 2u64.pow(retry as u32)));
                        }
                        Err(e) => return Err(AuditError::Connection(format!("TCP failed after 3 retries: {e}"))),
                    }
                }
            }
            SyslogProtocol::UDP => {
                let addr: std::net::SocketAddr = self.address.parse::<std::net::SocketAddr>()
                    .map_err(|e| AuditError::Connection(e.to_string()))?;
                let socket = std::net::UdpSocket::bind("0.0.0.0:0").map_err(|e| AuditError::Connection(e.to_string()))?;
                socket.send_to(frame.as_bytes(), addr).map_err(|e| AuditError::WriteFailed(e.to_string()))?;
                Ok(())
            }
            SyslogProtocol::TLS => Err(AuditError::Connection("TLS not yet implemented".into())),
        }
    }
}

impl AuditSink for SyslogSink {
    fn append_sync(&self, event: &ExtAuditEvent) -> Result<(), AuditError> {
        let frame = self.serialize(event);
        match self.flush_policy {
            FlushPolicy::Immediate => self.write_frame(&frame),
            FlushPolicy::Batch { max_events } => {
                self.buffer.write().unwrap().push(frame);
                if self.buffer.read().unwrap().len() >= max_events {
                    let buf: Vec<String> = self.buffer.read().unwrap().clone();
                    for f in &buf { self.write_frame(f)?; }
                    self.buffer.write().unwrap().clear();
                    *self.last_flush_ms.write().unwrap() = now_ms();
                }
                Ok(())
            }
            FlushPolicy::Periodic { interval_ms } => {
                self.buffer.write().unwrap().push(frame);
                if now_ms() - *self.last_flush_ms.read().unwrap() >= interval_ms as u64 {
                    let buf: Vec<String> = self.buffer.read().unwrap().clone();
                    for f in &buf { self.write_frame(f)?; }
                    self.buffer.write().unwrap().clear();
                    *self.last_flush_ms.write().unwrap() = now_ms();
                }
                Ok(())
            }
        }
    }

    fn flush(&self) -> Result<(), AuditError> {
        let buf: Vec<String> = self.buffer.read().unwrap().clone();
        for f in &buf { self.write_frame(f)?; }
        self.buffer.write().unwrap().clear();
        *self.last_flush_ms.write().unwrap() = now_ms();
        Ok(())
    }

    fn health_check(&self) -> Result<(), AuditError> {
        if matches!(self.protocol, SyslogProtocol::TCP) {
            let _ = self.connect_tcp().map_err(|e| AuditError::Connection(e.to_string()))?;
        }
        Ok(())
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_rfc5424() {
        let sink = SyslogSink::new("localhost:514", "tcp").with_app_name("test-app");
        let frame = sink.serialize(&super::super::event::test_event());
        assert!(frame.starts_with("<134>1 "));
        assert!(frame.contains("test-app"));
        assert!(frame.contains("flow.created"));
        assert!(frame.contains("test-tenant"));
    }
}
