pub async fn start_jsonrpc_server(port: u16) {
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use crate::transaction::Transaction;
    use rusqlite::Connection;
    use sha2::{Digest, Sha256};

    println!("🚀 启动 JSON-RPC 服务，监听端口 {}", port);
    let listener = TcpListener::bind(("0.0.0.0", port)).await.unwrap();
    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            let mut buf = [0; 4096];
            if let Ok(n) = socket.read(&mut buf).await {
                if let Ok(text) = std::str::from_utf8(&buf[..n]) {
                    // 解析 HTTP POST
                    if let Some(body_start) = text.find("\r\n\r\n") {
                        let body = &text[body_start + 4..];
                        if let Ok(req) = serde_json::from_str::<serde_json::Value>(body) {
                            let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
                            if method == "send_transaction" {
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
                                        let resp_str = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", resp.to_string().len(), resp.to_string());
                                        let _ = socket.write_all(resp_str.as_bytes()).await;
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    // 非法请求
                    let resp = json!({"jsonrpc":"2.0","error":"invalid request","id":null});
                    let resp_str = format!("HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", resp.to_string().len(), resp.to_string());
                    let _ = socket.write_all(resp_str.as_bytes()).await;
                }
            }
        });
    }
} 