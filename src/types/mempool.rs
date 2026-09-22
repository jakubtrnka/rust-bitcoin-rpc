//! Response types for the mempool RPCs.
//!
//! Transcribed from the `RPCResult` blocks of `src/rpc/mempool.cpp` in
//! Bitcoin Core v31.1. No struct rejects unknown fields, so a newer node adding
//! a field does not break deserialization.

use super::amount::{Amount, FeeRate, SignedAmount};

/// Result of `getmempoolinfo`.
///
/// `fullrbf` is omitted: Core always sends it as `true` and documents it as
/// deprecated (`mempool.cpp:1048,1075`), so it carries no information worth
/// modelling.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MempoolInfo {
    /// True if the initial load attempt of the persisted mempool finished.
    pub loaded: bool,
    /// Current tx count.
    pub size: u64,
    /// Sum of all virtual transaction sizes as defined in BIP 141. Differs
    /// from actual serialized size because witness data is discounted.
    pub bytes: u64,
    /// Total memory usage for the mempool.
    pub usage: u64,
    /// Total fees for the mempool, ignoring modified fees through
    /// prioritisetransaction.
    pub total_fee: Amount,
    /// Maximum memory usage for the mempool.
    #[cfg_attr(feature = "serde", serde(rename = "maxmempool"))]
    pub max_mempool: u64,
    /// Minimum fee rate for tx to be accepted. Is the maximum of
    /// minrelaytxfee and minimum mempool fee.
    #[cfg_attr(feature = "serde", serde(rename = "mempoolminfee"))]
    pub mempool_min_fee: FeeRate,
    /// Current minimum relay fee rate for transactions.
    #[cfg_attr(feature = "serde", serde(rename = "minrelaytxfee"))]
    pub min_relay_tx_fee: FeeRate,
    /// Minimum fee rate increment for mempool limiting or replacement.
    #[cfg_attr(feature = "serde", serde(rename = "incrementalrelayfee"))]
    pub incremental_relay_fee: FeeRate,
    /// Current number of transactions that haven't passed initial broadcast
    /// yet.
    #[cfg_attr(feature = "serde", serde(rename = "unbroadcastcount"))]
    pub unbroadcast_count: u64,
    /// True if the mempool accepts transactions with bare multisig outputs.
    ///
    /// Absent from nodes before Bitcoin Core v30, so `None` there.
    #[cfg_attr(feature = "serde", serde(default, rename = "permitbaremultisig"))]
    pub permit_bare_multisig: Option<bool>,
    /// Maximum number of bytes that can be used by OP_RETURN outputs in the
    /// mempool.
    ///
    /// Absent from nodes before Bitcoin Core v30, so `None` there.
    #[cfg_attr(feature = "serde", serde(default, rename = "maxdatacarriersize"))]
    pub max_datacarrier_size: Option<u64>,
    /// Maximum number of transactions that can be in a cluster (configured by
    /// `-limitclustercount`).
    ///
    /// Absent from nodes before Bitcoin Core v31, so `None` there.
    #[cfg_attr(feature = "serde", serde(default, rename = "limitclustercount"))]
    pub limit_cluster_count: Option<u64>,
    /// Maximum size of a cluster in virtual bytes (configured by
    /// `-limitclustersize`).
    ///
    /// Absent from nodes before Bitcoin Core v31, so `None` there.
    #[cfg_attr(feature = "serde", serde(default, rename = "limitclustersize"))]
    pub limit_cluster_size: Option<u64>,
    /// If the mempool is in a known-optimal transaction ordering.
    ///
    /// Absent from nodes before Bitcoin Core v31, so `None` there.
    #[cfg_attr(feature = "serde", serde(default))]
    pub optimal: Option<bool>,
}

/// Per-fee-context breakdown of a mempool entry's fees.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MempoolEntryFees {
    /// Transaction fee.
    pub base: Amount,
    /// Transaction fee with fee deltas used for mining priority; may be negative.
    pub modified: SignedAmount,
    /// Transaction fees of in-mempool ancestors (including this one) with fee
    /// deltas used for mining priority.
    pub ancestor: SignedAmount,
    /// Transaction fees of in-mempool descendants (including this one) with
    /// fee deltas used for mining priority.
    pub descendant: SignedAmount,
    /// Transaction fees of the chunk, including mining priority deltas.
    ///
    /// Absent from nodes before Bitcoin Core v31, so `None` there.
    #[cfg_attr(feature = "serde", serde(default))]
    pub chunk: Option<SignedAmount>,
}

/// Mempool data for a single transaction, as returned by `getmempoolentry` and
/// as the value type in the verbose form of `getrawmempool`,
/// `getmempoolancestors` and `getmempooldescendants`.
///
/// `bip125-replaceable` is omitted: Core documents it as deprecated
/// (`mempool.cpp:456`) in favor of full-RBF, where every transaction is
/// implicitly replaceable.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MempoolEntry {
    /// Virtual transaction size as defined in BIP 141. This is different from
    /// actual serialized size for witness transactions as witness data is
    /// discounted.
    pub vsize: u64,
    /// Transaction weight as defined in BIP 141.
    pub weight: u64,
    /// Local time transaction entered pool, in seconds since the epoch.
    pub time: i64,
    /// Block height when transaction entered pool.
    pub height: u64,
    /// Number of in-mempool descendant transactions (including this one).
    #[cfg_attr(feature = "serde", serde(rename = "descendantcount"))]
    pub descendant_count: u64,
    /// Virtual transaction size of in-mempool descendants (including this
    /// one).
    #[cfg_attr(feature = "serde", serde(rename = "descendantsize"))]
    pub descendant_size: u64,
    /// Number of in-mempool ancestor transactions (including this one).
    #[cfg_attr(feature = "serde", serde(rename = "ancestorcount"))]
    pub ancestor_count: u64,
    /// Virtual transaction size of in-mempool ancestors (including this one).
    #[cfg_attr(feature = "serde", serde(rename = "ancestorsize"))]
    pub ancestor_size: u64,
    /// Sigops-adjusted weight (as defined in BIP 141 and modified by
    /// `-bytespersigop`) of this transaction's chunk.
    ///
    /// Absent from nodes before Bitcoin Core v31, so `None` there.
    #[cfg_attr(feature = "serde", serde(default, rename = "chunkweight"))]
    pub chunk_weight: Option<u64>,
    /// Hash of serialized transaction, including witness data.
    pub wtxid: String,
    /// Breakdown of this transaction's fees.
    pub fees: MempoolEntryFees,
    /// Unconfirmed transactions used as inputs for this transaction.
    pub depends: Vec<String>,
    /// Unconfirmed transactions spending outputs from this transaction.
    #[cfg_attr(feature = "serde", serde(rename = "spentby"))]
    pub spent_by: Vec<String>,
    /// Whether this transaction is currently unbroadcast (initial broadcast
    /// not yet acknowledged by any peers).
    pub unbroadcast: bool,
}

/// Result of `getrawmempool` with `verbose = false` and `mempool_sequence =
/// true`: the transaction id list plus the mempool sequence number, a
/// distinct shape from the plain array `getrawmempool` otherwise returns.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RawMempoolSequence {
    /// The transaction ids currently in the mempool.
    pub txids: Vec<String>,
    /// The mempool sequence value.
    pub mempool_sequence: u64,
}
