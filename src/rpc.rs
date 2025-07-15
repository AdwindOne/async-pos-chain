use crate::mempool::Mempool;
use crate::storage;
use crate::transaction::Transaction;
use std::sync::{Arc, Mutex};
use serde_json::json;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn start_jsonrpc_server(port: u16, mempool: Arc<Mutex<Mempool>>, chain_db: storage::RocksDB) {
    println!("🚀 启动 JSON-RPC 服务，监听端口 {}", port);
    let listener = TcpListener::bind(("0.0.0.0", port)).await.unwrap();
    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mempool = Arc::clone(&mempool);
        let chain_db = Arc::clone(&chain_db);
        tokio::spawn(async move {
            let mut buf = [0; 4096];
            if let Ok(n) = socket.read(&mut buf).await {
                if let Ok(text) = std::str::from_utf8(&buf[..n]) {
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
                                        let tx = Transaction::new(from, to, amount);
                                        println!("[JSON-RPC] 交易提交: {} -> {} [{}]", from, to, amount);
                                        let tx_hash = tx.hash();
                                        {
                                            let mut mempool = mempool.lock().unwrap();
                                            mempool.add(tx, Some(&chain_db));
                                        }
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
                    let resp = json!({"jsonrpc":"2.0","error":"invalid request","id":null});
                    let resp_str = format!("HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", resp.to_string().len(), resp.to_string());
                    let _ = socket.write_all(resp_str.as_bytes()).await;
                }
            }
        });
    }
} 