//! Minimal client for the Bank Negara Malaysia (BNM) Open API.
//!
//! The core (path validation, URL building, the mandatory versioned `Accept`
//! header, retries, error mapping) uses only the standard library. HTTP is
//! behind the [`Transport`] trait; the default `ureq-transport` feature
//! provides a blocking HTTPS implementation, and `json` adds
//! [`Client::get_json`].
//!
//! HTTP 404 is reported as [`Error::NotFound`]: BNM uses it both for invalid
//! dates and for valid dates with no data (weekends, holidays).
//!
//! Unofficial; not affiliated with or endorsed by Bank Negara Malaysia.

use std::fmt;
use std::time::Duration;

/// The data host. The `apikijangportal` site is documentation only.
pub const DEFAULT_BASE_URL: &str = "https://api.bnm.gov.my/public";
/// Used to build the `Accept` header.
pub const DEFAULT_API_VERSION: u32 = 1;
/// Sent as `User-Agent` by the bundled transport.
pub const USER_AGENT: &str = "bnm-open-api-skill/0.1";

/// Errors returned by [`Client`].
#[derive(Debug)]
pub enum Error {
    /// The endpoint path failed validation.
    InvalidPath(String),
    /// HTTP 404: no data or invalid date. Holds the requested URL.
    NotFound(String),
    /// Any other non-2xx status (after retries for 429/5xx).
    Api { status: u16, body: String },
    /// The host could not be reached (after retries) or the body was unreadable.
    Network(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidPath(p) => write!(f, "invalid endpoint path: {p:?}"),
            Error::NotFound(u) => write!(f, "no data or invalid date for {u}"),
            Error::Api { status, body } => write!(f, "HTTP {status}: {body}"),
            Error::Network(m) => write!(f, "network error: {m}"),
        }
    }
}

impl std::error::Error for Error {}

/// What a [`Transport`] returns for any completed HTTP exchange.
#[derive(Debug, Clone)]
pub struct RawResponse {
    pub status: u16,
    pub body: String,
}

/// Performs one HTTP GET. Return `Err(message)` only for connection-level
/// failures; HTTP error statuses must come back as `Ok(RawResponse)`.
pub trait Transport {
    fn get(&self, url: &str, accept: &str, timeout: Duration) -> Result<RawResponse, String>;
}

/// Validate and normalise an endpoint path such as `opr/year/2025`.
pub fn clean_path(path: &str) -> Result<String, Error> {
    let p = path.trim().trim_matches('/');
    let ok_char = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-');
    let valid = !p.is_empty()
        && p.split('/')
            .all(|seg| !seg.is_empty() && seg != ".." && seg.chars().all(ok_char));
    if valid {
        Ok(p.to_string())
    } else {
        Err(Error::InvalidPath(path.to_string()))
    }
}

fn encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Build `{base_url}/{path}?{query}` with a validated path and encoded query.
pub fn build_url(base_url: &str, path: &str, query: &[(&str, &str)]) -> Result<String, Error> {
    let mut url = format!("{}/{}", base_url.trim_end_matches('/'), clean_path(path)?);
    if !query.is_empty() {
        let qs: Vec<String> = query
            .iter()
            .map(|(k, v)| format!("{}={}", encode_component(k), encode_component(v)))
            .collect();
        url.push('?');
        url.push_str(&qs.join("&"));
    }
    Ok(url)
}

/// The mandatory versioned `Accept` value.
pub fn accept_header(api_version: u32) -> String {
    format!("application/vnd.BNM.API.v{api_version}+json")
}

fn retryable(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 504)
}

/// A BNM Open API client over a [`Transport`].
pub struct Client<T: Transport> {
    transport: T,
    base_url: String,
    api_version: u32,
    timeout: Duration,
    retries: u32,
    backoff: Box<dyn Fn(u32) -> Duration + Send + Sync>,
}

impl<T: Transport> Client<T> {
    /// Build a client over any transport, with default settings.
    pub fn with_transport(transport: T) -> Self {
        Client {
            transport,
            base_url: DEFAULT_BASE_URL.to_string(),
            api_version: DEFAULT_API_VERSION,
            timeout: Duration::from_secs(30),
            retries: 2,
            backoff: Box::new(|attempt| Duration::from_secs((1u64 << attempt.min(3)).min(8))),
        }
    }

    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn api_version(mut self, api_version: u32) -> Self {
        self.api_version = api_version;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Retries for 429/5xx and connection errors (default 2).
    pub fn retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    /// Wait before retry number `attempt` (1-based).
    pub fn backoff(mut self, f: impl Fn(u32) -> Duration + Send + Sync + 'static) -> Self {
        self.backoff = Box::new(f);
        self
    }

    /// GET an endpoint and return the response body as text.
    pub fn get(&self, path: &str, query: &[(&str, &str)]) -> Result<String, Error> {
        let url = build_url(&self.base_url, path, query)?;
        let accept = accept_header(self.api_version);
        let mut attempt = 0u32;
        loop {
            match self.transport.get(&url, &accept, self.timeout) {
                Err(message) => {
                    if attempt < self.retries {
                        attempt += 1;
                        std::thread::sleep((self.backoff)(attempt));
                        continue;
                    }
                    return Err(Error::Network(message));
                }
                Ok(resp) => {
                    if resp.status == 404 {
                        return Err(Error::NotFound(url));
                    }
                    if (200..300).contains(&resp.status) {
                        return Ok(resp.body);
                    }
                    if retryable(resp.status) && attempt < self.retries {
                        attempt += 1;
                        std::thread::sleep((self.backoff)(attempt));
                        continue;
                    }
                    return Err(Error::Api {
                        status: resp.status,
                        body: resp.body.chars().take(300).collect(),
                    });
                }
            }
        }
    }
}

/// Decoded reply: `data` is the `data` field when present, else the whole body.
#[cfg(feature = "json")]
#[derive(Debug, Clone)]
pub struct Response {
    pub data: serde_json::Value,
    pub raw: serde_json::Value,
}

#[cfg(feature = "json")]
impl<T: Transport> Client<T> {
    /// GET an endpoint and decode the JSON body.
    pub fn get_json(&self, path: &str, query: &[(&str, &str)]) -> Result<Response, Error> {
        let body = self.get(path, query)?;
        let raw: serde_json::Value = serde_json::from_str(&body).map_err(|e| Error::Api {
            status: 200,
            body: format!("response was not valid JSON: {e}"),
        })?;
        let data = raw.get("data").cloned().unwrap_or_else(|| raw.clone());
        Ok(Response { data, raw })
    }
}

/// Blocking HTTPS transport backed by `ureq`.
#[cfg(feature = "ureq-transport")]
pub struct UreqTransport {
    agent: ureq::Agent,
}

#[cfg(feature = "ureq-transport")]
impl UreqTransport {
    pub fn new() -> Self {
        UreqTransport {
            agent: ureq::AgentBuilder::new().build(),
        }
    }
}

#[cfg(feature = "ureq-transport")]
impl Default for UreqTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "ureq-transport")]
impl Transport for UreqTransport {
    fn get(&self, url: &str, accept: &str, timeout: Duration) -> Result<RawResponse, String> {
        let result = self
            .agent
            .get(url)
            .set("Accept", accept)
            .set("User-Agent", USER_AGENT)
            .timeout(timeout)
            .call();
        match result {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.into_string().map_err(|e| e.to_string())?;
                Ok(RawResponse { status, body })
            }
            Err(ureq::Error::Status(status, resp)) => Ok(RawResponse {
                status,
                body: resp.into_string().unwrap_or_default(),
            }),
            Err(ureq::Error::Transport(t)) => Err(t.to_string()),
        }
    }
}

#[cfg(feature = "ureq-transport")]
impl Client<UreqTransport> {
    /// A client using the bundled HTTPS transport.
    pub fn new() -> Self {
        Self::with_transport(UreqTransport::new())
    }
}

#[cfg(feature = "ureq-transport")]
impl Default for Client<UreqTransport> {
    fn default() -> Self {
        Self::new()
    }
}
