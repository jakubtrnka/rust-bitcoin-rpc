//! A canned-response HTTP server, so the client tests need no mock-HTTP crate.

pub mod fixtures;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// What the server saw for one request.
#[derive(Debug, Clone)]
pub struct ReceivedRequest {
    pub authorization: Option<String>,
    pub body: String,
}

pub struct MockServer {
    url: String,
    received: Arc<Mutex<Vec<ReceivedRequest>>>,
    connections: Arc<AtomicUsize>,
}

/// Whether a canned reply's `id` is rewritten to echo the request's.
#[derive(Clone, Copy)]
enum IdMode {
    /// Rewrite: fixtures can hard-code `"id":1` while the client keeps
    /// counting, exactly as a real node echoes whatever id it was sent.
    Echo,
    /// Send the body byte-for-byte, so a test can deliberately answer with the
    /// wrong id (or none) and check the client rejects it.
    Verbatim,
}

impl MockServer {
    /// Serve one canned `(status, body)` reply per element, in order, then stop.
    ///
    /// Each reply's JSON-RPC `id` is rewritten to the id of the request that
    /// earned it (for a batch, item by item), the way a real node echoes the
    /// id back. Use [`MockServer::spawn_verbatim`] to send the bytes as given.
    pub fn spawn(replies: Vec<(u16, String)>) -> MockServer {
        Self::spawn_with(replies, IdMode::Echo)
    }

    /// Like [`MockServer::spawn`], but replies are sent exactly as written,
    /// with no `id` rewriting.
    pub fn spawn_verbatim(replies: Vec<(u16, String)>) -> MockServer {
        Self::spawn_with(replies, IdMode::Verbatim)
    }

    /// Accept connections and read each request, but never reply: the
    /// connection is held open, silent, until the client gives up.
    pub fn spawn_silent() -> MockServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let received = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&received);

        std::thread::spawn(move || {
            let mut held = Vec::new();
            while let Ok((mut stream, _)) = listener.accept() {
                if let Some((authorization, buf)) = read_request(&mut stream) {
                    sink.lock().expect("lock").push(ReceivedRequest {
                        authorization,
                        body: String::from_utf8_lossy(&buf).into_owned(),
                    });
                }
                // Keep the socket open so the client sees silence, not EOF.
                held.push(stream);
            }
        });

        MockServer {
            url: format!("http://{addr}"),
            received,
            connections: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Serve the canned replies over keep-alive connections: a connection
    /// stays open and answers request after request until the client closes
    /// it. [`MockServer::connections`] counts how many the client opened, so
    /// a test can tell a reused connection from a fresh one.
    pub fn spawn_keep_alive(replies: Vec<(u16, String)>) -> MockServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let received = Arc::new(Mutex::new(Vec::new()));
        let connections = Arc::new(AtomicUsize::new(0));
        let replies = Arc::new(Mutex::new(std::collections::VecDeque::from(replies)));
        let sink = Arc::clone(&received);
        let opened = Arc::clone(&connections);

        std::thread::spawn(move || {
            while let Ok((mut stream, _)) = listener.accept() {
                opened.fetch_add(1, Ordering::SeqCst);
                let sink = Arc::clone(&sink);
                let replies = Arc::clone(&replies);
                std::thread::spawn(move || {
                    // Read first, then take a reply: a reply popped while
                    // blocked on a connection the client has dropped would
                    // starve the connection it opened instead.
                    while let Some(request) = read_request(&mut stream) {
                        let Some((status, body)) = replies.lock().expect("lock").pop_front() else {
                            return;
                        };
                        reply(
                            &mut stream,
                            status,
                            &body,
                            IdMode::Echo,
                            false,
                            request,
                            &sink,
                        );
                    }
                });
            }
        });

        MockServer {
            url: format!("http://{addr}"),
            received,
            connections,
        }
    }

    /// How many TCP connections the client has opened so far.
    pub fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }

    fn spawn_with(replies: Vec<(u16, String)>, mode: IdMode) -> MockServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let received = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&received);

        std::thread::spawn(move || {
            for (status, body) in replies {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                handle(&mut stream, status, &body, mode, true, &sink);
            }
        });

        MockServer {
            url: format!("http://{addr}"),
            received,
            connections: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn url(&self) -> String {
        self.url.clone()
    }

    pub fn requests(&self) -> Vec<ReceivedRequest> {
        self.received.lock().expect("lock").clone()
    }
}

// Reads one request, then replies.
fn handle(
    stream: &mut TcpStream,
    status: u16,
    body: &str,
    mode: IdMode,
    close: bool,
    sink: &Mutex<Vec<ReceivedRequest>>,
) {
    if let Some(request) = read_request(stream) {
        reply(stream, status, body, mode, close, request, sink);
    }
}

// Records the request and writes the reply, strictly in that order, so a
// client can never observe the reply before `requests()` reflects the
// request that earned it.
fn reply(
    stream: &mut TcpStream,
    status: u16,
    body: &str,
    mode: IdMode,
    close: bool,
    (authorization, buf): (Option<String>, Vec<u8>),
    sink: &Mutex<Vec<ReceivedRequest>>,
) {
    let request = String::from_utf8_lossy(&buf).into_owned();
    let body = match mode {
        IdMode::Echo => echo_ids(&request, body),
        IdMode::Verbatim => body.to_string(),
    };
    let body = body.as_str();
    sink.lock().expect("lock").push(ReceivedRequest {
        authorization,
        body: request,
    });

    let reason = if status == 200 { "OK" } else { "Error" };
    let connection = if close { "close" } else { "keep-alive" };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: {connection}\r\n\r\n{body}",
        body.len()
    );
    // A write failure must not un-record a request we already fully read.
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn read_request(stream: &mut TcpStream) -> Option<(Option<String>, Vec<u8>)> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut authorization = None;
    let mut content_length = 0usize;

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim().to_string();
            if name == "authorization" {
                authorization = Some(value);
            } else if name == "content-length" {
                content_length = value.parse().unwrap_or(0);
            }
        }
    }

    let mut buf = vec![0u8; content_length];
    reader.read_exact(&mut buf).ok()?;
    Some((authorization, buf))
}

/// Copy the request's id(s) into the canned reply, so fixtures written with
/// `"id":1` answer whatever id the client actually sent. A reply that is not
/// JSON, or whose shape does not line up with the request's (object vs array,
/// or a different batch length), is left untouched.
fn echo_ids(request: &str, reply: &str) -> String {
    use serde_json::Value;

    let (Ok(req), Ok(mut rep)) = (
        serde_json::from_str::<Value>(request),
        serde_json::from_str::<Value>(reply),
    ) else {
        return reply.to_string();
    };
    match (&req, &mut rep) {
        (Value::Object(req), Value::Object(rep)) => {
            if let (Some(id), Some(slot)) = (req.get("id"), rep.get_mut("id")) {
                *slot = id.clone();
            }
        }
        (Value::Array(reqs), Value::Array(reps)) if reqs.len() == reps.len() => {
            for (req, rep) in reqs.iter().zip(reps.iter_mut()) {
                let Value::Object(rep) = rep else { continue };
                if let (Some(id), Some(slot)) = (req.get("id"), rep.get_mut("id")) {
                    *slot = id.clone();
                }
            }
        }
        _ => {}
    }
    rep.to_string()
}
