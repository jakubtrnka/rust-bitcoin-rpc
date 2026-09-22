//! Raw transaction request and response types.
//!
//! [`Transaction`] is the shape `getrawtransaction` and `decoderawtransaction`
//! both return, and the one `getblock` verbosity 2 embeds in each element of its
//! `tx` array (see [`crate::types::BlockTransaction`]). Bitcoin Core emits it
//! from a single function, `TxToUniv` in `src/core_io.cpp:430`, so one struct
//! covers every caller; the fields only one caller supplies are optional.

use super::amount::{Amount, FeeRate};

/// A transaction output script, as emitted by `ScriptToUniv`
/// (`bitcoin/src/core_io.cpp:409`) with `include_hex=true, include_address=true`.
/// Used both for transaction outputs (here) and for [`crate::types::TxOut`]
/// returned by `gettxout` (`bitcoin/src/rpc/blockchain.cpp:1253`), which calls
/// `ScriptToUniv` with the same arguments.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScriptPubKey {
    /// Disassembly of the output script.
    pub asm: String,
    /// Inferred descriptor for the output.
    pub desc: String,
    /// The raw output script bytes, hex-encoded.
    pub hex: String,
    /// The type, e.g. `pubkeyhash`.
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub script_type: String,
    /// The Bitcoin address. Only present if a well-defined address exists.
    #[cfg_attr(feature = "serde", serde(default))]
    pub address: Option<String>,
}

/// The signature script of a non-coinbase transaction input.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScriptSig {
    /// Disassembly of the signature script.
    pub asm: String,
    /// The raw signature script bytes, hex-encoded.
    pub hex: String,
}

/// The output a [`TxIn`] spends.
///
/// Core fills this in only when the caller asked for prevout detail
/// (`getrawtransaction` verbosity 2, `getblock` verbosity 3) *and* the block's
/// undo data is available (`bitcoin/src/core_io.cpp:479-487`).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TxInPrevout {
    /// Coinbase or not.
    pub generated: bool,
    /// The height of the prevout.
    pub height: u64,
    /// The value.
    pub value: Amount,
    /// The output script.
    #[cfg_attr(feature = "serde", serde(rename = "scriptPubKey"))]
    pub script_pub_key: ScriptPubKey,
}

/// One input of a [`Transaction`].
///
/// A coinbase input carries `coinbase` where every other input carries `txid`,
/// `vout` and `script_sig` (`bitcoin/src/core_io.cpp:454-462`); `sequence` is
/// common to both. The two shapes are one struct with optional fields, as Core
/// documents them.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TxIn {
    /// The coinbase value. Only present on a coinbase transaction's input.
    #[cfg_attr(feature = "serde", serde(default))]
    pub coinbase: Option<String>,
    /// The id of the transaction being spent. Absent on a coinbase input.
    #[cfg_attr(feature = "serde", serde(default))]
    pub txid: Option<String>,
    /// The output number being spent. Absent on a coinbase input.
    #[cfg_attr(feature = "serde", serde(default))]
    pub vout: Option<u64>,
    /// The signature script. Absent on a coinbase input.
    #[cfg_attr(feature = "serde", serde(default, rename = "scriptSig"))]
    pub script_sig: Option<ScriptSig>,
    /// The witness stack, hex-encoded per element. Absent when the input has
    /// no witness.
    #[cfg_attr(feature = "serde", serde(default, rename = "txinwitness"))]
    pub tx_in_witness: Option<Vec<String>>,
    /// The output being spent. Only present when the caller asked for prevout
    /// detail and the block's undo data is available.
    #[cfg_attr(feature = "serde", serde(default))]
    pub prevout: Option<TxInPrevout>,
    /// The script sequence number.
    pub sequence: u64,
}

/// One output of a [`Transaction`].
///
/// Named `TxOutput` rather than `TxOut` because [`crate::types::TxOut`] is
/// already the result of `gettxout`, a different shape.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TxOutput {
    /// The value.
    pub value: Amount,
    /// Index of this output within the transaction.
    pub n: u64,
    /// The output script.
    #[cfg_attr(feature = "serde", serde(rename = "scriptPubKey"))]
    pub script_pub_key: ScriptPubKey,
}

/// A transaction, as returned by `getrawtransaction` at verbosity 1 and above
/// and by `decoderawtransaction`.
///
/// The five block-context fields ([`Transaction::in_active_chain`],
/// [`Transaction::block_hash`], [`Transaction::confirmations`],
/// [`Transaction::time`] and [`Transaction::block_time`]) are added by
/// `getrawtransaction` alone, and only for a transaction found in a block; so
/// is `hex`, which `decoderawtransaction` suppresses. All six are therefore
/// optional even though Core's help text marks `hex` as always present.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Transaction {
    /// Whether the block given as `getrawtransaction`'s `blockhash` argument is
    /// in the active chain. Only present when that argument was given.
    #[cfg_attr(feature = "serde", serde(default))]
    pub in_active_chain: Option<bool>,
    /// The transaction id.
    pub txid: String,
    /// The transaction hash. Differs from `txid` for witness transactions.
    pub hash: String,
    /// The serialized transaction size.
    pub size: u64,
    /// The virtual transaction size. Differs from `size` for witness
    /// transactions.
    pub vsize: u64,
    /// The transaction's weight, between `vsize * 4 - 3` and `vsize * 4`.
    pub weight: u64,
    /// The version.
    pub version: u64,
    /// The lock time.
    pub locktime: u64,
    /// The transaction inputs.
    pub vin: Vec<TxIn>,
    /// The transaction outputs.
    pub vout: Vec<TxOutput>,
    /// The serialized, hex-encoded transaction. Absent from
    /// `decoderawtransaction`, which already received it from the caller.
    #[cfg_attr(feature = "serde", serde(default))]
    pub hex: Option<String>,
    /// The hash of the block containing this transaction, if it is in a block.
    #[cfg_attr(feature = "serde", serde(default, rename = "blockhash"))]
    pub block_hash: Option<String>,
    /// The number of confirmations, or `0` when the containing block is not in
    /// the active chain.
    #[cfg_attr(feature = "serde", serde(default))]
    pub confirmations: Option<i64>,
    /// Same as `block_time`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub time: Option<i64>,
    /// The block time, in seconds since the epoch.
    #[cfg_attr(feature = "serde", serde(default, rename = "blocktime"))]
    pub block_time: Option<i64>,
    /// The transaction fee. Only present at `getrawtransaction`
    /// verbosity 2, and only when the block's undo data is available
    /// (`bitcoin/src/core_io.cpp:520-524`). `getblock` verbosity 2 and 3
    /// inherit this field through [`crate::types::BlockTransaction`]'s
    /// `serde(flatten)`, since Core emits it from the same `TxToUniv` call.
    #[cfg_attr(feature = "serde", serde(default))]
    pub fee: Option<Amount>,
}

/// One input of `createrawtransaction`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CreateRawTransactionInput {
    /// The transaction id to spend from.
    pub txid: String,
    /// The output number to spend.
    pub vout: u32,
    /// The sequence number. Omitted lets the node derive it from the
    /// `replaceable` and `locktime` arguments.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub sequence: Option<u32>,
}

/// One output of `createrawtransaction`.
///
/// Serialized as a one-entry object whose *key* is the destination, which is why
/// this type has a hand-written `Serialize` impl rather than a derived one.
/// It is only ever sent, so it has no `Deserialize`.
#[derive(Debug, Clone, PartialEq)]
pub enum CreateRawTransactionOutput {
    /// Pay `amount` to `address`.
    Address {
        /// Destination address.
        address: String,
        /// Amount to pay. Serialized as the exact BTC decimal Core expects.
        amount: Amount,
    },
    /// An `OP_RETURN` output carrying `hex`.
    Data(String),
}

#[cfg(feature = "serde")]
impl serde::Serialize for CreateRawTransactionOutput {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut m = s.serialize_map(Some(1))?;
        match self {
            CreateRawTransactionOutput::Address { address, amount } => {
                m.serialize_entry(address, amount)?
            }
            CreateRawTransactionOutput::Data(hex) => m.serialize_entry("data", hex)?,
        }
        m.end()
    }
}

/// The fees of a transaction that would be accepted, as reported by
/// `testmempoolaccept`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TestMempoolAcceptFees {
    /// Transaction fee.
    pub base: Amount,
    /// The effective feerate. May differ from the base feerate if, for
    /// example, there are modified fees from `prioritisetransaction` or a
    /// package feerate was used.
    #[cfg_attr(feature = "serde", serde(rename = "effective-feerate"))]
    pub effective_feerate: FeeRate,
    /// Witness hashes of the transactions whose fees and vsizes are included in
    /// `effective_feerate`.
    #[cfg_attr(feature = "serde", serde(rename = "effective-includes"))]
    pub effective_includes: Vec<String>,
}

/// The mempool acceptance test result for one raw transaction.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TestMempoolAcceptResult {
    /// The transaction hash in hex.
    pub txid: String,
    /// The transaction witness hash in hex.
    pub wtxid: String,
    /// Package validation error, if any. Only possible when more than one raw
    /// transaction was submitted.
    #[cfg_attr(feature = "serde", serde(default, rename = "package-error"))]
    pub package_error: Option<String>,
    /// Whether this transaction would be accepted to the mempool and pass the
    /// client-specified `max_fee_rate`. Absent when the transaction was not
    /// fully validated because another transaction in the list failed.
    #[cfg_attr(feature = "serde", serde(default))]
    pub allowed: Option<bool>,
    /// Virtual transaction size as defined in BIP 141. Only present when
    /// `allowed` is `true`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub vsize: Option<u64>,
    /// Transaction fees. Only present when `allowed` is `true`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub fees: Option<TestMempoolAcceptFees>,
    /// Rejection reason. Only present when `allowed` is `false`.
    #[cfg_attr(feature = "serde", serde(default, rename = "reject-reason"))]
    pub reject_reason: Option<String>,
    /// Rejection details. Only present when `allowed` is `false` and rejection
    /// details exist.
    #[cfg_attr(feature = "serde", serde(default, rename = "reject-details"))]
    pub reject_details: Option<String>,
}
