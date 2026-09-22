//! Typed RPC method traits for the async client.
//!
//! Each trait is blanket-implemented for every [`RpcCallAsync`], so bringing one
//! into scope adds its methods to any client — including `&dyn RpcCallAsync`.
//!
//! Method names, parameters and result types mirror [`crate::sync`] exactly;
//! only the return-type wrapper differs.

use std::collections::BTreeMap;
use std::future::Future;

use serde_json::json;

use super::call::{RpcCallAsync, RpcCallAsyncExt};
use crate::Result;
use crate::params::positional;
use crate::types::{
    AddressValidation, Amount, Block, BlockHashAndHeight, BlockHeader, BlockTemplate,
    BlockTemplateRequest, BlockWithTxs, BlockchainInfo, ChainTip, CreateRawTransactionInput,
    CreateRawTransactionOutput, DeploymentInfo, DerivedAddresses, DescriptorRange,
    DescriptorRequest, FeeEstimate, FeeRate, IndexInfo, MempoolEntry, MempoolInfo, MiningInfo,
    NetTotals, NetworkInfo, PeerInfo, PsbtAnalysis, PsbtDecoded, PsbtFinalization,
    PsbtProcessResult, RawMempoolSequence, RpcInfo, SighashType, TestMempoolAcceptResult,
    Transaction, TxOut,
};

/// Blockchain RPCs.
pub trait BlockchainRpc: RpcCallAsync {
    /// Returns an object containing various state info regarding blockchain
    /// processing.
    fn get_blockchain_info(&self) -> impl Future<Output = Result<BlockchainInfo>> + Send + '_ {
        self.call("getblockchaininfo", positional(vec![]))
    }

    /// Returns the hash of the best (tip) block in the most-work
    /// fully-validated chain.
    fn get_best_block_hash(&self) -> impl Future<Output = Result<String>> + Send + '_ {
        self.call("getbestblockhash", positional(vec![]))
    }

    /// Returns the height of the most-work fully-validated chain. The genesis
    /// block has height 0.
    fn get_block_count(&self) -> impl Future<Output = Result<u64>> + Send + '_ {
        self.call("getblockcount", positional(vec![]))
    }

    /// Returns the hash of the block in the best-block-chain at `height`.
    fn get_block_hash(&self, height: u32) -> impl Future<Output = Result<String>> + Send + '_ {
        self.call("getblockhash", positional(vec![json!(height)]))
    }

    /// Returns the hashes of the blocks at `heights`, in one batched round
    /// trip; the result lines up with `heights` index for index.
    ///
    /// Any height the node rejects (past the tip, say) fails the whole call
    /// with that `Error::Rpc`.
    fn get_block_hashes(
        &self,
        heights: &[u32],
    ) -> impl Future<Output = Result<Vec<String>>> + Send + '_ {
        let params = heights.iter().map(|h| positional(vec![json!(h)])).collect();
        self.call_batch("getblockhash", params)
    }

    /// Returns the serialized, hex-encoded data for the block `hash`.
    fn get_block_hex(&self, hash: &str) -> impl Future<Output = Result<String>> + Send + '_ {
        self.call("getblock", positional(vec![json!(hash), json!(0)]))
    }

    /// Returns information about the block `hash`, with transaction ids only.
    fn get_block(&self, hash: &str) -> impl Future<Output = Result<Block>> + Send + '_ {
        self.call("getblock", positional(vec![json!(hash), json!(1)]))
    }

    /// Returns information about the block `hash` and, for each of its
    /// transactions, the full body `getrawtransaction` would return plus the
    /// transaction's fee when the block's undo data is available.
    fn get_block_with_txs(
        &self,
        hash: &str,
    ) -> impl Future<Output = Result<BlockWithTxs>> + Send + '_ {
        self.call("getblock", positional(vec![json!(hash), json!(2)]))
    }

    /// Returns information about the block header of block `hash`.
    fn get_block_header(
        &self,
        hash: &str,
    ) -> impl Future<Output = Result<BlockHeader>> + Send + '_ {
        self.call("getblockheader", positional(vec![json!(hash), json!(true)]))
    }

    /// Returns the headers of the blocks `hashes`, in one batched round trip;
    /// the result lines up with `hashes` index for index.
    ///
    /// Any hash the node does not know fails the whole call with that
    /// `Error::Rpc`.
    fn get_block_headers(
        &self,
        hashes: &[&str],
    ) -> impl Future<Output = Result<Vec<BlockHeader>>> + Send + '_ {
        let params = hashes
            .iter()
            .map(|h| positional(vec![json!(h), json!(true)]))
            .collect();
        self.call_batch("getblockheader", params)
    }

    /// Returns the serialized, hex-encoded data for the block header of block
    /// `hash`.
    fn get_block_header_hex(&self, hash: &str) -> impl Future<Output = Result<String>> + Send + '_ {
        self.call(
            "getblockheader",
            positional(vec![json!(hash), json!(false)]),
        )
    }

    /// Returns information about all known tips in the block tree, including
    /// the main chain as well as orphaned branches.
    fn get_chain_tips(&self) -> impl Future<Output = Result<Vec<ChainTip>>> + Send + '_ {
        self.call("getchaintips", positional(vec![]))
    }

    /// Returns the proof-of-work difficulty as a multiple of the minimum
    /// difficulty.
    fn get_difficulty(&self) -> impl Future<Output = Result<f64>> + Send + '_ {
        self.call("getdifficulty", positional(vec![]))
    }

    /// Returns an object containing various state info regarding deployments of
    /// consensus changes, at `hash` or at the current chain tip.
    fn get_deployment_info(
        &self,
        hash: Option<&str>,
    ) -> impl Future<Output = Result<DeploymentInfo>> + Send + '_ {
        self.call("getdeploymentinfo", positional(vec![json!(hash)]))
    }

    /// Returns details about the unspent transaction output `n` of `txid`, or
    /// `None` if it is not in the UTXO set.
    ///
    /// `include_mempool` defaults to `true` on the node; note that an output
    /// spent in the mempool then does not appear.
    fn get_tx_out(
        &self,
        txid: &str,
        n: u32,
        include_mempool: Option<bool>,
    ) -> impl Future<Output = Result<Option<TxOut>>> + Send + '_ {
        self.call(
            "gettxout",
            positional(vec![json!(txid), json!(n), json!(include_mempool)]),
        )
    }

    /// Waits for any new block and returns its hash and height.
    ///
    /// `timeout_ms` of `None` or `0` means no timeout at the RPC level, but
    /// the client's own read timeout (`ClientBuilder::read_timeout`, 60
    /// seconds by default) still applies and aborts the wait with
    /// `Error::Transport` unless the client was built with
    /// `.read_timeout(None)`.
    /// `current_tip` makes the node wait for the chain tip to differ from
    /// that hash, which is more reliable than letting it sample the tip
    /// itself.
    fn wait_for_new_block(
        &self,
        timeout_ms: Option<u64>,
        current_tip: Option<&str>,
    ) -> impl Future<Output = Result<BlockHashAndHeight>> + Send + '_ {
        self.call(
            "waitfornewblock",
            positional(vec![json!(timeout_ms), json!(current_tip)]),
        )
    }

    /// Waits for the chain to reach at least `height` and returns the hash and
    /// height of the current tip.
    ///
    /// `timeout_ms` of `None` or `0` means no timeout at the RPC level, but
    /// the client's own read timeout (`ClientBuilder::read_timeout`, 60
    /// seconds by default) still applies and aborts the wait with
    /// `Error::Transport` unless the client was built with
    /// `.read_timeout(None)`.
    fn wait_for_block_height(
        &self,
        height: u32,
        timeout_ms: Option<u64>,
    ) -> impl Future<Output = Result<BlockHashAndHeight>> + Send + '_ {
        self.call(
            "waitforblockheight",
            positional(vec![json!(height), json!(timeout_ms)]),
        )
    }
}

impl<T: RpcCallAsync + ?Sized> BlockchainRpc for T {}

/// Mempool RPCs.
pub trait MempoolRpc: RpcCallAsync {
    /// Returns details on the active state of the TX memory pool.
    fn get_mempool_info(&self) -> impl Future<Output = Result<MempoolInfo>> + Send + '_ {
        self.call("getmempoolinfo", positional(vec![]))
    }

    /// Returns all transaction ids in the mempool as a list of transaction
    /// ids.
    fn get_raw_mempool(&self) -> impl Future<Output = Result<Vec<String>>> + Send + '_ {
        self.call("getrawmempool", positional(vec![json!(false)]))
    }

    /// Returns all transactions in the mempool, keyed by transaction id, with
    /// their full mempool entry data.
    fn get_raw_mempool_verbose(
        &self,
    ) -> impl Future<Output = Result<BTreeMap<String, MempoolEntry>>> + Send + '_ {
        self.call("getrawmempool", positional(vec![json!(true)]))
    }

    /// Returns the transaction ids in the mempool together with the mempool
    /// sequence number, as of the moment the list was generated.
    fn get_raw_mempool_with_sequence(
        &self,
    ) -> impl Future<Output = Result<RawMempoolSequence>> + Send + '_ {
        self.call("getrawmempool", positional(vec![json!(false), json!(true)]))
    }

    /// Returns mempool data for the given transaction `txid`.
    fn get_mempool_entry(
        &self,
        txid: &str,
    ) -> impl Future<Output = Result<MempoolEntry>> + Send + '_ {
        self.call("getmempoolentry", positional(vec![json!(txid)]))
    }
}

impl<T: RpcCallAsync + ?Sized> MempoolRpc for T {}

/// Network RPCs.
pub trait NetworkRpc: RpcCallAsync {
    /// Returns an object containing various state info regarding P2P
    /// networking.
    fn get_network_info(&self) -> impl Future<Output = Result<NetworkInfo>> + Send + '_ {
        self.call("getnetworkinfo", positional(vec![]))
    }

    /// Returns data about each connected network peer as a json array of
    /// objects.
    fn get_peer_info(&self) -> impl Future<Output = Result<Vec<PeerInfo>>> + Send + '_ {
        self.call("getpeerinfo", positional(vec![]))
    }

    /// Returns the number of connections to other nodes.
    fn get_connection_count(&self) -> impl Future<Output = Result<u64>> + Send + '_ {
        self.call("getconnectioncount", positional(vec![]))
    }

    /// Returns information about network traffic, including bytes in, bytes
    /// out, and current system time.
    fn get_net_totals(&self) -> impl Future<Output = Result<NetTotals>> + Send + '_ {
        self.call("getnettotals", positional(vec![]))
    }

    /// Attempts to add or remove `node` from the addnode list, or try a
    /// connection to it once.
    ///
    /// `command` is one of `"add"`, `"remove"` or `"onetry"`.
    fn add_node(
        &self,
        node: &str,
        command: &str,
        v2transport: Option<bool>,
    ) -> impl Future<Output = Result<()>> + Send + '_ {
        self.call(
            "addnode",
            positional(vec![json!(node), json!(command), json!(v2transport)]),
        )
    }

    /// Immediately disconnects from the specified peer node.
    ///
    /// Strictly one of `address` and `node_id` can be provided to identify
    /// the node.
    fn disconnect_node(
        &self,
        address: Option<&str>,
        node_id: Option<u64>,
    ) -> impl Future<Output = Result<()>> + Send + '_ {
        self.call(
            "disconnectnode",
            positional(vec![json!(address), json!(node_id)]),
        )
    }
}

impl<T: RpcCallAsync + ?Sized> NetworkRpc for T {}

/// Mining RPCs.
pub trait MiningRpc: RpcCallAsync {
    /// Returns a json object containing mining-related information.
    fn get_mining_info(&self) -> impl Future<Output = Result<MiningInfo>> + Send + '_ {
        self.call("getmininginfo", positional(vec![]))
    }

    /// Returns data needed to construct a block to work on.
    ///
    /// `request` is sent as the single `template_request` object argument; see
    /// BIPs 22, 23, 9 and 145 for the full specification. Only the default
    /// `"template"` mode is modelled — `"proposal"` mode returns a different
    /// result shape. Supplying `request.longpoll_id` makes the node hold the
    /// response until a new template is available, which can take a while;
    /// the client's own read timeout (`ClientBuilder::read_timeout`, 60
    /// seconds by default) still applies and aborts the wait with
    /// `Error::Transport` unless the client was built with
    /// `.read_timeout(None)`.
    fn get_block_template(
        &self,
        request: &BlockTemplateRequest,
    ) -> impl Future<Output = Result<BlockTemplate>> + Send + '_ {
        // This fn returns `impl Future<..>`, not a `Try` type, so `?` can't be
        // used directly on it; encode here and carry the Result into the
        // block, where the enclosing type is a real async block and `?` works.
        let params = serde_json::to_value(request).map(|v| positional(vec![v]));
        async move { self.call("getblocktemplate", params?).await }
    }

    /// Attempts to submit new block `hex` to the network.
    ///
    /// Returns `None` if the block was accepted, or a rejection-reason string
    /// otherwise, per BIP 22.
    fn submit_block(&self, hex: &str) -> impl Future<Output = Result<Option<String>>> + Send + '_ {
        self.call("submitblock", positional(vec![json!(hex)]))
    }

    /// Decodes the given `hex` as a header and submits it as a candidate chain
    /// tip if valid. Throws when the header is invalid.
    fn submit_header(&self, hex: &str) -> impl Future<Output = Result<()>> + Send + '_ {
        self.call("submitheader", positional(vec![json!(hex)]))
    }

    /// Returns the estimated network hashes per second based on the last
    /// `nblocks` blocks, or since the last difficulty change if `-1`.
    ///
    /// `height` estimates the network speed at the time a certain block was
    /// found instead of using the current tip.
    fn get_network_hash_ps(
        &self,
        nblocks: Option<i64>,
        height: Option<i64>,
    ) -> impl Future<Output = Result<f64>> + Send + '_ {
        self.call(
            "getnetworkhashps",
            positional(vec![json!(nblocks), json!(height)]),
        )
    }
}

impl<T: RpcCallAsync + ?Sized> MiningRpc for T {}

/// Raw transaction RPCs.
pub trait RawTransactionsRpc: RpcCallAsync {
    /// Returns the serialized, hex-encoded data for `txid`.
    ///
    /// By default the node only looks in the mempool; `-txindex` extends the
    /// search to every block, and `block_hash` restricts it to one block.
    fn get_raw_transaction_hex(
        &self,
        txid: &str,
        block_hash: Option<&str>,
    ) -> impl Future<Output = Result<String>> + Send + '_ {
        self.call(
            "getrawtransaction",
            positional(vec![json!(txid), json!(0), json!(block_hash)]),
        )
    }

    /// Returns information about the transaction `txid`.
    ///
    /// By default the node only looks in the mempool; `-txindex` extends the
    /// search to every block, and `block_hash` restricts it to one block.
    /// Verbosity 1 is requested, so the result carries no `prevout` detail.
    fn get_raw_transaction(
        &self,
        txid: &str,
        block_hash: Option<&str>,
    ) -> impl Future<Output = Result<Transaction>> + Send + '_ {
        self.call(
            "getrawtransaction",
            positional(vec![json!(txid), json!(1), json!(block_hash)]),
        )
    }

    /// Submits the raw transaction `hex` to the network and returns its hash.
    ///
    /// `max_fee_rate` rejects the transaction if its fee rate is higher;
    /// `FeeRate::ZERO` accepts any fee rate. `max_burn_amount` rejects it if
    /// it has provably unspendable outputs worth more than that. Both are
    /// sent as the exact BTC decimals Core expects.
    fn send_raw_transaction(
        &self,
        hex: &str,
        max_fee_rate: Option<FeeRate>,
        max_burn_amount: Option<Amount>,
    ) -> impl Future<Output = Result<String>> + Send + '_ {
        self.call(
            "sendrawtransaction",
            positional(vec![
                json!(hex),
                json!(max_fee_rate),
                json!(max_burn_amount),
            ]),
        )
    }

    /// Creates an unsigned transaction spending `inputs` and creating
    /// `outputs`, and returns it hex-encoded.
    ///
    /// The transaction is neither signed, nor stored in a wallet, nor
    /// transmitted to the network. `replaceable` marks it BIP 125-replaceable
    /// and defaults to `true` on the node; `version` defaults to
    /// `CTransaction::CURRENT_VERSION` and must be within the standard range
    /// the node accepts.
    fn create_raw_transaction(
        &self,
        inputs: &[CreateRawTransactionInput],
        outputs: &[CreateRawTransactionOutput],
        locktime: Option<u32>,
        replaceable: Option<bool>,
        version: Option<u32>,
    ) -> impl Future<Output = Result<String>> + Send + '_ {
        // This fn returns `impl Future<..>`, not a `Try` type, so `?` can't be
        // used directly on it; encode here and carry the Result into the
        // block, where the enclosing type is a real async block and `?` works.
        let params = serde_json::to_value(inputs).and_then(|inputs| {
            serde_json::to_value(outputs).map(|outputs| {
                positional(vec![
                    inputs,
                    outputs,
                    json!(locktime),
                    json!(replaceable),
                    json!(version),
                ])
            })
        });
        async move { self.call("createrawtransaction", params?).await }
    }

    /// Decodes the serialized, hex-encoded transaction `hex`.
    ///
    /// `iswitness` says whether `hex` is a witness serialization; omitting it
    /// lets the node decide heuristically.
    fn decode_raw_transaction(
        &self,
        hex: &str,
        iswitness: Option<bool>,
    ) -> impl Future<Output = Result<Transaction>> + Send + '_ {
        self.call(
            "decoderawtransaction",
            positional(vec![json!(hex), json!(iswitness)]),
        )
    }

    /// Returns whether each of `raw_txs` would be accepted by the mempool,
    /// in the order they were given.
    ///
    /// More than one transaction is tested as a package, so parents must come
    /// before children. `max_fee_rate` rejects a transaction whose fee rate is
    /// higher.
    fn test_mempool_accept(
        &self,
        raw_txs: &[String],
        max_fee_rate: Option<FeeRate>,
    ) -> impl Future<Output = Result<Vec<TestMempoolAcceptResult>>> + Send + '_ {
        self.call(
            "testmempoolaccept",
            positional(vec![json!(raw_txs), json!(max_fee_rate)]),
        )
    }

    /// Creates a PSBT spending `inputs` and creating `outputs`, and returns it
    /// base64-encoded.
    ///
    /// Takes the same arguments, with the same defaults, as
    /// [`create_raw_transaction`](Self::create_raw_transaction); the result is
    /// the same transaction in PSBT form. It is neither signed, nor stored in a
    /// wallet, nor transmitted to the network.
    fn create_psbt(
        &self,
        inputs: &[CreateRawTransactionInput],
        outputs: &[CreateRawTransactionOutput],
        locktime: Option<u32>,
        replaceable: Option<bool>,
        version: Option<u32>,
    ) -> impl Future<Output = Result<String>> + Send + '_ {
        // See `create_raw_transaction` for why the params are encoded up front.
        let params = serde_json::to_value(inputs).and_then(|inputs| {
            serde_json::to_value(outputs).map(|outputs| {
                positional(vec![
                    inputs,
                    outputs,
                    json!(locktime),
                    json!(replaceable),
                    json!(version),
                ])
            })
        });
        async move { self.call("createpsbt", params?).await }
    }

    /// Decodes the base64-encoded `psbt` into its every field.
    ///
    /// Every per-input and per-output field is optional: a freshly created PSBT
    /// carries none of them, and each role along the way fills in what it can.
    /// [`analyze_psbt`](Self::analyze_psbt) answers "what does this PSBT still
    /// need" without walking the whole structure.
    fn decode_psbt(&self, psbt: &str) -> impl Future<Output = Result<PsbtDecoded>> + Send + '_ {
        self.call("decodepsbt", positional(vec![json!(psbt)]))
    }

    /// Analyzes `psbt` and reports what each input still needs, plus the fee
    /// and size the finished transaction is estimated to have.
    ///
    /// Reports the next role the PSBT must pass through rather than failing, so
    /// a caller can drive an unsigned PSBT forward without inspecting it
    /// field by field.
    fn analyze_psbt(&self, psbt: &str) -> impl Future<Output = Result<PsbtAnalysis>> + Send + '_ {
        self.call("analyzepsbt", positional(vec![json!(psbt)]))
    }

    /// Merges `psbts`, which must all be for the same transaction, into one.
    ///
    /// This is the Combiner role: it unions the signatures and metadata the
    /// inputs carry. Core errors if the PSBTs describe different transactions.
    fn combine_psbt(&self, psbts: &[String]) -> impl Future<Output = Result<String>> + Send + '_ {
        self.call("combinepsbt", positional(vec![json!(psbts)]))
    }

    /// Joins `psbts`, which must be for *distinct* transactions, into one
    /// transaction with all of their inputs and outputs.
    ///
    /// Unlike [`combine_psbt`](Self::combine_psbt), the inputs are not required
    /// to match: Core errors if the same input appears in more than one of them.
    /// No input or output ordering is guaranteed.
    fn join_psbts(&self, psbts: &[String]) -> impl Future<Output = Result<String>> + Send + '_ {
        self.call("joinpsbts", positional(vec![json!(psbts)]))
    }

    /// Converts the serialized, hex-encoded transaction `hex` into a PSBT.
    ///
    /// `permit_sig_data` allows the conversion to proceed by discarding any
    /// scriptSigs and witnesses `hex` carries, and defaults to `false` on the
    /// node — so by default a signed transaction is rejected rather than
    /// silently stripped. Omitting `iswitness` lets the node decide
    /// heuristically whether `hex` is a witness serialization.
    fn convert_to_psbt(
        &self,
        hex: &str,
        permit_sig_data: Option<bool>,
        iswitness: Option<bool>,
    ) -> impl Future<Output = Result<String>> + Send + '_ {
        self.call(
            "converttopsbt",
            positional(vec![json!(hex), json!(permit_sig_data), json!(iswitness)]),
        )
    }

    /// Fills in the UTXO of every segwit input of `psbt` from the UTXO set or
    /// the mempool, and returns the updated PSBT.
    ///
    /// `descriptors` additionally supplies the redeem scripts, witness scripts
    /// and BIP 32 derivation paths those inputs need. A ranged descriptor given
    /// as [`DescriptorRequest::Plain`] is expanded to the node's default of
    /// 1000 indices.
    fn utxo_update_psbt(
        &self,
        psbt: &str,
        descriptors: Option<&[DescriptorRequest]>,
    ) -> impl Future<Output = Result<String>> + Send + '_ {
        let params = serde_json::to_value(descriptors)
            .map(|descriptors| positional(vec![json!(psbt), descriptors]));
        async move { self.call("utxoupdatepsbt", params?).await }
    }

    /// Finalizes every input of `psbt` that has a complete set of signatures.
    ///
    /// `extract` defaults to `true` on the node, which returns the extracted
    /// network transaction as
    /// [`PsbtFinalization::hex`] — but only if *every* input finalized.
    /// Otherwise, and whenever `extract` is `false`, the PSBT comes back as
    /// [`PsbtFinalization::psbt`].
    fn finalize_psbt(
        &self,
        psbt: &str,
        extract: Option<bool>,
    ) -> impl Future<Output = Result<PsbtFinalization>> + Send + '_ {
        self.call(
            "finalizepsbt",
            positional(vec![json!(psbt), json!(extract)]),
        )
    }

    /// Updates every segwit input of `psbt` from `descriptors`, the UTXO set
    /// and the mempool, then signs whatever those descriptors can sign.
    ///
    /// Needs no wallet. A watch-only descriptor carries no private key, so it
    /// updates but cannot sign, and the result's `complete` stays `false` —
    /// which is the expected outcome, not an error. `sighash_type` applies only
    /// to inputs the PSBT does not already pin one for. `bip32_derivs`
    /// defaults to `true` on the node, `finalize` to `true`; finalizing an
    /// input a watch-only descriptor could not sign fails, so pass
    /// `Some(false)` when signing is expected to happen elsewhere.
    fn descriptor_process_psbt(
        &self,
        psbt: &str,
        descriptors: &[DescriptorRequest],
        sighash_type: Option<SighashType>,
        bip32_derivs: Option<bool>,
        finalize: Option<bool>,
    ) -> impl Future<Output = Result<PsbtProcessResult>> + Send + '_ {
        let params = serde_json::to_value(descriptors).map(|descriptors| {
            positional(vec![
                json!(psbt),
                descriptors,
                json!(sighash_type),
                json!(bip32_derivs),
                json!(finalize),
            ])
        });
        async move { self.call("descriptorprocesspsbt", params?).await }
    }
}

impl<T: RpcCallAsync + ?Sized> RawTransactionsRpc for T {}

/// Fee estimation RPCs.
pub trait FeeRpc: RpcCallAsync {
    /// Estimates the approximate fee per kilobyte needed for a transaction to
    /// begin confirmation within `conf_target` blocks if possible, and
    /// returns the number of blocks for which the estimate is valid.
    ///
    /// Uses virtual transaction size as defined in BIP 141 (witness data is
    /// discounted). `estimate_mode` defaults to `"economical"` on the node.
    fn estimate_smart_fee(
        &self,
        conf_target: u32,
        estimate_mode: Option<&str>,
    ) -> impl Future<Output = Result<FeeEstimate>> + Send + '_ {
        self.call(
            "estimatesmartfee",
            positional(vec![json!(conf_target), json!(estimate_mode)]),
        )
    }
}

impl<T: RpcCallAsync + ?Sized> FeeRpc for T {}

/// Control RPCs.
pub trait ControlRpc: RpcCallAsync {
    /// Returns the total uptime of the server, in seconds.
    fn uptime(&self) -> impl Future<Output = Result<u64>> + Send + '_ {
        self.call("uptime", positional(vec![]))
    }

    /// Requests a graceful shutdown of the node.
    fn stop(&self) -> impl Future<Output = Result<String>> + Send + '_ {
        self.call("stop", positional(vec![]))
    }

    /// Lists all commands, or gets help for `command` if given.
    ///
    /// The node also documents a `Type::ANY` result branch for internal,
    /// undocumented sub-commands (e.g. `dump_all_command_conversions`); the
    /// plain-string branch modelled here is what every public command
    /// returns.
    fn help(&self, command: Option<&str>) -> impl Future<Output = Result<String>> + Send + '_ {
        self.call("help", positional(vec![json!(command)]))
    }

    /// Returns details of the RPC server.
    fn get_rpc_info(&self) -> impl Future<Output = Result<RpcInfo>> + Send + '_ {
        self.call("getrpcinfo", positional(vec![]))
    }
}

impl<T: RpcCallAsync + ?Sized> ControlRpc for T {}

/// Utility RPCs.
pub trait UtilRpc: RpcCallAsync {
    /// Returns information about the given bitcoin `address`.
    fn validate_address(
        &self,
        address: &str,
    ) -> impl Future<Output = Result<AddressValidation>> + Send + '_ {
        self.call("validateaddress", positional(vec![json!(address)]))
    }

    /// Returns the status of one or all available indices currently running
    /// in the node, keyed by index name.
    fn get_index_info(
        &self,
        index_name: Option<&str>,
    ) -> impl Future<Output = Result<BTreeMap<String, IndexInfo>>> + Send + '_ {
        self.call("getindexinfo", positional(vec![json!(index_name)]))
    }

    /// Derives one or more addresses from the output `descriptor`.
    ///
    /// `range` is required for a ranged descriptor and rejected for any other.
    /// A multipath descriptor (BIP 389) returns
    /// [`DerivedAddresses::Multipath`], one address list per expansion.
    fn derive_addresses(
        &self,
        descriptor: &str,
        range: Option<DescriptorRange>,
    ) -> impl Future<Output = Result<DerivedAddresses>> + Send + '_ {
        self.call(
            "deriveaddresses",
            positional(vec![json!(descriptor), json!(range)]),
        )
    }
}

impl<T: RpcCallAsync + ?Sized> UtilRpc for T {}
