//! Offline tests for the dependency-free core. Run with:
//!   cargo test --no-default-features
//! A std-only HTTP transport talks to a local stub server; BNM is never contacted.

use bnm::{accept_header, build_url, clean_path, Client, Error, RawResponse, Transport};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn client_contract() -> std::collections::HashMap<String, Vec<String>> {
    let mut contract = std::collections::HashMap::new();
    for line in include_str!("client-contract.tsv")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let (key, value) = line.split_once('\t').expect("valid client contract row");
        contract
            .entry(key.to_string())
            .or_insert_with(Vec::new)
            .push(value.to_string());
    }
    for key in [
        "accept_header",
        "invalid_path",
        "not_found_status",
        "other_status",
        "response_body",
        "retry_attempts",
        "retry_status",
        "valid_path",
    ] {
        assert!(
            contract.get(key).is_some_and(|cases| !cases.is_empty()),
            "missing client contract cases for {key}"
        );
    }
    contract
}

struct StdTransport;

impl Transport for StdTransport {
    fn get(&self, url: &str, accept: &str, timeout: Duration) -> Result<RawResponse, String> {
        let rest = url
            .strip_prefix("http://")
            .ok_or("tests use http:// only")?;
        let (host, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        let mut stream = TcpStream::connect(host).map_err(|e| e.to_string())?;
        stream.set_read_timeout(Some(timeout)).ok();
        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: {host}\r\nAccept: {accept}\r\nConnection: close\r\n\r\n"
        );
        stream
            .write_all(request.as_bytes())
            .map_err(|e| e.to_string())?;
        let mut raw = String::new();
        stream.read_to_string(&mut raw).map_err(|e| e.to_string())?;
        let (head, body) = raw.split_once("\r\n\r\n").ok_or("malformed response")?;
        let status = head
            .split_whitespace()
            .nth(1)
            .and_then(|c| c.parse().ok())
            .ok_or("bad status line")?;
        Ok(RawResponse {
            status,
            body: body.to_string(),
        })
    }
}

type Seen = Arc<Mutex<Vec<(String, String)>>>;

fn spawn_stub(route: impl Fn(&str) -> (u16, String) + Send + 'static) -> (String, Seen) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/public", listener.local_addr().unwrap());
    let seen: Seen = Arc::new(Mutex::new(Vec::new()));
    let seen_in_thread = Arc::clone(&seen);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => break,
            };
            // Read until the end of the request headers (a request may arrive in pieces).
            let mut data: Vec<u8> = Vec::new();
            let mut buf = [0u8; 1024];
            while !data.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => data.extend_from_slice(&buf[..n]),
                }
            }
            let request = String::from_utf8_lossy(&data).to_string();
            let path = request.split_whitespace().nth(1).unwrap_or("").to_string();
            let accept = request
                .lines()
                .find(|l| l.to_ascii_lowercase().starts_with("accept:"))
                .map(|l| l[7..].trim().to_string())
                .unwrap_or_default();
            seen_in_thread.lock().unwrap().push((path.clone(), accept));
            let (status, body) = route(&path);
            let response = format!(
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    (base, seen)
}

fn client(base: &str) -> Client<StdTransport> {
    Client::with_transport(StdTransport)
        .base_url(base)
        .backoff(|_| Duration::ZERO)
}

#[test]
fn sends_versioned_accept_header_and_returns_body() {
    let contract = client_contract();
    let payload = contract["response_body"][0].clone();
    let (base, seen) = spawn_stub(move |_| (200, payload.clone()));
    let body = client(&base).get("opr", &[]).unwrap();
    assert_eq!(body, contract["response_body"][0]);
    let seen = seen.lock().unwrap();
    assert_eq!(seen[0].0, "/public/opr");
    assert_eq!(seen[0].1, contract["accept_header"][0]);
    assert_eq!(seen[0].1, accept_header(1));
}

#[cfg(feature = "json")]
#[test]
fn get_json_decodes_contract_payload() {
    let contract = client_contract();
    let payload = contract["response_body"][0].clone();
    let expected: serde_json::Value =
        serde_json::from_str(&payload).expect("valid response body in contract");
    let payload_in_route = payload.clone();
    let (base, _) = spawn_stub(move |_| (200, payload_in_route.clone()));
    let response = Client::with_transport(StdTransport)
        .base_url(&base)
        .get_json("opr", &[])
        .unwrap();
    assert_eq!(response.raw, expected);
    assert_eq!(response.data, expected["data"]);
}

#[test]
fn encodes_query_string() {
    let (base, seen) = spawn_stub(|_| (200, "{}".into()));
    client(&base)
        .get("opr", &[("session", "1700"), ("q", "r m")])
        .unwrap();
    assert_eq!(
        seen.lock().unwrap()[0].0,
        "/public/opr?session=1700&q=r%20m"
    );
}

#[test]
fn not_found_on_404() {
    let status = client_contract()["not_found_status"][0]
        .parse()
        .expect("valid not-found status");
    let (base, _) = spawn_stub(move |_| (status, "{}".into()));
    let err = client(&base).get("exchange-rate/XXX", &[]).unwrap_err();
    assert!(matches!(err, Error::NotFound(_)), "{err:?}");
}

#[test]
fn retries_contract_statuses() {
    let contract = client_contract();
    let retries: usize = contract["retry_attempts"][0]
        .parse()
        .expect("valid retry count");
    let payload = contract["response_body"][0].clone();
    for status_text in &contract["retry_status"] {
        let status: u16 = status_text.parse().expect("valid retry status");
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_in_route = Arc::clone(&hits);
        let payload_in_route = payload.clone();
        let (base, _) = spawn_stub(move |_| {
            if hits_in_route.fetch_add(1, Ordering::SeqCst) < retries {
                (status, "{}".into())
            } else {
                (200, payload_in_route.clone())
            }
        });
        let body = client(&base).get("retry", &[]).unwrap();
        assert_eq!(body, payload);
        assert_eq!(hits.load(Ordering::SeqCst), retries + 1, "status {status}");
    }
}

#[test]
fn other_statuses_are_api_errors() {
    let status = client_contract()["other_status"][0]
        .parse()
        .expect("valid API error status");
    let (base, _) = spawn_stub(move |_| (status, "denied".into()));
    let err = client(&base).get("opr", &[]).unwrap_err();
    assert!(
        matches!(err, Error::Api { status: actual, .. } if actual == status),
        "{err:?}"
    );
}

struct AlwaysDown(Arc<AtomicUsize>);

impl Transport for AlwaysDown {
    fn get(&self, _: &str, _: &str, _: Duration) -> Result<RawResponse, String> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err("connection refused".into())
    }
}

#[test]
fn network_errors_retry_then_fail() {
    let calls = Arc::new(AtomicUsize::new(0));
    let client = Client::with_transport(AlwaysDown(Arc::clone(&calls)))
        .retries(2)
        .backoff(|_| Duration::ZERO);
    let err = client.get("opr", &[]).unwrap_err();
    assert!(matches!(err, Error::Network(_)), "{err:?}");
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[test]
fn clean_path_rejects_bad_and_accepts_real() {
    let contract = client_contract();
    for bad in &contract["invalid_path"] {
        assert!(
            matches!(clean_path(bad), Err(Error::InvalidPath(_))),
            "{bad:?}"
        );
    }
    for good in &contract["valid_path"] {
        assert!(clean_path(good).is_ok(), "{good:?}");
    }
}

#[test]
fn build_url_joins_cleanly() {
    assert_eq!(
        build_url("https://api.bnm.gov.my/public/", "/opr/year/2025", &[]).unwrap(),
        "https://api.bnm.gov.my/public/opr/year/2025"
    );
}
