//! JSON-RPC 2.0 request and response envelopes.
//!
//! v31.1 answers 2.0 requests with HTTP 200 even for RPC errors, putting the
//! error in the `error` member, so callers have a single error path.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::{Error, Result, RpcError};

#[derive(Debug, Serialize)]
pub(crate) struct Request<'a> {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: &'a str,
    pub params: &'a Value,
}

impl<'a> Request<'a> {
    pub(crate) fn new(id: u64, method: &'a str, params: &'a Value) -> Self {
        Request {
            jsonrpc: "2.0",
            id,
            method,
            params,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Response {
    // `deserialize_with` (not plain `Option<Value>`) is needed because serde
    // collapses an explicit JSON `null` and an absent field to the same
    // `None`, and `into_result` must tell them apart.
    #[serde(default, deserialize_with = "some_if_present")]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<ErrorBody>,
    // Same trick: an explicit `null` id (what JSON-RPC 2.0 mandates when the
    // server could not read the request's id) must stay distinguishable from
    // a reply that omits `id` altogether.
    #[serde(default, deserialize_with = "some_if_present")]
    pub id: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ErrorBody {
    pub code: i32,
    pub message: String,
}

// Wraps a present field's value in `Some`, even when that value is `null`,
// so `#[serde(default)]` can supply `None` only when the field is absent.
fn some_if_present<'de, D>(deserializer: D) -> std::result::Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    Value::deserialize(deserializer).map(Some)
}

impl Response {
    /// Check that this reply answers the request with id `expected`, then
    /// unwrap it into its result or RPC error.
    ///
    /// A reply carrying an RPC error and a `null` id is accepted: JSON-RPC
    /// 2.0 requires `null` there when the server could not parse the request
    /// well enough to learn its id, and the error itself is still the most
    /// useful thing to surface. Any other id that is absent or does not match
    /// is a transport error: the reply belongs to some other request, or a
    /// proxy in between mangled it, and its payload must not be trusted.
    pub(crate) fn into_result(self, expected: u64) -> Result<Value> {
        let matches = match &self.id {
            Some(Value::Number(n)) => n.as_u64() == Some(expected),
            Some(Value::Null) => self.error.is_some(),
            _ => false,
        };
        if !matches {
            return Err(Error::Transport(format!(
                "reply id {} does not match request id {expected}",
                match &self.id {
                    Some(id) => id.to_string(),
                    None => "<absent>".to_string(),
                }
            )));
        }
        if let Some(e) = self.error {
            return Err(Error::Rpc(RpcError {
                code: e.code,
                message: e.message,
            }));
        }
        self.result
            .ok_or_else(|| Error::Transport("reply contained neither result nor error".into()))
    }
}

/// Turn an HTTP status and reply body into a JSON-RPC result for the request
/// sent with id `expected`.
///
/// v31.1 answers JSON-RPC 2.0 with HTTP 200 even for RPC errors, so a parseable
/// body always wins regardless of status. When the body is empty or not JSON,
/// a 2xx status is a malformed JSON-RPC reply, while anything else means the
/// failure happened below the JSON-RPC layer, where the status is the only
/// information we have.
pub(crate) fn parse_reply(status: u16, bytes: &[u8], expected: u64) -> Result<Value> {
    match serde_json::from_slice::<Response>(bytes) {
        Ok(reply) => reply.into_result(expected),
        // A 2xx that is not JSON is a malformed reply; anything else is an HTTP failure.
        Err(e) if (200..300).contains(&status) => Err(Error::Json(e)),
        Err(_) => Err(Error::Transport(http_failure(
            status,
            &String::from_utf8_lossy(bytes),
        ))),
    }
}

/// Turn an HTTP status and reply body into the per-call results of a JSON-RPC
/// batch whose requests carried the ids `first_id..first_id + count`.
///
/// The outer `Result` is the batch as a whole: the HTTP exchange, the shape
/// of the reply and its correlation with what was sent. Each inner `Result`
/// is one call, in the order the requests were given, regardless of the
/// order the node answered in (JSON-RPC 2.0 lets it reorder; Bitcoin Core
/// happens not to). Every expected id must be answered exactly once, or the
/// whole batch is a transport error: a missing or repeated reply means the
/// results cannot be trusted to line up with the requests.
///
/// Bitcoin Core answers a batch it could not read at all (not a JSON array)
/// with a single error object, id `null`; that surfaces as the batch-level
/// `Error::Rpc`.
pub(crate) fn parse_batch_reply(
    status: u16,
    bytes: &[u8],
    first_id: u64,
    count: usize,
) -> Result<Vec<Result<Value>>> {
    let replies: Vec<Response> = match serde_json::from_slice(bytes) {
        Ok(replies) => replies,
        Err(e) => {
            // Not an array. A lone error object is the node rejecting the
            // batch as a whole; anything else follows the single-call rules.
            return match serde_json::from_slice::<Response>(bytes) {
                Ok(reply @ Response { error: Some(_), .. }) => Err(reply
                    .into_result(first_id)
                    .expect_err("a reply carrying an error never yields Ok")),
                _ if (200..300).contains(&status) => Err(Error::Json(e)),
                _ => Err(Error::Transport(http_failure(
                    status,
                    &String::from_utf8_lossy(bytes),
                ))),
            };
        }
    };

    if replies.len() != count {
        return Err(Error::Transport(format!(
            "batch reply has {} entries for {count} requests",
            replies.len()
        )));
    }

    let mut slots: Vec<Option<Result<Value>>> = (0..count).map(|_| None).collect();
    for reply in replies {
        let offset = match &reply.id {
            Some(Value::Number(n)) => n
                .as_u64()
                .and_then(|id| id.checked_sub(first_id))
                .filter(|offset| *offset < count as u64),
            _ => None,
        };
        let Some(offset) = offset else {
            return Err(Error::Transport(format!(
                "batch reply id {} does not match any request id in {first_id}..{}",
                match &reply.id {
                    Some(id) => id.to_string(),
                    None => "<absent>".to_string(),
                },
                first_id + count as u64
            )));
        };
        let slot = &mut slots[offset as usize];
        if slot.is_some() {
            return Err(Error::Transport(format!(
                "batch reply answers request id {} twice",
                first_id + offset
            )));
        }
        *slot = Some(reply.into_result(first_id + offset));
    }

    // Lengths match and every id landed in a distinct slot, so none is empty.
    Ok(slots
        .into_iter()
        .map(|slot| slot.expect("every slot filled"))
        .collect())
}

fn http_failure(status: u16, body: &str) -> String {
    let hint = match status {
        401 | 403 => " — check the RPC credentials (--rpcuser/--rpcpassword or the cookie file)",
        404 => " — check the URL path",
        _ => "",
    };
    let body = body.trim();
    if body.is_empty() {
        format!("node returned HTTP {status}{hint}")
    } else {
        format!("node returned HTTP {status}{hint}: {body}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_serializes_as_jsonrpc_2_0() {
        let req = Request {
            jsonrpc: "2.0",
            id: 7,
            method: "getblockhash",
            params: &json!([100]),
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(
            v,
            json!({"jsonrpc":"2.0","id":7,"method":"getblockhash","params":[100]})
        );
    }

    #[test]
    fn success_response_yields_result() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":{"blocks":42}}"#;
        let resp: Response = serde_json::from_str(raw).unwrap();
        assert_eq!(resp.into_result(1).unwrap(), json!({"blocks":42}));
    }

    #[test]
    fn error_response_yields_rpc_error() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-8,"message":"Block not found"}}"#;
        let resp: Response = serde_json::from_str(raw).unwrap();
        match resp.into_result(1) {
            Err(crate::Error::Rpc(e)) => {
                assert_eq!(e.code, -8);
                assert_eq!(e.message, "Block not found");
            }
            other => panic!("expected Rpc error, got {other:?}"),
        }
    }

    #[test]
    fn null_result_is_ok_null() {
        // `stop` and `waitfornewblock` legitimately return null-ish payloads.
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":null}"#;
        let resp: Response = serde_json::from_str(raw).unwrap();
        assert_eq!(resp.into_result(1).unwrap(), json!(null));
    }

    #[test]
    fn response_without_result_or_error_is_a_transport_error() {
        let raw = r#"{"jsonrpc":"2.0","id":1}"#;
        let resp: Response = serde_json::from_str(raw).unwrap();
        assert!(matches!(
            resp.into_result(1),
            Err(crate::Error::Transport(_))
        ));
    }

    #[test]
    fn mismatched_id_is_a_transport_error_even_with_a_result() {
        // The payload may be perfectly well-formed; it still answers some
        // other request, so it must not be handed back as this one's result.
        let raw = r#"{"jsonrpc":"2.0","id":2,"result":{"blocks":42}}"#;
        let resp: Response = serde_json::from_str(raw).unwrap();
        match resp.into_result(1) {
            Err(Error::Transport(m)) => {
                assert!(m.contains("reply id 2"), "{m}");
                assert!(m.contains("request id 1"), "{m}");
            }
            other => panic!("expected Transport error, got {other:?}"),
        }
    }

    #[test]
    fn mismatched_id_wins_over_an_rpc_error() {
        let raw = r#"{"jsonrpc":"2.0","id":9,"error":{"code":-8,"message":"Block not found"}}"#;
        let resp: Response = serde_json::from_str(raw).unwrap();
        assert!(matches!(resp.into_result(1), Err(Error::Transport(_))));
    }

    #[test]
    fn absent_id_is_a_transport_error() {
        let raw = r#"{"jsonrpc":"2.0","result":true}"#;
        let resp: Response = serde_json::from_str(raw).unwrap();
        match resp.into_result(1) {
            Err(Error::Transport(m)) => assert!(m.contains("<absent>"), "{m}"),
            other => panic!("expected Transport error, got {other:?}"),
        }
    }

    #[test]
    fn null_id_is_accepted_only_alongside_an_rpc_error() {
        // JSON-RPC 2.0 §5: id MUST be null when the request's id could not be
        // determined, which only ever happens on the error path.
        let raw = r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}}"#;
        let resp: Response = serde_json::from_str(raw).unwrap();
        match resp.into_result(1) {
            Err(Error::Rpc(e)) => assert_eq!(e.code, -32700),
            other => panic!("expected Rpc error, got {other:?}"),
        }

        let raw = r#"{"jsonrpc":"2.0","id":null,"result":true}"#;
        let resp: Response = serde_json::from_str(raw).unwrap();
        assert!(matches!(resp.into_result(1), Err(Error::Transport(_))));
    }

    #[test]
    fn non_integer_id_is_a_transport_error() {
        let raw = r#"{"jsonrpc":"2.0","id":"1","result":true}"#;
        let resp: Response = serde_json::from_str(raw).unwrap();
        assert!(matches!(resp.into_result(1), Err(Error::Transport(_))));
    }

    #[test]
    fn parse_reply_200_with_result_is_ok() {
        let raw = br#"{"jsonrpc":"2.0","id":1,"result":{"blocks":42}}"#;
        assert_eq!(parse_reply(200, raw, 1).unwrap(), json!({"blocks": 42}));
    }

    #[test]
    fn parse_reply_200_with_empty_body_is_json_error() {
        // Only a non-2xx with an empty body is a transport failure; a 200
        // that answers with nothing at all is still a malformed JSON-RPC reply.
        assert!(matches!(parse_reply(200, b"", 1), Err(Error::Json(_))));
    }

    #[test]
    fn parse_reply_200_with_error_member_is_rpc_error() {
        let raw = br#"{"jsonrpc":"2.0","id":1,"error":{"code":-8,"message":"Block not found"}}"#;
        match parse_reply(200, raw, 1) {
            Err(Error::Rpc(e)) => {
                assert_eq!(e.code, -8);
                assert_eq!(e.message, "Block not found");
            }
            other => panic!("expected Rpc error, got {other:?}"),
        }
    }

    #[test]
    fn parse_reply_500_with_error_body_is_still_rpc_error() {
        // A 1.0-style node or a proxy may answer 500; the body still carries the error.
        let raw = br#"{"result":null,"error":{"code":-32601,"message":"Method not found"},"id":1}"#;
        match parse_reply(500, raw, 1) {
            Err(Error::Rpc(e)) => assert_eq!(e.code, -32601),
            other => panic!("expected Rpc error, got {other:?}"),
        }
    }

    #[test]
    fn parse_reply_401_with_empty_body_is_transport_error_mentioning_credentials() {
        match parse_reply(401, b"", 1) {
            Err(Error::Transport(m)) => {
                assert!(m.contains("401"));
                assert!(m.to_lowercase().contains("credentials"));
            }
            other => panic!("expected Transport error, got {other:?}"),
        }
    }

    #[test]
    fn parse_reply_403_with_empty_body_is_transport_error_mentioning_credentials() {
        match parse_reply(403, b"", 1) {
            Err(Error::Transport(m)) => {
                assert!(m.contains("403"));
                assert!(m.to_lowercase().contains("credentials"));
            }
            other => panic!("expected Transport error, got {other:?}"),
        }
    }

    #[test]
    fn parse_reply_200_with_garbage_is_json_error() {
        assert!(matches!(
            parse_reply(200, b"not json at all", 1),
            Err(Error::Json(_))
        ));
    }

    #[test]
    fn parse_reply_502_with_html_body_is_transport_error_with_body_included() {
        let body = b"<html><body>Bad Gateway</body></html>";
        match parse_reply(502, body, 1) {
            Err(Error::Transport(m)) => {
                assert!(m.contains("502"));
                assert!(m.contains("Bad Gateway"));
            }
            other => panic!("expected Transport error, got {other:?}"),
        }
    }

    fn ok_value(r: &Result<Value>) -> &Value {
        match r {
            Ok(v) => v,
            Err(e) => panic!("expected Ok, got {e:?}"),
        }
    }

    #[test]
    fn batch_results_come_back_in_request_order() {
        let raw =
            br#"[{"jsonrpc":"2.0","id":5,"result":"a"},{"jsonrpc":"2.0","id":6,"result":"b"}]"#;
        let out = parse_batch_reply(200, raw, 5, 2).unwrap();
        assert_eq!(ok_value(&out[0]), "a");
        assert_eq!(ok_value(&out[1]), "b");
    }

    #[test]
    fn batch_replies_are_matched_by_id_not_position() {
        let raw =
            br#"[{"jsonrpc":"2.0","id":6,"result":"b"},{"jsonrpc":"2.0","id":5,"result":"a"}]"#;
        let out = parse_batch_reply(200, raw, 5, 2).unwrap();
        assert_eq!(ok_value(&out[0]), "a");
        assert_eq!(ok_value(&out[1]), "b");
    }

    #[test]
    fn batch_keeps_a_per_call_rpc_error_in_its_slot() {
        let raw = br#"[{"jsonrpc":"2.0","id":1,"result":"a"},{"jsonrpc":"2.0","id":2,"error":{"code":-8,"message":"Block height out of range"}}]"#;
        let out = parse_batch_reply(200, raw, 1, 2).unwrap();
        assert_eq!(ok_value(&out[0]), "a");
        match &out[1] {
            Err(Error::Rpc(e)) => assert_eq!(e.code, -8),
            other => panic!("expected Rpc error, got {other:?}"),
        }
    }

    #[test]
    fn batch_with_the_wrong_number_of_replies_is_a_transport_error() {
        let raw = br#"[{"jsonrpc":"2.0","id":1,"result":"a"}]"#;
        match parse_batch_reply(200, raw, 1, 2) {
            Err(Error::Transport(m)) => assert!(m.contains("1 entries for 2 requests"), "{m}"),
            other => panic!("expected Transport error, got {other:?}"),
        }
    }

    #[test]
    fn batch_answering_one_id_twice_is_a_transport_error() {
        let raw =
            br#"[{"jsonrpc":"2.0","id":1,"result":"a"},{"jsonrpc":"2.0","id":1,"result":"a"}]"#;
        match parse_batch_reply(200, raw, 1, 2) {
            Err(Error::Transport(m)) => assert!(m.contains("twice"), "{m}"),
            other => panic!("expected Transport error, got {other:?}"),
        }
    }

    #[test]
    fn batch_with_an_id_outside_the_request_range_is_a_transport_error() {
        for id in ["0", "3", "null", "\"1\""] {
            let raw = format!(
                r#"[{{"jsonrpc":"2.0","id":1,"result":"a"}},{{"jsonrpc":"2.0","id":{id},"result":"b"}}]"#
            );
            match parse_batch_reply(200, raw.as_bytes(), 1, 2) {
                Err(Error::Transport(m)) => assert!(m.contains("does not match"), "{id}: {m}"),
                other => panic!("{id}: expected Transport error, got {other:?}"),
            }
        }
    }

    #[test]
    fn batch_rejected_as_a_whole_surfaces_the_nodes_rpc_error() {
        // What Core sends back when the batch body is not a JSON array.
        let raw = br#"{"result":null,"error":{"code":-32700,"message":"Parse error"},"id":null}"#;
        match parse_batch_reply(500, raw, 1, 2) {
            Err(Error::Rpc(e)) => assert_eq!(e.code, -32700),
            other => panic!("expected Rpc error, got {other:?}"),
        }
    }

    #[test]
    fn batch_2xx_that_is_not_an_array_is_a_json_error() {
        assert!(matches!(
            parse_batch_reply(200, br#"{"jsonrpc":"2.0","id":1,"result":true}"#, 1, 2),
            Err(Error::Json(_))
        ));
        assert!(matches!(
            parse_batch_reply(200, b"nope", 1, 2),
            Err(Error::Json(_))
        ));
    }

    #[test]
    fn batch_http_failure_is_a_transport_error_naming_the_status() {
        match parse_batch_reply(401, b"", 1, 2) {
            Err(Error::Transport(m)) => assert!(m.contains("401"), "{m}"),
            other => panic!("expected Transport error, got {other:?}"),
        }
    }

    #[test]
    fn empty_batch_parses_to_nothing() {
        assert!(parse_batch_reply(200, b"[]", 1, 0).unwrap().is_empty());
    }
}
