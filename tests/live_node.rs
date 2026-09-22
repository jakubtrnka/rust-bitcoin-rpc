//! Read-only survey of the typed methods against a live node.
//!
//! Ignored by default; point it at a node and run it explicitly:
//!
//! ```text
//! BITCOIN_RPC_URL=http://127.0.0.1:8332 \
//! BITCOIN_RPC_USER=user BITCOIN_RPC_PASSWORD=pass \
//! cargo test --features sync,aio --test live_node -- --ignored --nocapture
//! ```
//!
//! Nothing here mutates the node: no submissions, no peer changes, no `stop`.
//! `createrawtransaction`, `createpsbt` and the PSBT decoders are pure
//! functions of their arguments, and `testmempoolaccept` validates without
//! broadcasting. Nothing the node returns is printed beyond counts and
//! heights, so the output stays free of peer addresses and the like.

#![cfg(any(feature = "sync", feature = "aio"))]

use std::time::Duration;

use bitcoin_rpc::Auth;
use bitcoin_rpc::types::*;

// A public-key-only descriptor from the Bitcoin Core docs.
const DESCRIPTOR: &str = "wpkh([d34db33f/84h/0h/0h]xpub6ERApfZwUNrhLCkDtcHTcxd75RbzS1ed54G1LkBUHQVHQKqhMkhgbmJbZRkrgZw4koxb5JaHWkY4ALHY2grBGRjaDMzQLcgJvLJuZZvRcEL/0/*)#h36s06su";
const ADDRESS: &str = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4";

struct Node {
    url: String,
    auth: Auth,
}

fn node() -> Node {
    let var = |name: &str| {
        std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set to run the live test"))
    };
    Node {
        url: var("BITCOIN_RPC_URL"),
        auth: Auth::user_pass(var("BITCOIN_RPC_USER"), var("BITCOIN_RPC_PASSWORD")),
    }
}

/// Runs every check, then fails once with the whole list, so one incompatible
/// type does not hide the others.
struct Survey {
    failures: Vec<String>,
}

impl Survey {
    fn new() -> Self {
        Survey {
            failures: Vec::new(),
        }
    }

    fn check<T>(&mut self, name: &str, result: bitcoin_rpc::Result<T>) -> Option<T> {
        match result {
            Ok(v) => {
                println!("  ok   {name}");
                Some(v)
            }
            Err(e) => {
                println!("  FAIL {name}: {e}");
                self.failures.push(format!("{name}: {e}"));
                None
            }
        }
    }

    fn finish(self) {
        assert!(
            self.failures.is_empty(),
            "{} method(s) failed against the live node:\n  {}",
            self.failures.len(),
            self.failures.join("\n  ")
        );
    }
}

#[cfg(feature = "sync")]
#[test]
#[ignore = "needs a live node; see the module docs"]
fn sync_client_read_only_survey() {
    use bitcoin_rpc::sync::*;

    let node = node();
    let client = ClientBuilder::new(&node.url)
        .auth(node.auth.clone())
        .read_timeout(Duration::from_secs(120))
        .build()
        .unwrap();
    let mut s = Survey::new();
    println!("sync client against {}", node.url);

    let tip = s
        .check("get_block_count", client.get_block_count())
        .unwrap_or(0) as u32;
    let best = s
        .check("get_best_block_hash", client.get_best_block_hash())
        .unwrap_or_default();
    s.check("get_blockchain_info", client.get_blockchain_info());
    s.check("get_block_hash", client.get_block_hash(tip));
    let heights: Vec<u32> = (tip.saturating_sub(9)..=tip).collect();
    let hashes = s
        .check(
            "get_block_hashes (batch)",
            client.get_block_hashes(&heights),
        )
        .unwrap_or_default();
    let refs: Vec<&str> = hashes.iter().map(String::as_str).collect();
    s.check("get_block_headers (batch)", client.get_block_headers(&refs));
    s.check("get_block_hex", client.get_block_hex(&best));
    let block = s.check("get_block", client.get_block(&best));
    let coinbase = block
        .as_ref()
        .and_then(|b| b.tx.first().cloned())
        .unwrap_or_default();
    s.check("get_block_with_txs", client.get_block_with_txs(&best));
    s.check("get_block_header", client.get_block_header(&best));
    s.check("get_block_header_hex", client.get_block_header_hex(&best));
    s.check("get_chain_tips", client.get_chain_tips());
    s.check("get_difficulty", client.get_difficulty());
    s.check("get_deployment_info", client.get_deployment_info(None));
    s.check("get_tx_out", client.get_tx_out(&coinbase, 0, None));
    s.check(
        "wait_for_new_block (1 ms)",
        client.wait_for_new_block(Some(1), None),
    );
    s.check(
        "wait_for_block_height (1 ms)",
        client.wait_for_block_height(tip, Some(1)),
    );

    s.check("get_mempool_info", client.get_mempool_info());
    let mempool = s
        .check("get_raw_mempool", client.get_raw_mempool())
        .unwrap_or_default();
    s.check("get_raw_mempool_verbose", client.get_raw_mempool_verbose());
    s.check(
        "get_raw_mempool_with_sequence",
        client.get_raw_mempool_with_sequence(),
    );
    if let Some(txid) = mempool.first() {
        s.check("get_mempool_entry", client.get_mempool_entry(txid));
        let hex = s
            .check(
                "get_raw_transaction_hex",
                client.get_raw_transaction_hex(txid, None),
            )
            .unwrap_or_default();
        s.check(
            "get_raw_transaction",
            client.get_raw_transaction(txid, None),
        );
        s.check(
            "decode_raw_transaction",
            client.decode_raw_transaction(&hex, None),
        );
    }

    s.check("get_network_info", client.get_network_info());
    s.check("get_peer_info", client.get_peer_info());
    s.check("get_connection_count", client.get_connection_count());
    s.check("get_net_totals", client.get_net_totals());
    s.check("get_mining_info", client.get_mining_info());
    s.check(
        "get_network_hash_ps",
        client.get_network_hash_ps(None, None),
    );

    let inputs = [CreateRawTransactionInput {
        txid: coinbase.clone(),
        vout: 0,
        sequence: None,
    }];
    let outputs = [CreateRawTransactionOutput::Address {
        address: ADDRESS.into(),
        amount: Amount::from_sat(1_000),
    }];
    let unsigned = s
        .check(
            "create_raw_transaction (pure)",
            client.create_raw_transaction(&inputs, &outputs, None, None, None),
        )
        .unwrap_or_default();
    s.check(
        "test_mempool_accept (unsigned, not broadcast)",
        client.test_mempool_accept(std::slice::from_ref(&unsigned), None),
    );
    // `converttopsbt` refuses a signed transaction unless told to drop the
    // signatures, so it gets the unsigned one, not a mempool transaction.
    s.check(
        "convert_to_psbt",
        client.convert_to_psbt(&unsigned, None, None),
    );
    let psbt = s
        .check(
            "create_psbt (pure)",
            client.create_psbt(&inputs, &outputs, None, None, None),
        )
        .unwrap_or_default();
    s.check("decode_psbt", client.decode_psbt(&psbt));
    s.check("analyze_psbt", client.analyze_psbt(&psbt));
    s.check("utxo_update_psbt", client.utxo_update_psbt(&psbt, None));
    s.check(
        "combine_psbt",
        client.combine_psbt(&[psbt.clone(), psbt.clone()]),
    );
    s.check(
        "finalize_psbt (unsigned)",
        client.finalize_psbt(&psbt, None),
    );

    s.check("estimate_smart_fee", client.estimate_smart_fee(6, None));
    s.check("uptime", client.uptime());
    s.check("help", client.help(Some("getblockcount")));
    s.check("get_rpc_info", client.get_rpc_info());
    s.check("validate_address", client.validate_address(ADDRESS));
    s.check("get_index_info", client.get_index_info(None));
    s.check(
        "derive_addresses",
        client.derive_addresses(DESCRIPTOR, Some(DescriptorRange::End(2))),
    );

    // Transport behaviour that only a real node can confirm.
    let mixed = s.check(
        "call_batch_raw (one bad call)",
        client.call_batch_raw(&[
            ("getblockcount", serde_json::json!([])),
            ("getblockhash", serde_json::json!([u32::MAX])),
        ]),
    );
    if let Some(results) = mixed {
        assert!(results[0].is_ok(), "good call in a batch must succeed");
        assert!(
            matches!(results[1], Err(bitcoin_rpc::Error::Rpc(_))),
            "bad call in a batch must carry its own RPC error"
        );
    }
    let capped = ClientBuilder::new(&node.url)
        .auth(node.auth.clone())
        .max_response_size(1024)
        .build()
        .unwrap();
    assert!(
        matches!(
            capped.get_block_hex(&best),
            Err(bitcoin_rpc::Error::ResponseTooLarge { limit: 1024 })
        ),
        "a full block must trip a 1 KiB cap"
    );
    println!("  ok   size cap");

    s.finish();
}

#[cfg(feature = "aio")]
#[tokio::test]
#[ignore = "needs a live node; see the module docs"]
async fn aio_client_read_only_survey() {
    use bitcoin_rpc::aio::*;

    let node = node();
    let client = ClientBuilder::new(&node.url)
        .auth(node.auth.clone())
        .read_timeout(Duration::from_secs(120))
        .build()
        .unwrap();
    let mut s = Survey::new();
    println!("async client against {}", node.url);

    let tip = s
        .check("get_block_count", client.get_block_count().await)
        .unwrap_or(0) as u32;
    let best = s
        .check("get_best_block_hash", client.get_best_block_hash().await)
        .unwrap_or_default();
    s.check("get_blockchain_info", client.get_blockchain_info().await);
    let heights: Vec<u32> = (tip.saturating_sub(9)..=tip).collect();
    let hashes = s
        .check(
            "get_block_hashes (batch)",
            client.get_block_hashes(&heights).await,
        )
        .unwrap_or_default();
    let refs: Vec<&str> = hashes.iter().map(String::as_str).collect();
    s.check(
        "get_block_headers (batch)",
        client.get_block_headers(&refs).await,
    );
    s.check("get_block", client.get_block(&best).await);
    s.check("get_block_with_txs", client.get_block_with_txs(&best).await);
    s.check("get_block_header", client.get_block_header(&best).await);
    s.check("get_chain_tips", client.get_chain_tips().await);
    s.check(
        "get_deployment_info",
        client.get_deployment_info(None).await,
    );
    s.check("get_mempool_info", client.get_mempool_info().await);
    let mempool = s
        .check("get_raw_mempool", client.get_raw_mempool().await)
        .unwrap_or_default();
    s.check(
        "get_raw_mempool_verbose",
        client.get_raw_mempool_verbose().await,
    );
    if let Some(txid) = mempool.first() {
        s.check("get_mempool_entry", client.get_mempool_entry(txid).await);
        s.check(
            "get_raw_transaction",
            client.get_raw_transaction(txid, None).await,
        );
    }
    s.check("get_network_info", client.get_network_info().await);
    s.check("get_peer_info", client.get_peer_info().await);
    s.check("get_net_totals", client.get_net_totals().await);
    s.check("get_mining_info", client.get_mining_info().await);
    s.check(
        "estimate_smart_fee",
        client.estimate_smart_fee(6, None).await,
    );
    s.check("uptime", client.uptime().await);
    s.check("get_rpc_info", client.get_rpc_info().await);
    s.check("get_index_info", client.get_index_info(None).await);
    s.check(
        "wait_for_new_block (1 ms)",
        client.wait_for_new_block(Some(1), None).await,
    );
    s.check(
        "derive_addresses",
        client
            .derive_addresses(DESCRIPTOR, Some(DescriptorRange::End(2)))
            .await,
    );

    // A spawned task must be able to drive a batch: the futures are `Send`.
    let shared = std::sync::Arc::new(
        ClientBuilder::new(&node.url)
            .auth(node.auth.clone())
            .build()
            .unwrap(),
    );
    let spawned = tokio::spawn(async move { shared.get_block_hashes(&[0, 1]).await })
        .await
        .unwrap();
    s.check("spawned batch", spawned);

    s.finish();
}
