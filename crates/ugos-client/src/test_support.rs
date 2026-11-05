//! Local stand-in for the UGOS login endpoints.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::similar_names
)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::Duration;

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};

use crate::totp;

/// What `/v1/verify/code/login` does with `public_key`.
#[derive(Debug)]
pub enum SecondKey {
    /// Field left out of the JSON object.
    Omit,
    /// Field present and empty.
    Empty,
    /// Field present, base64 of this PEM, the same shape `/verify/login` uses.
    Pem(String),
}

/// Scripted answers for one fake NAS.
pub struct Script {
    /// Password login answers with `enable_otp` and no token.
    pub requires_otp: bool,
    /// PEM (not base64) carried by the password-login response.
    pub first_public_key: String,
    /// `public_key` on the code-login response. Ignored when `requires_otp` is false.
    pub second_public_key: SecondKey,
    /// Pending-factor handle returned by the password login.
    pub token_id: String,
    /// The first call that is not a verify endpoint answers 1024 once.
    pub expire_once: bool,
    /// Moved onto the test clock before that 1024, so the next login is a new step.
    pub advance_clock_to: Option<u64>,
}

/// One request the fake NAS accepted.
#[derive(Debug, Clone)]
pub struct Hit {
    /// Path without the query string.
    pub path: String,
    /// Parsed JSON body, or `Null` when there was none.
    pub body: serde_json::Value,
    /// [`totp::unix_now`] at the moment the request arrived.
    pub unix: u64,
    /// Raw request headers, so tests can see what was not sent.
    pub headers: String,
}

/// Fake NAS bound to `127.0.0.1` on an ephemeral port.
pub struct Mock {
    base_url: String,
    addr: SocketAddr,
    log: Arc<Mutex<Vec<Hit>>>,
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Mock {
    /// Start listening. The RSA key used for `/verify/check` is generated once per process.
    pub fn spawn(script: Script) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let log = Arc::new(Mutex::new(Vec::new()));
        let shutdown = Arc::new(AtomicBool::new(false));
        let pem = shared_rsa_pem().to_owned();
        let log_thread = Arc::clone(&log);
        let shutdown_thread = Arc::clone(&shutdown);
        let thread = std::thread::spawn(move || {
            serve(&listener, &pem, &script, &log_thread, &shutdown_thread);
        });
        Self {
            base_url: format!("http://{addr}/ugreen"),
            addr,
            log,
            shutdown,
            thread: Some(thread),
        }
    }

    /// Base URL shaped the way [`crate::auth::login`] expects (`…/ugreen`).
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Requests whose path ends with `suffix`.
    pub fn ending_with(&self, suffix: &str) -> Vec<Hit> {
        self.hits()
            .into_iter()
            .filter(|hit| hit.path.ends_with(suffix))
            .collect()
    }

    fn hits(&self) -> Vec<Hit> {
        self.log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl Drop for Mock {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Serialises tests that move the TOTP clock. Held across `.await` on purpose:
/// a std mutex would pin the worker thread.
pub async fn lock() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    LOCK.lock().await
}

pub fn rsa_pem() -> &'static str {
    shared_rsa_pem()
}

fn shared_rsa_pem() -> &'static str {
    static PEM: OnceLock<String> = OnceLock::new();
    PEM.get_or_init(|| {
        use rsa::pkcs8::EncodePublicKey;
        use rsa::rand_core::OsRng;
        let private = rsa::RsaPrivateKey::new(&mut OsRng, 2048).expect("rsa key");
        private
            .to_public_key()
            .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
            .expect("public key pem")
    })
}

fn serve(
    listener: &TcpListener,
    rsa_pem: &str,
    script: &Script,
    log: &Mutex<Vec<Hit>>,
    shutdown: &AtomicBool,
) {
    let mut expired = false;
    let mut codes = 0u32;
    while !shutdown.load(Ordering::SeqCst) {
        let Ok((mut stream, _)) = listener.accept() else {
            break;
        };
        if shutdown.load(Ordering::SeqCst) {
            break;
        }
        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
        let _ = stream.set_nodelay(true);
        let Some(request) = read_request(&mut stream) else {
            continue;
        };
        if script.expire_once && !expired && is_api(&request.path) {
            if let Some(instant) = script.advance_clock_to {
                totp::force_unix_time(instant);
            }
            expired = true;
            let _ = write_json(
                &mut stream,
                "",
                &envelope(1024, "login expired", &serde_json::json!({})),
            );
            record(log, &request);
            continue;
        }
        let (extra, body) = reply(&request.path, script, rsa_pem, &mut codes);
        let _ = write_json(&mut stream, &extra, &body);
        record(log, &request);
    }
}

fn is_api(path: &str) -> bool {
    !path.ends_with("/verify/check")
        && !path.ends_with("/verify/login")
        && !path.ends_with("/verify/code/login")
}

fn reply(path: &str, script: &Script, rsa_pem: &str, codes: &mut u32) -> (String, String) {
    if path.ends_with("/verify/check") {
        let token = B64.encode(rsa_pem.as_bytes());
        return (
            format!("x-rsa-token: {token}\r\n"),
            envelope(200, "success", &serde_json::json!({})),
        );
    }
    if path.ends_with("/verify/code/login") {
        let token = format!("token-{codes}");
        *codes += 1;
        let mut data = serde_json::json!({ "token": token });
        match &script.second_public_key {
            SecondKey::Omit => {}
            SecondKey::Empty => data["public_key"] = serde_json::json!(""),
            SecondKey::Pem(pem) => data["public_key"] = serde_json::json!(B64.encode(pem)),
        }
        return (String::new(), envelope(200, "success", &data));
    }
    if path.ends_with("/verify/login") {
        let data = if script.requires_otp {
            serde_json::json!({
                "enable_otp": true,
                "token_id": script.token_id,
                "public_key": B64.encode(&script.first_public_key),
            })
        } else {
            serde_json::json!({
                "token": "token-0",
                "enable_otp": false,
                "public_key": B64.encode(&script.first_public_key),
            })
        };
        return (String::new(), envelope(200, "success", &data));
    }
    (
        String::new(),
        envelope(200, "success", &serde_json::json!({"ok": true})),
    )
}

fn envelope(code: i32, msg: &str, data: &serde_json::Value) -> String {
    serde_json::json!({
        "code": code,
        "msg": msg,
        "data": data,
    })
    .to_string()
}

fn record(log: &Mutex<Vec<Hit>>, request: &Request) {
    let unix = totp::unix_now().unwrap_or(0);
    log.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(Hit {
            path: request.path.clone(),
            body: serde_json::from_slice(&request.body).unwrap_or(serde_json::Value::Null),
            unix,
            headers: request.headers.clone(),
        });
}

struct Request {
    path: String,
    headers: String,
    body: Vec<u8>,
}

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 2048];
    let header_end = loop {
        let n = stream.read(&mut tmp).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = buf.windows(4).position(|window| window == b"\r\n\r\n") {
            break pos;
        }
        if buf.len() > 65_536 {
            return None;
        }
    };
    let header_len = header_end + 4;
    let headers = String::from_utf8_lossy(&buf[..header_len]).into_owned();
    let content_length = content_length(&headers);
    let mut body = buf[header_len..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut tmp).ok()?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }
    body.truncate(content_length);
    let path = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/")
        .split('?')
        .next()
        .unwrap_or("/")
        .to_owned();
    Some(Request {
        path,
        headers,
        body,
    })
}

fn content_length(headers: &str) -> usize {
    for line in headers.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            return value.trim().parse().unwrap_or(0);
        }
    }
    0
}

fn write_json(stream: &mut TcpStream, extra_headers: &str, body: &str) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{extra_headers}\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()
}
