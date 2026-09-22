# bitcoind-rpc-client

A JSON-RPC client for Bitcoin Core, with blocking and async implementations
that share one typed method set. The types follow Core v31.1 and accept
replies from v29 onward.

## Install

```toml
[dependencies]
bitcoind-rpc-client = { version = "0.1", features = ["sync"] }  # or ["aio"], or both
```

The library is imported as `bitcoin_rpc`, not as `bitcoind_rpc_client` — the
package name is the long one, the crate you `use` is the short one.

## Features

Nothing is enabled by default; pick the client(s) you need.

| Feature | What it does |
| --- | --- |
| `serde` | `Serialize`/`Deserialize` on the crate's types. Pulled in automatically by `sync` and `aio`; on its own it adds no client. |
| `sync` | A blocking client built on [`ureq`]. Links no async runtime. |
| `aio` | An async client built on [`reqwest`]. Because of that, it runs on a `tokio` reactor: `tokio` is a mandatory transitive dependency (via `reqwest` → `hyper`) and awaiting the client's futures from another executor (e.g. `smol`, `async-std`) fails at runtime. This crate has no *direct* dependency on `tokio` — it's a dev-dependency only, used by this crate's own tests. If you need to opt out of `tokio` entirely, use `sync` instead. |
| `tls` | TLS support for whichever client(s) are enabled. Rustls only — `native-tls`/OpenSSL is never enabled under any feature combination. `aio` + `tls` needs `cmake` and a C compiler available at build time, to build `aws-lc-sys`. |

[`ureq`]: https://crates.io/crates/ureq
[`reqwest`]: https://crates.io/crates/reqwest

> [!NOTE]
> With `aio` + `tls`, your lockfile will show `openssl-probe` (pulled in via
> `reqwest` → `rustls-platform-verifier` → `rustls-native-certs`). Despite the
> name, it is not a TLS backend: it's a small pure-Rust crate, published by the
> `rustls` org, that only checks well-known filesystem paths for a CA bundle
> (the same paths OpenSSL conventionally uses) so rustls can load one. It has
> no build script and no `-sys` dependency, and never links against the
> OpenSSL library. Its presence does not contradict the "no OpenSSL" claim
> above.

## Quickstart: sync

```rust,no_run
use bitcoin_rpc::prelude::{sync::*, Auth};

# fn main() -> bitcoin_rpc::Result<()> {
let client = ClientBuilder::new("http://127.0.0.1:8332")
    .auth(Auth::cookie_file("/home/user/.bitcoin/.cookie"))
    .build()?;

let info = client.get_blockchain_info()?;
println!("{} blocks on {}", info.blocks, info.chain);
# Ok(())
# }
```

## Quickstart: async

```rust,no_run
use bitcoin_rpc::prelude::{aio::*, Auth};

# #[tokio::main]
# async fn main() -> bitcoin_rpc::Result<()> {
let client = ClientBuilder::new("http://127.0.0.1:8332")
    .auth(Auth::cookie_file("/home/user/.bitcoin/.cookie"))
    .build()?;

let info = client.get_blockchain_info().await?;
println!("{} blocks on {}", info.blocks, info.chain);
# Ok(())
# }
```

These two snippets really do compile: with `sync` and `aio` both enabled,
this whole README is pulled into the crate root as a doctest (see
`src/lib.rs`), so `cargo test --features sync,aio --doc` fails if either one
stops compiling. Beyond compiling, the builder-and-`auth`-and-`build` pattern
is exactly what `tests/sync_client.rs` and `tests/aio_client.rs` use
throughout, `Auth::cookie_file` is unit-tested in `src/auth.rs` and used live
in `examples/btc-cli.rs`, and the `blocks`/`chain` fields on the
`get_blockchain_info` result are pinned by the `get_blockchain_info_full`
fixture test in `tests/serde_fixtures.rs`. Only the network round-trip to a
real node is untested — see [Scope](#scope).

## Configuration

Both `ClientBuilder`s take the same knobs; every one accepts `None` to turn
the limit off.

| Setter | Default | What it bounds |
| --- | --- | --- |
| `auth` | `Auth::None` | Credentials: `Auth::user_pass(..)` or `Auth::cookie_file(..)`. |
| `connect_timeout` | 30 s | Establishing the TCP connection (and TLS handshake). |
| `read_timeout` | 60 s | How long the node may take to start replying, and to keep the body coming. Async: an idle timeout that restarts on every read. Sync: one budget for the headers, another for the body. |
| `timeout` | `None` | A hard overall deadline for the whole request, on top of the two above. |
| `max_response_size` | 64 MiB | The reply body; a larger one fails with `Error::ResponseTooLarge` instead of being buffered. |

A long-polling call (`wait_for_new_block`, `wait_for_block_height`,
`get_block_template` with a `longpollid`) is cut short by `read_timeout`
long before the RPC-level wait it asked for. Build a separate client with
`.read_timeout(None)` for those.

## Batching

Both transport traits can send several calls in one HTTP round trip as a
JSON-RPC 2.0 batch. `call_batch` runs one method over many argument lists and
types every result; `call_batch_raw` mixes methods and keeps each call's own
`Result`, so one rejected call does not hide the others:

```rust,no_run
use bitcoin_rpc::prelude::sync::*;
use serde_json::json;

# fn main() -> bitcoin_rpc::Result<()> {
let client = ClientBuilder::new("http://127.0.0.1:8332").build()?;

// One request, one reply array, results in argument order.
let hashes: Vec<String> = client.call_batch(
    "getblockhash",
    (800_000..800_010).map(|h| json!([h])).collect(),
)?;

// Per-call results, mixed methods.
let results = client.call_batch_raw(&[
    ("getblockcount", json!([])),
    ("getblockhash", json!([u32::MAX])), // this one fails with Error::Rpc
])?;
assert!(results[0].is_ok());
assert!(results[1].is_err());
# let _ = hashes;
# Ok(())
# }
```

`BlockchainRpc::get_block_hashes` and `get_block_headers` are the two
ready-made batched methods, for walking a range of the chain. Replies are
matched to requests by id, never by position, and a reply array that answers
an id twice, skips one, or has the wrong length fails the whole batch with
`Error::Transport`.

Extension traits over `RpcCall`/`RpcCallAsync` get batching for free: the
trait's default `call_batch_raw` falls back to sequential calls, and the
shipped clients override it with a real batch.

## Adding your own RPC

This crate ships 56 typed methods (see [Scope](#scope) below) but not every
RPC Bitcoin Core exposes. Reaching anything else — the wallet RPCs, the
hidden/regtest-only RPCs, or a method a future Core release adds — means
writing your own extension trait over `RpcCall` (sync) or `RpcCallAsync`
(async). This is exactly the mechanism the crate's own typed methods are
built on, and it is exercised end-to-end in `tests/extension_trait.rs`:

```rust
use bitcoin_rpc::Result;
use bitcoin_rpc::sync::{RpcCall, RpcCallExt};
use serde_json::json;

/// A response type a user of this crate would define themselves.
#[derive(Debug, serde::Deserialize)]
pub struct OrphanTx {
    txid: String,
}

/// Written exactly as a downstream crate would, for an RPC we do not ship.
pub trait OrphanRpc: RpcCall {
    fn get_orphan_txs(&self) -> Result<Vec<OrphanTx>> {
        self.call("getorphantxs", json!([2]))
    }
}
impl<T: RpcCall + ?Sized> OrphanRpc for T {}
```

The async version is the same shape, using `RpcCallAsync`/`RpcCallAsyncExt`
and returning `impl Future` instead of a plain `Result`:

```rust
# /// A response type a user of this crate would define themselves.
# #[derive(Debug, serde::Deserialize)]
# pub struct OrphanTx {
#     txid: String,
# }
use bitcoin_rpc::Result;
use bitcoin_rpc::aio::{RpcCallAsync, RpcCallAsyncExt};
use serde_json::json;
use std::future::Future;

/// Written exactly as a downstream crate would, for an RPC we do not ship.
pub trait OrphanRpc: RpcCallAsync {
    fn get_orphan_txs(&self) -> impl Future<Output = Result<Vec<OrphanTx>>> + Send + '_ {
        self.call("getorphantxs", json!([2]))
    }
}
impl<T: RpcCallAsync + ?Sized> OrphanRpc for T {}
```

Both forms use the same blanket-`impl`-over-`RpcCall`/`RpcCallAsync`
mechanism the crate's own shipped traits use, so they compose with those
traits on the same client value rather than conflicting with them — proven
for the sync side by
`user_extension_composes_with_a_shipped_trait_on_the_same_client` in
`tests/extension_trait.rs`.

## Scope

56 typed methods across eight traits: `BlockchainRpc` (17),
`RawTransactionsRpc` (15), `NetworkRpc` (6), `MempoolRpc` (5), `MiningRpc` (5),
`ControlRpc` (4), `UtilRpc` (3), `FeeRpc` (1) — over 48 distinct RPC commands.
The surplus of 8 comes from four commands whose result shape depends on a
verbosity or mode argument, so each is split into a separate typed method: `getblock` (x3:
`get_block_hex`, `get_block`, `get_block_with_txs`), `getrawmempool` (x3:
`get_raw_mempool`, `get_raw_mempool_verbose`, `get_raw_mempool_with_sequence`),
`getblockheader` (x2: `get_block_header`, `get_block_header_hex`), and
`getrawtransaction` (x2: `get_raw_transaction`, `get_raw_transaction_hex`);
plus the two batched forms `get_block_hashes` and `get_block_headers` (see
[Batching](#batching)).

`RawTransactionsRpc` covers the whole wallet-free PSBT family: `createpsbt`,
`decodepsbt`, `analyzepsbt`, `finalizepsbt`, `descriptorprocesspsbt`,
`combinepsbt`, `joinpsbts`, `converttopsbt` and `utxoupdatepsbt`, with
`deriveaddresses` on `UtilRpc`. `decodepsbt`'s result is modelled in full,
including the Taproot fields and the BIP 373 MuSig2 fields added in v31.

Every BTC-denominated field is an `Amount` (whole satoshis) and every fee rate
a `FeeRate` (satoshis per kvB), never an `f64`. Both convert to and from the
eight-decimal JSON number Core puts on the wire exactly, so a value read from
the node and sent back is byte-for-byte what the node emitted, and
`Amount::from_btc(0.1 + 0.2)` is an error rather than a silently wrong
`0.30000000000000004` for Core to reject. Do arithmetic in satoshis.

Not covered, deliberately:

- No wallet RPCs (everything under Bitcoin Core's `wallet` category).
- Three non-wallet `rawtransactions` RPCs: `decodescript`,
  `combinerawtransaction`, and `signrawtransactionwithkey` — the last takes
  raw private keys as an argument, which this crate deliberately has no API
  for.
- No hidden or regtest-only RPCs (e.g. `generatetoaddress`, `invalidateblock`).
- No request batching — one JSON-RPC request per call.
- Deprecated arguments are omitted from method signatures, with one
  exception: `warnings` accepts both Core's modern array form and its legacy
  bare-string form (emitted by a node run with `-deprecatedrpc=warnings`),
  because rejecting the legacy form would otherwise break
  `getblockchaininfo` outright against such a node.

Use the extension-trait pattern above for anything on the "not covered" list.

### Node versions

The result types are transcribed from Bitcoin Core v31.1. A node running
v29 or v30 works too: every field Core added after v29 is an `Option` that
comes back `None` from an older node, and each such field's doc comment
names the release that introduced it (`getblock`'s `coinbase_tx`,
`getmempoolinfo`'s cluster-mempool limits, `getpeerinfo`'s inventory
counters, and so on). Fields a newer node adds that this crate does not know
about are ignored. Nodes before v29 are not supported: v29 is where Core
switched to JSON-RPC 2.0 replies, which the transport relies on.

This crate has been verified against the Bitcoin Core v31.1 source and
against a mock HTTP server (see `tests/`). The PSBT result types are
additionally checked against payloads captured from a live bitcoind v31.1 on
regtest (`tests/data/`): each one is deserialized and re-serialized, so a
field this crate failed to model would show up as missing. Every read-only
method has also been exercised, through both clients, against a live Bitcoin
Core v29.0 mainnet node; `tests/live_node.rs` repeats that survey against any
node you point it at (ignored by default, see its module docs).

## Errors

Every fallible call returns `bitcoin_rpc::Result<T>`, an alias for
`Result<T, Error>`. `Error` has five variants:

- `Config` — the client was misconfigured: a bad URL, or an unreadable or
  malformed cookie file.
- `Transport` — the HTTP request failed, or the node's HTTP-level response
  could not be turned into a JSON-RPC reply (for example, a `401` from a
  node that rejected the supplied credentials arrives this way, naming the
  status in the message). A reply whose JSON-RPC `id` does not match the
  request's is also rejected here: every request carries a fresh id, and a
  reply answering some other id is never handed back as this call's result.
- `Json` — the reply body was not the JSON expected at the JSON-RPC layer.
- `Rpc` — the node executed the request and returned a JSON-RPC error. Its
  `code` is Core's raw `RPC_*` constant from `src/rpc/protocol.h`.
- `ResponseTooLarge` — the reply body exceeded `ClientBuilder::max_response_size`
  (64 MiB by default) and was abandoned rather than buffered. Raise the cap,
  or pass `None` to lift it, for calls that legitimately return more.

`Error` is `#[non_exhaustive]`, so a future release can add variants without
that being a breaking change. Downstream `match` expressions must include a
wildcard arm (`_ => ...`) — matching all five variants today and nothing else
will fail to compile.

## License

MIT, Copyright (c) 2026 Jakub Trnka. See [`LICENSE`](./LICENSE).
