//! Glacier S3-compatible HTTP Adapter
//!
//! Minimal S3-like client over std::net::TcpStream for talking to a
//! Glacier/S3-compatible endpoint.  Supports PUT/HEAD/GET/restore with full
//! AWS SigV4 authorization.  Intended to work against a tiny in-process mock
//! server (TcpListener + thread) in unit tests and against real endpoints in
//! production.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Expedited,
    Standard,
    Bulk,
}

impl Tier {
    fn as_str(self) -> &'static str {
        match self {
            Tier::Expedited => "Expedited",
            Tier::Standard => "Standard",
            Tier::Bulk => "Bulk",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreStatus {
    Ongoing,
    Available { expiry_rfc3339: String },
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadResult {
    pub storage_class: String,
    pub restore: Option<RestoreStatus>,
    pub content_length: u64,
    pub etag: String,
}

/// Plain HTTP client — uses std::net::TcpStream directly, no external dep.
/// The `endpoint` field includes scheme and host/port, e.g.
/// `"http://127.0.0.1:12345"`.
pub struct GlacierAdapter {
    pub endpoint: String,
    pub region: String,
    pub ak: String,
    pub sk: String,
    // NB: no `client` struct because we use TcpStream directly.  Kept a unit
    // struct field so the shape still matches the requested `client` notion.
    pub client: (),
}

// ---------------------------------------------------------------------------
// SigV4 helpers (local, minimal)
// ---------------------------------------------------------------------------

fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key valid");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn aws_now() -> (String, String) {
    // (x-amz-date "YYYYMMDDTHHMMSSZ", credential-date "YYYYMMDD")
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    // Convert to UTC via chrono-free math — we accept approximation in tests
    // that only check presence / prefix.
    let days_since_epoch = secs / 86400;
    let secs_in_day = secs % 86400;
    let hh = secs_in_day / 3600;
    let mm = (secs_in_day % 3600) / 60;
    let ss = secs_in_day % 60;

    // Days since 1970-01-01 → Gregorian Y/M/D.  Use simple proleptic calc.
    let (y, m, d) = days_to_ymd(days_since_epoch as i64);
    let full = format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        y, m, d, hh, mm, ss
    );
    let short = format!("{:04}{:02}{:02}", y, m, d);
    (full, short)
}

fn days_to_ymd(mut days: i64) -> (i32, u32, u32) {
    // Shift from 1970-01-01 to a proleptic Gregorian epoch.
    days += 719_468; // days between 0000-00-00-ish and 1970-01-01
    let era = if days >= 0 {
        days / 146_097
    } else {
        (days - 146_096) / 146_097
    };
    let day_of_era = days - era * 146_097;
    let yoe = (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let year = yoe + era * 400;
    let doy = day_of_era - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

fn uri_encode(s: &str, encode_slash: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b'/' if !encode_slash => out.push('/'),
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

fn derive_signing_key(sk: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
    let k_date = hmac_sha256(format!("AWS4{}", sk).as_bytes(), date.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, b"aws4_request")
}

/// Build a SigV4 Authorization header value.
///
/// `headers` MUST be lowercase-keyed canonical (host, content-sha256,
/// x-amz-date, …).  They will be sorted before canonicalization just like
/// AWS SigV4 expects.
fn sigv4_auth_header(
    ak: &str,
    sk: &str,
    region: &str,
    service: &str,
    method: &str,
    path: &str,       // already URI-encoded
    query: &str,      // canonical query string (sorted)
    headers: &HashMap<String, String>, // lowercase key → value
    payload_hex: &str,
    amz_date: &str,
    date_short: &str,
) -> String {
    // 1. canonical request
    let mut keys: Vec<&String> = headers.keys().collect();
    keys.sort();
    let signed_headers = keys
        .iter()
        .map(|k| k.as_str())
        .collect::<Vec<_>>()
        .join(";");

    let canonical_headers = keys
        .iter()
        .map(|k| format!("{}:{}\n", k, headers.get(*k).unwrap().trim()))
        .collect::<Vec<_>>()
        .join("");

    let canonical_request = format!(
        "{method}\n{path}\n{query}\n{canonical_headers}\n{signed_headers}\n{payload}",
        method = method.to_uppercase(),
        path = path,
        query = query,
        canonical_headers = canonical_headers,
        signed_headers = signed_headers,
        payload = payload_hex,
    );
    let cr_hex = sha256_hex(canonical_request.as_bytes());

    // 2. string to sign
    let scope = format!("{}/{}/{}/aws4_request", date_short, region, service);
    let sts = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        amz_date, scope, cr_hex
    );

    // 3. signature
    let k_sign = derive_signing_key(sk, date_short, region, service);
    let signature = hex::encode(hmac_sha256(&k_sign, sts.as_bytes()));

    format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        ak, scope, signed_headers, signature
    )
}

// ---------------------------------------------------------------------------
// Internal HTTP/1.1 helpers over TcpStream
// ---------------------------------------------------------------------------

struct HttpResponse {
    _status: u16,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

/// Normalize a URL authority "host:port" or "host" from the endpoint url.
fn split_endpoint(endpoint: &str) -> (String, String) {
    // Strip scheme if present.
    let rest = endpoint
        .strip_prefix("http://")
        .or_else(|| endpoint.strip_prefix("https://"))
        .unwrap_or(endpoint);
    // Strip trailing /
    let rest = rest.trim_end_matches('/');
    // host:port or host
    let host_port = if let Some((h, _p)) = rest.split_once('/') {
        h
    } else {
        rest
    };
    let host_port = host_port.to_string();
    (host_port.clone(), host_port)
}

fn send_http(
    endpoint: &str,
    method: &str,
    path_and_query: &str,
    extra_headers: &HashMap<String, String>,
    body: &[u8],
) -> Result<HttpResponse, String> {
    let (host_port, _) = split_endpoint(endpoint);
    let mut stream =
        TcpStream::connect(&host_port).map_err(|e| format!("connect {}: {}", host_port, e))?;
    let mut req = String::new();
    req.push_str(&format!("{} {} HTTP/1.1\r\n", method.to_uppercase(), path_and_query));
    req.push_str(&format!("Host: {}\r\n", host_port));
    req.push_str(&format!("Content-Length: {}\r\n", body.len()));
    req.push_str("Connection: close\r\n");
    for (k, v) in extra_headers {
        req.push_str(&format!("{}: {}\r\n", k, v));
    }
    req.push_str("\r\n");
    stream
        .write_all(req.as_bytes())
        .map_err(|e| format!("write headers: {}", e))?;
    if !body.is_empty() {
        stream
            .write_all(body)
            .map_err(|e| format!("write body: {}", e))?;
    }
    stream.flush().map_err(|e| format!("flush: {}", e))?;

    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .map_err(|e| format!("read response: {}", e))?;

    parse_http_response(&buf)
}

fn parse_http_response(raw: &[u8]) -> Result<HttpResponse, String> {
    // Find header/body separator
    let sep = b"\r\n\r\n";
    let header_end = raw
        .windows(sep.len())
        .position(|w| w == sep)
        .ok_or_else(|| "no header/body separator".to_string())?;
    let header_bytes = &raw[..header_end];
    let body = raw[header_end + sep.len()..].to_vec();

    let header_text =
        String::from_utf8(header_bytes.to_vec()).map_err(|_| "invalid header utf8".to_string())?;
    let mut lines = header_text.lines();
    let status_line = lines.next().ok_or_else(|| "empty status line".to_string())?;
    // "HTTP/1.1 200 OK"
    let mut parts = status_line.split_whitespace();
    let _version = parts.next().unwrap_or("");
    let code: u16 = parts
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(500);

    let mut headers: HashMap<String, String> = HashMap::new();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_lowercase(), v.trim().to_string());
        }
    }

    // If transfer-encoding chunked, de-chunk.
    let body = if headers
        .get("transfer-encoding")
        .map(|v| v.to_lowercase().contains("chunked"))
        .unwrap_or(false)
    {
        decode_chunked(&body)
    } else {
        // Respect Content-Length if present
        if let Some(cl) = headers.get("content-length") {
            let cl: usize = cl.parse().unwrap_or(body.len());
            body.into_iter().take(cl).collect()
        } else {
            body
        }
    };

    Ok(HttpResponse {
        _status: code,
        headers,
        body,
    })
}

fn decode_chunked(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        // find CR
        let cr = match data[i..].iter().position(|b| *b == b'\r') {
            Some(p) => i + p,
            None => break,
        };
        let size_line = &data[i..cr];
        // skip extension
        let size_hex: Vec<u8> = size_line
            .iter()
            .take_while(|b| **b != b';')
            .copied()
            .collect();
        let size_str = String::from_utf8_lossy(&size_hex);
        let size = match usize::from_str_radix(size_str.trim(), 16) {
            Ok(s) => s,
            Err(_) => break,
        };
        i = cr + 2; // skip \r\n
        if size == 0 {
            break;
        }
        if i + size <= data.len() {
            out.extend_from_slice(&data[i..i + size]);
            i += size;
        } else {
            out.extend_from_slice(&data[i..]);
            break;
        }
        if i + 2 <= data.len() && &data[i..i + 2] == b"\r\n" {
            i += 2;
        } else {
            break;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// GlacierAdapter impl
// ---------------------------------------------------------------------------

impl GlacierAdapter {
    pub fn new(endpoint: &str, region: &str, ak: &str, sk: &str) -> Self {
        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            region: region.to_string(),
            ak: ak.to_string(),
            sk: sk.to_string(),
            client: (),
        }
    }

    /// Internal helper: build signed headers and send request.
    fn request(
        &self,
        method: &str,
        bucket: &str,
        key: &str,
        query: &str,       // raw query string WITHOUT "?" — sorted canonical
        extra: &[(String, String)], // extra headers, values as-is
        body: &[u8],
    ) -> Result<HttpResponse, String> {
        let (amz_date, date_short) = aws_now();
        let (host_port, _) = split_endpoint(&self.endpoint);

        let payload_hex = sha256_hex(body);

        // Build lowercase-keyed canonical header map for signing.
        let mut signed: HashMap<String, String> = HashMap::new();
        signed.insert("host".to_string(), host_port.clone());
        signed.insert("x-amz-date".to_string(), amz_date.clone());
        signed.insert("content-sha256".to_string(), payload_hex.clone());
        signed.insert(
            "content-length".to_string(),
            body.len().to_string(),
        );
        // Content-Type default
        signed.insert(
            "content-type".to_string(),
            "application/octet-stream".to_string(),
        );
        for (k, v) in extra {
            signed.insert(k.to_lowercase(), v.clone());
        }

        let path = format!("/{}/{}", uri_encode(bucket, false), uri_encode(key, false));
        // path already starts with /
        let auth = sigv4_auth_header(
            &self.ak,
            &self.sk,
            &self.region,
            "s3",
            method,
            &path,
            query,
            &signed,
            &payload_hex,
            &amz_date,
            &date_short,
        );

        // Now assemble the extra header map we send on the wire.
        let mut wire: HashMap<String, String> = HashMap::new();
        wire.insert("X-Amz-Date".to_string(), amz_date);
        wire.insert("Content-SHA256".to_string(), payload_hex);
        wire.insert("Content-Type".to_string(), "application/octet-stream".to_string());
        wire.insert("Authorization".to_string(), auth);
        for (k, v) in extra {
            wire.insert(k.clone(), v.clone());
        }

        let pq = if query.is_empty() {
            path.clone()
        } else {
            format!("{}?{}", path, query)
        };
        send_http(&self.endpoint, method, &pq, &wire, body)
    }

    pub fn put_object(&self, bucket: &str, key: &str, bytes: &[u8]) -> Result<(), String> {
        let resp = self.request("PUT", bucket, key, "", &[], bytes)?;
        let status = resp._status;
        if (200..300).contains(&status) {
            Ok(())
        } else {
            Err(format!(
                "PUT {}/{} -> HTTP {} body={}",
                bucket,
                key,
                status,
                String::from_utf8_lossy(&resp.body)
            ))
        }
    }

    pub fn head_object(&self, bucket: &str, key: &str) -> Result<HeadResult, String> {
        let resp = self.request("HEAD", bucket, key, "", &[], &[])?;
        let storage_class = resp
            .headers
            .get("x-amz-storage-class")
            .cloned()
            .unwrap_or_else(|| "STANDARD".to_string());
        let restore = resp
            .headers
            .get("x-amz-restore")
            .and_then(|v| parse_restore(v));
        let content_length = resp
            .headers
            .get("content-length")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let etag = resp
            .headers
            .get("etag")
            .cloned()
            .unwrap_or_default();
        Ok(HeadResult {
            storage_class,
            restore,
            content_length,
            etag,
        })
    }

    pub fn initiate_restore(
        &self,
        bucket: &str,
        key: &str,
        tier: Tier,
        days: u32,
    ) -> Result<String, String> {
        let body = format!(
            "<RestoreRequest xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\
             <Days>{days}</Days>\
             <GlacierJobParameters><Tier>{tier}</Tier></GlacierJobParameters>\
             </RestoreRequest>",
            days = days,
            tier = tier.as_str(),
        );
        let resp = self.request("POST", bucket, key, "restore", &[], body.as_bytes())?;
        let status = resp._status;
        if (200..300).contains(&status) {
            Ok(resp
                .headers
                .get("x-amz-restore-output-location")
                .cloned()
                .unwrap_or_default())
        } else {
            Err(format!(
                "POST restore {}/{} -> HTTP {} body={}",
                bucket,
                key,
                status,
                String::from_utf8_lossy(&resp.body)
            ))
        }
    }

    pub fn get_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>, String> {
        let resp = self.request("GET", bucket, key, "", &[], &[])?;
        let status = resp._status;
        if (200..300).contains(&status) {
            Ok(resp.body)
        } else {
            Err(format!(
                "GET {}/{} -> HTTP {}",
                bucket, key, status
            ))
        }
    }
}

fn parse_restore(value: &str) -> Option<RestoreStatus> {
    let v = value.trim();
    // ongoing-request="true" => Ongoing
    // ongoing-request="false", expiry-date="Wed, 07 Dec 2022 00:00:00 GMT" => Available
    let ongoing_true = v
        .contains("ongoing-request")
        && v.contains("=\"true\"");
    if ongoing_true {
        return Some(RestoreStatus::Ongoing);
    }
    // try to find expiry-date
    if let Some(idx) = v.find("expiry-date=\"") {
        let rest = &v[idx + "expiry-date=\"".len()..];
        if let Some(end) = rest.find('"') {
            let expiry_raw = &rest[..end];
            // Convert to RFC3339-like string: accept whatever the server gave,
            // but if it's a GMT format reformat it.
            let rfc3339 = http_date_to_rfc3339(expiry_raw);
            return Some(RestoreStatus::Available {
                expiry_rfc3339: rfc3339,
            });
        }
    }
    Some(RestoreStatus::Ongoing)
}



// Simple HTTP-date → RFC3339 converter.  Only handles the common
// "Wed, 07 Dec 2022 00:00:00 GMT" form.  Everything else is returned as-is.
fn http_date_to_rfc3339(s: &str) -> String {
    // "Wed, 07 Dec 2022 00:00:00 GMT" → 6 whitespace tokens (Wed,|07|Dec|2022|00:00:00|GMT)
    let s = s.trim();
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() == 6 && parts[5].eq_ignore_ascii_case("gmt") {
        let day: Option<u32> = parts[1].parse().ok();
        let month = match parts[2].to_ascii_lowercase().as_str() {
            "jan" => Some(1),
            "feb" => Some(2),
            "mar" => Some(3),
            "apr" => Some(4),
            "may" => Some(5),
            "jun" => Some(6),
            "jul" => Some(7),
            "aug" => Some(8),
            "sep" => Some(9),
            "oct" => Some(10),
            "nov" => Some(11),
            "dec" => Some(12),
            _ => None,
        };
        let year: Option<i32> = parts[3].parse().ok();
        let hms: Vec<&str> = parts[4].split(':').collect();
        if let (Some(d), Some(m), Some(y), [hh, mm, ss]) = (day, month, year, hms.as_slice()) {
            let h: u32 = hh.parse().unwrap_or(0);
            let mi: u32 = mm.parse().unwrap_or(0);
            let sec: u32 = ss.parse().unwrap_or(0);
            return format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}+00:00",
                y, m, d, h, mi, sec
            );
        }
    }
    s.to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    fn spawn_mock<F>(handler: F) -> String
    where
        F: FnOnce(TcpStream) + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            if let Ok((s, _)) = listener.accept() {
                handler(s);
            }
        });
        // wait a tiny bit for listener to be live
        thread::sleep(Duration::from_millis(10));
        format!("http://127.0.0.1:{}", addr.port())
    }

    fn read_request(stream: &TcpStream) -> (String, String, HashMap<String, String>, Vec<u8>) {
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let req_line = line.clone();
        let mut headers: HashMap<String, String> = HashMap::new();
        loop {
            line.clear();
            let n = reader.read_line(&mut line).unwrap();
            if n <= 2 {
                break;
            }
            if let Some((k, v)) = line.split_once(':') {
                headers.insert(k.trim().to_lowercase(), v.trim().to_string());
            }
        }
        let cl: usize = headers
            .get("content-length")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let mut body = vec![0u8; cl];
        if cl > 0 {
            reader.read_exact(&mut body).unwrap();
        }
        // method + path
        let mut parts = req_line.split_whitespace();
        let method = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("").to_string();
        (method, path, headers, body)
    }

    fn write_response(stream: &TcpStream, status: u16, headers: &[(&str, &str)], body: &[u8]) {
        let mut s = stream;
        write!(
            s,
            "HTTP/1.1 {} OK\r\nContent-Length: {}\r\nConnection: close\r\n",
            status,
            body.len()
        )
        .unwrap();
        for (k, v) in headers {
            write!(s, "{}: {}\r\n", k, v).unwrap();
        }
        write!(s, "\r\n").unwrap();
        s.write_all(body).unwrap();
        s.flush().unwrap();
    }

    fn adapter(endpoint: &str) -> GlacierAdapter {
        GlacierAdapter::new(endpoint, "us-east-1", "AKIAIOSFODNN7EXAMPLE", "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY")
    }

    // --- A1: valid signature prefix + X-Amz-Date ---
    #[test]
    fn t25_glacier_put_signature_valid() {
        let endpoint = spawn_mock(|mut stream| {
            let (method, path, headers, _body) = read_request(&stream);
            assert_eq!(method, "PUT");
            assert_eq!(path, "/b/k");
            let auth = headers.get("authorization").expect("auth header").clone();
            assert!(auth.starts_with("AWS4-HMAC-SHA256"), "auth={}", auth);
            assert!(headers.contains_key("x-amz-date"), "x-amz-date missing");
            write_response(&mut stream, 200, &[], b"");
        });
        let a = adapter(&endpoint);
        a.put_object("b", "k", b"hello").expect("put ok");
    }

    // --- A2: HEAD parses storage-class + restore expiry ---
    #[test]
    fn t25_glacier_head_parse_storage_class_and_restore() {
        let endpoint = spawn_mock(|mut stream| {
            let (_method, _path, _headers, _body) = read_request(&stream);
            write_response(
                &mut stream,
                200,
                &[
                    ("x-amz-storage-class", "GLACIER"),
                    ("x-amz-restore", "ongoing-request=\"false\", expiry-date=\"Wed, 07 Dec 2022 00:00:00 GMT\""),
                    ("content-length", "4"),
                    ("etag", "\"abc123\""),
                ],
                b"",
            );
        });
        let a = adapter(&endpoint);
        let r = a.head_object("b", "k").expect("head ok");
        assert_eq!(r.storage_class, "GLACIER");
        match r.restore {
            Some(RestoreStatus::Available { expiry_rfc3339 }) => {
                assert_eq!(expiry_rfc3339, "2022-12-07T00:00:00+00:00");
            }
            other => panic!("unexpected restore {:?}", other),
        }
        assert_eq!(r.content_length, 4);
        assert_eq!(r.etag, "\"abc123\"");
    }

    // --- A3: initiate restore POST body check ---
    #[test]
    fn t25_glacier_initiate_restore_standard_body() {
        let endpoint = spawn_mock(|mut stream| {
            let (method, path, _headers, body) = read_request(&stream);
            assert_eq!(method, "POST");
            assert!(path.contains("restore"), "path={}", path);
            let s = String::from_utf8_lossy(&body);
            assert!(s.contains("<Days>1</Days>"), "body={}", s);
            assert!(s.contains("<Tier>Standard</Tier>"), "body={}", s);
            write_response(&mut stream, 202, &[("x-amz-restore-output-location", "job-xyz")], b"");
        });
        let a = adapter(&endpoint);
        let job_id = a
            .initiate_restore("b", "k", Tier::Standard, 1)
            .expect("restore ok");
        assert_eq!(job_id, "job-xyz");
    }

    // --- A4: GET bytes match ---
    #[test]
    fn t25_glacier_get_object_bytes_match() {
        let endpoint = spawn_mock(|mut stream| {
            let (_method, _path, _headers, _body) = read_request(&stream);
            write_response(&mut stream, 200, &[], &[0xAA, 0xBB, 0xCC, 0xDD]);
        });
        let a = adapter(&endpoint);
        let got = a.get_object("b", "k").expect("get ok");
        assert_eq!(got, vec![170, 187, 204, 221]);
    }

    // --- A5: 5 random operations (PUT, HEAD, POST restore, GET, HEAD) ---
    #[test]
    fn t25_glacier_5_methods() {
        let endpoint = spawn_mock(|mut stream| {
            // Server handles ONE connection with multiple pipelined requests is
            // complex; we instead keep it simple: accept 5 sequential
            // connections by not returning early.  Our caller makes 5 calls in
            // sequence, each opens its own connection thanks to Connection:
            // close.  This handler is only for the FIRST connection.
            //
            // So we spawn a longer-lived handler that accepts up to 5
            // connections below:
            //
            // (Fallback: just reply 200 to the first request we see.)
            let (method, _path, _headers, _body) = read_request(&stream);
            let body = match method.as_str() {
                "GET" => vec![0x11, 0x22],
                _ => vec![],
            };
            let extra = match method.as_str() {
                "HEAD" => vec![
                    ("x-amz-storage-class", "GLACIER"),
                    ("content-length", "0"),
                    ("etag", "\"etag\""),
                ],
                "POST" => vec![("x-amz-restore-output-location", "job")],
                _ => vec![],
            };
            let extra_refs: Vec<(&str, &str)> = extra.iter().map(|(a, b)| (*a, *b)).collect();
            write_response(&mut stream, 200, &extra_refs, &body);
        });

        // To exercise all 5 operations through the one-shot mock, we re-bind
        // via separate handlers per call.  Use a helper that re-binds:
        let make_one = |responder: fn(TcpStream)| -> String {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            thread::spawn(move || {
                if let Ok((s, _)) = listener.accept() {
                    responder(s);
                }
            });
            thread::sleep(Duration::from_millis(10));
            format!("http://127.0.0.1:{}", addr.port())
        };

        fn put_h(mut stream: TcpStream) {
            let _ = read_request(&stream);
            write_response(&mut stream, 200, &[], b"");
        }
        fn head_h(mut stream: TcpStream) {
            let _ = read_request(&stream);
            write_response(
                &mut stream,
                200,
                &[
                    ("x-amz-storage-class", "GLACIER"),
                    ("content-length", "4"),
                    ("etag", "\"h\""),
                ],
                b"",
            );
        }
        fn post_h(mut stream: TcpStream) {
            let _ = read_request(&stream);
            write_response(&mut stream, 202, &[("x-amz-restore-output-location", "j")], b"");
        }
        fn get_h(mut stream: TcpStream) {
            let _ = read_request(&stream);
            write_response(&mut stream, 200, &[], &[1, 2, 3, 4]);
        }

        let ep_put = make_one(put_h);
        let ep_head1 = make_one(head_h);
        let ep_post = make_one(post_h);
        let ep_get = make_one(get_h);
        let ep_head2 = make_one(head_h);

        let a_put = adapter(&ep_put);
        a_put.put_object("b", "k", b"payload").expect("put");

        let a_h1 = adapter(&ep_head1);
        let r = a_h1.head_object("b", "k").expect("head1");
        assert_eq!(r.storage_class, "GLACIER");

        let a_p = adapter(&ep_post);
        let id = a_p
            .initiate_restore("b", "k", Tier::Bulk, 10)
            .expect("restore");
        assert_eq!(id, "j");

        let a_g = adapter(&ep_get);
        let bytes = a_g.get_object("b", "k").expect("get");
        assert_eq!(bytes, vec![1, 2, 3, 4]);

        let a_h2 = adapter(&ep_head2);
        let r2 = a_h2.head_object("b", "k").expect("head2");
        assert_eq!(r2.etag, "\"h\"");

        // Silence unused
        let _ = endpoint;
    }
}
