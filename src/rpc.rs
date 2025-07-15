use crate::transaction::Transaction;
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub async fn start_jsonrpc_server(port: u16) {
    println!("🚀 启动 JSON-RPC 服务，监听端口 {}", port);
    let listener = TcpListener::bind(("0.0.0.0", port)).await.unwrap();
    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            let mut buf = [0; 4096];
            if let Ok(n) = socket.read(&mut buf).await {
                if let Ok(text) = std::str::from_utf8(&buf[..n]) {
                    let resp_str = handle_jsonrpc_http(text);
                    let _ = socket.write_all(resp_str.as_bytes()).await;
                }
            }
        });
    }
}

fn handle_jsonrpc_http(text: &str) -> String {
    if let Some(body_start) = text.find("\r\n\r\n") {
        let body = &text[body_start + 4..];
        let (status, resp) = handle_jsonrpc_body(body);
        return format!(
            "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            status,
            resp.to_string().len(),
            resp.to_string()
        );
    }
    let resp = json!({"jsonrpc":"2.0","error":"invalid request","id":null});
    format!(
        "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        resp.to_string().len(),
        resp.to_string()
    )
}

fn handle_jsonrpc_body(body: &str) -> (&'static str, serde_json::Value) {
    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(req) => {
            let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
            match method {
                "send_transaction" => handle_send_transaction(&req),
                _ => (
                    "400 Bad Request",
                    json!({"jsonrpc":"2.0","error":"unknown method","id":req.get("id").cloned().unwrap_or(json!(1))}),
                ),
            }
        }
        Err(_) => (
            "400 Bad Request",
            json!({"jsonrpc":"2.0","error":"invalid request","id":null}),
        ),
    }
}

fn handle_send_transaction(req: &serde_json::Value) -> (&'static str, serde_json::Value) {
    if let Some(params) = req.get("params").and_then(|p| p.as_array()) {
        if params.len() == 3 {
            let from = params[0].as_str().unwrap_or("");
            let to = params[1].as_str().unwrap_or("");
            let amount = params[2].as_u64().unwrap_or(0);
            let _tx = Transaction::new(from, to, amount);
            println!("[JSON-RPC] 交易提交: {} -> {} [{}]", from, to, amount);
            // 计算交易hash
            let tx_str = format!("{}{}{}", from, to, amount);
            let mut hasher = Sha256::new();
            hasher.update(tx_str.as_bytes());
            let tx_hash = format!("0x{:x}", hasher.finalize());
            let resp = json!({
                "jsonrpc": "2.0",
                "result": {"status": "ok", "tx_hash": tx_hash},
                "id": req.get("id").cloned().unwrap_or(json!(1))
            });
            return ("200 OK", resp);
        }
    }
    (
        "400 Bad Request",
        json!({"jsonrpc":"2.0","error":"invalid params","id":req.get("id").cloned().unwrap_or(json!(1))}),
    )
}
