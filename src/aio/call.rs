//! The async transport trait and its typed extension.

use std::future::Future;
use std::pin::Pin;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::Result;

/// The boxed, `Send` future every [`RpcCallAsync`] method returns.
///
/// Boxing is what keeps the trait object-safe; `Send` is what lets the
/// future be `tokio::spawn`ed.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// An async JSON-RPC transport.
///
/// Boxing the returned future keeps this trait object-safe, so `&dyn
/// RpcCallAsync` works and every typed method trait is available on it. The
/// `Sync` supertrait is what makes the futures `Send`, and therefore spawnable.
pub trait RpcCallAsync: Sync {
    /// Perform one JSON-RPC call.
    ///
    /// `params` is a JSON array for positional arguments or a JSON object for
    /// named arguments; Bitcoin Core accepts either.
    fn call_raw<'a>(&'a self, method: &'a str, params: Value) -> BoxFuture<'a, Value>;

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
    fn call_batch_raw<'a>(
        &'a self,
        calls: &'a [(&'a str, Value)],
    ) -> BoxFuture<'a, Vec<Result<Value>>> {
        Box::pin(async move {
            let mut results = Vec::with_capacity(calls.len());
            for (method, params) in calls {
                results.push(self.call_raw(method, params.clone()).await);
            }
            Ok(results)
        })
    }
}

/// Typed calling convenience, blanket-implemented for every [`RpcCallAsync`].
///
/// This is the hook for adding your own RPC methods: declare a trait bounded on
/// [`RpcCallAsync`], give its methods default bodies that call
/// [`RpcCallAsyncExt::call`], and blanket-implement it.
pub trait RpcCallAsyncExt: RpcCallAsync {
    /// Call `method` and deserialize the result into `R`.
    fn call<'a, R: DeserializeOwned>(
        &'a self,
        method: &'a str,
        params: Value,
    ) -> impl Future<Output = Result<R>> + Send + 'a {
        async move {
            let value = self.call_raw(method, params).await?;
            Ok(serde_json::from_value(value)?)
        }
    }

    /// Call `method` once per element of `params`, as one batch, and
    /// deserialize every result into `R`.
    ///
    /// This is the shape most batching needs take: the same RPC over many
    /// arguments (`getblockhash` for a range of heights, `getblockheader`
    /// for a list of hashes). The first call the node rejects fails the
    /// whole batch with its `Error::Rpc`; use
    /// [`RpcCallAsync::call_batch_raw`] to keep per-call results, or to mix
    /// methods.
    ///
    /// An empty `params` makes no request and returns an empty `Vec`.
    fn call_batch<'a, R: DeserializeOwned>(
        &'a self,
        method: &'a str,
        params: Vec<Value>,
    ) -> impl Future<Output = Result<Vec<R>>> + Send + 'a {
        async move {
            let calls: Vec<(&str, Value)> = params.into_iter().map(|p| (method, p)).collect();
            self.call_batch_raw(&calls)
                .await?
                .into_iter()
                .map(|result| Ok(serde_json::from_value(result?)?))
                .collect()
        }
    }
}

impl<T: RpcCallAsync + ?Sized> RpcCallAsyncExt for T {}
