//! Request and response types for the mining RPCs.
//!
//! Transcribed from the `RPCArg`/`RPCResult` blocks of `src/rpc/mining.cpp` in
//! Bitcoin Core v31.1. No response struct rejects unknown fields, so a newer
//! node adding a field does not break deserialization.
//!
//! Mining RPCs follow BIP 22 in using raw satoshi integers rather than the
//! BTC decimal `ValueFromAmount` form used elsewhere, so amount-shaped fields
//! here (`coinbasevalue`, a template transaction's `fee`) are plain `u64`
//! satoshis, not [`Amount`](super::Amount), whose wire form is the decimal.
//! `getmininginfo`'s `blockmintxfee` is the exception: it is a `CFeeRate`
//! printed the usual way.

use std::collections::BTreeMap;

use super::amount::FeeRate;

/// Result of `getmininginfo`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MiningInfo {
    /// The current block height.
    pub blocks: u64,
    /// The block weight (including reserved weight for block header, txs
    /// count and coinbase tx) of the last assembled block. Only present if a
    /// block was ever assembled.
    #[cfg_attr(feature = "serde", serde(default, rename = "currentblockweight"))]
    pub current_block_weight: Option<u64>,
    /// The number of block transactions (excluding coinbase) of the last
    /// assembled block. Only present if a block was ever assembled.
    #[cfg_attr(feature = "serde", serde(default, rename = "currentblocktx"))]
    pub current_block_tx: Option<u64>,
    /// The current nBits, compact representation of the block difficulty target.
    pub bits: String,
    /// The current difficulty.
    pub difficulty: f64,
    /// The current target.
    pub target: String,
    /// The network hashes per second.
    #[cfg_attr(feature = "serde", serde(rename = "networkhashps"))]
    pub network_hash_ps: f64,
    /// The size of the mempool.
    #[cfg_attr(feature = "serde", serde(rename = "pooledtx"))]
    pub pooled_tx: u64,
    /// Minimum feerate of packages selected for block inclusion.
    ///
    /// Absent from nodes before Bitcoin Core v30, so `None` there.
    #[cfg_attr(feature = "serde", serde(default, rename = "blockmintxfee"))]
    pub block_min_tx_fee: Option<FeeRate>,
    /// Current network name.
    pub chain: String,
    /// The block challenge (aka. block script), in hexadecimal. Only present
    /// if the current network is a signet.
    #[cfg_attr(feature = "serde", serde(default))]
    pub signet_challenge: Option<String>,
    /// Information about the next block.
    pub next: MiningInfoNext,
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

/// Information about the next block, held in [`MiningInfo::next`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MiningInfoNext {
    /// The next height.
    pub height: u64,
    /// The next target nBits.
    pub bits: String,
    /// The next difficulty.
    pub difficulty: f64,
    /// The next target.
    pub target: String,
}

/// Argument to `getblocktemplate`: the single `template_request` object.
///
/// `rules` is required by Bitcoin Core, and a segwit-active chain rejects a
/// template request that does not include the `"segwit"` rule, hence the
/// [`Default`] impl sets it.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockTemplateRequest {
    /// This must be set to "template", "proposal" (see BIP 23), or omitted.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub mode: Option<String>,
    /// A list of client-side supported features, e.g. 'longpoll', 'coinbasevalue'.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub capabilities: Option<Vec<String>>,
    /// A list of client side supported softfork deployment rule names; must
    /// include `"segwit"` on a segwit-active chain.
    pub rules: Vec<String>,
    /// Delay processing request until the result would vary significantly
    /// from the "longpollid" of a prior template.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            rename = "longpollid",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub longpoll_id: Option<String>,
    /// Proposed block data to check, encoded in hexadecimal; valid only for
    /// `mode == "proposal"`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub data: Option<String>,
}

impl Default for BlockTemplateRequest {
    /// Defaults to the one rule Bitcoin Core requires on a segwit-active chain.
    fn default() -> Self {
        BlockTemplateRequest {
            mode: None,
            capabilities: None,
            rules: vec!["segwit".to_string()],
            longpoll_id: None,
            data: None,
        }
    }
}

/// One non-coinbase transaction offered in a [`BlockTemplate`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockTemplateTransaction {
    /// Transaction data encoded in hexadecimal (byte-for-byte).
    pub data: String,
    /// Transaction hash excluding witness data, shown in byte-reversed hex.
    pub txid: String,
    /// Transaction hash including witness data, shown in byte-reversed hex.
    pub hash: String,
    /// Transactions before this one (by 1-based index in the `transactions`
    /// list) that must be present in the final block if this one is.
    pub depends: Vec<u64>,
    /// Difference in value between transaction inputs and outputs, in
    /// satoshis. The coinbase transaction never appears in this list, so this
    /// crate's transcription of Core v31.1 never observes the negative value
    /// BIP 22 documents for a coinbase entry (`mining.cpp:907` skips it via
    /// `if (tx.IsCoinBase()) continue;`), hence unsigned.
    pub fee: u64,
    /// Total SigOps cost, as counted for purposes of block limits.
    #[cfg_attr(feature = "serde", serde(rename = "sigops"))]
    pub sig_ops: u64,
    /// Total transaction weight, as counted for purposes of block limits.
    pub weight: u64,
}

/// Result of `getblocktemplate` in the default `"template"` mode.
///
/// `"proposal"` mode returns a different shape (`null` or a rejection-reason
/// string) and is not modelled here.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockTemplate {
    /// The preferred block version. `nVersion` is a signed 32-bit field that
    /// may have high bits set by version-bits signalling, mirroring
    /// `BlockHeader::version` elsewhere in this crate.
    pub version: i64,
    /// Specific block rules that are to be enforced.
    pub rules: Vec<String>,
    /// Set of pending, supported versionbit (BIP 9) softfork deployments,
    /// keyed by rule name, valued by bit number.
    #[cfg_attr(feature = "serde", serde(rename = "vbavailable"))]
    pub vb_available: BTreeMap<String, u64>,
    /// Supported features, for example 'proposal'.
    pub capabilities: Vec<String>,
    /// Bit mask of versionbits the server requires set in submissions.
    #[cfg_attr(feature = "serde", serde(rename = "vbrequired"))]
    pub vb_required: u64,
    /// The hash of current highest block.
    #[cfg_attr(feature = "serde", serde(rename = "previousblockhash"))]
    pub previous_block_hash: String,
    /// Contents of non-coinbase transactions that should be included in the
    /// next block.
    pub transactions: Vec<BlockTemplateTransaction>,
    /// Data that should be included in the coinbase's scriptSig content,
    /// keyed by name.
    #[cfg_attr(feature = "serde", serde(rename = "coinbaseaux"))]
    pub coinbase_aux: BTreeMap<String, String>,
    /// Maximum allowable input to coinbase transaction, including the
    /// generation award and transaction fees, in satoshis. Pushed directly
    /// from `block.vtx[0]->vout[0].nValue` (`mining.cpp:994`) rather than
    /// through `ValueFromAmount`, so this is an integer per BIP 22.
    #[cfg_attr(feature = "serde", serde(rename = "coinbasevalue"))]
    pub coinbase_value: u64,
    /// An id to include with a request to longpoll on an update to this template.
    #[cfg_attr(feature = "serde", serde(rename = "longpollid"))]
    pub longpoll_id: String,
    /// The hash target.
    pub target: String,
    /// The minimum timestamp appropriate for the next block time, in seconds
    /// since the epoch.
    #[cfg_attr(feature = "serde", serde(rename = "mintime"))]
    pub min_time: i64,
    /// List of ways the block template may be changed, e.g. 'time', 'transactions', 'prevblock'.
    pub mutable: Vec<String>,
    /// A range of valid nonces.
    #[cfg_attr(feature = "serde", serde(rename = "noncerange"))]
    pub nonce_range: String,
    /// Limit of sigops in blocks.
    #[cfg_attr(feature = "serde", serde(rename = "sigoplimit"))]
    pub sigop_limit: u64,
    /// Limit of block size.
    #[cfg_attr(feature = "serde", serde(rename = "sizelimit"))]
    pub size_limit: u64,
    /// Limit of block weight.
    #[cfg_attr(feature = "serde", serde(default, rename = "weightlimit"))]
    pub weight_limit: Option<u64>,
    /// Current timestamp, in seconds since the epoch.
    #[cfg_attr(feature = "serde", serde(rename = "curtime"))]
    pub cur_time: i64,
    /// Compressed target of next block.
    pub bits: String,
    /// The height of the next block.
    pub height: u64,
    /// The block challenge, in hexadecimal. Only present on signet.
    #[cfg_attr(feature = "serde", serde(default))]
    pub signet_challenge: Option<String>,
    /// A valid witness commitment for the unmodified block template.
    #[cfg_attr(feature = "serde", serde(default))]
    pub default_witness_commitment: Option<String>,
}
