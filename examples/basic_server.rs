use anyhow::Result;
use async_trait::async_trait;
use anet_mcp_server::{
    Content, ServerBuilder, ServerCapabilities, Tool, 
    transport::nats::NatsTransport,
};
use serde_json::{json, Value};
use tracing::info;
use tracing_subscriber::EnvFilter;

struct ExampleTool;

#[async_trait]
impl Tool for ExampleTool {
    fn name(&self) -> String { "example_tool".to_string() }
    fn description(&self) -> String { "An example tool".to_string() }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "param": {"type": "string"}
            },
            "required": ["param"]
        })
    }
    async fn call(&self, input: Option<Value>) -> Result<Vec<Content>> {
        let param = input.and_then(|v| v.get("param").and_then(|p| p.as_str().map(String::from)))
            .ok_or_else(|| anyhow::anyhow!("Missing 'param' in input"))?;
        Ok(vec![Content::Text { text: format!("Tool executed with param: {}", param) }])
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    info!("Starting MCP server example");
    
    let transport = NatsTransport::new("nats://localhost:4222", "mcp.requests").await?;
    
    let server = ServerBuilder::new()
        .transport(transport)
        .name("example-mcp")
        .version("0.1.0")
        .capabilities(ServerCapabilities {
            tools: Some(json!({})),
            prompts: Some(json!({})),
            resources: Some(json!({})),
            notification_options: None,
            experimental_capabilities: None,
        })
        .add_tool(ExampleTool)
        .build()?;

    info!("Server built, ready to run!");
    server.run().await
}
