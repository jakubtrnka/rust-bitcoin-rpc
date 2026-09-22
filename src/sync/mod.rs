//! Blocking client. Requires the `sync` feature.
//!
//! This module pulls in no async runtime: `ureq` has no `tokio` in its
//! dependency graph.

mod call;
mod methods;

pub use call::{RpcCall, RpcCallExt};
pub use methods::*;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde_json::Value;

use crate::config::Config;
use crate::jsonrpc::{Request, parse_batch_reply, parse_reply};
use crate::{Auth, Error, Result};

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

    /// Set how long the node may take to start replying, and then how long
    /// the whole reply body may take to arrive, or `None` for no limit.
    /// Defaults to 60 seconds.
    ///
    /// Each phase gets its own budget of this length: one for the response
    /// headers to appear, another for the body to finish. A node that is
    /// alive but slow to answer a heavy call has the full budget to start
    /// replying, while a dead connection is noticed within it.
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
        // silently re-send credentials to another host. `max_redirects(0)`
        // disables redirect handling entirely (ureq's default is 10).
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(self.config.timeout)
            .timeout_connect(self.config.connect_timeout)
            // `ureq` has no idle timeout; the closest equivalent is one
            // budget for the headers to arrive and another for the body.
            .timeout_recv_response(self.config.read_timeout)
            .timeout_recv_body(self.config.read_timeout)
            .http_status_as_error(false)
            .max_redirects(0)
            .build()
            .into();
        Ok(Client {
            agent,
            url: self.config.url,
            authorization,
            max_response_size: self.config.max_response_size,
            next_id: AtomicU64::new(1),
        })
    }
}

/// A blocking Bitcoin Core RPC client.
///
/// Bring the method traits into scope to use it, e.g.
/// `use bitcoin_rpc::sync::BlockchainRpc;` or `use bitcoin_rpc::prelude::*;`.
#[derive(Debug)]
pub struct Client {
    agent: ureq::Agent,
    url: String,
    authorization: Option<String>,
    max_response_size: Option<usize>,
    next_id: AtomicU64,
}

impl Client {
    /// Reserve `count` consecutive request ids, returning the first.
    fn reserve_ids(&self, count: usize) -> u64 {
        self.next_id.fetch_add(count as u64, Ordering::Relaxed)
    }

    /// POST `body` and return the status and (bounded) reply body.
    fn post(&self, body: String) -> Result<(u16, Vec<u8>)> {
        let mut request = self
            .agent
            .post(&self.url)
            .header("Content-Type", "application/json");
        if let Some(auth) = &self.authorization {
            request = request.header("Authorization", auth);
        }

        let mut resp = request
            .send(&body)
            .map_err(|e| Error::Transport(e.to_string()))?;
        let status = resp.status().as_u16();
        // A 500 from the node still carries a usable JSON-RPC error body.
        //
        // `ureq` caps `read_to_vec` at 10 MB unless told otherwise, so the
        // limit is always set explicitly here: to ours, or to "unbounded"
        // when the caller asked for no cap.
        let limit = self.max_response_size;
        let bytes = resp
            .body_mut()
            .with_config()
            .limit(limit.map_or(u64::MAX, |n| n as u64))
            .read_to_vec()
            .map_err(|e| match (e, limit) {
                (ureq::Error::BodyExceedsLimit(_), Some(limit)) => {
                    Error::ResponseTooLarge { limit }
                }
                (e, _) => Error::Transport(e.to_string()),
            })?;

        Ok((status, bytes))
    }
}

impl RpcCall for Client {
    fn call_raw(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.reserve_ids(1);
        let body = serde_json::to_string(&Request::new(id, method, &params))?;
        let (status, bytes) = self.post(body)?;
        parse_reply(status, &bytes, id)
    }

    /// One HTTP request carrying a JSON array of requests with consecutive
    /// ids; the reply array is matched back up by those ids.
    fn call_batch_raw(&self, calls: &[(&str, Value)]) -> Result<Vec<Result<Value>>> {
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
        let (status, bytes) = self.post(body)?;
        parse_batch_reply(status, &bytes, first_id, calls.len())
    }
}
