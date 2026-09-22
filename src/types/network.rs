//! Response types for the network RPCs.
//!
//! Transcribed from the `RPCResult` blocks of `src/rpc/net.cpp` in Bitcoin
//! Core v31.1. No struct rejects unknown fields, so a newer node adding a
//! field does not break deserialization.

use std::collections::BTreeMap;

use super::amount::FeeRate;

/// Result of `getnetworkinfo`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NetworkInfo {
    /// The server version.
    pub version: u64,
    /// The server subversion string.
    pub subversion: String,
    /// The protocol version.
    #[cfg_attr(feature = "serde", serde(rename = "protocolversion"))]
    pub protocol_version: u64,
    /// The services we offer to the network, as a hex bitmask.
    #[cfg_attr(feature = "serde", serde(rename = "localservices"))]
    pub local_services: String,
    /// The services we offer to the network, in human-readable form.
    #[cfg_attr(feature = "serde", serde(rename = "localservicesnames"))]
    pub local_services_names: Vec<String>,
    /// True if transaction relay is requested from peers.
    #[cfg_attr(feature = "serde", serde(rename = "localrelay"))]
    pub local_relay: bool,
    /// The time offset, in seconds.
    #[cfg_attr(feature = "serde", serde(rename = "timeoffset"))]
    pub time_offset: i64,
    /// The total number of connections.
    pub connections: u64,
    /// The number of inbound connections.
    pub connections_in: u64,
    /// The number of outbound connections.
    pub connections_out: u64,
    /// Whether p2p networking is enabled.
    #[cfg_attr(feature = "serde", serde(rename = "networkactive"))]
    pub network_active: bool,
    /// Information per network.
    pub networks: Vec<NetworkEntry>,
    /// Minimum relay fee rate for transactions.
    #[cfg_attr(feature = "serde", serde(rename = "relayfee"))]
    pub relay_fee: FeeRate,
    /// Minimum fee rate increment for mempool limiting or replacement.
    #[cfg_attr(feature = "serde", serde(rename = "incrementalfee"))]
    pub incremental_fee: FeeRate,
    /// List of local addresses.
    #[cfg_attr(feature = "serde", serde(rename = "localaddresses"))]
    pub local_addresses: Vec<LocalAddress>,
    /// Any network and blockchain warnings.
    ///
    /// Accepts either the modern array wire form or the legacy bare-string
    /// form emitted by a node run with `-deprecatedrpc=warnings`.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "crate::types::serde_helpers::string_or_seq_string"
        )
    )]
    pub warnings: Vec<String>,
}

/// One entry of [`NetworkInfo::networks`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NetworkEntry {
    /// Network name.
    pub name: String,
    /// Whether the network is limited using `-onlynet`.
    pub limited: bool,
    /// Whether the network is reachable.
    pub reachable: bool,
    /// The proxy used for this network as `host:port`, or empty if none.
    pub proxy: String,
    /// Whether randomized credentials are used.
    pub proxy_randomize_credentials: bool,
}

/// One entry of [`NetworkInfo::local_addresses`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LocalAddress {
    /// Network address.
    pub address: String,
    /// Network port.
    pub port: u16,
    /// Relative score.
    pub score: i64,
}

/// Data about one connected network peer, as returned by `getpeerinfo`.
///
/// `startingheight` is omitted: it is only present under
/// `-deprecatedrpc=startingheight`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PeerInfo {
    /// Peer index.
    pub id: u64,
    /// The IP address/hostname optionally followed by :port of the peer.
    pub addr: String,
    /// Bind address of the connection to the peer.
    #[cfg_attr(feature = "serde", serde(default, rename = "addrbind"))]
    pub addr_bind: Option<String>,
    /// Local address as reported by the peer.
    #[cfg_attr(feature = "serde", serde(default, rename = "addrlocal"))]
    pub addr_local: Option<String>,
    /// Network the peer connected through.
    pub network: String,
    /// Mapped AS (Autonomous System) number at the end of the BGP route to
    /// the peer, used for diversifying peer selection (only present if the
    /// `-asmap` config option is set).
    #[cfg_attr(feature = "serde", serde(default))]
    pub mapped_as: Option<u64>,
    /// The services offered, as a hex bitmask.
    pub services: String,
    /// The services offered, in human-readable form.
    #[cfg_attr(feature = "serde", serde(rename = "servicesnames"))]
    pub services_names: Vec<String>,
    /// Whether we relay transactions to this peer.
    #[cfg_attr(feature = "serde", serde(rename = "relaytxes"))]
    pub relay_txes: bool,
    /// Mempool sequence number of this peer's last INV.
    ///
    /// Absent from nodes before Bitcoin Core v31, so `None` there.
    #[cfg_attr(feature = "serde", serde(default))]
    pub last_inv_sequence: Option<u64>,
    /// How many txs we have queued to announce to this peer.
    ///
    /// Absent from nodes before Bitcoin Core v31, so `None` there.
    #[cfg_attr(feature = "serde", serde(default))]
    pub inv_to_send: Option<u64>,
    /// The unix epoch time of the last send.
    #[cfg_attr(feature = "serde", serde(rename = "lastsend"))]
    pub last_send: i64,
    /// The unix epoch time of the last receive.
    #[cfg_attr(feature = "serde", serde(rename = "lastrecv"))]
    pub last_recv: i64,
    /// The unix epoch time of the last valid transaction received from this
    /// peer.
    pub last_transaction: i64,
    /// The unix epoch time of the last block received from this peer.
    pub last_block: i64,
    /// The total bytes sent.
    #[cfg_attr(feature = "serde", serde(rename = "bytessent"))]
    pub bytes_sent: u64,
    /// The total bytes received.
    #[cfg_attr(feature = "serde", serde(rename = "bytesrecv"))]
    pub bytes_recv: u64,
    /// The unix epoch time of the connection.
    #[cfg_attr(feature = "serde", serde(rename = "conntime"))]
    pub conn_time: i64,
    /// The time offset in seconds.
    #[cfg_attr(feature = "serde", serde(rename = "timeoffset"))]
    pub time_offset: i64,
    /// The last ping time in seconds, if any.
    #[cfg_attr(feature = "serde", serde(default, rename = "pingtime"))]
    pub ping_time: Option<f64>,
    /// The minimum observed ping time in seconds, if any.
    #[cfg_attr(feature = "serde", serde(default, rename = "minping"))]
    pub min_ping: Option<f64>,
    /// The duration in seconds of an outstanding ping (if non-zero).
    #[cfg_attr(feature = "serde", serde(default, rename = "pingwait"))]
    pub ping_wait: Option<f64>,
    /// The peer version, such as 70001.
    pub version: u64,
    /// The string version.
    #[cfg_attr(feature = "serde", serde(rename = "subver"))]
    pub sub_ver: String,
    /// Inbound (true) or Outbound (false).
    pub inbound: bool,
    /// Whether we selected peer as (compact blocks) high-bandwidth peer.
    pub bip152_hb_to: bool,
    /// Whether peer selected us as (compact blocks) high-bandwidth peer.
    pub bip152_hb_from: bool,
    /// The current height of header pre-synchronization with this peer, or
    /// -1 if no low-work sync is in progress.
    pub presynced_headers: i64,
    /// The last header we have in common with this peer, or -1 if unknown.
    pub synced_headers: i64,
    /// The last block we have in common with this peer, or -1 if unknown.
    pub synced_blocks: i64,
    /// The heights of blocks we're currently asking from this peer.
    pub inflight: Vec<u64>,
    /// Whether we participate in address relay with this peer.
    pub addr_relay_enabled: bool,
    /// The total number of addresses processed, excluding those dropped due
    /// to rate limiting.
    pub addr_processed: u64,
    /// The total number of addresses dropped due to rate limiting.
    pub addr_rate_limited: u64,
    /// Any special permissions that have been granted to this peer.
    pub permissions: Vec<String>,
    /// The minimum fee rate for transactions this peer accepts.
    #[cfg_attr(feature = "serde", serde(rename = "minfeefilter"))]
    pub min_fee_filter: FeeRate,
    /// The total bytes sent, aggregated by message type. A message type
    /// missing from this map means 0 bytes were sent for it.
    #[cfg_attr(feature = "serde", serde(rename = "bytessent_per_msg"))]
    pub bytes_sent_per_msg: BTreeMap<String, u64>,
    /// The total bytes received, aggregated by message type. A message type
    /// missing from this map means 0 bytes were received for it.
    #[cfg_attr(feature = "serde", serde(rename = "bytesrecv_per_msg"))]
    pub bytes_recv_per_msg: BTreeMap<String, u64>,
    /// Type of connection.
    pub connection_type: String,
    /// Type of transport protocol.
    pub transport_protocol_type: String,
    /// The session ID for this connection, or "" if there is none (v2
    /// transport protocol only).
    pub session_id: String,
}

/// Result of `getnettotals`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NetTotals {
    /// Total bytes received.
    #[cfg_attr(feature = "serde", serde(rename = "totalbytesrecv"))]
    pub total_bytes_recv: u64,
    /// Total bytes sent.
    #[cfg_attr(feature = "serde", serde(rename = "totalbytessent"))]
    pub total_bytes_sent: u64,
    /// Current system unix epoch time, in milliseconds.
    #[cfg_attr(feature = "serde", serde(rename = "timemillis"))]
    pub time_millis: i64,
    /// The outbound traffic limit state.
    #[cfg_attr(feature = "serde", serde(rename = "uploadtarget"))]
    pub upload_target: UploadTarget,
}

/// The outbound traffic limit state, held in [`NetTotals::upload_target`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UploadTarget {
    /// Length of the measuring timeframe in seconds.
    pub timeframe: u64,
    /// Target in bytes.
    pub target: u64,
    /// True if target is reached.
    pub target_reached: bool,
    /// True if serving historical blocks.
    pub serve_historical_blocks: bool,
    /// Bytes left in current time cycle.
    pub bytes_left_in_cycle: u64,
    /// Seconds left in current time cycle.
    pub time_left_in_cycle: u64,
}
