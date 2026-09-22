//! Request and response types for the PSBT RPCs.
//!
//! Transcribed from the `RPCResult` blocks of `decodepsbt`, `analyzepsbt`,
//! `finalizepsbt` and `descriptorprocesspsbt` (`src/rpc/rawtransaction.cpp`)
//! in Bitcoin Core v31.1. No response struct rejects unknown fields, so a
//! newer node adding a field does not break deserialization.

use std::collections::BTreeMap;

use super::amount::{Amount, FeeRate};
use super::rawtx::{ScriptPubKey, ScriptSig, Transaction};

/// The signature hash type to sign a PSBT input with.
///
/// Only ever sent, so it has no `Deserialize`. `Default` means BIP 341 default
/// signing for Taproot inputs; Core substitutes `All` for everything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum SighashType {
    /// `DEFAULT`
    #[cfg_attr(feature = "serde", serde(rename = "DEFAULT"))]
    Default,
    /// `ALL`
    #[cfg_attr(feature = "serde", serde(rename = "ALL"))]
    All,
    /// `NONE`
    #[cfg_attr(feature = "serde", serde(rename = "NONE"))]
    None,
    /// `SINGLE`
    #[cfg_attr(feature = "serde", serde(rename = "SINGLE"))]
    Single,
    /// `ALL|ANYONECANPAY`
    #[cfg_attr(feature = "serde", serde(rename = "ALL|ANYONECANPAY"))]
    AllAnyoneCanPay,
    /// `NONE|ANYONECANPAY`
    #[cfg_attr(feature = "serde", serde(rename = "NONE|ANYONECANPAY"))]
    NoneAnyoneCanPay,
    /// `SINGLE|ANYONECANPAY`
    #[cfg_attr(feature = "serde", serde(rename = "SINGLE|ANYONECANPAY"))]
    SingleAnyoneCanPay,
}

/// Result of `finalizepsbt`.
///
/// Exactly one of `psbt` and `hex` is present: `hex` when the caller asked to
/// extract and the transaction was complete, `psbt` otherwise.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtFinalization {
    /// The base64-encoded PSBT, if it was not extracted.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub psbt: Option<String>,
    /// The hex-encoded network transaction, if it was extracted.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub hex: Option<String>,
    /// Whether the transaction has a complete set of signatures.
    pub complete: bool,
}

/// Result of `descriptorprocesspsbt`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtProcessResult {
    /// The base64-encoded PSBT, always present.
    pub psbt: String,
    /// Whether the transaction has a complete set of signatures.
    pub complete: bool,
    /// The hex-encoded network transaction. Present only when `complete`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub hex: Option<String>,
}

/// Result of `analyzepsbt`.
///
/// `next` is the only field Core always fills. A PSBT it cannot even
/// deserialize yields `error` and no `inputs`; the fee and size estimates
/// appear only once every input's UTXO is known.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtAnalysis {
    /// One entry per input, in transaction order.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub inputs: Option<Vec<PsbtInputAnalysis>>,
    /// Estimated vsize of the final signed transaction.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub estimated_vsize: Option<u64>,
    /// Estimated feerate of the final signed transaction. Present only once
    /// all of the PSBT's UTXO slots are filled.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub estimated_feerate: Option<FeeRate>,
    /// The transaction fee paid. Present only once all of the PSBT's UTXO
    /// slots are filled.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub fee: Option<Amount>,
    /// Role of the next person this PSBT needs to go to: one of `creator`,
    /// `updater`, `signer`, `finalizer` or `extractor`.
    pub next: String,
    /// Why the PSBT cannot proceed, if it cannot.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub error: Option<String>,
}

/// One input's entry in an [`PsbtAnalysis`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtInputAnalysis {
    /// Whether a UTXO is provided for this input.
    pub has_utxo: bool,
    /// Whether this input is finalized.
    pub is_final: bool,
    /// What this input still needs. Absent once nothing is missing.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub missing: Option<PsbtMissing>,
    /// Role of the next person this input needs to go to.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub next: Option<String>,
}

/// What an input of an analyzed PSBT still needs.
///
/// Core spells these `redeemscript` and `witnessscript` here, but
/// `redeem_script` and `witness_script` in `decodepsbt`. The Rust names are
/// snake_case in both places; only the wire names differ.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtMissing {
    /// Key IDs (hash160 of the public key) whose BIP 32 derivation path is
    /// missing.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub pubkeys: Option<Vec<String>>,
    /// Key IDs (hash160 of the public key) whose signature is missing.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub signatures: Option<Vec<String>>,
    /// Hash160 of the missing redeem script.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            rename = "redeemscript",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub redeem_script: Option<String>,
    /// SHA256 of the missing witness script.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            rename = "witnessscript",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub witness_script: Option<String>,
}

/// A BIP 32 extended public key of a PSBT's global map.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtGlobalXpub {
    /// The extended public key this path corresponds to.
    pub xpub: String,
    /// The fingerprint of the master key.
    pub master_fingerprint: String,
    /// The derivation path.
    pub path: String,
}

/// One entry of a PSBT proprietary map.
///
/// Proprietary fields are namespaced by `identifier` and `subtype`, so a
/// consumer that does not recognise a namespace can pass the entry through
/// untouched.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtProprietary {
    /// The proprietary identifier, hex-encoded. May be empty.
    pub identifier: String,
    /// The subtype number. `uint64_t` in Core, not a byte.
    pub subtype: u64,
    /// The key, hex-encoded.
    pub key: String,
    /// The value, hex-encoded.
    pub value: String,
}

/// The output a PSBT input spends, when it is a witness output.
///
/// A non-witness input carries the whole previous transaction instead, as
/// [`PsbtInput::non_witness_utxo`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtWitnessUtxo {
    /// The value.
    pub amount: Amount,
    /// The output script.
    #[cfg_attr(feature = "serde", serde(rename = "scriptPubKey"))]
    pub script_pub_key: ScriptPubKey,
}

/// A redeem or witness script of a PSBT input or output.
///
/// Unlike [`ScriptPubKey`] this carries no `desc` or `address`: Core emits only
/// the disassembly, the bytes and the type for these.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtScript {
    /// Disassembly of the script.
    pub asm: String,
    /// The raw script bytes, hex-encoded.
    pub hex: String,
    /// The type, e.g. `pubkeyhash`.
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub script_type: String,
}

/// A BIP 32 derivation path for one public key of a PSBT input or output.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtBip32Deriv {
    /// The public key this path corresponds to.
    pub pubkey: String,
    /// The fingerprint of the master key.
    pub master_fingerprint: String,
    /// The derivation path.
    pub path: String,
}

/// A Taproot script path signature.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtTaprootScriptPathSig {
    /// The x-only pubkey for this signature.
    pub pubkey: String,
    /// The leaf hash for this signature.
    pub leaf_hash: String,
    /// The signature itself.
    pub sig: String,
}

/// A Taproot leaf script of a PSBT input, with the control blocks that prove
/// its inclusion in the tree.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtTaprootScript {
    /// The leaf script, hex-encoded.
    pub script: String,
    /// The leaf version, e.g. 192 (`0xc0`) for BIP 342 tapscript.
    pub leaf_ver: u8,
    /// The control blocks for this script, hex-encoded.
    pub control_blocks: Vec<String>,
}

/// A Taproot BIP 32 derivation path, which additionally records every leaf the
/// key appears in.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtTaprootBip32Deriv {
    /// The x-only public key this path corresponds to.
    pub pubkey: String,
    /// The fingerprint of the master key.
    pub master_fingerprint: String,
    /// The derivation path.
    pub path: String,
    /// Hashes of the leaves this pubkey appears in. Empty for a key-path-only
    /// key.
    pub leaf_hashes: Vec<String>,
}

/// One leaf of a PSBT output's Taproot tree, in depth-first search order.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtTaprootLeaf {
    /// The depth of this element in the tree.
    pub depth: u8,
    /// The version of this leaf.
    pub leaf_ver: u8,
    /// The script itself, hex-encoded.
    pub script: String,
}

/// The participants aggregated into one MuSig2 (BIP 373) aggregate public key.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtMusig2Participants {
    /// The compressed aggregate public key the participants create.
    pub aggregate_pubkey: String,
    /// The compressed public keys aggregated into `aggregate_pubkey`.
    pub participant_pubkeys: Vec<String>,
}

/// One participant's MuSig2 public nonce.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtMusig2Pubnonce {
    /// The compressed public key of the participant that created this pubnonce.
    pub participant_pubkey: String,
    /// The compressed aggregate public key this pubnonce is for.
    pub aggregate_pubkey: String,
    /// The hash of the leaf script containing the aggregate pubkey being signed
    /// for. Absent when signing for the internal key.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub leaf_hash: Option<String>,
    /// The public nonce itself.
    pub pubnonce: String,
}

/// One participant's MuSig2 partial signature.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtMusig2PartialSig {
    /// The compressed public key of the participant that created this partial
    /// signature.
    pub participant_pubkey: String,
    /// The compressed aggregate public key this partial signature is for.
    pub aggregate_pubkey: String,
    /// The hash of the leaf script containing the aggregate pubkey being signed
    /// for. Absent when signing for the internal key.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub leaf_hash: Option<String>,
    /// The partial signature itself.
    pub partial_sig: String,
}

/// One input of a decoded PSBT.
///
/// Every field is optional: a freshly created PSBT has none of them, and each
/// role fills in the ones it can. The field order below is Core's emission
/// order.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtInput {
    /// The previous transaction in full, for a non-witness input.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub non_witness_utxo: Option<Transaction>,
    /// The output being spent, for a witness input.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub witness_utxo: Option<PsbtWitnessUtxo>,
    /// Signatures collected so far, keyed by the public key that made
    /// each one.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub partial_signatures: Option<BTreeMap<String, String>>,
    /// The sighash type to be used, e.g. `ALL`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub sighash: Option<String>,
    /// The redeem script, for a P2SH input.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub redeem_script: Option<PsbtScript>,
    /// The witness script, for a P2WSH input.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub witness_script: Option<PsbtScript>,
    /// BIP 32 derivation paths for the keys this input needs.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub bip32_derivs: Option<Vec<PsbtBip32Deriv>>,
    /// The finalized signature script. Present only once the input is
    /// finalized, and only for a non-witness or P2SH-wrapped input.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            rename = "final_scriptSig",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub final_script_sig: Option<ScriptSig>,
    /// The finalized witness stack, hex-encoded. Present only once the
    /// input is finalized.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub final_scriptwitness: Option<Vec<String>>,
    /// RIPEMD160 preimages, keyed by hash.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub ripemd160_preimages: Option<BTreeMap<String, String>>,
    /// SHA256 preimages, keyed by hash.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub sha256_preimages: Option<BTreeMap<String, String>>,
    /// HASH160 preimages, keyed by hash.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub hash160_preimages: Option<BTreeMap<String, String>>,
    /// HASH256 preimages, keyed by hash.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub hash256_preimages: Option<BTreeMap<String, String>>,
    /// The signature for a Taproot key path spend, hex-encoded.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub taproot_key_path_sig: Option<String>,
    /// Signatures for Taproot script path spends.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub taproot_script_path_sigs: Option<Vec<PsbtTaprootScriptPathSig>>,
    /// Taproot leaf scripts and their control blocks.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub taproot_scripts: Option<Vec<PsbtTaprootScript>>,
    /// Taproot BIP 32 derivation paths.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub taproot_bip32_derivs: Option<Vec<PsbtTaprootBip32Deriv>>,
    /// The Taproot x-only internal key, hex-encoded.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub taproot_internal_key: Option<String>,
    /// The Taproot merkle root, hex-encoded.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub taproot_merkle_root: Option<String>,
    /// MuSig2 participant public keys, per aggregate key.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub musig2_participant_pubkeys: Option<Vec<PsbtMusig2Participants>>,
    /// MuSig2 public nonces.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub musig2_pubnonces: Option<Vec<PsbtMusig2Pubnonce>>,
    /// MuSig2 partial signatures.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub musig2_partial_sigs: Option<Vec<PsbtMusig2PartialSig>>,
    /// Input fields this version of Core does not recognise, hex-encoded
    /// key to hex-encoded value.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub unknown: Option<BTreeMap<String, String>>,
    /// The input proprietary map.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub proprietary: Option<Vec<PsbtProprietary>>,
}

/// One output of a decoded PSBT.
///
/// As with [`PsbtInput`], every field is optional and the order below is
/// Core's emission order.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtOutput {
    /// The redeem script, for a P2SH output.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub redeem_script: Option<PsbtScript>,
    /// The witness script, for a P2WSH output.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub witness_script: Option<PsbtScript>,
    /// BIP 32 derivation paths for this output's keys.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub bip32_derivs: Option<Vec<PsbtBip32Deriv>>,
    /// The Taproot x-only internal key, hex-encoded.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub taproot_internal_key: Option<String>,
    /// The leaves of the Taproot tree, in depth-first search order.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub taproot_tree: Option<Vec<PsbtTaprootLeaf>>,
    /// Taproot BIP 32 derivation paths.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub taproot_bip32_derivs: Option<Vec<PsbtTaprootBip32Deriv>>,
    /// MuSig2 participant public keys, per aggregate key.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub musig2_participant_pubkeys: Option<Vec<PsbtMusig2Participants>>,
    /// Output fields this version of Core does not recognise,
    /// hex-encoded key to hex-encoded value.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub unknown: Option<BTreeMap<String, String>>,
    /// The output proprietary map.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub proprietary: Option<Vec<PsbtProprietary>>,
}

/// Result of `decodepsbt`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PsbtDecoded {
    /// The unsigned transaction the PSBT is built around. Same layout as
    /// `decoderawtransaction` produces.
    pub tx: Transaction,
    /// The global BIP 32 extended public keys. Emitted even when empty.
    pub global_xpubs: Vec<PsbtGlobalXpub>,
    /// The PSBT version number, not to be confused with the unsigned
    /// transaction's version.
    pub psbt_version: u32,
    /// The global proprietary map. Emitted even when empty.
    pub proprietary: Vec<PsbtProprietary>,
    /// Global fields this version of Core does not recognise, hex-encoded key
    /// to hex-encoded value. Emitted even when empty.
    pub unknown: BTreeMap<String, String>,
    /// One entry per input, in transaction order.
    pub inputs: Vec<PsbtInput>,
    /// One entry per output, in transaction order.
    pub outputs: Vec<PsbtOutput>,
    /// The fee. Present only once every input's UTXO is known.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub fee: Option<Amount>,
}
