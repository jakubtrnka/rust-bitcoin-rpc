#![cfg(feature = "serde")]

use bitcoin_rpc::types::*;
use serde_json::json;

#[test]
fn get_blockchain_info_full() {
    let v = json!({
        "chain": "main", "blocks": 800000, "headers": 800000,
        "bestblockhash": "0000000000000000000",
        "bits": "17034219", "target": "000000000000000000034219",
        "difficulty": 53911173001054.59, "time": 1690000000,
        "mediantime": 1689999000, "verificationprogress": 0.9999,
        "initialblockdownload": false, "chainwork": "00000000000000abc",
        "size_on_disk": 570000000000_u64, "pruned": true,
        "pruneheight": 700000, "automatic_pruning": true,
        "prune_target_size": 550000000_u64, "signet_challenge": "51",
        "warnings": ["unknown new rules activated"]
    });
    let info: BlockchainInfo = serde_json::from_value(v).unwrap();
    assert_eq!(info.chain, "main");
    assert_eq!(info.best_block_hash, "0000000000000000000");
    assert_eq!(info.median_time, 1689999000);
    assert_eq!(info.verification_progress, 0.9999);
    assert!(!info.initial_block_download);
    assert_eq!(info.size_on_disk, 570000000000);
    assert_eq!(info.prune_height, Some(700000));
    assert_eq!(info.automatic_pruning, Some(true));
    assert_eq!(info.prune_target_size, Some(550000000));
    assert_eq!(info.signet_challenge.as_deref(), Some("51"));
    assert_eq!(info.warnings, vec!["unknown new rules activated"]);
}

#[test]
fn get_blockchain_info_minimal_and_forward_compatible() {
    let v = json!({
        "chain": "regtest", "blocks": 0, "headers": 0,
        "bestblockhash": "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206",
        "bits": "207fffff", "target": "7fffff0000000000",
        "difficulty": 4.656542373871732e-10, "time": 1296688602,
        "mediantime": 1296688602, "verificationprogress": 1.0,
        "initialblockdownload": true, "chainwork": "02",
        "size_on_disk": 293, "pruned": false, "warnings": [],
        "some_field_from_a_future_release": 1
    });
    let info: BlockchainInfo = serde_json::from_value(v).unwrap();
    assert_eq!(info.chain, "regtest");
    assert_eq!(info.prune_height, None);
    assert_eq!(info.automatic_pruning, None);
    assert_eq!(info.prune_target_size, None);
    assert_eq!(info.signet_challenge, None);
    assert!(info.warnings.is_empty());
}

#[test]
fn get_blockchain_info_warnings_legacy_string_form() {
    // A node run with `-deprecatedrpc=warnings` emits a bare string instead of
    // an array; ruling R25 requires both wire shapes to deserialize.
    let v = json!({
        "chain": "main", "blocks": 800000, "headers": 800000,
        "bestblockhash": "0000000000000000000",
        "bits": "17034219", "target": "000000000000000000034219",
        "difficulty": 53911173001054.59, "time": 1690000000,
        "mediantime": 1689999000, "verificationprogress": 0.9999,
        "initialblockdownload": false, "chainwork": "00000000000000abc",
        "size_on_disk": 570000000000_u64, "pruned": false,
        "warnings": "single legacy warning"
    });
    let info: BlockchainInfo = serde_json::from_value(v).unwrap();
    assert_eq!(info.warnings, vec!["single legacy warning"]);

    // The other half of ruling R25: whichever wire form came in, the public
    // type serializes `warnings` back out as an array.
    assert_eq!(
        serde_json::to_value(&info).unwrap()["warnings"],
        json!(["single legacy warning"])
    );
}

#[test]
fn get_blockchain_info_warnings_legacy_empty_string_is_no_warnings() {
    // `GetWarningsForRpc` returns `""`, not a one-element list, when there is
    // nothing to warn about (`bitcoin/src/node/warnings.cpp:56-58`), so a
    // caller testing `warnings.is_empty()` must not see a phantom warning.
    let v = json!({
        "chain": "main", "blocks": 800000, "headers": 800000,
        "bestblockhash": "0000000000000000000",
        "bits": "17034219", "target": "000000000000000000034219",
        "difficulty": 53911173001054.59, "time": 1690000000,
        "mediantime": 1689999000, "verificationprogress": 0.9999,
        "initialblockdownload": false, "chainwork": "00000000000000abc",
        "size_on_disk": 570000000000_u64, "pruned": false,
        "warnings": ""
    });
    let info: BlockchainInfo = serde_json::from_value(v).unwrap();
    assert!(info.warnings.is_empty(), "got {:?}", info.warnings);
}

#[test]
fn block_header_full() {
    let v = json!({
        "hash": "00000000c937983704a73af28acdec37b049d214adbda81d7e2a3dd146f6ed09",
        "confirmations": 799901, "height": 100, "version": 536870912,
        "versionHex": "20000000",
        "merkleroot": "2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866",
        "time": 1231660825, "mediantime": 1231658656, "nonce": 2573394689_u64,
        "bits": "1d00ffff", "target": "00000000ffff0000000000000000000000000000000000000000000000000000",
        "difficulty": 1.0,
        "chainwork": "0000000000000000000000000000000000000000000000000000006500650065",
        "nTx": 1,
        "previousblockhash": "000000007bc154e0fa7ea32218a72fe2c1bb9f86cf8c9ebf9a715ed27fdb229a",
        "nextblockhash": "00000000fe0b4e0d84a91ad24b23d5b45cd0e21ac4cd0d0d5e13c6a2f4a0e17c"
    });
    let header: BlockHeader = serde_json::from_value(v).unwrap();
    assert_eq!(header.confirmations, 799901);
    assert_eq!(header.height, 100);
    assert_eq!(header.version, 536870912);
    assert_eq!(header.version_hex, "20000000");
    assert_eq!(header.median_time, 1231658656);
    assert_eq!(header.nonce, 2573394689);
    assert_eq!(header.n_tx, 1);
    assert!(header.previous_block_hash.is_some());
    assert!(header.next_block_hash.is_some());
}

#[test]
fn block_header_minimal_and_forward_compatible() {
    let v = json!({
        "hash": "00000000c937983704a73af28acdec37b049d214adbda81d7e2a3dd146f6ed09",
        "confirmations": -1, "height": 100, "version": 536870912,
        "versionHex": "20000000",
        "merkleroot": "2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866",
        "time": 1231660825, "mediantime": 1231658656, "nonce": 0,
        "bits": "1d00ffff", "target": "00000000ffff0000000000000000000000000000000000000000000000000000",
        "difficulty": 1.0, "chainwork": "65", "nTx": 1,
        "some_field_from_a_future_release": true
    });
    let header: BlockHeader = serde_json::from_value(v).unwrap();
    // A block off the main chain reports -1 confirmations, so the field is signed.
    assert_eq!(header.confirmations, -1);
    assert_eq!(header.previous_block_hash, None);
    assert_eq!(header.next_block_hash, None);
}

#[test]
fn block_full() {
    let v = json!({
        "hash": "00000000c937983704a73af28acdec37b049d214adbda81d7e2a3dd146f6ed09",
        "confirmations": 799901, "size": 285, "strippedsize": 285, "weight": 1140,
        "coinbase_tx": {
            "version": 1, "locktime": 0, "sequence": 4294967295_u64,
            "coinbase": "04ffff001d0102", "witness": "0000000000000000000000000000000000000000000000000000000000000000"
        },
        "height": 100, "version": 536870912, "versionHex": "20000000",
        "merkleroot": "2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866",
        "tx": ["2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866"],
        "time": 1231660825, "mediantime": 1231658656, "nonce": 2573394689_u64,
        "bits": "1d00ffff", "target": "00000000ffff0000000000000000000000000000000000000000000000000000",
        "difficulty": 1.0, "chainwork": "6500650065", "nTx": 1,
        "previousblockhash": "000000007bc154e0fa7ea32218a72fe2c1bb9f86cf8c9ebf9a715ed27fdb229a",
        "nextblockhash": "00000000fe0b4e0d84a91ad24b23d5b45cd0e21ac4cd0d0d5e13c6a2f4a0e17c"
    });
    let block: Block = serde_json::from_value(v).unwrap();
    assert_eq!(block.size, 285);
    assert_eq!(block.stripped_size, 285);
    assert_eq!(block.weight, 1140);
    assert_eq!(block.coinbase_tx.as_ref().unwrap().version, 1);
    assert_eq!(block.coinbase_tx.as_ref().unwrap().sequence, 4294967295);
    assert_eq!(
        block.coinbase_tx.as_ref().unwrap().coinbase,
        "04ffff001d0102"
    );
    assert!(block.coinbase_tx.as_ref().unwrap().witness.is_some());
    assert_eq!(
        block.tx,
        vec!["2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866"]
    );
    assert_eq!(block.n_tx, 1);
    assert!(block.next_block_hash.is_some());
}

#[test]
fn block_minimal_and_forward_compatible() {
    let v = json!({
        "hash": "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206",
        "confirmations": 1, "size": 285, "strippedsize": 285, "weight": 1140,
        "coinbase_tx": { "version": 1, "locktime": 0, "sequence": 4294967295_u64, "coinbase": "51" },
        "height": 0, "version": 1, "versionHex": "00000001",
        "merkleroot": "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
        "tx": [], "time": 1296688602, "mediantime": 1296688602, "nonce": 2,
        "bits": "207fffff", "target": "7fffff0000000000",
        "difficulty": 4.656542373871732e-10, "chainwork": "02", "nTx": 1,
        "some_field_from_a_future_release": ["anything"]
    });
    let block: Block = serde_json::from_value(v).unwrap();
    assert_eq!(block.height, 0);
    assert_eq!(block.coinbase_tx.as_ref().unwrap().witness, None);
    assert!(block.tx.is_empty());
    assert_eq!(block.previous_block_hash, None);
    assert_eq!(block.next_block_hash, None);
}

#[test]
fn block_with_txs_full() {
    let v = json!({
        "hash": "00000000c937983704a73af28acdec37b049d214adbda81d7e2a3dd146f6ed09",
        "confirmations": 799901, "size": 285, "strippedsize": 285, "weight": 1140,
        "coinbase_tx": {
            "version": 1, "locktime": 0, "sequence": 4294967295_u64,
            "coinbase": "04ffff001d0102", "witness": "00"
        },
        "height": 100, "version": 536870912, "versionHex": "20000000",
        "merkleroot": "2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866",
        "tx": [
            {
                "txid": "2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866",
                "hash": "2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866",
                "version": 1, "size": 204, "vsize": 204, "weight": 816, "locktime": 0,
                "vin": [{ "coinbase": "04ffff001d0102", "sequence": 4294967295_u64 }],
                "vout": [{
                    "value": 50.0, "n": 0,
                    "scriptPubKey": {
                        "asm": "04678afdb0 OP_CHECKSIG",
                        "desc": "pk(04678afdb0)#checksum",
                        "hex": "4104678afdb0ac",
                        "type": "pubkey"
                    }
                }],
                "hex": "0100000001",
                "fee": 0.00012345
            }
        ],
        "time": 1231660825, "mediantime": 1231658656, "nonce": 2573394689_u64,
        "bits": "1d00ffff", "target": "00000000ffff0000000000000000000000000000000000000000000000000000",
        "difficulty": 1.0, "chainwork": "6500650065", "nTx": 1,
        "previousblockhash": "000000007bc154e0fa7ea32218a72fe2c1bb9f86cf8c9ebf9a715ed27fdb229a",
        "nextblockhash": "00000000fe0b4e0d84a91ad24b23d5b45cd0e21ac4cd0d0d5e13c6a2f4a0e17c"
    });
    let block: BlockWithTxs = serde_json::from_value(v).unwrap();
    assert_eq!(block.height, 100);
    assert_eq!(block.coinbase_tx.as_ref().unwrap().locktime, 0);
    assert_eq!(block.tx.len(), 1);
    // `fee` is a JSON number even though Core documents it as STR_AMOUNT.
    // Reached *through* the flattened `tx`: proves `serde(flatten)` really
    // routes the sibling keys into `Transaction` instead of dropping them.
    assert_eq!(block.tx[0].tx.fee, Some(Amount::from_sat(12_345)));
    assert_eq!(
        block.tx[0].tx.txid,
        "2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866"
    );
    assert_eq!(block.tx[0].tx.vsize, 204);
    assert_eq!(
        block.tx[0].tx.vin[0].coinbase.as_deref(),
        Some("04ffff001d0102")
    );
    assert_eq!(
        block.tx[0].tx.vout[0].value,
        Amount::from_sat(5_000_000_000)
    );
    assert_eq!(block.tx[0].tx.vout[0].script_pub_key.script_type, "pubkey");
    // `getblock` calls `TxToUniv` with a null block hash, so no block context.
    assert_eq!(block.tx[0].tx.block_hash, None);
    assert_eq!(block.tx[0].tx.confirmations, None);
    assert!(block.previous_block_hash.is_some());
}

#[test]
fn block_with_txs_minimal_and_forward_compatible() {
    let v = json!({
        "hash": "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206",
        "confirmations": 1, "size": 285, "strippedsize": 285, "weight": 1140,
        "coinbase_tx": { "version": 1, "locktime": 0, "sequence": 4294967295_u64, "coinbase": "51" },
        "height": 0, "version": 1, "versionHex": "00000001",
        "merkleroot": "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
        "tx": [{
            "txid": "4a5e1e", "hash": "4a5e1e",
            "version": 1, "size": 204, "vsize": 204, "weight": 816, "locktime": 0,
            "vin": [], "vout": []
        }],
        "time": 1296688602, "mediantime": 1296688602, "nonce": 2,
        "bits": "207fffff", "target": "7fffff0000000000",
        "difficulty": 4.656542373871732e-10, "chainwork": "02", "nTx": 1,
        "some_field_from_a_future_release": 1
    });
    let block: BlockWithTxs = serde_json::from_value(v).unwrap();
    assert_eq!(block.tx.len(), 1);
    // Blocks whose undo data is unavailable (e.g. pruned) carry no per-tx fee.
    assert_eq!(block.tx[0].tx.fee, None);
    assert_eq!(block.tx[0].tx.hash, "4a5e1e");
    assert_eq!(block.tx[0].tx.weight, 816);
    assert!(block.tx[0].tx.vin.is_empty());
    assert_eq!(block.next_block_hash, None);
}

#[test]
fn chain_tip_full() {
    let v = json!({
        "height": 800000,
        "hash": "00000000000000000002a7c4c1e48d76c5a37902165a270156b7a8d72728a054",
        "branchlen": 0,
        "status": "active"
    });
    let tip: ChainTip = serde_json::from_value(v).unwrap();
    assert_eq!(tip.height, 800000);
    assert_eq!(tip.branch_len, 0);
    assert_eq!(tip.status, "active");
}

#[test]
fn chain_tip_forward_compatible() {
    let v = json!({
        "height": 799999,
        "hash": "000000000000000000019dbb35b19b8e0a2f4a6be4b78f6b47cee0d0b2f4f9a1",
        "branchlen": 2,
        "status": "valid-fork",
        "some_field_from_a_future_release": "x"
    });
    let tip: ChainTip = serde_json::from_value(v).unwrap();
    assert_eq!(tip.branch_len, 2);
    assert_eq!(tip.status, "valid-fork");
}

#[test]
fn deployment_info_full() {
    let v = json!({
        "hash": "00000000000000000002a7c4c1e48d76c5a37902165a270156b7a8d72728a054",
        "height": 800000,
        "script_flags": ["P2SH", "WITNESS", "TAPROOT"],
        "deployments": {
            "taproot": {
                "type": "bip9",
                "height": 709632,
                "active": true,
                "bip9": {
                    "bit": 2,
                    "start_time": 1619222400,
                    "timeout": 1628640000,
                    "min_activation_height": 709632,
                    "status": "active",
                    "since": 709632,
                    "status_next": "active",
                    "statistics": {
                        "period": 2016,
                        "threshold": 1815,
                        "elapsed": 2016,
                        "count": 1916,
                        "possible": true
                    },
                    "signalling": "#####--#####"
                }
            }
        }
    });
    let info: DeploymentInfo = serde_json::from_value(v).unwrap();
    assert_eq!(info.height, 800000);
    assert_eq!(
        info.script_flags.clone().unwrap(),
        vec!["P2SH", "WITNESS", "TAPROOT"]
    );
    let taproot = &info.deployments["taproot"];
    assert_eq!(taproot.deployment_type, "bip9");
    assert_eq!(taproot.height, Some(709632));
    assert!(taproot.active);
    let bip9 = taproot.bip9.as_ref().unwrap();
    assert_eq!(bip9.bit, Some(2));
    assert_eq!(bip9.start_time, 1619222400);
    assert_eq!(bip9.min_activation_height, 709632);
    assert_eq!(bip9.status_next, "active");
    assert_eq!(bip9.signalling.as_deref(), Some("#####--#####"));
    let stats = bip9.statistics.as_ref().unwrap();
    assert_eq!(stats.period, 2016);
    assert_eq!(stats.threshold, Some(1815));
    assert_eq!(stats.count, 1916);
    assert_eq!(stats.possible, Some(true));
}

#[test]
fn deployment_info_minimal_and_forward_compatible() {
    let v = json!({
        "hash": "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206",
        "height": 0,
        "script_flags": [],
        "deployments": {
            "segwit": { "type": "buried", "active": true },
            "testdummy": {
                "type": "bip9",
                "active": false,
                "bip9": {
                    "start_time": 0,
                    "timeout": 9223372036854775807_i64,
                    "min_activation_height": 0,
                    "status": "defined",
                    "since": 0,
                    "status_next": "defined"
                }
            }
        },
        "some_field_from_a_future_release": {}
    });
    let info: DeploymentInfo = serde_json::from_value(v).unwrap();
    assert!(info.script_flags.as_ref().unwrap().is_empty());
    let segwit = &info.deployments["segwit"];
    assert_eq!(segwit.deployment_type, "buried");
    assert_eq!(segwit.height, None);
    assert_eq!(segwit.bip9, None);
    let bip9 = info.deployments["testdummy"].bip9.as_ref().unwrap();
    assert_eq!(bip9.bit, None);
    assert_eq!(bip9.timeout, i64::MAX);
    assert_eq!(bip9.statistics, None);
    assert_eq!(bip9.signalling, None);
}

#[test]
fn tx_out_full() {
    let v = json!({
        "bestblock": "00000000000000000002a7c4c1e48d76c5a37902165a270156b7a8d72728a054",
        "confirmations": 42,
        "value": 0.00012345,
        "scriptPubKey": {
            "asm": "OP_DUP OP_HASH160 0000 OP_EQUALVERIFY OP_CHECKSIG",
            "desc": "addr(bc1qexample)#checksum",
            "hex": "76a914000088ac",
            "type": "pubkeyhash",
            "address": "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH"
        },
        "coinbase": true
    });
    let out: TxOut = serde_json::from_value(v).unwrap();
    assert_eq!(out.confirmations, 42);
    // STR_AMOUNT is an unquoted JSON number, hence f64.
    assert_eq!(out.value, Amount::from_sat(12_345));
    assert_eq!(out.script_pub_key.script_type, "pubkeyhash");
    assert_eq!(out.script_pub_key.hex, "76a914000088ac");
    assert_eq!(
        out.script_pub_key.address.as_deref(),
        Some("1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH")
    );
    assert!(out.coinbase);
    assert!(out.best_block.starts_with("00000000"));
}

#[test]
fn tx_out_minimal_and_forward_compatible() {
    let v = json!({
        "bestblock": "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206",
        "confirmations": 0,
        "value": 50.0,
        "scriptPubKey": {
            "asm": "OP_RETURN",
            "desc": "raw(6a)#checksum",
            "hex": "6a",
            "type": "nulldata",
            "some_field_from_a_future_release": 1
        },
        "coinbase": false,
        "another_field_from_a_future_release": null
    });
    let out: TxOut = serde_json::from_value(v).unwrap();
    assert_eq!(out.confirmations, 0);
    assert_eq!(out.value, Amount::from_sat(5_000_000_000));
    assert_eq!(out.script_pub_key.address, None);
    assert!(!out.coinbase);
}

#[test]
fn tx_out_miss_deserializes_to_none() {
    // `gettxout` answers JSON null when the output is not in the UTXO set.
    let out: Option<TxOut> = serde_json::from_value(json!(null)).unwrap();
    assert_eq!(out, None);
}

#[test]
fn block_hash_and_height_full() {
    let v = json!({
        "hash": "00000000000000000002a7c4c1e48d76c5a37902165a270156b7a8d72728a054",
        "height": 800000
    });
    let tip: BlockHashAndHeight = serde_json::from_value(v).unwrap();
    assert_eq!(tip.height, 800000);
    assert!(tip.hash.starts_with("000000000000"));
}

#[test]
fn block_hash_and_height_forward_compatible() {
    let v = json!({
        "hash": "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206",
        "height": 0,
        "some_field_from_a_future_release": 1
    });
    let tip: BlockHashAndHeight = serde_json::from_value(v).unwrap();
    assert_eq!(tip.height, 0);
}

#[test]
fn get_mempool_info_full() {
    let v = json!({
        "loaded": true, "size": 120, "bytes": 45000, "usage": 987654,
        "total_fee": 0.01234567, "maxmempool": 300000000_u64,
        "mempoolminfee": 0.00001000, "minrelaytxfee": 0.00001000,
        "incrementalrelayfee": 0.00001000, "unbroadcastcount": 3,
        "permitbaremultisig": true, "maxdatacarriersize": 83,
        "limitclustercount": 500, "limitclustersize": 101000,
        "optimal": true
    });
    let info: MempoolInfo = serde_json::from_value(v).unwrap();
    assert!(info.loaded);
    assert_eq!(info.size, 120);
    assert_eq!(info.bytes, 45000);
    assert_eq!(info.usage, 987654);
    // STR_AMOUNT is an unquoted JSON number, hence f64.
    assert_eq!(info.total_fee, Amount::from_sat(1_234_567));
    assert_eq!(info.max_mempool, 300000000);
    assert_eq!(info.mempool_min_fee, FeeRate::from_sat_per_kvb(1_000));
    assert_eq!(info.min_relay_tx_fee, FeeRate::from_sat_per_kvb(1_000));
    assert_eq!(info.incremental_relay_fee, FeeRate::from_sat_per_kvb(1_000));
    assert_eq!(info.unbroadcast_count, 3);
    assert_eq!(info.permit_bare_multisig, Some(true));
    assert_eq!(info.max_datacarrier_size, Some(83));
    assert_eq!(info.limit_cluster_count, Some(500));
    assert_eq!(info.limit_cluster_size, Some(101000));
    assert_eq!(info.optimal, Some(true));
}

#[test]
fn get_mempool_info_forward_compatible() {
    // Every field is required in `getmempoolinfo`'s RPCResult, so this test's
    // forward-compatibility burden falls entirely on the unknown field, plus
    // proving the deprecated `fullrbf` field (present on a real node) is
    // tolerated even though `MempoolInfo` has no field for it.
    let v = json!({
        "loaded": false, "size": 0, "bytes": 0, "usage": 0,
        "total_fee": 0.0, "maxmempool": 300000000_u64,
        "mempoolminfee": 0.00001000, "minrelaytxfee": 0.00001000,
        "incrementalrelayfee": 0.00001000, "unbroadcastcount": 0,
        "permitbaremultisig": false, "maxdatacarriersize": 0,
        "limitclustercount": 0, "limitclustersize": 0,
        "optimal": false,
        "fullrbf": true,
        "some_field_from_a_future_release": 1
    });
    let info: MempoolInfo = serde_json::from_value(v).unwrap();
    assert!(!info.loaded);
    assert_eq!(info.size, 0);
    assert_eq!(info.optimal, Some(false));
}

#[test]
fn mempool_entry_full() {
    let v = json!({
        "vsize": 204, "weight": 816, "time": 1690000000, "height": 800000,
        "descendantcount": 2, "descendantsize": 408,
        "ancestorcount": 1, "ancestorsize": 204,
        "chunkweight": 816,
        "wtxid": "2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866",
        "fees": {
            "base": 0.00012345, "modified": 0.00012400,
            "ancestor": 0.00012345, "descendant": 0.00024690,
            "chunk": 0.00012345
        },
        "depends": ["4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b"],
        "spentby": ["9b1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b1"],
        "bip125-replaceable": true,
        "unbroadcast": false
    });
    let entry: MempoolEntry = serde_json::from_value(v).unwrap();
    assert_eq!(entry.vsize, 204);
    assert_eq!(entry.weight, 816);
    assert_eq!(entry.time, 1690000000);
    assert_eq!(entry.height, 800000);
    assert_eq!(entry.descendant_count, 2);
    assert_eq!(entry.descendant_size, 408);
    assert_eq!(entry.ancestor_count, 1);
    assert_eq!(entry.ancestor_size, 204);
    assert_eq!(entry.chunk_weight, Some(816));
    assert_eq!(
        entry.wtxid,
        "2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866"
    );
    // BTC wire numbers are held as exact satoshi amounts.
    assert_eq!(entry.fees.base, Amount::from_sat(12_345));
    assert_eq!(entry.fees.modified, SignedAmount::from_sat(12_400));
    assert_eq!(entry.fees.ancestor, SignedAmount::from_sat(12_345));
    assert_eq!(entry.fees.descendant, SignedAmount::from_sat(24_690));
    assert_eq!(entry.fees.chunk, Some(SignedAmount::from_sat(12_345)));
    assert_eq!(entry.depends.len(), 1);
    assert_eq!(entry.spent_by.len(), 1);
    assert!(!entry.unbroadcast);
}

#[test]
fn mempool_entry_negative_priority_delta() {
    // An isolated transaction paying 12,345 sat with a -20,000 sat delta.
    let v = json!({
        "vsize": 204, "weight": 816, "time": 1690000000, "height": 800000,
        "descendantcount": 1, "descendantsize": 204,
        "ancestorcount": 1, "ancestorsize": 204,
        "chunkweight": 816, "wtxid": "txid",
        "fees": {
            "base": 0.00012345, "modified": -0.00007655,
            "ancestor": -0.00007655, "descendant": -0.00007655,
            "chunk": -0.00007655
        },
        "depends": [], "spentby": [], "unbroadcast": false
    });
    for has_chunk in [true, false] {
        let mut wire = v.clone();
        if !has_chunk {
            wire.as_object_mut().unwrap().remove("chunkweight");
            wire["fees"].as_object_mut().unwrap().remove("chunk");
        }
        let entry: MempoolEntry = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(entry.fees.base.to_sat(), 12_345);
        assert_eq!(entry.fees.modified.to_sat(), -7_655);
        assert_eq!(entry.fees.ancestor.to_sat(), -7_655);
        assert_eq!(entry.fees.descendant.to_sat(), -7_655);
        assert_eq!(
            entry.fees.chunk.map(SignedAmount::to_sat),
            has_chunk.then_some(-7_655)
        );
        let entries: std::collections::HashMap<String, MempoolEntry> =
            serde_json::from_value(json!({"txid": wire})).unwrap();
        assert_eq!(entries["txid"], entry);
        let encoded = serde_json::to_value(&entry).unwrap();
        assert_eq!(encoded["fees"]["modified"], json!(-0.00007655));
        assert_eq!(
            serde_json::from_value::<MempoolEntry>(encoded).unwrap(),
            entry
        );
    }
    let mut invalid = v;
    invalid["fees"]["base"] = json!(-0.00000001);
    assert!(serde_json::from_value::<MempoolEntry>(invalid).is_err());
}

#[test]
fn mempool_entry_minimal_and_forward_compatible() {
    // Every field is required in `MempoolEntryDescription`, so this test's
    // forward-compatibility burden falls on the unknown field, plus proving
    // the deprecated `bip125-replaceable` field is simply absent here and
    // that omitting it from `MempoolEntry` does not break deserialization.
    let v = json!({
        "vsize": 110, "weight": 440, "time": 1600000000, "height": 700000,
        "descendantcount": 1, "descendantsize": 110,
        "ancestorcount": 1, "ancestorsize": 110,
        "chunkweight": 440,
        "wtxid": "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
        "fees": {
            "base": 0.0, "modified": 0.0, "ancestor": 0.0,
            "descendant": 0.0, "chunk": 0.0
        },
        "depends": [], "spentby": [],
        "unbroadcast": true,
        "some_field_from_a_future_release": "x"
    });
    let entry: MempoolEntry = serde_json::from_value(v).unwrap();
    assert_eq!(entry.vsize, 110);
    assert!(entry.depends.is_empty());
    assert!(entry.spent_by.is_empty());
    assert!(entry.unbroadcast);
    assert_eq!(entry.fees.base, Amount::ZERO);
}

#[test]
fn get_raw_mempool_sequence_full() {
    let v = json!({
        "txids": ["2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866"],
        "mempool_sequence": 12345_u64
    });
    let seq: RawMempoolSequence = serde_json::from_value(v).unwrap();
    assert_eq!(seq.txids.len(), 1);
    assert_eq!(
        seq.txids[0],
        "2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866"
    );
    assert_eq!(seq.mempool_sequence, 12345);
}

#[test]
fn get_raw_mempool_sequence_forward_compatible() {
    let v = json!({
        "txids": [],
        "mempool_sequence": 0,
        "some_field_from_a_future_release": 1
    });
    let seq: RawMempoolSequence = serde_json::from_value(v).unwrap();
    assert!(seq.txids.is_empty());
    assert_eq!(seq.mempool_sequence, 0);
}

#[test]
fn get_network_info_full() {
    let v = json!({
        "version": 250000, "subversion": "/Satoshi:25.0.0/",
        "protocolversion": 70016, "localservices": "0000000000000409",
        "localservicesnames": ["NETWORK", "WITNESS"], "localrelay": true,
        "timeoffset": 0, "connections": 10, "connections_in": 5,
        "connections_out": 5, "networkactive": true,
        "networks": [
            {"name": "ipv4", "limited": false, "reachable": true, "proxy": "", "proxy_randomize_credentials": false},
            {"name": "onion", "limited": true, "reachable": false, "proxy": "127.0.0.1:9050", "proxy_randomize_credentials": true}
        ],
        "relayfee": 0.00001000, "incrementalfee": 0.00001000,
        "localaddresses": [
            {"address": "1.2.3.4", "port": 8333, "score": 10}
        ],
        "warnings": ["This is a pre-release test build"]
    });
    let info: NetworkInfo = serde_json::from_value(v).unwrap();
    assert_eq!(info.version, 250000);
    assert_eq!(info.protocol_version, 70016);
    assert_eq!(info.local_services_names, vec!["NETWORK", "WITNESS"]);
    assert_eq!(info.time_offset, 0);
    assert_eq!(info.connections_in, 5);
    assert_eq!(info.connections_out, 5);
    assert_eq!(info.networks.len(), 2);
    assert_eq!(info.networks[1].name, "onion");
    assert!(info.networks[1].limited);
    assert_eq!(info.networks[1].proxy, "127.0.0.1:9050");
    assert!(info.networks[1].proxy_randomize_credentials);
    // relayfee/incrementalfee are plain NUM in net.cpp, not STR_AMOUNT, but still f64.
    assert_eq!(info.relay_fee, FeeRate::from_sat_per_kvb(1_000));
    assert_eq!(info.incremental_fee, FeeRate::from_sat_per_kvb(1_000));
    assert_eq!(info.local_addresses.len(), 1);
    assert_eq!(info.local_addresses[0].address, "1.2.3.4");
    assert_eq!(info.local_addresses[0].port, 8333);
    assert_eq!(info.local_addresses[0].score, 10);
    assert_eq!(
        info.warnings,
        vec!["This is a pre-release test build".to_string()]
    );
}

#[test]
fn get_network_info_minimal_and_forward_compatible() {
    let v = json!({
        "version": 250000, "subversion": "/Satoshi:25.0.0/",
        "protocolversion": 70016, "localservices": "0000000000000000",
        "localservicesnames": [], "localrelay": false,
        // A peer clock behind ours yields a negative offset.
        "timeoffset": -3, "connections": 0, "connections_in": 0,
        "connections_out": 0, "networkactive": false,
        "networks": [
            {"name": "ipv4", "limited": true, "reachable": false, "proxy": "", "proxy_randomize_credentials": false}
        ],
        "relayfee": 0.0, "incrementalfee": 0.0,
        "localaddresses": [],
        "some_field_from_a_future_release": 1
    });
    let info: NetworkInfo = serde_json::from_value(v).unwrap();
    assert_eq!(info.time_offset, -3);
    assert!(info.local_addresses.is_empty());
    // `warnings` has no ARR in this fixture at all; `serde(default)` covers it.
    assert!(info.warnings.is_empty());
}

#[test]
fn get_network_info_warnings_legacy_string_form() {
    // A node run with `-deprecatedrpc=warnings` emits a bare string instead of
    // an array; ruling R25 requires both wire shapes to deserialize.
    let v = json!({
        "version": 250000, "subversion": "/Satoshi:25.0.0/",
        "protocolversion": 70016, "localservices": "0000000000000000",
        "localservicesnames": [], "localrelay": false,
        "timeoffset": 0, "connections": 0, "connections_in": 0,
        "connections_out": 0, "networkactive": false,
        "networks": [],
        "relayfee": 0.0, "incrementalfee": 0.0,
        "localaddresses": [],
        "warnings": "single legacy warning"
    });
    let info: NetworkInfo = serde_json::from_value(v).unwrap();
    assert_eq!(info.warnings, vec!["single legacy warning"]);
}

#[test]
fn peer_info_full() {
    let v = json!({
        "id": 7, "addr": "192.168.0.6:8333", "addrbind": "10.0.0.1:8333",
        "addrlocal": "203.0.113.5:8333", "network": "ipv4", "mapped_as": 12345,
        "services": "0000000000000409", "servicesnames": ["NETWORK", "WITNESS"],
        "relaytxes": true, "last_inv_sequence": 42, "inv_to_send": 3,
        "lastsend": 1690000100, "lastrecv": 1690000099,
        "last_transaction": 1690000000, "last_block": 1689999000,
        "bytessent": 123456, "bytesrecv": 654321, "conntime": 1689990000,
        "timeoffset": -2, "pingtime": 0.05, "minping": 0.04, "pingwait": 0.01,
        "version": 70016, "subver": "/Satoshi:25.0.0/", "inbound": false,
        "bip152_hb_to": true, "bip152_hb_from": false,
        "presynced_headers": -1, "synced_headers": 800000, "synced_blocks": 799999,
        "inflight": [800001, 800002], "addr_relay_enabled": true,
        "addr_processed": 100, "addr_rate_limited": 2,
        "permissions": ["noban"], "minfeefilter": 0.00001000,
        "bytessent_per_msg": {"ping": 32, "verack": 24},
        "bytesrecv_per_msg": {"pong": 32},
        "connection_type": "outbound-full-relay",
        "transport_protocol_type": "v2", "session_id": "abcd1234"
    });
    let peer: PeerInfo = serde_json::from_value(v).unwrap();
    assert_eq!(peer.id, 7);
    assert_eq!(peer.addr_bind.as_deref(), Some("10.0.0.1:8333"));
    assert_eq!(peer.addr_local.as_deref(), Some("203.0.113.5:8333"));
    assert_eq!(peer.mapped_as, Some(12345));
    assert_eq!(peer.services_names, vec!["NETWORK", "WITNESS"]);
    assert!(peer.relay_txes);
    assert_eq!(peer.last_inv_sequence, Some(42));
    assert_eq!(peer.inv_to_send, Some(3));
    assert_eq!(peer.last_send, 1690000100);
    assert_eq!(peer.last_recv, 1690000099);
    assert_eq!(peer.bytes_sent, 123456);
    assert_eq!(peer.bytes_recv, 654321);
    assert_eq!(peer.conn_time, 1689990000);
    // A peer's clock can lag ours, so timeoffset is signed.
    assert_eq!(peer.time_offset, -2);
    assert_eq!(peer.ping_time, Some(0.05));
    assert_eq!(peer.min_ping, Some(0.04));
    assert_eq!(peer.ping_wait, Some(0.01));
    assert_eq!(peer.sub_ver, "/Satoshi:25.0.0/");
    assert!(peer.bip152_hb_to);
    assert!(!peer.bip152_hb_from);
    // -1 means no low-work sync is currently in progress.
    assert_eq!(peer.presynced_headers, -1);
    assert_eq!(peer.synced_headers, 800000);
    assert_eq!(peer.synced_blocks, 799999);
    assert_eq!(peer.inflight, vec![800001, 800002]);
    assert_eq!(peer.addr_processed, 100);
    assert_eq!(peer.addr_rate_limited, 2);
    assert_eq!(peer.permissions, vec!["noban"]);
    assert_eq!(peer.min_fee_filter, FeeRate::from_sat_per_kvb(1_000));
    assert_eq!(peer.bytes_sent_per_msg.get("ping"), Some(&32));
    assert_eq!(peer.bytes_recv_per_msg.get("pong"), Some(&32));
    assert_eq!(peer.connection_type, "outbound-full-relay");
    assert_eq!(peer.transport_protocol_type, "v2");
    assert_eq!(peer.session_id, "abcd1234");
}

#[test]
fn peer_info_minimal_and_forward_compatible() {
    // Every optional field absent, plus the deprecated `startingheight` field
    // (present on a real node under -deprecatedrpc=startingheight) tolerated
    // as an unknown extra since `PeerInfo` has no field for it.
    let v = json!({
        "id": 0, "addr": "10.0.0.2:8333", "network": "ipv4",
        "services": "0000000000000000", "servicesnames": [],
        "relaytxes": false, "last_inv_sequence": 0, "inv_to_send": 0,
        "lastsend": 0, "lastrecv": 0, "last_transaction": 0, "last_block": 0,
        "bytessent": 0, "bytesrecv": 0, "conntime": 0, "timeoffset": 0,
        "version": 70016, "subver": "/Satoshi:25.0.0/", "inbound": true,
        "bip152_hb_to": false, "bip152_hb_from": false,
        "presynced_headers": -1, "synced_headers": -1, "synced_blocks": -1,
        "inflight": [], "addr_relay_enabled": false,
        "addr_processed": 0, "addr_rate_limited": 0,
        "permissions": [], "minfeefilter": 0.0,
        "bytessent_per_msg": {}, "bytesrecv_per_msg": {},
        "connection_type": "inbound", "transport_protocol_type": "v1",
        "session_id": "",
        "startingheight": 800000,
        "some_field_from_a_future_release": 1
    });
    let peer: PeerInfo = serde_json::from_value(v).unwrap();
    assert_eq!(peer.addr_bind, None);
    assert_eq!(peer.addr_local, None);
    assert_eq!(peer.mapped_as, None);
    assert_eq!(peer.ping_time, None);
    assert_eq!(peer.min_ping, None);
    assert_eq!(peer.ping_wait, None);
    // Right after connecting, before any header sync, these read -1.
    assert_eq!(peer.presynced_headers, -1);
    assert_eq!(peer.synced_headers, -1);
    assert_eq!(peer.synced_blocks, -1);
    assert!(peer.inflight.is_empty());
    assert!(peer.permissions.is_empty());
    assert!(peer.bytes_sent_per_msg.is_empty());
    assert!(peer.bytes_recv_per_msg.is_empty());
}

#[test]
fn get_net_totals_full() {
    let v = json!({
        "totalbytesrecv": 1000000, "totalbytessent": 2000000,
        "timemillis": 1690000000000_i64,
        "uploadtarget": {
            "timeframe": 86400, "target": 5000000000_u64,
            "target_reached": false, "serve_historical_blocks": true,
            "bytes_left_in_cycle": 3000000000_u64, "time_left_in_cycle": 43200
        }
    });
    let totals: NetTotals = serde_json::from_value(v).unwrap();
    assert_eq!(totals.total_bytes_recv, 1000000);
    assert_eq!(totals.total_bytes_sent, 2000000);
    assert_eq!(totals.time_millis, 1690000000000);
    assert_eq!(totals.upload_target.timeframe, 86400);
    assert_eq!(totals.upload_target.target, 5000000000);
    assert!(!totals.upload_target.target_reached);
    assert!(totals.upload_target.serve_historical_blocks);
    assert_eq!(totals.upload_target.bytes_left_in_cycle, 3000000000);
    assert_eq!(totals.upload_target.time_left_in_cycle, 43200);
}

#[test]
fn get_mining_info_full() {
    let v = json!({
        "blocks": 800000, "currentblockweight": 4000000_u64, "currentblocktx": 2500,
        "bits": "170d6b91", "difficulty": 53911173001054.59,
        "target": "0000000000000000000340190000000000000000000000000000000000000",
        "networkhashps": 350000000000000000000.0, "pooledtx": 120,
        "blockmintxfee": 0.00001000, "chain": "main", "signet_challenge": "51",
        "next": {
            "height": 800001, "bits": "170d6b90", "difficulty": 53911173001054.6,
            "target": "0000000000000000000340180000000000000000000000000000000000000",
            // Planted to prove unknown-field tolerance nested inside a child struct.
            "some_nested_field_from_a_future_release": 1
        },
        "warnings": ["This is a pre-release test build"]
    });
    let info: MiningInfo = serde_json::from_value(v).unwrap();
    assert_eq!(info.blocks, 800000);
    assert_eq!(info.current_block_weight, Some(4000000));
    assert_eq!(info.current_block_tx, Some(2500));
    assert_eq!(info.bits, "170d6b91");
    assert_eq!(info.difficulty, 53911173001054.59);
    // networkhashps is a plain NUM (a double), not STR_AMOUNT, but still f64.
    assert_eq!(info.network_hash_ps, 350000000000000000000.0);
    assert_eq!(info.pooled_tx, 120);
    // blockmintxfee is the one STR_AMOUNT in mining.cpp: an unquoted JSON number.
    assert_eq!(
        info.block_min_tx_fee,
        Some(FeeRate::from_sat_per_kvb(1_000))
    );
    assert_eq!(info.chain, "main");
    assert_eq!(info.signet_challenge.as_deref(), Some("51"));
    assert_eq!(info.next.height, 800001);
    assert_eq!(info.next.bits, "170d6b90");
    assert_eq!(info.next.difficulty, 53911173001054.6);
    assert_eq!(
        info.warnings,
        vec!["This is a pre-release test build".to_string()]
    );
}

#[test]
fn get_mining_info_minimal_and_forward_compatible() {
    let v = json!({
        "blocks": 0, "bits": "207fffff", "difficulty": 4.656542373871732e-10,
        "target": "7fffff0000000000", "networkhashps": 0.0, "pooledtx": 0,
        "blockmintxfee": 0.00001000, "chain": "regtest",
        "next": {
            "height": 1, "bits": "207fffff", "difficulty": 4.656542373871732e-10,
            "target": "7fffff0000000000"
        },
        "warnings": [],
        "some_field_from_a_future_release": 1
    });
    let info: MiningInfo = serde_json::from_value(v).unwrap();
    assert_eq!(info.current_block_weight, None);
    assert_eq!(info.current_block_tx, None);
    assert_eq!(info.signet_challenge, None);
    assert!(info.warnings.is_empty());
}

#[test]
fn get_mining_info_warnings_legacy_string_form() {
    // A node run with `-deprecatedrpc=warnings` emits a bare string instead of
    // an array; ruling R25 requires both wire shapes to deserialize.
    let v = json!({
        "blocks": 0, "bits": "207fffff", "difficulty": 4.656542373871732e-10,
        "target": "7fffff0000000000", "networkhashps": 0.0, "pooledtx": 0,
        "blockmintxfee": 0.00001000, "chain": "regtest",
        "next": {
            "height": 1, "bits": "207fffff", "difficulty": 4.656542373871732e-10,
            "target": "7fffff0000000000"
        },
        "warnings": "single legacy warning"
    });
    let info: MiningInfo = serde_json::from_value(v).unwrap();
    assert_eq!(info.warnings, vec!["single legacy warning"]);
}

#[test]
fn block_template_full() {
    let v = json!({
        "version": 536870912, "rules": ["csv", "!segwit"],
        "vbavailable": {"!testdummy": 28}, "capabilities": ["proposal"],
        "vbrequired": 0,
        "previousblockhash": "00000000000000000002a7c4c1e48d76c5a37902165a270156b7a8d72728a054",
        "transactions": [
            {
                "data": "0200000001abcd",
                "txid": "2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866",
                "hash": "2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866",
                "depends": [1], "fee": 1000, "sigops": 4, "weight": 565
            }
        ],
        "coinbaseaux": {"flags": ""},
        "coinbasevalue": 625000000_u64,
        "longpollid": "00000000000000000002a7c4c1e48d76c5a37902165a270156b7a8d72728a05412345",
        "target": "0000000000000000000340190000000000000000000000000000000000000",
        "mintime": 1690000000, "mutable": ["time", "transactions", "prevblock"],
        "noncerange": "00000000ffffffff", "sigoplimit": 80000, "sizelimit": 4000000,
        "weightlimit": 4000000_u64, "curtime": 1690000100, "bits": "170d6b91",
        "height": 800001, "signet_challenge": "51",
        "default_witness_commitment": "6a24aa21a9ed00"
    });
    let tpl: BlockTemplate = serde_json::from_value(v).unwrap();
    assert_eq!(tpl.version, 536870912);
    assert_eq!(tpl.rules, vec!["csv", "!segwit"]);
    assert_eq!(tpl.vb_available.get("!testdummy"), Some(&28));
    assert_eq!(tpl.capabilities, vec!["proposal"]);
    assert_eq!(tpl.vb_required, 0);
    assert!(tpl.previous_block_hash.starts_with("00000000"));
    assert_eq!(tpl.transactions.len(), 1);
    let tx = &tpl.transactions[0];
    assert_eq!(tx.depends, vec![1]);
    // fee/sigops/weight are raw satoshi/count integers per BIP 22, not f64.
    assert_eq!(tx.fee, 1000);
    assert_eq!(tx.sig_ops, 4);
    assert_eq!(tx.weight, 565);
    assert_eq!(tpl.coinbase_aux.get("flags"), Some(&"".to_string()));
    // coinbasevalue is a raw CAmount integer, not run through ValueFromAmount.
    assert_eq!(tpl.coinbase_value, 625000000);
    assert_eq!(tpl.min_time, 1690000000);
    assert_eq!(tpl.mutable, vec!["time", "transactions", "prevblock"]);
    assert_eq!(tpl.sigop_limit, 80000);
    assert_eq!(tpl.size_limit, 4000000);
    assert_eq!(tpl.weight_limit, Some(4000000));
    assert_eq!(tpl.cur_time, 1690000100);
    assert_eq!(tpl.height, 800001);
    assert_eq!(tpl.signet_challenge.as_deref(), Some("51"));
    assert_eq!(
        tpl.default_witness_commitment.as_deref(),
        Some("6a24aa21a9ed00")
    );
}

#[test]
fn block_template_minimal_and_forward_compatible() {
    let v = json!({
        "version": 536870912, "rules": [], "vbavailable": {}, "capabilities": [],
        "vbrequired": 0,
        "previousblockhash": "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206",
        "transactions": [], "coinbaseaux": {}, "coinbasevalue": 5000000000_u64,
        "longpollid": "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e22061",
        "target": "7fffff0000000000", "mintime": 1296688602, "mutable": ["time"],
        "noncerange": "00000000ffffffff", "sigoplimit": 80000, "sizelimit": 4000000,
        "curtime": 1296688602, "bits": "207fffff", "height": 1,
        "some_field_from_a_future_release": 1
    });
    let tpl: BlockTemplate = serde_json::from_value(v).unwrap();
    assert!(tpl.rules.is_empty());
    assert!(tpl.transactions.is_empty());
    assert_eq!(tpl.weight_limit, None);
    assert_eq!(tpl.signet_challenge, None);
    assert_eq!(tpl.default_witness_commitment, None);
}

#[test]
fn block_template_request_default_sets_segwit_rule_only() {
    let req = BlockTemplateRequest::default();
    assert_eq!(req.rules, vec!["segwit".to_string()]);
    assert_eq!(req.mode, None);
    // Absent optionals must be omitted, not serialized as explicit nulls,
    // since `positional` only trims trailing nulls at the top level.
    assert_eq!(
        serde_json::to_value(&req).unwrap(),
        json!({"rules": ["segwit"]})
    );
}

#[test]
fn block_template_request_round_trips_all_fields() {
    let req = BlockTemplateRequest {
        mode: Some("template".to_string()),
        capabilities: Some(vec!["coinbasevalue".to_string()]),
        rules: vec!["segwit".to_string()],
        longpoll_id: Some("abc123".to_string()),
        data: Some("deadbeef".to_string()),
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["longpollid"], "abc123");
    let back: BlockTemplateRequest = serde_json::from_value(v).unwrap();
    assert_eq!(back, req);
}

#[test]
fn get_net_totals_forward_compatible() {
    // Every field is required in `getnettotals`'s RPCResult, so this test's
    // forward-compatibility burden falls entirely on the unknown field.
    let v = json!({
        "totalbytesrecv": 0, "totalbytessent": 0, "timemillis": 0,
        "uploadtarget": {
            "timeframe": 0, "target": 0, "target_reached": false,
            "serve_historical_blocks": false, "bytes_left_in_cycle": 0,
            "time_left_in_cycle": 0
        },
        "some_field_from_a_future_release": 1
    });
    let totals: NetTotals = serde_json::from_value(v).unwrap();
    assert_eq!(totals.total_bytes_recv, 0);
    assert_eq!(totals.upload_target.target, 0);
    assert!(!totals.upload_target.target_reached);
}

#[test]
fn transaction_verbose_full() {
    // `getrawtransaction` verbosity 2 output: every optional field present,
    // including the `prevout` that only undo data can supply.
    let v = json!({
        "in_active_chain": true,
        "txid": "9e1a2d3f4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f6",
        "hash": "1f2e3d4c5b6a798807162534435261708f9e0d1c2b3a4958475664738291a0b1",
        "version": 2, "size": 226, "vsize": 144, "weight": 574,
        "locktime": 800000,
        "vin": [
            {
                "txid": "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
                "vout": 1,
                "scriptSig": { "asm": "3045022100[ALL] 03a1b2", "hex": "483045022100" },
                "txinwitness": ["3045022100", "03a1b2"],
                "prevout": {
                    "generated": false,
                    "height": 799999,
                    "value": 0.05,
                    "scriptPubKey": {
                        "asm": "0 abcdef01",
                        "desc": "addr(bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4)#8rar0j5g",
                        "hex": "0014abcdef01",
                        "address": "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4",
                        "type": "witness_v0_keyhash"
                    }
                },
                "sequence": 4294967293_u64
            }
        ],
        "vout": [
            {
                "value": 0.04998,
                "n": 0,
                "scriptPubKey": {
                    "asm": "OP_DUP OP_HASH160 abcdef01 OP_EQUALVERIFY OP_CHECKSIG",
                    "desc": "pkh(03a1b2)#checksum",
                    "hex": "76a914abcdef0188ac",
                    "address": "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH",
                    "type": "pubkeyhash"
                }
            }
        ],
        "hex": "0200000001abcdef",
        "blockhash": "00000000000000000002a7c4c1e48d76c5a37902165a270156b7a8d72728a054",
        "confirmations": 12,
        "time": 1690000000,
        "blocktime": 1690000000,
        "fee": 0.00012345
    });
    let tx: Transaction = serde_json::from_value(v).unwrap();
    assert_eq!(tx.in_active_chain, Some(true));
    assert_eq!(tx.vsize, 144);
    assert_eq!(tx.locktime, 800000);
    assert_eq!(tx.confirmations, Some(12));
    assert_eq!(tx.block_time, Some(1690000000));
    assert_eq!(tx.hex.as_deref(), Some("0200000001abcdef"));
    // Only present at verbosity 2, and only with undo data available.
    assert_eq!(tx.fee, Some(Amount::from_sat(12_345)));

    assert_eq!(tx.vin.len(), 1);
    let vin = &tx.vin[0];
    assert_eq!(vin.coinbase, None);
    assert_eq!(vin.vout, Some(1));
    assert_eq!(vin.sequence, 4294967293);
    assert_eq!(
        vin.script_sig.as_ref().unwrap().hex,
        "483045022100".to_string()
    );
    assert_eq!(
        vin.tx_in_witness.as_deref(),
        Some(["3045022100".to_string(), "03a1b2".to_string()].as_slice())
    );
    let prevout = vin.prevout.as_ref().unwrap();
    assert!(!prevout.generated);
    assert_eq!(prevout.height, 799999);
    // `value` goes through `ValueFromAmount`, so it arrives as a JSON number.
    assert_eq!(prevout.value, Amount::from_sat(5_000_000));
    assert_eq!(prevout.script_pub_key.script_type, "witness_v0_keyhash");

    assert_eq!(tx.vout.len(), 1);
    assert_eq!(tx.vout[0].value, Amount::from_sat(4_998_000));
    assert_eq!(tx.vout[0].n, 0);
    assert_eq!(tx.vout[0].script_pub_key.script_type, "pubkeyhash");
    assert_eq!(
        tx.vout[0].script_pub_key.address.as_deref(),
        Some("1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH")
    );
    assert_eq!(tx.vout[0].script_pub_key.desc, "pkh(03a1b2)#checksum");
}

#[test]
fn transaction_minimal_and_forward_compatible() {
    // `decoderawtransaction` output for a coinbase transaction: no block
    // context, no `hex` (Core passes `include_hex=false`), a `vin` with only
    // `coinbase`/`sequence` and a `scriptPubKey` with no well-defined address.
    let v = json!({
        "txid": "2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866",
        "hash": "2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866",
        "version": 1, "size": 204, "vsize": 204, "weight": 816, "locktime": 0,
        "vin": [
            { "coinbase": "04ffff001d0102", "sequence": 4294967295_u64 }
        ],
        "vout": [
            {
                "value": 50.0,
                "n": 0,
                "scriptPubKey": {
                    "asm": "04678afdb0 OP_CHECKSIG",
                    "desc": "pk(04678afdb0)#checksum",
                    "hex": "4104678afdb0ac",
                    "type": "pubkey",
                    "some_nested_field_from_a_future_release": true
                }
            }
        ],
        "some_field_from_a_future_release": 1
    });
    let tx: Transaction = serde_json::from_value(v).unwrap();
    assert_eq!(tx.in_active_chain, None);
    assert_eq!(tx.hex, None);
    assert_eq!(tx.block_hash, None);
    assert_eq!(tx.confirmations, None);
    assert_eq!(tx.time, None);
    assert_eq!(tx.block_time, None);
    assert_eq!(tx.fee, None);
    assert_eq!(tx.vin[0].coinbase.as_deref(), Some("04ffff001d0102"));
    assert_eq!(tx.vin[0].txid, None);
    assert_eq!(tx.vin[0].vout, None);
    assert_eq!(tx.vin[0].script_sig, None);
    assert_eq!(tx.vin[0].tx_in_witness, None);
    assert_eq!(tx.vin[0].prevout, None);
    assert_eq!(tx.vout[0].value, Amount::from_sat(5_000_000_000));
    assert_eq!(tx.vout[0].script_pub_key.script_type, "pubkey");
    assert_eq!(tx.vout[0].script_pub_key.address, None);
}

#[test]
fn test_mempool_accept_result_full() {
    let v = json!({
        "txid": "9e1a2d3f4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f6",
        "wtxid": "1f2e3d4c5b6a798807162534435261708f9e0d1c2b3a4958475664738291a0b1",
        "package-error": "package-not-child-with-unconfirmed-parents",
        "allowed": true,
        "vsize": 144,
        "fees": {
            "base": 0.00001234,
            "effective-feerate": 0.00008567,
            "effective-includes": [
                "1f2e3d4c5b6a798807162534435261708f9e0d1c2b3a4958475664738291a0b1"
            ],
            "some_nested_field_from_a_future_release": 1
        },
        "reject-reason": "max-fee-exceeded",
        "reject-details": "max-fee-exceeded, tx feerate too high"
    });
    let result: TestMempoolAcceptResult = serde_json::from_value(v).unwrap();
    assert_eq!(result.allowed, Some(true));
    assert_eq!(result.vsize, Some(144));
    assert_eq!(
        result.package_error.as_deref(),
        Some("package-not-child-with-unconfirmed-parents")
    );
    assert_eq!(result.reject_reason.as_deref(), Some("max-fee-exceeded"));
    assert!(result.reject_details.is_some());
    let fees = result.fees.as_ref().unwrap();
    // Both are STR_AMOUNT, i.e. JSON numbers.
    assert_eq!(fees.base, Amount::from_sat(1_234));
    assert_eq!(fees.effective_feerate, FeeRate::from_sat_per_kvb(8_567));
    assert_eq!(fees.effective_includes.len(), 1);
    assert_eq!(
        fees.effective_includes[0],
        "1f2e3d4c5b6a798807162534435261708f9e0d1c2b3a4958475664738291a0b1"
    );
}

#[test]
fn test_mempool_accept_result_minimal_and_forward_compatible() {
    // Validation left unfinished by a failure in another transaction of the
    // package: only the two hashes are returned.
    let v = json!({
        "txid": "9e1a2d3f4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f6",
        "wtxid": "1f2e3d4c5b6a798807162534435261708f9e0d1c2b3a4958475664738291a0b1",
        "some_field_from_a_future_release": 1
    });
    let result: TestMempoolAcceptResult = serde_json::from_value(v).unwrap();
    assert_eq!(
        result.txid,
        "9e1a2d3f4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f6"
    );
    assert_eq!(result.package_error, None);
    assert_eq!(result.allowed, None);
    assert_eq!(result.vsize, None);
    assert_eq!(result.fees, None);
    assert_eq!(result.reject_reason, None);
    assert_eq!(result.reject_details, None);
}

#[test]
fn create_raw_transaction_input_omits_an_absent_sequence() {
    let with_sequence = CreateRawTransactionInput {
        txid: "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b".to_string(),
        vout: 1,
        sequence: Some(4294967293),
    };
    assert_eq!(
        serde_json::to_value(&with_sequence).unwrap(),
        json!({
            "txid": "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
            "vout": 1,
            "sequence": 4294967293_u64
        })
    );

    let without = CreateRawTransactionInput {
        sequence: None,
        ..with_sequence
    };
    assert_eq!(
        serde_json::to_value(&without).unwrap(),
        json!({
            "txid": "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
            "vout": 1
        })
    );
}

#[test]
fn create_raw_transaction_output_serializes_the_address_as_the_key() {
    let pay = CreateRawTransactionOutput::Address {
        address: "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4".to_string(),
        amount: Amount::from_sat(1_000_000),
    };
    assert_eq!(
        serde_json::to_value(&pay).unwrap(),
        json!({ "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4": 0.01 })
    );

    let data = CreateRawTransactionOutput::Data("00010203".to_string());
    assert_eq!(
        serde_json::to_value(&data).unwrap(),
        json!({ "data": "00010203" })
    );
}

#[test]
fn estimate_smart_fee_full() {
    let v = json!({
        "feerate": 0.00001200,
        "errors": ["some warning"],
        "blocks": 6
    });
    let est: FeeEstimate = serde_json::from_value(v).unwrap();
    assert_eq!(est.feerate, Some(FeeRate::from_sat_per_kvb(1_200)));
    assert_eq!(est.errors, Some(vec!["some warning".to_string()]));
    assert_eq!(est.blocks, 6);
}

#[test]
fn estimate_smart_fee_errors_only() {
    // When the node cannot produce an estimate it omits `feerate` entirely
    // and returns only `errors` and `blocks`.
    let v = json!({
        "errors": ["Insufficient data or no feerate found"],
        "blocks": 1008,
        "some_field_from_a_future_release": 1
    });
    let est: FeeEstimate = serde_json::from_value(v).unwrap();
    assert_eq!(est.feerate, None);
    assert_eq!(
        est.errors,
        Some(vec!["Insufficient data or no feerate found".to_string()])
    );
    assert_eq!(est.blocks, 1008);
}

#[test]
fn get_rpc_info_full() {
    let v = json!({
        "active_commands": [
            {
                "method": "getblockchaininfo",
                "duration": 1234,
                "some_nested_field_from_a_future_release": 1
            }
        ],
        "logpath": "/home/user/.bitcoin/debug.log"
    });
    let info: RpcInfo = serde_json::from_value(v).unwrap();
    assert_eq!(info.active_commands.len(), 1);
    assert_eq!(info.active_commands[0].method, "getblockchaininfo");
    assert_eq!(info.active_commands[0].duration, 1234);
    assert_eq!(info.logpath, "/home/user/.bitcoin/debug.log");
}

#[test]
fn get_rpc_info_minimal_and_forward_compatible() {
    let v = json!({
        "active_commands": [],
        "logpath": "/home/user/.bitcoin/debug.log",
        "some_field_from_a_future_release": 1
    });
    let info: RpcInfo = serde_json::from_value(v).unwrap();
    assert!(info.active_commands.is_empty());
    assert_eq!(info.logpath, "/home/user/.bitcoin/debug.log");
}

#[test]
fn validate_address_full() {
    // The node never sets every field simultaneously (the success and error
    // paths are mutually exclusive in `output_script.cpp`), but the fixture
    // sets all eight optionals at once purely to exercise every field's
    // deserialization.
    let v = json!({
        "isvalid": true,
        "address": "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4",
        "scriptPubKey": "0014abcdef01",
        "isscript": false,
        "iswitness": true,
        "witness_version": 0,
        "witness_program": "abcdef01",
        "error": "Invalid Bech32 checksum",
        "error_locations": [9, 10]
    });
    let addr: AddressValidation = serde_json::from_value(v).unwrap();
    assert!(addr.isvalid);
    assert_eq!(
        addr.address.as_deref(),
        Some("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4")
    );
    assert_eq!(addr.script_pub_key.as_deref(), Some("0014abcdef01"));
    assert_eq!(addr.isscript, Some(false));
    assert_eq!(addr.iswitness, Some(true));
    assert_eq!(addr.witness_version, Some(0));
    assert_eq!(addr.witness_program.as_deref(), Some("abcdef01"));
    assert_eq!(addr.error.as_deref(), Some("Invalid Bech32 checksum"));
    assert_eq!(addr.error_locations, Some(vec![9, 10]));
}

#[test]
fn validate_address_invalid_returns_only_isvalid_and_error_fields() {
    // An invalid address returns little more than `isvalid: false`, plus the
    // error fields; none of the success-path optionals are present.
    let v = json!({
        "isvalid": false,
        "error": "Invalid Bech32 checksum",
        "error_locations": [9, 10],
        "some_field_from_a_future_release": 1
    });
    let addr: AddressValidation = serde_json::from_value(v).unwrap();
    assert!(!addr.isvalid);
    assert_eq!(addr.address, None);
    assert_eq!(addr.script_pub_key, None);
    assert_eq!(addr.isscript, None);
    assert_eq!(addr.iswitness, None);
    assert_eq!(addr.witness_version, None);
    assert_eq!(addr.witness_program, None);
    assert_eq!(addr.error.as_deref(), Some("Invalid Bech32 checksum"));
    assert_eq!(addr.error_locations, Some(vec![9, 10]));
}

#[test]
fn index_info_map_full() {
    let v = json!({
        "txindex": {
            "synced": true,
            "best_block_height": 800000,
            "some_nested_field_from_a_future_release": 1
        }
    });
    let map: std::collections::BTreeMap<String, IndexInfo> = serde_json::from_value(v).unwrap();
    let txindex = map.get("txindex").unwrap();
    assert!(txindex.synced);
    assert_eq!(txindex.best_block_height, 800000);
}

#[test]
fn index_info_map_minimal_and_forward_compatible() {
    let v = json!({});
    let map: std::collections::BTreeMap<String, IndexInfo> = serde_json::from_value(v).unwrap();
    assert!(map.is_empty());
}

#[test]
fn psbt_analysis_full() {
    // Every field the `analyzepsbt` RPCResult can produce, including the four
    // `missing` members, which no single live PSBT populates at once.
    let v = json!({
        "inputs": [{
            "has_utxo": true,
            "is_final": false,
            "missing": {
                "pubkeys": ["a9e59e4bf859870fee727e80083a784b6ae59f2f"],
                "signatures": ["c51b66bced5e4491001bd702669770dccf440982"],
                "redeemscript": "15cc49e191cbc520d91944600a5cb77af6aa3291",
                "witnessscript": "4bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459a"
            },
            "next": "signer"
        }],
        "estimated_vsize": 110,
        "estimated_feerate": 90.9090909,
        "fee": 10.0,
        "next": "signer",
        "error": "PSBT is not valid. Input 0 spends unspendable output"
    });
    let a: PsbtAnalysis = serde_json::from_value(v).unwrap();
    let inputs = a.inputs.as_ref().unwrap();
    assert_eq!(inputs.len(), 1);
    assert!(inputs[0].has_utxo);
    assert!(!inputs[0].is_final);
    assert_eq!(inputs[0].next.as_deref(), Some("signer"));
    let missing = inputs[0].missing.as_ref().unwrap();
    assert_eq!(
        missing.pubkeys.as_deref(),
        Some(["a9e59e4bf859870fee727e80083a784b6ae59f2f".to_string()].as_slice())
    );
    assert_eq!(
        missing.signatures.as_deref(),
        Some(["c51b66bced5e4491001bd702669770dccf440982".to_string()].as_slice())
    );
    assert_eq!(
        missing.redeem_script.as_deref(),
        Some("15cc49e191cbc520d91944600a5cb77af6aa3291")
    );
    assert_eq!(
        missing.witness_script.as_deref(),
        Some("4bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459a")
    );
    assert_eq!(a.estimated_vsize, Some(110));
    assert_eq!(
        a.estimated_feerate,
        Some(FeeRate::from_sat_per_kvb(9_090_909_090))
    );
    assert_eq!(a.fee, Some(Amount::from_sat(1_000_000_000)));
    assert_eq!(a.next, "signer");
    assert!(a.error.is_some());
}

#[test]
fn psbt_analysis_stages_from_a_live_node() {
    // Captured from bitcoind v31.1 on regtest, walking one PSBT from creation
    // to finalization. Each stage drops or adds fields, so all four shapes
    // must deserialize.
    let no_utxo: PsbtAnalysis = serde_json::from_value(json!({
        "inputs": [{"has_utxo": false, "is_final": false, "next": "updater"}],
        "next": "updater"
    }))
    .unwrap();
    assert!(!no_utxo.inputs.as_ref().unwrap()[0].has_utxo);
    // Before the UTXOs are known, Core cannot compute a fee or a size.
    assert_eq!(no_utxo.fee, None);
    assert_eq!(no_utxo.estimated_vsize, None);
    assert_eq!(no_utxo.estimated_feerate, None);

    let updated: PsbtAnalysis = serde_json::from_value(json!({
        "inputs": [{
            "has_utxo": true,
            "is_final": false,
            "next": "updater",
            "missing": {"pubkeys": ["a9e59e4bf859870fee727e80083a784b6ae59f2f"]}
        }],
        "fee": 10.0,
        "next": "updater"
    }))
    .unwrap();
    assert_eq!(updated.fee, Some(Amount::from_sat(1_000_000_000)));
    // Only `pubkeys` is missing; the other three members stay absent.
    let missing = updated.inputs.as_ref().unwrap()[0]
        .missing
        .as_ref()
        .unwrap();
    assert!(missing.pubkeys.is_some());
    assert_eq!(missing.signatures, None);
    assert_eq!(missing.redeem_script, None);
    assert_eq!(missing.witness_script, None);

    let signed: PsbtAnalysis = serde_json::from_value(json!({
        "inputs": [{"has_utxo": true, "is_final": false, "next": "finalizer"}],
        "estimated_vsize": 110,
        "estimated_feerate": 90.9090909,
        "fee": 10.0,
        "next": "finalizer"
    }))
    .unwrap();
    assert_eq!(signed.next, "finalizer");
    assert_eq!(signed.estimated_vsize, Some(110));
    assert_eq!(signed.inputs.as_ref().unwrap()[0].missing, None);

    let complete: PsbtAnalysis = serde_json::from_value(json!({
        "inputs": [{"has_utxo": true, "is_final": true, "next": "extractor"}],
        "estimated_vsize": 110,
        "estimated_feerate": 90.9090909,
        "fee": 10.0,
        "next": "extractor"
    }))
    .unwrap();
    assert!(complete.inputs.as_ref().unwrap()[0].is_final);
    assert_eq!(complete.next, "extractor");
}

#[test]
fn psbt_analysis_minimal_and_forward_compatible() {
    // `next` is the only field Core always fills; an unparsable PSBT yields
    // `error` with no `inputs` at all.
    let v = json!({
        "next": "creator",
        "error": "TX decode failed",
        "some_field_from_a_future_release": "x"
    });
    let a: PsbtAnalysis = serde_json::from_value(v).unwrap();
    assert_eq!(a.next, "creator");
    assert_eq!(a.error.as_deref(), Some("TX decode failed"));
    assert_eq!(a.inputs, None);
}

#[test]
fn psbt_finalization_and_process_result_shapes() {
    let extracted: PsbtFinalization =
        serde_json::from_value(json!({"hex": "0200000000010128", "complete": true})).unwrap();
    assert_eq!(extracted.hex.as_deref(), Some("0200000000010128"));
    assert_eq!(extracted.psbt, None);
    assert!(extracted.complete);

    let kept: PsbtFinalization =
        serde_json::from_value(json!({"psbt": "cHNidP8BAFIC", "complete": false})).unwrap();
    assert_eq!(kept.psbt.as_deref(), Some("cHNidP8BAFIC"));
    assert_eq!(kept.hex, None);
    assert!(!kept.complete);

    // `hex` appears only once the PSBT is complete.
    let signed: PsbtProcessResult = serde_json::from_value(
        json!({"psbt": "cHNidP8BAFIC", "complete": true, "hex": "0200000000010128"}),
    )
    .unwrap();
    assert_eq!(signed.hex.as_deref(), Some("0200000000010128"));

    let watch_only: PsbtProcessResult =
        serde_json::from_value(json!({"psbt": "cHNidP8BAFIC", "complete": false})).unwrap();
    assert!(!watch_only.complete);
    assert_eq!(watch_only.hex, None);
}

#[test]
fn derived_addresses_picks_the_shape_from_the_payload() {
    let flat: DerivedAddresses =
        serde_json::from_value(json!(["bcrt1q6mrgxcz4953pk5g7xge8t5vnlwt4m8hypsqppq"])).unwrap();
    assert_eq!(
        flat,
        DerivedAddresses::Single(vec![
            "bcrt1q6mrgxcz4953pk5g7xge8t5vnlwt4m8hypsqppq".to_string()
        ])
    );

    let nested: DerivedAddresses = serde_json::from_value(json!([
        ["bcrt1qp5wfcq48h6d63wyy9qz0awtpfqwwv4sm4gc9mc"],
        ["bcrt1q7zwtzcqsm3k43ha0ac7nl8cz0hqrhckyxxcw45"]
    ]))
    .unwrap();
    assert_eq!(
        nested,
        DerivedAddresses::Multipath(vec![
            vec!["bcrt1qp5wfcq48h6d63wyy9qz0awtpfqwwv4sm4gc9mc".to_string()],
            vec!["bcrt1q7zwtzcqsm3k43ha0ac7nl8cz0hqrhckyxxcw45".to_string()],
        ])
    );

    // An empty result is a single empty list, not an empty multipath list.
    let empty: DerivedAddresses = serde_json::from_value(json!([])).unwrap();
    assert_eq!(empty, DerivedAddresses::Single(vec![]));
}

#[test]
fn psbt_decoded_full() {
    // Every field `decodepsbt` can emit, including the ones no single live PSBT
    // populates: all four preimage maps, a key-path signature, and output-level
    // MuSig2 participants.
    let v = json!({
        "tx": {
            "txid": "9e1a2d3f4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f6",
            "hash": "1f2e3d4c5b6a798807162534435261708f9e0d1c2b3a4958475664738291a0b1",
            "version": 2,
            "size": 226,
            "vsize": 144,
            "weight": 574,
            "locktime": 0,
            "vin": [],
            "vout": []
        },
        "global_xpubs": [{
            "xpub": "tpubD6NzVbkrYhZ4XgiXtGrdW5XDAPFCL9h7we1vwNCpn8tGbBcgfVYjXyhWo4E1xkh56hjod1RhGjxbaTLV3X4FyWuejifB9jusQ46QzG87VKp",
            "master_fingerprint": "b5da67b1",
            "path": "m/84h/1h/0h"
        }],
        "psbt_version": 0,
        "proprietary": [{"identifier": "aaaa", "subtype": 0, "key": "fc02aaaa", "value": "bb"}],
        "unknown": {"0f": "aabb"},
        "inputs": [{
            "non_witness_utxo": {
                "txid": "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
                "hash": "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
                "version": 1,
                "size": 204,
                "vsize": 204,
                "weight": 816,
                "locktime": 0,
                "vin": [],
                "vout": []
            },
            "witness_utxo": {
                "amount": 0.5,
                "scriptPubKey": {
                    "asm": "0 abcdef01",
                    "desc": "addr(bcrt1q6mrgxcz4953pk5g7xge8t5vnlwt4m8hypsqppq)#8rar0j5g",
                    "hex": "0014abcdef01",
                    "type": "witness_v0_keyhash",
                    "address": "bcrt1q6mrgxcz4953pk5g7xge8t5vnlwt4m8hypsqppq"
                }
            },
            "partial_signatures": {"02e5a2b3": "304402203c78"},
            "sighash": "ALL",
            "redeem_script": {"asm": "0 abcd", "hex": "0014abcd", "type": "witness_v0_keyhash"},
            "witness_script": {"asm": "OP_1 02aa OP_1 OP_CHECKMULTISIG", "hex": "512102aa51ae", "type": "multisig"},
            "bip32_derivs": [{
                "pubkey": "029f9a0409dbd39eb724f5e1266c07eeeed3cec1e72907e0228e9db636c6b72847",
                "master_fingerprint": "b5da67b1",
                "path": "m/84h/1h/0h/0/0"
            }],
            "final_scriptSig": {"asm": "304402203c78", "hex": "47304402203c78"},
            "final_scriptwitness": ["304402203c78", "029f9a04"],
            "ripemd160_preimages": {"189f7c8b1a386ffe8eed91b3830c7a7bcd1e778c": "0011"},
            "sha256_preimages": {"4bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459a": "0022"},
            "hash160_preimages": {"15cc49e191cbc520d91944600a5cb77af6aa3291": "0033"},
            "hash256_preimages": {"76a56aced915d2513dcd84c2c378b2e8aa5cd632b5b71ca2f2ac5b0e3a649bdb": "0044"},
            "taproot_key_path_sig": "a1b2c3",
            "taproot_script_path_sigs": [{
                "pubkey": "736e572900fe1252589a2143c8f3c79f71a0412d2353af755e9701c782694a02",
                "leaf_hash": "b11fedaa63a0956501a7308c93b5637371e7613d9b8ade1783d49e26c06cfa2c",
                "sig": "d1d2d3"
            }],
            "taproot_scripts": [{
                "script": "20736e572900fe1252589a2143c8f3c79f71a0412d2353af755e9701c782694a02ac",
                "leaf_ver": 192,
                "control_blocks": ["c0736e5729"]
            }],
            "taproot_bip32_derivs": [{
                "pubkey": "736e572900fe1252589a2143c8f3c79f71a0412d2353af755e9701c782694a02",
                "master_fingerprint": "b5da67b1",
                "path": "m/86h/1h/0h/0/0",
                "leaf_hashes": ["b11fedaa63a0956501a7308c93b5637371e7613d9b8ade1783d49e26c06cfa2c"]
            }],
            "taproot_internal_key": "736e572900fe1252589a2143c8f3c79f71a0412d2353af755e9701c782694a02",
            "taproot_merkle_root": "b11fedaa63a0956501a7308c93b5637371e7613d9b8ade1783d49e26c06cfa2c",
            "musig2_participant_pubkeys": [{
                "aggregate_pubkey": "030b58e337aa4d3852a8c29387c42408d8cfbe3a613a5e397e0a9f01a5fb7107d4",
                "participant_pubkeys": ["02346b99593357107c9d3459e9deba8d3eaf44e6636c85c7f853eb90ba52e8cd00"]
            }],
            "musig2_pubnonces": [{
                "participant_pubkey": "02346b99593357107c9d3459e9deba8d3eaf44e6636c85c7f853eb90ba52e8cd00",
                "aggregate_pubkey": "030b58e337aa4d3852a8c29387c42408d8cfbe3a613a5e397e0a9f01a5fb7107d4",
                "leaf_hash": "b11fedaa63a0956501a7308c93b5637371e7613d9b8ade1783d49e26c06cfa2c",
                "pubnonce": "02d99e7c87"
            }],
            "musig2_partial_sigs": [{
                "participant_pubkey": "02346b99593357107c9d3459e9deba8d3eaf44e6636c85c7f853eb90ba52e8cd00",
                "aggregate_pubkey": "030b58e337aa4d3852a8c29387c42408d8cfbe3a613a5e397e0a9f01a5fb7107d4",
                "partial_sig": "e1e2e3"
            }],
            "unknown": {"1f": "ccdd"},
            "proprietary": [{"identifier": "bbbb", "subtype": 1, "key": "fc02bbbb", "value": "cc"}]
        }],
        "outputs": [{
            "redeem_script": {"asm": "0 dcba", "hex": "0014dcba", "type": "witness_v0_keyhash"},
            "witness_script": {"asm": "OP_1 03bb OP_1 OP_CHECKMULTISIG", "hex": "512103bb51ae", "type": "multisig"},
            "bip32_derivs": [{
                "pubkey": "03479e36670626632edd1c8610971508d53ca97d4b36550b47179e1a4d281ba60",
                "master_fingerprint": "b5da67b1",
                "path": "m/84h/1h/0h/0/4"
            }],
            "taproot_internal_key": "736e572900fe1252589a2143c8f3c79f71a0412d2353af755e9701c782694a02",
            "taproot_tree": [{
                "depth": 2,
                "leaf_ver": 192,
                "script": "20736e572900fe1252589a2143c8f3c79f71a0412d2353af755e9701c782694a02ac"
            }],
            "taproot_bip32_derivs": [{
                "pubkey": "736e572900fe1252589a2143c8f3c79f71a0412d2353af755e9701c782694a02",
                "master_fingerprint": "b5da67b1",
                "path": "m/86h/1h/0h/1/0",
                "leaf_hashes": []
            }],
            "musig2_participant_pubkeys": [{
                "aggregate_pubkey": "030b58e337aa4d3852a8c29387c42408d8cfbe3a613a5e397e0a9f01a5fb7107d4",
                "participant_pubkeys": ["02346b99593357107c9d3459e9deba8d3eaf44e6636c85c7f853eb90ba52e8cd00"]
            }],
            "unknown": {"2f": "eeff"},
            "proprietary": [{"identifier": "cccc", "subtype": 2, "key": "fc02cccc", "value": "dd"}]
        }],
        "fee": 0.0001
    });
    let p: PsbtDecoded = serde_json::from_value(v).unwrap();

    assert_eq!(p.psbt_version, 0);
    assert_eq!(p.fee, Some(Amount::from_sat(10_000)));
    assert_eq!(p.tx.version, 2);
    assert_eq!(p.global_xpubs.len(), 1);
    assert_eq!(p.global_xpubs[0].master_fingerprint, "b5da67b1");
    assert_eq!(p.global_xpubs[0].path, "m/84h/1h/0h");
    assert_eq!(p.proprietary[0].subtype, 0);
    assert_eq!(p.unknown.get("0f").map(String::as_str), Some("aabb"));

    let i = &p.inputs[0];
    assert_eq!(i.non_witness_utxo.as_ref().unwrap().version, 1);
    let witness_utxo = i.witness_utxo.as_ref().unwrap();
    assert_eq!(witness_utxo.amount, Amount::from_sat(50_000_000));
    assert_eq!(
        witness_utxo.script_pub_key.script_type,
        "witness_v0_keyhash"
    );
    assert_eq!(
        i.partial_signatures
            .as_ref()
            .unwrap()
            .get("02e5a2b3")
            .map(String::as_str),
        Some("304402203c78")
    );
    assert_eq!(i.sighash.as_deref(), Some("ALL"));
    assert_eq!(i.redeem_script.as_ref().unwrap().hex, "0014abcd");
    assert_eq!(i.witness_script.as_ref().unwrap().script_type, "multisig");
    assert_eq!(i.bip32_derivs.as_ref().unwrap()[0].path, "m/84h/1h/0h/0/0");
    assert_eq!(i.final_script_sig.as_ref().unwrap().hex, "47304402203c78");
    assert_eq!(i.final_scriptwitness.as_ref().unwrap().len(), 2);
    // All four preimage maps are distinct fields, so each rename is exercised.
    assert!(
        i.ripemd160_preimages
            .as_ref()
            .unwrap()
            .values()
            .eq(["0011"])
    );
    assert!(i.sha256_preimages.as_ref().unwrap().values().eq(["0022"]));
    assert!(i.hash160_preimages.as_ref().unwrap().values().eq(["0033"]));
    assert!(i.hash256_preimages.as_ref().unwrap().values().eq(["0044"]));
    assert_eq!(i.taproot_key_path_sig.as_deref(), Some("a1b2c3"));
    assert_eq!(
        i.taproot_script_path_sigs.as_ref().unwrap()[0].sig,
        "d1d2d3"
    );
    let script = &i.taproot_scripts.as_ref().unwrap()[0];
    assert_eq!(script.leaf_ver, 192);
    assert_eq!(script.control_blocks, ["c0736e5729"]);
    assert_eq!(
        i.taproot_bip32_derivs.as_ref().unwrap()[0]
            .leaf_hashes
            .len(),
        1
    );
    assert!(i.taproot_internal_key.is_some());
    assert!(i.taproot_merkle_root.is_some());
    assert_eq!(
        i.musig2_participant_pubkeys.as_ref().unwrap()[0]
            .participant_pubkeys
            .len(),
        1
    );
    assert_eq!(
        i.musig2_pubnonces.as_ref().unwrap()[0].pubnonce,
        "02d99e7c87"
    );
    // `leaf_hash` is omitted when signing for the internal key.
    assert_eq!(i.musig2_partial_sigs.as_ref().unwrap()[0].leaf_hash, None);
    assert_eq!(
        i.musig2_partial_sigs.as_ref().unwrap()[0].partial_sig,
        "e1e2e3"
    );
    assert_eq!(
        i.unknown.as_ref().unwrap().get("1f").map(String::as_str),
        Some("ccdd")
    );
    assert_eq!(i.proprietary.as_ref().unwrap()[0].identifier, "bbbb");

    let o = &p.outputs[0];
    assert_eq!(o.redeem_script.as_ref().unwrap().hex, "0014dcba");
    assert_eq!(o.witness_script.as_ref().unwrap().script_type, "multisig");
    assert_eq!(o.bip32_derivs.as_ref().unwrap()[0].path, "m/84h/1h/0h/0/4");
    assert!(o.taproot_internal_key.is_some());
    let leaf = &o.taproot_tree.as_ref().unwrap()[0];
    assert_eq!(leaf.depth, 2);
    assert_eq!(leaf.leaf_ver, 192);
    assert_eq!(
        o.taproot_bip32_derivs.as_ref().unwrap()[0]
            .leaf_hashes
            .len(),
        0
    );
    assert_eq!(o.musig2_participant_pubkeys.as_ref().unwrap().len(), 1);
    assert_eq!(
        o.unknown.as_ref().unwrap().get("2f").map(String::as_str),
        Some("eeff")
    );
    assert_eq!(o.proprietary.as_ref().unwrap()[0].subtype, 2);
}

#[test]
fn psbt_decoded_minimal_and_forward_compatible() {
    // A freshly created PSBT: every per-input and per-output field absent, no
    // fee yet, and a field from a hypothetical future release.
    let v = json!({
        "tx": {
            "txid": "9e1a2d3f4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f6",
            "hash": "9e1a2d3f4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f6",
            "version": 2,
            "size": 82,
            "vsize": 82,
            "weight": 328,
            "locktime": 0,
            "vin": [],
            "vout": []
        },
        "global_xpubs": [],
        "psbt_version": 0,
        "proprietary": [],
        "unknown": {},
        "inputs": [{}],
        "outputs": [{}],
        "some_field_from_a_future_release": "x"
    });
    let p: PsbtDecoded = serde_json::from_value(v).unwrap();
    assert_eq!(p.fee, None);
    assert!(p.global_xpubs.is_empty());
    assert!(p.unknown.is_empty());
    let i = &p.inputs[0];
    assert_eq!(i.witness_utxo, None);
    assert_eq!(i.non_witness_utxo, None);
    assert_eq!(i.partial_signatures, None);
    assert_eq!(i.taproot_scripts, None);
    assert_eq!(i.musig2_pubnonces, None);
    assert_eq!(p.outputs[0].taproot_tree, None);
}

#[test]
fn psbt_decoded_live_segwit_payload() {
    // Captured verbatim from `decodepsbt` on bitcoind v31.1.
    let p: PsbtDecoded = serde_json::from_str(include_str!("data/psbt_segwit.json")).unwrap();
    assert_eq!(p.psbt_version, 0);
    assert_eq!(p.inputs.len(), 1);
    assert_eq!(p.outputs.len(), 2);
    // A global proprietary map with an empty identifier is valid.
    assert_eq!(p.proprietary[0].identifier, "");
    assert_eq!(p.proprietary[0].key, "fc0000");
    assert!(p.inputs[0].non_witness_utxo.is_some());
    assert!(p.inputs[0].proprietary.is_some());
    assert!(p.fee.is_some());
}

#[test]
fn psbt_decoded_live_combined_payload() {
    // Two PSBTs merged by `combinepsbt`, so the inputs carry signatures.
    let p: PsbtDecoded = serde_json::from_str(include_str!("data/psbt_combined.json")).unwrap();
    let i = &p.inputs[0];
    assert!(!i.partial_signatures.as_ref().unwrap().is_empty());
    assert_eq!(i.sighash.as_deref(), Some("ALL"));
    assert!(i.redeem_script.is_some());
    assert!(i.bip32_derivs.is_some());
    assert!(p.outputs[0].bip32_derivs.is_some());
}

#[test]
fn psbt_decoded_live_finalized_payload() {
    // After `finalizepsbt`, the per-input signature data is replaced by the
    // final scriptSig / scriptwitness.
    let p: PsbtDecoded = serde_json::from_str(include_str!("data/psbt_finalized.json")).unwrap();
    assert!(p.inputs[0].final_script_sig.is_some());
    assert_eq!(p.inputs[0].partial_signatures, None);
    assert!(p.inputs.iter().any(|i| i.final_scriptwitness.is_some()));
}

#[test]
fn psbt_decoded_live_taproot_tree_payload() {
    let p: PsbtDecoded = serde_json::from_str(include_str!("data/psbt_taproot_tree.json")).unwrap();
    assert!(p.inputs[0].taproot_internal_key.is_some());
    assert!(p.inputs[0].taproot_bip32_derivs.is_some());
    let tree = p.outputs[0].taproot_tree.as_ref().unwrap();
    assert_eq!(tree[0].depth, 2);
    assert_eq!(tree[0].leaf_ver, 192);
    assert!(!tree[0].script.is_empty());
}

#[test]
fn psbt_decoded_live_musig2_payload() {
    // MuSig2 (BIP 373) fields, new in Core v31.
    let p: PsbtDecoded = serde_json::from_str(include_str!("data/psbt_musig2.json")).unwrap();
    let i = &p.inputs[0];
    assert!(!i.musig2_participant_pubkeys.as_ref().unwrap().is_empty());
    let nonce = &i.musig2_pubnonces.as_ref().unwrap()[0];
    assert!(nonce.leaf_hash.is_some());
    assert!(!nonce.pubnonce.is_empty());
    assert!(!i.musig2_partial_sigs.as_ref().unwrap().is_empty());
    assert_eq!(i.taproot_scripts.as_ref().unwrap()[0].leaf_ver, 192);
    assert!(i.taproot_merkle_root.is_some());
    assert!(i.witness_utxo.is_some());
}

// ---------------------------------------------------------------------------
// Bitcoin Core v29 wire shapes. Key sets captured from a live v29.0 mainnet
// node; every field Core added in v30 or v31 is absent and must come back
// `None`, and fields v29 still emitted but later releases dropped
// (`fullrbf`, `startingheight`) must be tolerated as unknown extras.
// ---------------------------------------------------------------------------

#[test]
fn v29_block_has_no_coinbase_tx() {
    let v = json!({
        "hash": "0000000000000000000077daa3b846a63cf32ddde28261fc8ba5e612b7fa1f9e",
        "confirmations": 1, "size": 1556897, "strippedsize": 774889, "weight": 3881564,
        "height": 967118, "version": 536870912, "versionHex": "20000000",
        "merkleroot": "2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866",
        "tx": ["2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866"],
        "time": 1757950000, "mediantime": 1757946000, "nonce": 2573394689_u64,
        "bits": "17023a04", "target": "00000000000000000002a7c4000000000000000000000000000000000000000",
        "difficulty": 1.274507897158431e14, "chainwork": "6500650065", "nTx": 1,
        "previousblockhash": "000000007bc154e0fa7ea32218a72fe2c1bb9f86cf8c9ebf9a715ed27fdb229a"
    });
    let block: Block = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(block.coinbase_tx, None);
    assert_eq!(block.height, 967118);

    let mut v2 = v;
    v2["tx"] = json!([{
        "txid": "2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866",
        "hash": "2d05f0c9c3e1c226e63b5fac240137687544cf631cd616fd34fd188fc9020866",
        "version": 1, "size": 204, "vsize": 204, "weight": 816, "locktime": 0,
        "vin": [{"coinbase": "04ffff001d0102", "sequence": 4294967295_u64}],
        "vout": [{"value": 3.125, "n": 0, "scriptPubKey": {
            "asm": "0 abcdef01", "desc": "addr(bc1q...)#x", "hex": "0014abcdef01",
            "address": "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4", "type": "witness_v0_keyhash"}}],
        "hex": "0100000001", "fee": 0.0
    }]);
    let block: BlockWithTxs = serde_json::from_value(v2).unwrap();
    assert_eq!(block.coinbase_tx, None);
    assert_eq!(block.tx[0].tx.vout[0].value, Amount::from_sat(312_500_000));
}

#[test]
fn v29_deployment_info_has_no_script_flags() {
    let v = json!({
        "hash": "0000000000000000000077daa3b846a63cf32ddde28261fc8ba5e612b7fa1f9e",
        "height": 967118,
        "deployments": {
            "bip34": {"type": "buried", "active": true, "height": 227931},
            "segwit": {"type": "buried", "active": true, "height": 481824},
            "taproot": {"type": "bip9", "active": true, "height": 709632, "bip9": {
                "start_time": 1619222400, "timeout": 1628640000, "min_activation_height": 709632,
                "status": "active", "since": 709632, "status_next": "active"}}
        }
    });
    let info: DeploymentInfo = serde_json::from_value(v).unwrap();
    assert_eq!(info.script_flags, None);
    assert_eq!(info.deployments.len(), 3);
    assert!(
        info.deployments["taproot"]
            .bip9
            .as_ref()
            .unwrap()
            .statistics
            .is_none()
    );
}

#[test]
fn v29_mempool_info_lacks_the_v30_and_v31_fields_and_still_has_fullrbf() {
    let v = json!({
        "loaded": true, "size": 35953, "bytes": 21694447, "usage": 130469424,
        "total_fee": 0.30655184, "maxmempool": 300000000_u64,
        "mempoolminfee": 0.00001000, "minrelaytxfee": 0.00000100,
        "incrementalrelayfee": 0.00001000, "unbroadcastcount": 0,
        "fullrbf": true
    });
    let info: MempoolInfo = serde_json::from_value(v).unwrap();
    assert_eq!(info.size, 35953);
    assert_eq!(info.total_fee, Amount::from_sat(30_655_184));
    assert_eq!(info.min_relay_tx_fee, FeeRate::from_sat_per_kvb(100));
    assert_eq!(info.permit_bare_multisig, None);
    assert_eq!(info.max_datacarrier_size, None);
    assert_eq!(info.limit_cluster_count, None);
    assert_eq!(info.limit_cluster_size, None);
    assert_eq!(info.optimal, None);
}

#[test]
fn v29_mempool_entry_has_no_chunk_fields() {
    let v = json!({
        "vsize": 141, "weight": 561, "time": 1757950123, "height": 967118,
        "descendantcount": 1, "descendantsize": 141,
        "ancestorcount": 1, "ancestorsize": 141,
        "wtxid": "0b81d33652f7dcc3cabd44f62c0fb5a9578a6c3a2d2cd6661bf3ef6aec59cba5",
        "fees": {"base": 0.00000141, "modified": 0.00000141, "ancestor": 0.00000141, "descendant": 0.00000141},
        "depends": [], "spentby": [], "bip125-replaceable": true, "unbroadcast": false
    });
    let entry: MempoolEntry = serde_json::from_value(v).unwrap();
    assert_eq!(entry.chunk_weight, None);
    assert_eq!(entry.fees.chunk, None);
    assert_eq!(entry.fees.base, Amount::from_sat(141));
}

#[test]
fn v29_peer_info_has_no_inventory_counters_and_still_has_startingheight() {
    let v = json!({
        "id": 3, "addr": "203.0.113.7:8333", "addrbind": "10.0.0.1:41234", "network": "ipv4",
        "services": "0000000000000c09", "servicesnames": ["NETWORK", "WITNESS", "NETWORK_LIMITED", "P2P_V2"],
        "relaytxes": true, "lastsend": 1757950100, "lastrecv": 1757950099,
        "last_transaction": 1757950000, "last_block": 1757949000,
        "bytessent": 123456, "bytesrecv": 654321, "conntime": 1757900000, "timeoffset": 0,
        "pingtime": 0.05, "minping": 0.04, "version": 70016, "subver": "/Satoshi:29.0.0/",
        "inbound": false, "bip152_hb_to": false, "bip152_hb_from": false,
        "startingheight": 967000, "presynced_headers": -1,
        "synced_headers": 967118, "synced_blocks": 967118, "inflight": [],
        "addr_relay_enabled": true, "addr_processed": 100, "addr_rate_limited": 0,
        "permissions": [], "minfeefilter": 0.00001000,
        "bytessent_per_msg": {"ping": 32}, "bytesrecv_per_msg": {"pong": 32},
        "connection_type": "outbound-full-relay",
        "transport_protocol_type": "v2", "session_id": "abcd1234"
    });
    let peer: PeerInfo = serde_json::from_value(v).unwrap();
    assert_eq!(peer.last_inv_sequence, None);
    assert_eq!(peer.inv_to_send, None);
    assert_eq!(peer.sub_ver, "/Satoshi:29.0.0/");
}

#[test]
fn v29_mining_info_has_no_blockmintxfee() {
    let v = json!({
        "blocks": 967118, "currentblockweight": 3999000, "currentblocktx": 4000,
        "bits": "17023a04", "difficulty": 1.274507897158431e14,
        "target": "00000000000000000002a7c4000000000000000000000000000000000000000",
        "networkhashps": 1.011699535963271e21, "pooledtx": 35953, "chain": "main",
        "next": {"height": 967119, "bits": "17023a04", "difficulty": 1.274507897158431e14,
                 "target": "00000000000000000002a7c4000000000000000000000000000000000000000"},
        "warnings": []
    });
    let info: MiningInfo = serde_json::from_value(v).unwrap();
    assert_eq!(info.block_min_tx_fee, None);
    assert_eq!(info.next.height, 967119);
}
