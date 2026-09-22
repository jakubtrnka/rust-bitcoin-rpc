//! Request and response types for the fee-estimation RPCs.
//!
//! Transcribed from the `RPCResult` block of `estimatesmartfee` in
//! `src/rpc/fees.cpp` in Bitcoin Core v31.1. No response struct rejects
//! unknown fields, so a newer node adding a field does not break
//! deserialization.

use super::amount::FeeRate;

/// Result of `estimatesmartfee`.
///
/// When the node cannot produce an estimate, `feerate` is absent and
/// `errors` carries the reason instead.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FeeEstimate {
    /// Estimated fee rate (only present if no errors were encountered).
    #[cfg_attr(feature = "serde", serde(default))]
    pub feerate: Option<FeeRate>,
    /// Errors encountered during processing, if there are any.
    #[cfg_attr(feature = "serde", serde(default))]
    pub errors: Option<Vec<String>>,
    /// Block number where the estimate was found.
    ///
    /// The request target is clamped between 2 and the highest target fee
    /// estimation is able to return based on how long it has been running.
    /// `FeeCalculation::returnedTarget` (`block_policy_estimator.h:96`) is
    /// filled from a clamped confirmation target, never negative.
    pub blocks: u64,
}
