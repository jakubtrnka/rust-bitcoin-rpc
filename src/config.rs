//! Shared client configuration.

use std::time::Duration;

use crate::{Auth, Error, Result};

/// The default cap on a reply body, 64 MiB.
///
/// Generous for every response this crate types (a verbosity-3 `getblock`
/// of a full 4 MB block is on the order of 20-30 MB of JSON) while still
/// bounding what a misbehaving node or proxy can make the client allocate.
pub(crate) const DEFAULT_MAX_RESPONSE_SIZE: usize = 64 * 1024 * 1024;

/// Default time allowed to establish the TCP (and TLS) connection.
pub(crate) const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Default time allowed for the node to produce the next piece of its reply.
///
/// Long enough that a heavy call (`gettxoutsetinfo`, a verbosity-3
/// `getblock`) is not cut off while the node is still working, short enough
/// that a dead connection is noticed within a minute.
pub(crate) const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub url: String,
    pub auth: Auth,
    /// Total deadline for one request, connect to last body byte.
    pub timeout: Option<Duration>,
    /// Deadline for establishing the connection.
    pub connect_timeout: Option<Duration>,
    /// Deadline for the node to start replying, and to keep the body coming.
    pub read_timeout: Option<Duration>,
    pub max_response_size: Option<usize>,
}

impl Config {
    pub(crate) fn new(url: impl Into<String>) -> Self {
        Config {
            url: url.into(),
            auth: Auth::None,
            timeout: None,
            connect_timeout: Some(DEFAULT_CONNECT_TIMEOUT),
            read_timeout: Some(DEFAULT_READ_TIMEOUT),
            max_response_size: Some(DEFAULT_MAX_RESPONSE_SIZE),
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if !(self.url.starts_with("http://") || self.url.starts_with("https://")) {
            return Err(Error::Config(format!(
                "url must start with http:// or https://, got `{}`",
                self.url
            )));
        }
        if self.max_response_size == Some(0) {
            return Err(Error::Config(
                "max_response_size must be greater than zero (use None for no limit)".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_no_auth_split_timeouts_and_a_64mib_cap() {
        let c = Config::new("http://127.0.0.1:8332");
        assert_eq!(c.auth, crate::Auth::None);
        assert_eq!(c.timeout, None);
        assert_eq!(c.connect_timeout, Some(Duration::from_secs(30)));
        assert_eq!(c.read_timeout, Some(Duration::from_secs(60)));
        assert_eq!(c.max_response_size, Some(64 * 1024 * 1024));
    }

    #[test]
    fn rejects_a_zero_response_size_limit() {
        let mut c = Config::new("http://127.0.0.1:8332");
        c.max_response_size = Some(0);
        assert!(matches!(c.validate(), Err(crate::Error::Config(_))));
        c.max_response_size = None;
        assert!(c.validate().is_ok());
    }

    #[test]
    fn rejects_non_http_url() {
        let c = Config::new("127.0.0.1:8332");
        assert!(matches!(c.validate(), Err(crate::Error::Config(_))));
    }

    #[test]
    fn accepts_http_and_https() {
        assert!(Config::new("http://127.0.0.1:8332").validate().is_ok());
        assert!(Config::new("https://node.example:8332").validate().is_ok());
    }
}
