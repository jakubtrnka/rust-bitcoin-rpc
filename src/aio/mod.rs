//! Async client. Requires the `aio` feature.
//!
//! This client is built on [`reqwest`](https://crates.io/crates/reqwest) and
//! runs on a `tokio` reactor: `tokio` is a mandatory transitive dependency
//! (via `reqwest` -> `hyper`), and awaiting its futures from another
//! executor (e.g. `smol`, `async-std`) fails at runtime, not compile time.
//! This crate has no *direct* dependency on `tokio` (it is a dev-dependency
//! only, used by this crate's own tests). To opt out of `tokio` entirely,
//! use the `sync` feature instead.

mod call;
mod methods;

pub use call::{BoxFuture, RpcCallAsync, RpcCallAsyncExt};
pub use methods::*;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde_json::Value;

use crate::config::Config;
use crate::jsonrpc::{Request, parse_batch_reply, parse_reply};
use crate::{Auth, Error, Result};

/// Turn a `reqwest` failure into [`Error::Transport`], keeping the cause.
///
/// `reqwest::Error`'s `Display` stops at "error sending request for url
/// (...)"; the reason (timed out, connection refused, ...) is only in its
/// `source()` chain, so walk it and join it on, or the message is useless.
fn transport_error(e: reqwest::Error) -> Error {
    use std::error::Error as _;
    let mut message = e.to_string();
    let mut source = e.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    Error::Transport(message)
}

/// Builds a [`Client`].
#[derive(Debug, Clone)]
pub struct ClientBuilder {
    config: Config,
}

impl ClientBuilder {
    /// Start building a client for the node at `url`, e.g.
    /// `http://127.0.0.1:8332`.
    pub fn new(url: impl Into<String>) -> Self {
        ClientBuilder {
            config: Config::new(url),
        }
    }

    /// Set the credentials. Defaults to [`Auth::None`].
    pub fn auth(mut self, auth: Auth) -> Self {
        self.config.auth = auth;
        self
    }

    /// Set a total deadline for one request, from opening the connection to
    /// the last byte of the reply, or `None` for no overall deadline.
    /// Defaults to `None`: the connect and read timeouts below bound each
    /// phase instead, so a large reply that keeps arriving is never cut off
    /// merely for being large.
    ///
    /// Set this when a caller needs a hard upper bound on how long a call
    /// can take regardless of progress.
    pub fn timeout(mut self, timeout: impl Into<Option<Duration>>) -> Self {
        self.config.timeout = timeout.into();
        self
    }

    /// Set how long to wait for the TCP connection (and TLS handshake, with
    /// the `tls` feature) to be established, or `None` for no limit.
    /// Defaults to 30 seconds.
    pub fn connect_timeout(mut self, timeout: impl Into<Option<Duration>>) -> Self {
        self.config.connect_timeout = timeout.into();
        self
    }

    /// Set how long the node may go without sending anything before the
    /// request is abandoned, or `None` for no limit. Defaults to 60 seconds.
    ///
    /// This is an idle timeout: it restarts after every successful read, so
    /// a large reply that keeps arriving is never cut off, while a node that
    /// has gone silent, before or during its reply, is noticed within it.
    ///
    /// A long-polling method (`wait_for_new_block`, `wait_for_block_height`,
    /// `get_block_template` with a `longpollid`) is cut short by this
    /// timeout well before the RPC-level wait it was asked to make. Pass
    /// `None` here, on a client dedicated to such calls, to let them block
    /// for as long as the node takes to reply.
    pub fn read_timeout(mut self, timeout: impl Into<Option<Duration>>) -> Self {
        self.config.read_timeout = timeout.into();
        self
    }

    /// Cap the size of a reply body, or `None` for no cap. Defaults to
    /// 64 MiB.
    ///
    /// A reply larger than this fails with [`Error::ResponseTooLarge`]
    /// instead of being buffered, so a misbehaving node or proxy cannot make
    /// the client allocate without bound. The default comfortably fits every
    /// response this crate types, including a verbosity-3 `getblock` of a
    /// full block; raise it or pass `None` for a verbose `getrawmempool` on
    /// a very busy node.
    pub fn max_response_size(mut self, limit: impl Into<Option<usize>>) -> Self {
        self.config.max_response_size = limit.into();
        self
    }

    /// Validate the configuration, read the cookie file if one was given, and
    /// construct the client.
    pub fn build(self) -> Result<Client> {
        self.config.validate()?;
        let authorization = self.config.auth.header_value()?;
        // A JSON-RPC endpoint should never redirect; following one could
        // silently re-send credentials to another host.
        let mut builder = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none());
        if let Some(timeout) = self.config.timeout {
            builder = builder.timeout(timeout);
        }
        if let Some(timeout) = self.config.connect_timeout {
            builder = builder.connect_timeout(timeout);
        }
        if let Some(timeout) = self.config.read_timeout {
            builder = builder.read_timeout(timeout);
        }
        let http = builder.build().map_err(|e| Error::Config(e.to_string()))?;
        Ok(Client {
            http,
            url: self.config.url,
            authorization,
            max_response_size: self.config.max_response_size,
            next_id: AtomicU64::new(1),
        })
    }
}

/// An async Bitcoin Core RPC client.
///
/// Bring the method traits into scope to use it, e.g.
/// `use bitcoin_rpc::aio::BlockchainRpc;` or `use bitcoin_rpc::prelude::*;`.
#[derive(Debug)]
pub struct Client {
    http: reqwest::Client,
    url: String,
    authorization: Option<String>,
    max_response_size: Option<usize>,
    next_id: AtomicU64,
}

impl Client {
    /// Read the body into memory, stopping as soon as it is known to exceed
    /// the configured cap: up front from `Content-Length` when the node sends
    /// one, otherwise as the chunks arrive.
    async fn read_body(&self, mut response: reqwest::Response) -> Result<Vec<u8>> {
        let Some(limit) = self.max_response_size else {
            return response
                .bytes()
                .await
                .map(|b| b.to_vec())
                .map_err(transport_error);
        };
        if response.content_length().is_some_and(|n| n > limit as u64) {
            return Err(Error::ResponseTooLarge { limit });
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
            if body.len().saturating_add(chunk.len()) > limit {
                return Err(Error::ResponseTooLarge { limit });
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }
}

impl Client {
    /// Reserve `count` consecutive request ids, returning the first.
    fn reserve_ids(&self, count: usize) -> u64 {
        self.next_id.fetch_add(count as u64, Ordering::Relaxed)
    }

    /// POST `body` and return the status and (bounded) reply body.
    async fn post(&self, body: String) -> Result<(u16, Vec<u8>)> {
        let mut request = self
            .http
            .post(&self.url)
            .header("Content-Type", "application/json")
            .body(body);
        if let Some(auth) = &self.authorization {
            request = request.header("Authorization", auth);
        }

        // No `error_for_status`: a 500 still carries a usable error body.
        let response = request.send().await.map_err(transport_error)?;
        let status = response.status().as_u16();
        let bytes = self.read_body(response).await?;
        Ok((status, bytes))
    }
}

impl RpcCallAsync for Client {
    fn call_raw<'a>(&'a self, method: &'a str, params: Value) -> BoxFuture<'a, Value> {
        Box::pin(async move {
            let id = self.reserve_ids(1);
            let body = serde_json::to_string(&Request::new(id, method, &params))?;
            let (status, bytes) = self.post(body).await?;
            parse_reply(status, &bytes, id)
        })
    }

    /// One HTTP request carrying a JSON array of requests with consecutive
    /// ids; the reply array is matched back up by those ids.
    fn call_batch_raw<'a>(
        &'a self,
        calls: &'a [(&'a str, Value)],
    ) -> BoxFuture<'a, Vec<Result<Value>>> {
        Box::pin(async move {
            if calls.is_empty() {
                return Ok(Vec::new());
            }
            let first_id = self.reserve_ids(calls.len());
            let requests: Vec<Request<'_>> = calls
                .iter()
                .enumerate()
                .map(|(offset, (method, params))| {
                    Request::new(first_id + offset as u64, method, params)
                })
                .collect();
            let body = serde_json::to_string(&requests)?;
            let (status, bytes) = self.post(body).await?;
            parse_batch_reply(status, &bytes, first_id, calls.len())
        })
    }
}
