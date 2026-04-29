//! Integration tests for the `/mcp/ws` WebSocket transport.
//!
//! `handle_websocket_connection` in `src/mcp/server.rs` was previously
//! exercised only manually: HTTP dispatch had broad in-process tests
//! (`mcp_server_routing_tests.rs`), but the parallel WebSocket transport
//! had none. This file closes that gap by booting the full server router
//! on an ephemeral port and driving the WebSocket endpoint with
//! `tokio-tungstenite`.
//!
//! All scenarios here resolve before any network call to the upstream
//! Velib API would be made: either the method is served from static JSON
//! (`tools/list`), the handler validates input pre-network (radius too
//! large, query too short), or the server's parse path rejects malformed
//! frames. No `#[ignore]` is needed and no live API access is required.

use std::net::SocketAddr;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use velib_mcp::Server;

mod common;

/// Boot the full server router (the same one `main.rs` serves) on a
/// caller-chosen ephemeral port and return its WebSocket URL.
///
/// The server task is leaked deliberately: each test gets its own port,
/// the runtime is torn down at test end, and we never need to gracefully
/// stop it. Returning the URL keeps tests focused on behaviour.
async fn spawn_server() -> String {
    let port = common::find_available_port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let listener = TcpListener::bind(addr).await.expect("bind ephemeral port");
    let app = Server::new(addr).router();

    tokio::spawn(async move {
        // If the test runtime drops before this returns, that's fine.
        let _ = axum::serve(listener, app).await;
    });

    // Yield once so the listener is ready to accept before the client
    // connects. axum::serve is otherwise polled lazily.
    tokio::task::yield_now().await;

    format!("ws://127.0.0.1:{port}/mcp/ws")
}

/// Send one JSON-RPC frame and read one response with a short timeout
/// so a wedged server fails fast rather than hanging the suite.
async fn ws_round_trip(url: &str, request: Value) -> Value {
    let (mut socket, _resp) = connect_async(url).await.expect("ws handshake");

    socket
        .send(Message::Text(request.to_string()))
        .await
        .expect("send frame");

    let msg = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("response within timeout")
        .expect("socket still open")
        .expect("frame ok");

    let text = match msg {
        Message::Text(t) => t,
        other => panic!("expected text frame, got {other:?}"),
    };

    serde_json::from_str(&text).expect("response is valid JSON")
}

#[tokio::test]
async fn ws_tools_list_returns_jsonrpc_response() {
    // Smoke check: the WebSocket transport parses a JSON-RPC frame, dispatches
    // to `tools/list` (static JSON, no network), serializes the response, and
    // sends it back as a text frame.
    let url = spawn_server().await;

    let response = ws_round_trip(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        }),
    )
    .await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    let tools = response["result"]["tools"].as_array().expect("tools array");
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"find_nearby_stations"));
    assert!(names.contains(&"get_station_by_code"));
    assert_eq!(tools.len(), 5);
}

#[tokio::test]
async fn ws_resources_list_returns_jsonrpc_response() {
    // Confirms the dispatch arm for `resources/list` works over WebSocket
    // identically to its HTTP counterpart.
    let url = spawn_server().await;

    let response = ws_round_trip(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": "abc",
            "method": "resources/list",
            "params": {}
        }),
    )
    .await;

    assert_eq!(response["id"], "abc");
    let resources = response["result"]["resources"]
        .as_array()
        .expect("resources array");
    assert_eq!(resources.len(), 4);
}

#[tokio::test]
async fn ws_unknown_method_returns_jsonrpc_error_envelope() {
    // Unknown methods must come back in the JSON-RPC error envelope (not a
    // dropped frame or a TCP close), so MCP clients can surface a useful
    // error to the user.
    let url = spawn_server().await;

    let response = ws_round_trip(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "totally/unknown",
            "params": {}
        }),
    )
    .await;

    assert_eq!(response["id"], 42);
    assert!(response["result"].is_null());
    let error = &response["error"];
    assert!(error.is_object(), "expected JSON-RPC error envelope");
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("Unknown method"));
}

#[tokio::test]
async fn ws_malformed_json_returns_parse_error_with_null_id() {
    // The handler's invalid-JSON branch builds a JSON-RPC parse error with
    // code -32700 and id=null. This guards against accidental drops of
    // malformed frames (which would manifest as a hung client).
    let url = spawn_server().await;

    let (mut socket, _resp) = connect_async(&url).await.expect("ws handshake");

    socket
        .send(Message::Text("{ this is not valid json".to_string()))
        .await
        .expect("send frame");

    let msg = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("response within timeout")
        .expect("socket still open")
        .expect("frame ok");

    let text = match msg {
        Message::Text(t) => t,
        other => panic!("expected text frame, got {other:?}"),
    };

    let response: Value = serde_json::from_str(&text).expect("response is valid JSON");
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["id"].is_null());
    assert_eq!(response["error"]["code"], -32700);
    assert_eq!(response["error"]["message"], "Parse error");
}

#[tokio::test]
async fn ws_handler_validation_error_returns_jsonrpc_error() {
    // A `find_nearby_stations` call with a 99,999m radius fails handler
    // validation pre-network. The error must travel back inside the
    // JSON-RPC error envelope, with the request id preserved -- never as
    // a 500 over WebSocket (which has no HTTP envelope).
    let url = spawn_server().await;

    let response = ws_round_trip(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "find_nearby_stations",
                "arguments": {
                    "latitude": 48.8566,
                    "longitude": 2.3522,
                    "radius_meters": 99999,
                    "limit": 10
                }
            }
        }),
    )
    .await;

    assert_eq!(response["id"], 7);
    let error = &response["error"];
    assert!(error.is_object(), "expected JSON-RPC error envelope");
    let msg = error["message"].as_str().unwrap();
    assert!(msg.contains("radius") || msg.contains("99999"), "{msg}");
}

#[tokio::test]
async fn ws_unknown_tool_returns_jsonrpc_error() {
    // `tools/call` with an unknown tool name should produce a JSON-RPC
    // error envelope, not close the socket.
    let url = spawn_server().await;

    let response = ws_round_trip(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {
                "name": "not_a_real_tool",
                "arguments": {}
            }
        }),
    )
    .await;

    assert_eq!(response["id"], 9);
    assert!(response["error"].is_object());
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Unknown tool"));
}

#[tokio::test]
async fn ws_socket_stays_open_for_multiple_requests() {
    // Regression guard: the server's WebSocket loop should keep accepting
    // frames until the client closes. This sends three sequential JSON-RPC
    // requests over a single connection and verifies all three responses
    // arrive on the same socket -- catching any accidental
    // `break`-after-first-response or per-frame socket teardown.
    let url = spawn_server().await;
    let (mut socket, _resp) = connect_async(&url).await.expect("ws handshake");

    for id in 1..=3 {
        socket
            .send(Message::Text(
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "method": "tools/list",
                    "params": {}
                })
                .to_string(),
            ))
            .await
            .expect("send frame");

        let msg = tokio::time::timeout(Duration::from_secs(5), socket.next())
            .await
            .expect("response within timeout")
            .expect("socket still open")
            .expect("frame ok");

        let text = match msg {
            Message::Text(t) => t,
            other => panic!("expected text frame, got {other:?}"),
        };
        let response: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(response["id"], id);
        assert!(response["result"]["tools"].is_array());
    }
}

#[tokio::test]
async fn ws_close_frame_terminates_connection_cleanly() {
    // After receiving a Close frame the server's loop must `break` and
    // tear down the connection without panicking. We verify by issuing
    // one request, closing, and asserting the stream ends.
    let url = spawn_server().await;
    let (mut socket, _resp) = connect_async(&url).await.expect("ws handshake");

    socket
        .send(Message::Text(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/list",
                "params": {}
            })
            .to_string(),
        ))
        .await
        .expect("send frame");

    // Drain the response so we know the loop processed our request.
    let _ = tokio::time::timeout(Duration::from_secs(5), socket.next()).await;

    socket.close(None).await.expect("close socket");

    // After Close the stream should end (None) within a short window. We
    // accept either explicit Close acks or a clean stream end -- both
    // indicate the server's `Message::Close` arm fired.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(500), socket.next()).await {
            Ok(None) => return,                        // stream ended
            Ok(Some(Ok(Message::Close(_)))) => return, // explicit close ack
            Ok(Some(Ok(_))) => continue,               // drain stragglers
            Ok(Some(Err(_))) => return,                // closed with error: also fine
            Err(_) => continue,                        // 500ms tick
        }
    }
    panic!("server did not close socket after client Close frame");
}
