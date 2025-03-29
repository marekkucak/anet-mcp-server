use anyhow::{Result, Context};
use async_nats::{Client, ConnectOptions};
use serde_json::{json, Value};
use tokio;
// Add this import for the StreamExt trait
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to NATS
    let client = async_nats::connect_with_options(
        "nats://localhost:4222",
        ConnectOptions::new().retry_on_initial_connect(),
    )
    .await
    .context("Failed to connect to NATS")?;
    // Helper function to send a request
    async fn send_request(client: &Client, method: &str, params: Value, id: &str) -> Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        let inbox = client.new_inbox();
        let mut sub = client.subscribe(inbox.clone()).await?;
        client
            .publish_with_reply("mcp.requests".to_string(), inbox, serde_json::to_vec(&request)?.into())
            .await?;
        let msg = sub.next().await.ok_or_else(|| anyhow::anyhow!("No response"))?;
        serde_json::from_slice(&msg.payload).context("Failed to parse response")
    }
    // Test 1: Initialize
    let init_response = send_request(&client, "initialize", json!({"clientInfo": {"name": "test-client"}}), "1").await?;
    println!("Initialize response: {}", serde_json::to_string_pretty(&init_response)?);
    // Test 2: List Tools
    let tools_response = send_request(&client, "listTools", json!({}), "2").await?;
    println!("ListTools response: {}", serde_json::to_string_pretty(&tools_response)?);
    // Test 3: Call Tool
    let call_response = send_request(&client, "callTool", json!({"name": "example_tool", "arguments": {"param": "test-value"}}), "3").await?;
    println!("CallTool response: {}", serde_json::to_string_pretty(&call_response)?);
    // Test 4: Unknown Method
    let error_response = send_request(&client, "unknownMethod", json!({}), "4").await?;
    println!("UnknownMethod response: {}", serde_json::to_string_pretty(&error_response)?);
    Ok(())
}
