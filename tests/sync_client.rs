#![cfg(feature = "sync")]

mod common;

use bitcoin_rpc::sync::{
    BlockchainRpc, ClientBuilder, ControlRpc, FeeRpc, MempoolRpc, MiningRpc, NetworkRpc,
    RawTransactionsRpc, RpcCall, RpcCallExt, UtilRpc,
};
use bitcoin_rpc::types::{
    Amount, BlockTemplateRequest, CreateRawTransactionInput, CreateRawTransactionOutput,
    DerivedAddresses, DescriptorRange, DescriptorRequest, FeeRate, SighashType,
};
use bitcoin_rpc::{Auth, Error};
use common::fixtures::{
    BLOCK_TEMPLATE_REPLY, BLOCK_WITH_TXS_REPLY, MEMPOOL_ENTRY_REPLY, PEER_INFO_REPLY,
    RAW_TRANSACTION_REPLY, TEST_MEMPOOL_ACCEPT_REPLY,
};
use serde_json::json;

#[test]
fn successful_call_returns_result_and_sends_jsonrpc_2_0() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":{"blocks":42}}"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let v = client.call_raw("getblockchaininfo", json!([])).unwrap();
    assert_eq!(v, json!({"blocks": 42}));

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["jsonrpc"], "2.0");
    assert_eq!(sent["method"], "getblockchaininfo");
    assert_eq!(sent["params"], json!([]));
    assert!(sent["id"].is_number());
}

#[test]
fn typed_call_deserializes() {
    #[derive(serde::Deserialize)]
    struct Info {
        blocks: u64,
    }

    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":{"blocks":42}}"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let info: Info = client.call("getblockchaininfo", json!([])).unwrap();
    assert_eq!(info.blocks, 42);
}

#[test]
fn rpc_error_becomes_error_rpc() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"error":{"code":-8,"message":"Block not found"}}"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    match client.call_raw("getblockhash", json!([999999999])) {
        Err(Error::Rpc(e)) => {
            assert_eq!(e.code, -8);
            assert_eq!(e.message, "Block not found");
        }
        other => panic!("expected Rpc error, got {other:?}"),
    }
}

#[test]
fn basic_auth_header_is_sent() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":true}"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url())
        .auth(Auth::user_pass("user", "pass"))
        .build()
        .unwrap();

    client.call_raw("uptime", json!([])).unwrap();
    assert_eq!(
        server.requests()[0].authorization.as_deref(),
        Some("Basic dXNlcjpwYXNz")
    );
}

#[test]
fn unreachable_node_is_a_transport_error() {
    // Port 1 on loopback refuses connections.
    let client = ClientBuilder::new("http://127.0.0.1:1").build().unwrap();
    assert!(matches!(
        client.call_raw("uptime", json!([])),
        Err(Error::Transport(_))
    ));
}

#[test]
fn bad_url_fails_at_build_time() {
    assert!(matches!(
        ClientBuilder::new("127.0.0.1:8332").build(),
        Err(Error::Config(_))
    ));
}

#[test]
fn timeout_none_builds_a_client_that_still_makes_requests() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":true}"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url())
        .timeout(None)
        .build()
        .unwrap();

    let v = client.call_raw("uptime", json!([])).unwrap();
    assert_eq!(v, json!(true));
}

#[test]
fn http_error_status_with_error_body_is_reported_as_rpc_error() {
    // A 1.0-style node or a proxy may answer 500; the body still carries the error.
    let server = common::MockServer::spawn(vec![(
        500,
        r#"{"result":null,"error":{"code":-32601,"message":"Method not found"},"id":1}"#
            .to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    match client.call_raw("nosuchmethod", json!([])) {
        Err(Error::Rpc(e)) => assert_eq!(e.code, -32601),
        other => panic!("expected Rpc error, got {other:?}"),
    }
}

#[test]
fn default_auth_sends_no_authorization_header() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":true}"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    client.call_raw("uptime", json!([])).unwrap();
    assert_eq!(server.requests()[0].authorization, None);
}

#[test]
fn http_401_with_empty_body_is_a_transport_error_mentioning_credentials() {
    // Bitcoin Core answers a failed auth check with HTTP 401 and an empty
    // body (src/httprpc.cpp), so this is the regression guard for the
    // most common misconfiguration surfacing as an unhelpful JSON error.
    let server = common::MockServer::spawn(vec![(401, String::new())]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    match client.call_raw("uptime", json!([])) {
        Err(Error::Transport(m)) => {
            assert!(m.contains("401"), "message was: {m}");
            assert!(m.to_lowercase().contains("credentials"), "message was: {m}");
        }
        other => panic!("expected Transport error, got {other:?}"),
    }
}

#[test]
fn reply_with_both_result_and_error_reports_the_error() {
    // Error-wins precedence is documented in `Response::into_result`
    // (src/jsonrpc.rs) but nothing pinned it before this test.
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":42,"error":{"code":-8,"message":"Block not found"}}"#
            .to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    match client.call_raw("getblockhash", json!([999999999])) {
        Err(Error::Rpc(e)) => assert_eq!(e.code, -8),
        other => panic!("expected Rpc error, got {other:?}"),
    }
}

#[test]
fn malformed_response_body_is_a_json_error() {
    let server = common::MockServer::spawn(vec![(200, "not json at all".to_string())]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    assert!(matches!(
        client.call_raw("uptime", json!([])),
        Err(Error::Json(_))
    ));
}

#[test]
fn get_block_with_txs_sends_verbosity_2_and_deserializes() {
    let server = common::MockServer::spawn(vec![(200, BLOCK_WITH_TXS_REPLY.to_string())]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let block = client
        .get_block_with_txs("00000000c937983704a73af28acdec37b049d214adbda81d7e2a3dd146f6ed09")
        .unwrap();
    assert_eq!(block.height, 100);
    assert_eq!(block.stripped_size, 285);
    assert_eq!(block.coinbase_tx.as_ref().unwrap().sequence, 4294967295);
    assert_eq!(
        block.coinbase_tx.as_ref().unwrap().witness.as_deref(),
        Some("00")
    );
    assert_eq!(block.tx.len(), 1);
    assert_eq!(block.tx[0].tx.fee, Some(Amount::from_sat(12_345)));
    // Reached through the `serde(flatten)`-ed transaction body.
    assert_eq!(block.tx[0].tx.vsize, 204);
    assert_eq!(block.tx[0].tx.vout[0].script_pub_key.script_type, "pubkey");
    assert!(block.previous_block_hash.is_some());
    assert_eq!(block.next_block_hash, None);

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["method"], "getblock");
    assert_eq!(
        sent["params"],
        json!([
            "00000000c937983704a73af28acdec37b049d214adbda81d7e2a3dd146f6ed09",
            2
        ])
    );
}

#[test]
fn get_tx_out_miss_returns_none_and_omits_default_include_mempool() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":null}"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    assert_eq!(client.get_tx_out("abc123", 0, None).unwrap(), None);

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["method"], "gettxout");
    assert_eq!(sent["params"], json!(["abc123", 0]));
}

#[test]
fn get_deployment_info_omits_absent_block_hash() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":{"hash":"0f91","height":0,"script_flags":["P2SH"],"deployments":{"segwit":{"type":"buried","height":0,"active":true}}}}"#
            .to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let info = client.get_deployment_info(None).unwrap();
    assert_eq!(info.script_flags.clone().unwrap(), vec!["P2SH"]);
    assert_eq!(info.deployments["segwit"].deployment_type, "buried");
    assert_eq!(info.deployments["segwit"].bip9, None);

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["params"], json!([]));
}

#[test]
fn wait_for_new_block_keeps_interior_null_timeout() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":{"hash":"0f91","height":7}}"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let tip = client.wait_for_new_block(None, Some("dead")).unwrap();
    assert_eq!(tip.height, 7);

    // An omitted `timeout` before a present `current_tip` must stay an explicit
    // null so the node still sees `current_tip` in position 1.
    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["method"], "waitfornewblock");
    assert_eq!(sent["params"], json!([null, "dead"]));
}

#[test]
fn get_mempool_entry_sends_txid_and_deserializes() {
    let server = common::MockServer::spawn(vec![(200, MEMPOOL_ENTRY_REPLY.to_string())]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let entry = client
        .get_mempool_entry("2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866")
        .unwrap();
    assert_eq!(entry.vsize, 204);
    assert_eq!(entry.height, 800000);
    assert_eq!(entry.fees.base, Amount::from_sat(12_345));
    assert_eq!(entry.depends.len(), 1);
    assert!(entry.spent_by.is_empty());
    assert!(!entry.unbroadcast);

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["method"], "getmempoolentry");
    assert_eq!(
        sent["params"],
        json!(["2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866"])
    );
}

#[test]
fn get_raw_mempool_with_sequence_sends_verbose_false_and_sequence_true() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":{"txids":["abc123"],"mempool_sequence":42}}"#
            .to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let seq = client.get_raw_mempool_with_sequence().unwrap();
    assert_eq!(seq.txids, vec!["abc123"]);
    assert_eq!(seq.mempool_sequence, 42);

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["method"], "getrawmempool");
    assert_eq!(sent["params"], json!([false, true]));
}

#[test]
fn get_peer_info_sends_no_params_and_deserializes() {
    let server = common::MockServer::spawn(vec![(200, PEER_INFO_REPLY.to_string())]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let peers = client.get_peer_info().unwrap();
    assert_eq!(peers.len(), 1);
    let peer = &peers[0];
    assert_eq!(peer.id, 7);
    assert_eq!(peer.addr_bind.as_deref(), Some("10.0.0.1:8333"));
    // A peer's clock can lag ours, so timeoffset is signed.
    assert_eq!(peer.time_offset, -2);
    assert_eq!(peer.presynced_headers, -1);
    assert_eq!(peer.bytes_sent_per_msg.get("ping"), Some(&32));
    assert_eq!(peer.bytes_recv_per_msg.get("pong"), Some(&32));
    assert_eq!(peer.connection_type, "outbound-full-relay");

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["method"], "getpeerinfo");
    assert_eq!(sent["params"], json!([]));
}

#[test]
fn disconnect_node_by_id_only_sends_leading_null_address() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":null}"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    client.disconnect_node(None, Some(7)).unwrap();

    // `address` is omitted but not trailing (nodeid follows), so it must stay
    // an explicit null rather than being dropped.
    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["method"], "disconnectnode");
    assert_eq!(sent["params"], json!([null, 7]));
}

#[test]
fn add_node_omits_absent_v2transport() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":null}"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    client.add_node("192.168.0.6:8333", "onetry", None).unwrap();

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["method"], "addnode");
    assert_eq!(sent["params"], json!(["192.168.0.6:8333", "onetry"]));
}

#[test]
fn get_block_template_sends_request_object_and_deserializes() {
    let server = common::MockServer::spawn(vec![(200, BLOCK_TEMPLATE_REPLY.to_string())]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let request = BlockTemplateRequest::default();
    let template = client.get_block_template(&request).unwrap();
    assert_eq!(template.height, 800001);
    assert_eq!(template.coinbase_value, 625000000);
    assert_eq!(template.transactions.len(), 1);
    assert_eq!(template.transactions[0].fee, 1000);
    assert_eq!(template.vb_available.get("!testdummy"), Some(&28));
    assert_eq!(template.weight_limit, None);

    // Sent as a single object argument, not a bare array, and the default
    // request carries the segwit rule Core requires.
    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["method"], "getblocktemplate");
    assert_eq!(sent["params"], json!([{"rules": ["segwit"]}]));
}

#[test]
fn submit_block_returns_none_on_success() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":null}"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    assert_eq!(client.submit_block("aabbcc").unwrap(), None);

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["method"], "submitblock");
    assert_eq!(sent["params"], json!(["aabbcc"]));
}

#[test]
fn submit_block_returns_rejection_reason() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":"duplicate"}"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    assert_eq!(
        client.submit_block("aabbcc").unwrap(),
        Some("duplicate".to_string())
    );
}

#[test]
fn get_network_hash_ps_sends_both_args() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":1234.5}"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let hps = client.get_network_hash_ps(Some(120), Some(-1)).unwrap();
    assert_eq!(hps, 1234.5);

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["method"], "getnetworkhashps");
    assert_eq!(sent["params"], json!([120, -1]));
}

#[test]
fn get_raw_transaction_sends_verbosity_1_and_deserializes() {
    let server = common::MockServer::spawn(vec![(200, RAW_TRANSACTION_REPLY.to_string())]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let tx = client
        .get_raw_transaction(
            "9e1a2d3f4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f6",
            None,
        )
        .unwrap();
    assert_eq!(tx.vsize, 144);
    assert_eq!(tx.confirmations, Some(12));
    assert_eq!(tx.in_active_chain, Some(true));
    assert_eq!(tx.vin[0].vout, Some(1));
    assert_eq!(tx.vin[0].script_sig.as_ref().unwrap().hex, "483045022100");
    assert_eq!(tx.vin[0].tx_in_witness.as_ref().unwrap().len(), 2);
    assert_eq!(tx.vout[0].value, Amount::from_sat(4_998_000));
    assert_eq!(tx.vout[0].script_pub_key.script_type, "witness_v0_keyhash");

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["method"], "getrawtransaction");
    // The absent `block_hash` is trimmed, but verbosity is still explicit.
    assert_eq!(
        sent["params"],
        json!([
            "9e1a2d3f4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f6",
            1
        ])
    );
}

#[test]
fn create_raw_transaction_sends_both_output_forms() {
    let reply = r#"{"jsonrpc":"2.0","id":1,"result":"0200000001abcdef"}"#.to_string();
    let server = common::MockServer::spawn(vec![(200, reply.clone()), (200, reply)]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let inputs = [CreateRawTransactionInput {
        txid: "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b".to_string(),
        vout: 1,
        sequence: None,
    }];
    let outputs = [
        CreateRawTransactionOutput::Address {
            address: "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4".to_string(),
            amount: Amount::from_sat(1_000_000),
        },
        CreateRawTransactionOutput::Data("00010203".to_string()),
    ];
    let expected_inputs = json!([{"txid": "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b", "vout": 1}]);
    let expected_outputs =
        json!([{"bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4": 0.01}, {"data": "00010203"}]);

    // All five arguments supplied.
    let hex = client
        .create_raw_transaction(&inputs, &outputs, Some(800000), Some(false), Some(3))
        .unwrap();
    assert_eq!(hex, "0200000001abcdef");

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["method"], "createrawtransaction");
    assert_eq!(
        sent["params"],
        json!([expected_inputs, expected_outputs, 800000, false, 3])
    );

    // `version` omitted: trimmed as a trailing null, not sent explicitly.
    client
        .create_raw_transaction(&inputs, &outputs, Some(800000), Some(false), None)
        .unwrap();

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[1].body).unwrap();
    assert_eq!(
        sent["params"],
        json!([expected_inputs, expected_outputs, 800000, false])
    );
}

#[test]
fn test_mempool_accept_deserializes_hyphenated_fee_keys() {
    let server = common::MockServer::spawn(vec![(200, TEST_MEMPOOL_ACCEPT_REPLY.to_string())]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let raw_txs = [
        "0200000001abcdef".to_string(),
        "0200000001fedcba".to_string(),
    ];
    let results = client.test_mempool_accept(&raw_txs, None).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].allowed, Some(true));
    let fees = results[0].fees.as_ref().unwrap();
    assert_eq!(fees.base, Amount::from_sat(1_234));
    assert_eq!(fees.effective_feerate, FeeRate::from_sat_per_kvb(8_567));
    assert_eq!(fees.effective_includes.len(), 1);
    // Validation left unfinished by the first transaction: no `allowed` key.
    assert_eq!(results[1].allowed, None);
    assert_eq!(results[1].fees, None);

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["method"], "testmempoolaccept");
    assert_eq!(
        sent["params"],
        json!([["0200000001abcdef", "0200000001fedcba"]])
    );
}

#[test]
fn validate_address_sends_positional_address_and_deserializes_the_invalid_shape() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":{"isvalid":false,"error":"Invalid Bech32 checksum","error_locations":[9,10]}}"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let addr = client
        .validate_address("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t5")
        .unwrap();
    assert!(!addr.isvalid);
    assert_eq!(addr.address, None);
    assert_eq!(addr.error.as_deref(), Some("Invalid Bech32 checksum"));
    assert_eq!(addr.error_locations, Some(vec![9, 10]));

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["method"], "validateaddress");
    assert_eq!(
        sent["params"],
        json!(["bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t5"])
    );
}

#[test]
fn estimate_smart_fee_sends_both_params_and_deserializes_the_errors_only_shape() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":{"errors":["Insufficient data or no feerate found"],"blocks":1008}}"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let est = client.estimate_smart_fee(6, Some("conservative")).unwrap();
    assert_eq!(est.feerate, None);
    assert_eq!(
        est.errors,
        Some(vec!["Insufficient data or no feerate found".to_string()])
    );
    assert_eq!(est.blocks, 1008);

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["method"], "estimatesmartfee");
    assert_eq!(sent["params"], json!([6, "conservative"]));
}

#[test]
fn get_index_info_deserializes_the_dynamic_map() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":{"txindex":{"synced":true,"best_block_height":800000}}}"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let map = client.get_index_info(None).unwrap();
    let txindex = map.get("txindex").unwrap();
    assert!(txindex.synced);
    assert_eq!(txindex.best_block_height, 800000);

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["method"], "getindexinfo");
    assert_eq!(sent["params"], json!([]));
}

#[test]
fn control_rpcs_send_expected_params_and_deserialize_string_results() {
    let server = common::MockServer::spawn(vec![
        (200, r#"{"jsonrpc":"2.0","id":1,"result":3600}"#.to_string()),
        (
            200,
            r#"{"jsonrpc":"2.0","id":1,"result":"Bitcoin Core stopping"}"#.to_string(),
        ),
        (
            200,
            r#"{"jsonrpc":"2.0","id":1,"result":"== Blockchain ==\ngetblockcount\n"}"#.to_string(),
        ),
    ]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    assert_eq!(client.uptime().unwrap(), 3600);
    assert_eq!(client.stop().unwrap(), "Bitcoin Core stopping");
    let help_text = client.help(Some("getblockcount")).unwrap();
    assert!(help_text.contains("getblockcount"));

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[2].body).unwrap();
    assert_eq!(sent["method"], "help");
    assert_eq!(sent["params"], json!(["getblockcount"]));
}

#[test]
fn all_54_typed_methods_send_the_expected_wire_form() {
    // A method's whole behaviour can be a hardcoded literal argument (the
    // verbosity/verbose constants below); the 53 fixture tests only check
    // deserialization, so a swapped literal would ship silently without a
    // test like this one that pins every method's outgoing (method, params).
    // The result of every call is discarded: only the wire form matters here.
    let reply = || (200, r#"{"jsonrpc":"2.0","id":1,"result":null}"#.to_string());
    let server = common::MockServer::spawn(std::iter::repeat_with(reply).take(54).collect());
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let inputs = [CreateRawTransactionInput {
        txid: "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b".to_string(),
        vout: 1,
        sequence: None,
    }];
    let outputs = [CreateRawTransactionOutput::Address {
        address: "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4".to_string(),
        amount: Amount::from_sat(1_000_000),
    }];
    let raw_txs = ["0200000001abcdef".to_string()];
    let request = BlockTemplateRequest::default();
    let psbts = ["cHNidP8BAFIC".to_string(), "cHNidP8BAFID".to_string()];
    let descriptors = [
        DescriptorRequest::Plain("wpkh(02aa)#checksum".to_string()),
        DescriptorRequest::Ranged {
            desc: "wpkh(xpub/0/*)#checksum".to_string(),
            range: DescriptorRange::Span { begin: 0, end: 100 },
        },
    ];

    let _ = client.get_blockchain_info();
    let _ = client.get_best_block_hash();
    let _ = client.get_block_count();
    let _ = client.get_block_hash(800000);
    let _ = client.get_block_hex("h1");
    let _ = client.get_block("h1");
    let _ = client.get_block_with_txs("h1");
    let _ = client.get_block_header("h1");
    let _ = client.get_block_header_hex("h1");
    let _ = client.get_chain_tips();
    let _ = client.get_difficulty();
    let _ = client.get_deployment_info(Some("h1"));
    let _ = client.get_tx_out("txid1", 0, Some(true));
    let _ = client.wait_for_new_block(Some(5000), Some("tip1"));
    let _ = client.wait_for_block_height(800000, Some(5000));
    let _ = client.get_mempool_info();
    let _ = client.get_raw_mempool();
    let _ = client.get_raw_mempool_verbose();
    let _ = client.get_raw_mempool_with_sequence();
    let _ = client.get_mempool_entry("txid1");
    let _ = client.get_network_info();
    let _ = client.get_peer_info();
    let _ = client.get_connection_count();
    let _ = client.get_net_totals();
    let _ = client.add_node("1.2.3.4:8333", "add", Some(true));
    let _ = client.disconnect_node(Some("addr1"), Some(7));
    let _ = client.get_mining_info();
    let _ = client.get_block_template(&request);
    let _ = client.submit_block("aabbcc");
    let _ = client.submit_header("aabbcc");
    let _ = client.get_network_hash_ps(Some(120), Some(-1));
    let _ = client.get_raw_transaction_hex("txid1", Some("bh1"));
    let _ = client.get_raw_transaction("txid1", Some("bh1"));
    let _ = client.send_raw_transaction(
        "hex1",
        Some(FeeRate::from_sat_per_kvb(10_000_000)),
        Some(Amount::from_sat(1_000_000)),
    );
    let _ = client.create_raw_transaction(&inputs, &outputs, Some(800000), Some(true), Some(2));
    let _ = client.decode_raw_transaction("hex1", Some(true));
    let _ = client.test_mempool_accept(&raw_txs, Some(FeeRate::from_sat_per_kvb(50_000_000)));
    let _ = client.estimate_smart_fee(6, Some("conservative"));
    let _ = client.uptime();
    let _ = client.stop();
    let _ = client.help(Some("getblockcount"));
    let _ = client.get_rpc_info();
    let _ = client.validate_address("addr1");
    let _ = client.get_index_info(Some("txindex"));
    let _ = client.create_psbt(&inputs, &outputs, Some(800000), Some(true), Some(2));
    let _ = client.combine_psbt(&psbts);
    let _ = client.join_psbts(&psbts);
    let _ = client.convert_to_psbt("hex1", Some(true), Some(false));
    let _ = client.utxo_update_psbt("cHNidP8BAFIC", Some(&descriptors));
    let _ = client.finalize_psbt("cHNidP8BAFIC", Some(false));
    let _ = client.descriptor_process_psbt(
        "cHNidP8BAFIC",
        &descriptors,
        Some(SighashType::AllAnyoneCanPay),
        Some(true),
        Some(false),
    );
    let _ = client.derive_addresses("wpkh(02aa)#checksum", Some(DescriptorRange::End(5)));
    let _ = client.analyze_psbt("cHNidP8BAFIC");
    let _ = client.decode_psbt("cHNidP8BAFIC");

    let expected: &[(&str, serde_json::Value)] = &[
        ("getblockchaininfo", json!([])),
        ("getbestblockhash", json!([])),
        ("getblockcount", json!([])),
        ("getblockhash", json!([800000])),
        ("getblock", json!(["h1", 0])),
        ("getblock", json!(["h1", 1])),
        ("getblock", json!(["h1", 2])),
        ("getblockheader", json!(["h1", true])),
        ("getblockheader", json!(["h1", false])),
        ("getchaintips", json!([])),
        ("getdifficulty", json!([])),
        ("getdeploymentinfo", json!(["h1"])),
        ("gettxout", json!(["txid1", 0, true])),
        ("waitfornewblock", json!([5000, "tip1"])),
        ("waitforblockheight", json!([800000, 5000])),
        ("getmempoolinfo", json!([])),
        ("getrawmempool", json!([false])),
        ("getrawmempool", json!([true])),
        ("getrawmempool", json!([false, true])),
        ("getmempoolentry", json!(["txid1"])),
        ("getnetworkinfo", json!([])),
        ("getpeerinfo", json!([])),
        ("getconnectioncount", json!([])),
        ("getnettotals", json!([])),
        ("addnode", json!(["1.2.3.4:8333", "add", true])),
        ("disconnectnode", json!(["addr1", 7])),
        ("getmininginfo", json!([])),
        ("getblocktemplate", json!([{"rules": ["segwit"]}])),
        ("submitblock", json!(["aabbcc"])),
        ("submitheader", json!(["aabbcc"])),
        ("getnetworkhashps", json!([120, -1])),
        ("getrawtransaction", json!(["txid1", 0, "bh1"])),
        ("getrawtransaction", json!(["txid1", 1, "bh1"])),
        ("sendrawtransaction", json!(["hex1", 0.1, 0.01])),
        (
            "createrawtransaction",
            json!([
                [{"txid": "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b", "vout": 1}],
                [{"bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4": 0.01}],
                800000,
                true,
                2
            ]),
        ),
        ("decoderawtransaction", json!(["hex1", true])),
        ("testmempoolaccept", json!([["0200000001abcdef"], 0.5])),
        ("estimatesmartfee", json!([6, "conservative"])),
        ("uptime", json!([])),
        ("stop", json!([])),
        ("help", json!(["getblockcount"])),
        ("getrpcinfo", json!([])),
        ("validateaddress", json!(["addr1"])),
        ("getindexinfo", json!(["txindex"])),
        (
            "createpsbt",
            json!([
                [{"txid": "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b", "vout": 1}],
                [{"bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4": 0.01}],
                800000,
                true,
                2
            ]),
        ),
        ("combinepsbt", json!([["cHNidP8BAFIC", "cHNidP8BAFID"]])),
        ("joinpsbts", json!([["cHNidP8BAFIC", "cHNidP8BAFID"]])),
        ("converttopsbt", json!(["hex1", true, false])),
        (
            "utxoupdatepsbt",
            json!([
                "cHNidP8BAFIC",
                [
                    "wpkh(02aa)#checksum",
                    {"desc": "wpkh(xpub/0/*)#checksum", "range": [0, 100]}
                ]
            ]),
        ),
        ("finalizepsbt", json!(["cHNidP8BAFIC", false])),
        (
            "descriptorprocesspsbt",
            json!([
                "cHNidP8BAFIC",
                [
                    "wpkh(02aa)#checksum",
                    {"desc": "wpkh(xpub/0/*)#checksum", "range": [0, 100]}
                ],
                "ALL|ANYONECANPAY",
                true,
                false
            ]),
        ),
        ("deriveaddresses", json!(["wpkh(02aa)#checksum", 5])),
        ("analyzepsbt", json!(["cHNidP8BAFIC"])),
        ("decodepsbt", json!(["cHNidP8BAFIC"])),
    ];

    let requests = server.requests();
    assert_eq!(requests.len(), expected.len(), "expected 54 requests");
    for (i, (method, params)) in expected.iter().enumerate() {
        let sent: serde_json::Value = serde_json::from_str(&requests[i].body).unwrap();
        assert_eq!(sent["method"], *method, "request #{i}");
        assert_eq!(&sent["params"], params, "request #{i} ({method})");
    }
}

#[test]
fn derive_addresses_deserializes_both_result_shapes() {
    // A multipath descriptor (BIP 389) makes Core return one address array per
    // expansion instead of a flat array, so the result type must model both.
    let flat =
        r#"{"jsonrpc":"2.0","id":1,"result":["bcrt1q6mrgxcz4953pk5g7xge8t5vnlwt4m8hypsqppq"]}"#;
    let nested = r#"{"jsonrpc":"2.0","id":1,"result":[["bcrt1qp5wfcq48h6d63wyy9qz0awtpfqwwv4sm4gc9mc","bcrt1qrfxr69jqnhwufxgkqgcdep9prq4j4vuwzpxkrk"],["bcrt1q7zwtzcqsm3k43ha0ac7nl8cz0hqrhckyxxcw45","bcrt1qf7x2v0de6hvgv6tke54pyzmkc9022wh567tygw"]]}"#;
    let server =
        common::MockServer::spawn(vec![(200, flat.to_string()), (200, nested.to_string())]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let single = client
        .derive_addresses("wpkh(02aa)#checksum", None)
        .unwrap();
    assert_eq!(
        single,
        DerivedAddresses::Single(vec![
            "bcrt1q6mrgxcz4953pk5g7xge8t5vnlwt4m8hypsqppq".to_string()
        ])
    );

    let multi = client
        .derive_addresses("wpkh(xpub/<0;1>/*)#checksum", Some(DescriptorRange::End(1)))
        .unwrap();
    assert_eq!(
        multi,
        DerivedAddresses::Multipath(vec![
            vec![
                "bcrt1qp5wfcq48h6d63wyy9qz0awtpfqwwv4sm4gc9mc".to_string(),
                "bcrt1qrfxr69jqnhwufxgkqgcdep9prq4j4vuwzpxkrk".to_string(),
            ],
            vec![
                "bcrt1q7zwtzcqsm3k43ha0ac7nl8cz0hqrhckyxxcw45".to_string(),
                "bcrt1qf7x2v0de6hvgv6tke54pyzmkc9022wh567tygw".to_string(),
            ],
        ])
    );

    // A bare `range` end is sent as a number, not a one-element array.
    let sent: serde_json::Value = serde_json::from_str(&server.requests()[1].body).unwrap();
    assert_eq!(sent["params"], json!(["wpkh(xpub/<0;1>/*)#checksum", 1]));
}

#[test]
fn finalize_psbt_returns_hex_when_extracted_and_psbt_otherwise() {
    let extracted =
        r#"{"jsonrpc":"2.0","id":1,"result":{"hex":"0200000000010128","complete":true}}"#;
    let kept = r#"{"jsonrpc":"2.0","id":1,"result":{"psbt":"cHNidP8BAFIC","complete":true}}"#;
    let server =
        common::MockServer::spawn(vec![(200, extracted.to_string()), (200, kept.to_string())]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let out = client.finalize_psbt("cHNidP8BAFIC", None).unwrap();
    assert_eq!(out.hex.as_deref(), Some("0200000000010128"));
    assert_eq!(out.psbt, None);
    assert!(out.complete);

    let out = client.finalize_psbt("cHNidP8BAFIC", Some(false)).unwrap();
    assert_eq!(out.psbt.as_deref(), Some("cHNidP8BAFIC"));
    assert_eq!(out.hex, None);

    // `extract` omitted must not be sent as null: Core's default is true.
    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(sent["params"], json!(["cHNidP8BAFIC"]));
}

#[test]
fn utxo_update_psbt_omits_absent_descriptors_and_sends_both_descriptor_forms() {
    let reply = || {
        (
            200,
            r#"{"jsonrpc":"2.0","id":1,"result":"cHNidP8BAFIC"}"#.to_string(),
        )
    };
    let server = common::MockServer::spawn(vec![reply(), reply()]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let _ = client.utxo_update_psbt("cHNidP8BAFIC", None).unwrap();
    let descriptors = [
        DescriptorRequest::Plain("wpkh(02aa)#checksum".to_string()),
        DescriptorRequest::Ranged {
            desc: "wpkh(xpub/0/*)#checksum".to_string(),
            range: DescriptorRange::Span { begin: 0, end: 100 },
        },
    ];
    let _ = client
        .utxo_update_psbt("cHNidP8BAFIC", Some(&descriptors))
        .unwrap();

    let requests = server.requests();
    let first: serde_json::Value = serde_json::from_str(&requests[0].body).unwrap();
    assert_eq!(first["params"], json!(["cHNidP8BAFIC"]));
    let second: serde_json::Value = serde_json::from_str(&requests[1].body).unwrap();
    assert_eq!(
        second["params"],
        json!([
            "cHNidP8BAFIC",
            [
                "wpkh(02aa)#checksum",
                {"desc": "wpkh(xpub/0/*)#checksum", "range": [0, 100]}
            ]
        ])
    );
}

#[test]
fn descriptor_process_psbt_deserializes_the_incomplete_shape() {
    // A watch-only descriptor cannot sign, so `complete` is false and `hex` is
    // absent -- the shape a caller processing an offline PSBT actually sees.
    let reply = r#"{"jsonrpc":"2.0","id":1,"result":{"psbt":"cHNidP8BAFIC","complete":false}}"#;
    let server = common::MockServer::spawn(vec![(200, reply.to_string())]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let descriptors = [DescriptorRequest::Plain("wpkh(02aa)#checksum".to_string())];
    let out = client
        .descriptor_process_psbt(
            "cHNidP8BAFIC",
            &descriptors,
            Some(SighashType::All),
            Some(true),
            Some(false),
        )
        .unwrap();
    assert_eq!(out.psbt, "cHNidP8BAFIC");
    assert!(!out.complete);
    assert_eq!(out.hex, None);

    let sent: serde_json::Value = serde_json::from_str(&server.requests()[0].body).unwrap();
    assert_eq!(
        sent["params"],
        json!(["cHNidP8BAFIC", ["wpkh(02aa)#checksum"], "ALL", true, false])
    );
}

#[test]
fn reply_with_a_different_id_is_rejected_as_a_transport_error() {
    // The client's first request carries id 1; the "node" answers id 7 with a
    // perfectly well-formed result. That result belongs to some other request
    // and must not be returned as this one's.
    let server = common::MockServer::spawn_verbatim(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":7,"result":true}"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    match client.call_raw("uptime", json!([])) {
        Err(Error::Transport(m)) => {
            assert!(m.contains("reply id 7"), "{m}");
            assert!(m.contains("request id 1"), "{m}");
        }
        other => panic!("expected Transport error, got {other:?}"),
    }
}

#[test]
fn request_ids_increase_across_calls_and_each_reply_is_matched_to_its_own() {
    let server = common::MockServer::spawn(vec![
        (200, r#"{"jsonrpc":"2.0","id":1,"result":1}"#.to_string()),
        (200, r#"{"jsonrpc":"2.0","id":1,"result":2}"#.to_string()),
    ]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    assert_eq!(client.call_raw("uptime", json!([])).unwrap(), json!(1));
    assert_eq!(client.call_raw("uptime", json!([])).unwrap(), json!(2));

    let ids: Vec<u64> = server
        .requests()
        .iter()
        .map(|r| {
            serde_json::from_str::<serde_json::Value>(&r.body).unwrap()["id"]
                .as_u64()
                .unwrap()
        })
        .collect();
    assert_eq!(ids, vec![1, 2]);
}

/// A well-formed reply whose `result` string pads the body to about `bytes`.
fn padded_reply(bytes: usize) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":1,"result":"{}"}}"#,
        "x".repeat(bytes)
    )
}

#[test]
fn reply_over_the_size_cap_is_rejected_without_being_buffered() {
    let server = common::MockServer::spawn(vec![(200, padded_reply(4096))]);
    let client = ClientBuilder::new(server.url())
        .max_response_size(1024)
        .build()
        .unwrap();

    match client.call_raw("help", json!([])) {
        Err(Error::ResponseTooLarge { limit }) => assert_eq!(limit, 1024),
        other => panic!("expected ResponseTooLarge, got {other:?}"),
    }
}

#[test]
fn reply_under_the_size_cap_is_accepted() {
    let server = common::MockServer::spawn(vec![(200, padded_reply(512))]);
    let client = ClientBuilder::new(server.url())
        .max_response_size(1024)
        .build()
        .unwrap();

    assert_eq!(
        client.call_raw("help", json!([])).unwrap(),
        json!("x".repeat(512))
    );
}

#[test]
fn size_cap_none_lets_a_reply_over_ureqs_own_default_through() {
    // `ureq` caps `read_to_vec` at 10 MB on its own; `None` must override
    // that, not fall back to it.
    let server = common::MockServer::spawn(vec![(200, padded_reply(11 * 1024 * 1024))]);
    let client = ClientBuilder::new(server.url())
        .max_response_size(None)
        .build()
        .unwrap();

    let v = client.call_raw("help", json!([])).unwrap();
    assert_eq!(v.as_str().unwrap().len(), 11 * 1024 * 1024);
}

#[test]
fn zero_size_cap_fails_at_build_time() {
    assert!(matches!(
        ClientBuilder::new("http://127.0.0.1:8332")
            .max_response_size(0)
            .build(),
        Err(Error::Config(_))
    ));
}

#[test]
fn read_timeout_aborts_a_call_the_node_never_answers() {
    let server = common::MockServer::spawn_silent();
    let client = ClientBuilder::new(server.url())
        .read_timeout(std::time::Duration::from_millis(200))
        .build()
        .unwrap();

    let started = std::time::Instant::now();
    match client.call_raw("uptime", json!([])) {
        Err(Error::Transport(m)) => assert!(m.to_lowercase().contains("timeout"), "{m}"),
        other => panic!("expected Transport error, got {other:?}"),
    }
    // Well under the 60 s default: the per-call setting took effect.
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
    // The request did reach the server; it was the silence that failed us.
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn all_timeouts_none_still_builds_a_working_client() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":true}"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url())
        .timeout(None)
        .connect_timeout(None)
        .read_timeout(None)
        .build()
        .unwrap();

    assert_eq!(client.call_raw("uptime", json!([])).unwrap(), json!(true));
}

#[test]
fn overall_timeout_still_applies_on_top_of_the_phase_timeouts() {
    let server = common::MockServer::spawn_silent();
    let client = ClientBuilder::new(server.url())
        .read_timeout(None)
        .timeout(std::time::Duration::from_millis(200))
        .build()
        .unwrap();

    let started = std::time::Instant::now();
    assert!(matches!(
        client.call_raw("uptime", json!([])),
        Err(Error::Transport(_))
    ));
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}

#[test]
fn batch_sends_one_request_with_consecutive_ids_and_returns_results_in_order() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"[{"jsonrpc":"2.0","id":1,"result":"h1"},{"jsonrpc":"2.0","id":2,"result":"h2"},{"jsonrpc":"2.0","id":3,"result":"h3"}]"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let results = client
        .call_batch_raw(&[
            ("getblockhash", json!([1])),
            ("getblockhash", json!([2])),
            ("getblockhash", json!([3])),
        ])
        .unwrap();
    let values: Vec<&str> = results
        .iter()
        .map(|r| r.as_ref().unwrap().as_str().unwrap())
        .collect();
    assert_eq!(values, ["h1", "h2", "h3"]);

    let requests = server.requests();
    assert_eq!(requests.len(), 1, "a batch is one HTTP request");
    let sent: serde_json::Value = serde_json::from_str(&requests[0].body).unwrap();
    let sent = sent.as_array().expect("batch body is a JSON array");
    assert_eq!(sent.len(), 3);
    for (i, req) in sent.iter().enumerate() {
        assert_eq!(req["jsonrpc"], "2.0");
        assert_eq!(req["id"], i as u64 + 1);
        assert_eq!(req["method"], "getblockhash");
        assert_eq!(req["params"], json!([i as u64 + 1]));
    }
}

#[test]
fn batch_replies_out_of_order_are_matched_by_id() {
    let server = common::MockServer::spawn_verbatim(vec![(
        200,
        r#"[{"jsonrpc":"2.0","id":2,"result":"second"},{"jsonrpc":"2.0","id":1,"result":"first"}]"#
            .to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let results = client
        .call_batch_raw(&[("uptime", json!([])), ("uptime", json!([]))])
        .unwrap();
    assert_eq!(results[0].as_ref().unwrap(), "first");
    assert_eq!(results[1].as_ref().unwrap(), "second");
}

#[test]
fn batch_keeps_per_call_rpc_errors_in_place() {
    let server = common::MockServer::spawn(vec![(
        200,
        r#"[{"jsonrpc":"2.0","id":1,"result":800000},{"jsonrpc":"2.0","id":2,"error":{"code":-8,"message":"Block height out of range"}}]"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let results = client
        .call_batch_raw(&[
            ("getblockcount", json!([])),
            ("getblockhash", json!([u32::MAX])),
        ])
        .unwrap();
    assert_eq!(results[0].as_ref().unwrap(), 800000);
    match &results[1] {
        Err(Error::Rpc(e)) => assert_eq!(e.code, -8),
        other => panic!("expected Rpc error, got {other:?}"),
    }
}

#[test]
fn batch_with_a_missing_reply_fails_as_a_whole() {
    let server = common::MockServer::spawn_verbatim(vec![(
        200,
        r#"[{"jsonrpc":"2.0","id":1,"result":true}]"#.to_string(),
    )]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    assert!(matches!(
        client.call_batch_raw(&[("uptime", json!([])), ("uptime", json!([]))]),
        Err(Error::Transport(_))
    ));
}

#[test]
fn empty_batch_makes_no_request() {
    let server = common::MockServer::spawn(vec![]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    assert!(client.call_batch_raw(&[]).unwrap().is_empty());
    let none: Vec<u64> = client.call_batch("uptime", vec![]).unwrap();
    assert!(none.is_empty());
    assert!(server.requests().is_empty());
}

#[test]
fn typed_batch_deserializes_every_result_and_fails_on_the_first_rpc_error() {
    let server = common::MockServer::spawn(vec![
        (
            200,
            r#"[{"jsonrpc":"2.0","id":1,"result":1},{"jsonrpc":"2.0","id":2,"result":2}]"#.to_string(),
        ),
        (
            200,
            r#"[{"jsonrpc":"2.0","id":3,"result":1},{"jsonrpc":"2.0","id":4,"error":{"code":-8,"message":"nope"}}]"#.to_string(),
        ),
    ]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    let ok: Vec<u64> = client
        .call_batch("getblockcount", vec![json!([]), json!([])])
        .unwrap();
    assert_eq!(ok, [1, 2]);

    match client.call_batch::<u64>("getblockcount", vec![json!([]), json!([])]) {
        Err(Error::Rpc(e)) => assert_eq!(e.code, -8),
        other => panic!("expected Rpc error, got {other:?}"),
    }
}

#[test]
fn ids_keep_counting_after_a_batch() {
    let server = common::MockServer::spawn(vec![
        (
            200,
            r#"[{"jsonrpc":"2.0","id":1,"result":1},{"jsonrpc":"2.0","id":2,"result":2}]"#
                .to_string(),
        ),
        (200, r#"{"jsonrpc":"2.0","id":1,"result":3}"#.to_string()),
    ]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    client
        .call_batch_raw(&[("uptime", json!([])), ("uptime", json!([]))])
        .unwrap();
    client.call_raw("uptime", json!([])).unwrap();

    let single: serde_json::Value = serde_json::from_str(&server.requests()[1].body).unwrap();
    assert_eq!(single["id"], 3);
}

#[test]
fn get_block_hashes_and_headers_are_batched() {
    let server = common::MockServer::spawn(vec![
        (
            200,
            r#"[{"jsonrpc":"2.0","id":1,"result":"aa"},{"jsonrpc":"2.0","id":2,"result":"bb"}]"#
                .to_string(),
        ),
        (
            200,
            format!(
                "[{},{}]",
                r#"{"jsonrpc":"2.0","id":1,"result":{"hash":"aa","confirmations":2,"height":1,"version":1,"versionHex":"00000001","merkleroot":"m","time":1,"mediantime":1,"nonce":0,"bits":"1d00ffff","target":"t","difficulty":1.0,"chainwork":"c","nTx":1}}"#,
                r#"{"jsonrpc":"2.0","id":2,"result":{"hash":"bb","confirmations":1,"height":2,"version":1,"versionHex":"00000001","merkleroot":"m","time":2,"mediantime":2,"nonce":0,"bits":"1d00ffff","target":"t","difficulty":1.0,"chainwork":"c","nTx":1}}"#
            ),
        ),
    ]);
    let client = ClientBuilder::new(server.url()).build().unwrap();

    assert_eq!(client.get_block_hashes(&[1, 2]).unwrap(), ["aa", "bb"]);
    let headers = client.get_block_headers(&["aa", "bb"]).unwrap();
    assert_eq!(headers.len(), 2);
    assert_eq!(headers[1].height, 2);

    let requests = server.requests();
    assert_eq!(requests.len(), 2);
    let hashes: serde_json::Value = serde_json::from_str(&requests[0].body).unwrap();
    assert_eq!(hashes[1]["method"], "getblockhash");
    assert_eq!(hashes[1]["params"], json!([2]));
    let headers: serde_json::Value = serde_json::from_str(&requests[1].body).unwrap();
    assert_eq!(headers[0]["method"], "getblockheader");
    assert_eq!(headers[0]["params"], json!(["aa", true]));
}
