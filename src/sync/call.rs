//! The blocking transport trait and its typed extension.

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::Result;

/// A blocking JSON-RPC transport.
///
/// This is the only trait a client implements, and it is deliberately
/// object-safe: `&dyn RpcCall` and `Box<dyn RpcCall>` both work, and every typed
/// method trait in this crate is available on them.
pub trait RpcCall {
    /// Perform one JSON-RPC call.
    ///
    /// `params` is a JSON array for positional arguments or a JSON object for
    /// named arguments; Bitcoin Core accepts either.
    fn call_raw(&self, method: &str, params: Value) -> Result<Value>;

    /// Perform several JSON-RPC calls in one round trip.
    ///
    /// The outer `Result` is the exchange as a whole (the HTTP request, and
    /// whether the reply could be matched up with what was sent); each inner
    /// `Result` is one call, in the order given, carrying that call's own
    /// `Error::Rpc` when the node rejected just it.
    ///
    /// The default implementation simply makes the calls one after another,
    /// so any transport gets the method for free. [`super::Client`] overrides
    /// it with a real JSON-RPC 2.0 batch: one HTTP request, one reply array,
    /// answers correlated by id.
    fn call_batch_raw(&self, calls: &[(&str, Value)]) -> Result<Vec<Result<Value>>> {
        Ok(calls
            .iter()
            .map(|(method, params)| self.call_raw(method, params.clone()))
            .collect())
    }
}

/// Typed calling convenience, blanket-implemented for every [`RpcCall`].
///
/// This is the hook for adding your own RPC methods: declare a trait bounded on
/// [`RpcCall`], give its methods default bodies that call [`RpcCallExt::call`],
/// and blanket-implement it.
pub trait RpcCallExt: RpcCall {
    /// Call `method` and deserialize the result into `R`.
    fn call<R: DeserializeOwned>(&self, method: &str, params: Value) -> Result<R> {
        Ok(serde_json::from_value(self.call_raw(method, params)?)?)
    }

    /// Call `method` once per element of `params`, as one batch, and
    /// deserialize every result into `R`.
    ///
    /// This is the shape most batching needs take: the same RPC over many
    /// arguments (`getblockhash` for a range of heights, `getblockheader`
    /// for a list of hashes). The first call the node rejects fails the
    /// whole batch with its `Error::Rpc`; use [`RpcCall::call_batch_raw`] to
    /// keep per-call results, or to mix methods.
    ///
    /// An empty `params` makes no request and returns an empty `Vec`.
    fn call_batch<R: DeserializeOwned>(&self, method: &str, params: Vec<Value>) -> Result<Vec<R>> {
        let calls: Vec<(&str, Value)> = params.into_iter().map(|p| (method, p)).collect();
        self.call_batch_raw(&calls)?
            .into_iter()
            .map(|result| Ok(serde_json::from_value(result?)?))
            .collect()
    }
}

impl<T: RpcCall + ?Sized> RpcCallExt for T {}
