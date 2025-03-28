use anyhow::{Result, Context};
use async_nats::{Client, ConnectOptions};
use async_trait::async_trait;
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{info, error};
use tracing_subscriber::EnvFilter;

// --- Types (unchanged) ---
#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<String>,
    method: String,
    params: Option<Value>,
}

#[derive(Serialize, Debug)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<String>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Serialize, Debug)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

#[derive(Serialize, Debug)]
struct ServerInfo {
    server_name: String,
    server_version: String,
    capabilities: ServerCapabilities,
}

#[derive(Serialize, Debug)]
struct ServerCapabilities {
    tools: Option<Value>,
    prompts: Option<Value>,
    resources: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notification_options: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    experimental_capabilities: Option<Value>,
}

#[derive(Serialize, Debug)]
struct ToolDefinition {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: Value,
}

#[derive(Serialize, Debug)]
struct Resource {
    uri: String,
    name: String,
    description: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
}

#[derive(Serialize, Debug)]
struct Prompt {
    name: String,
    description: String,
    arguments: Vec<PromptArgument>,
}

#[derive(Serialize, Debug)]
struct PromptArgument {
    name: String,
    description: String,
    required: bool,
}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
enum Content {
    #[serde(rename = "text")]
    Text { text: String },
}

// --- Server ---
pub struct Server {
    transport: Box<dyn Transport>,
    tools: Tools,
    server_name: String,
    server_version: String,
    capabilities: ServerCapabilities,
    runtime: tokio::runtime::Runtime,
}

impl Server {
    pub fn new(
        transport: Box<dyn Transport>,
        tools: Tools,
        server_name: String,
        server_version: String,
        capabilities: ServerCapabilities,
        runtime: tokio::runtime::Runtime,
    ) -> Self {
        Self {
            transport,
            tools,
            server_name,
            server_version,
            capabilities,
            runtime,
        }
    }

    pub async fn run(&self) -> Result<()> {
        self.transport.run(self).await
    }

    pub async fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let id = req.id.clone();
        let method = req.method.as_str();
        let params = req.params.unwrap_or(json!({}));

        let result = match method {
            "initialize" => self.handle_initialize(params).await,
            "listTools" => self.handle_list_tools(params).await,
            "callTool" => self.handle_call_tool(params).await,
            "listResources" => self.handle_list_resources(params).await,
            "readResource" => self.handle_read_resource(params).await,
            "listPrompts" => self.handle_list_prompts(params).await,
            "getPrompt" => self.handle_get_prompt(params).await,
            _ => Err(anyhow::anyhow!("Method not found: {}", method)),
        };

        match result {
            Ok(value) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(value),
                error: None,
            },
            Err(e) => {
                error!("Error handling {}: {:?}", method, e);
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32601,
                        message: e.to_string(),
                        data: None,
                    }),
                }
            }
        }
    }

    async fn handle_initialize(&self, params: Value) -> Result<Value> {
        let client_info = params.get("clientInfo").and_then(|v| v.as_object());
        info!("Initializing with client: {:?}", client_info);
        Ok(json!({
            "server_name": self.server_name,
            "server_version": self.server_version,
            "capabilities": self.capabilities,
        }))
    }

    async fn handle_list_tools(&self, _params: Value) -> Result<Value> {
        Ok(json!({
            "tools": self.tools.list()
        }))
    }

    async fn handle_call_tool(&self, params: Value) -> Result<Value> {
        let name = params.get("name").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'name' in params"))?;
        let arguments = params.get("arguments");
        let content = self.tools.call(name, arguments.cloned()).await?;
        Ok(json!({
            "content": content
        }))
    }

    async fn handle_list_resources(&self, _params: Value) -> Result<Value> {
        Ok(json!({
            "resources": [],
            "next_cursor": null,
            "meta": null
        }))
    }

    async fn handle_read_resource(&self, params: Value) -> Result<Value> {
        let uri = params.get("uri").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'uri' in params"))?;
        Ok(json!(format!("Resource content for {}", uri)))
    }

    async fn handle_list_prompts(&self, _params: Value) -> Result<Value> {
        Ok(json!({
            "prompts": []
        }))
    }

    async fn handle_get_prompt(&self, params: Value) -> Result<Value> {
        let name = params.get("name").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'name' in params"))?;
        let arguments = params.get("arguments")
            .ok_or_else(|| anyhow::anyhow!("Missing 'arguments' in params"))?;
        Ok(json!({
            "description": format!("Prompt '{}'", name),
            "messages": [
                {
                    "role": "user",
                    "content": {
                        "type": "text",
                        "text": format!("Prompt with args: {:?}", arguments)
                    }
                }
            ]
        }))
    }
}

// --- Transport ---
#[async_trait]
pub trait Transport: Send + Sync + 'static {
    async fn run(&self, server: &Server) -> Result<()>;
}

struct NatsTransport {
    client: Client,
    subject: String,
}

#[async_trait]
impl Transport for NatsTransport {
    async fn run(&self, server: &Server) -> Result<()> {
        info!("Connecting to NATS on subject '{}'", self.subject);
        let mut subscription = self.client.subscribe(self.subject.clone()).await?;
        
        while let Some(message) = subscription.next().await {
            let request: JsonRpcRequest = serde_json::from_slice(&message.payload)
                .context("Failed to parse JSON-RPC request from NATS")?;
            let response = server.handle_request(request).await;
            let serialized = serde_json::to_vec(&response)?;
            if let Some(reply) = message.reply {
                self.client.publish(reply, serialized.into()).await?;
            }
        }
        Ok(())
    }
}

impl NatsTransport {
    async fn new(nats_url: &str, subject: &str) -> Result<Self> {
        let client = async_nats::connect_with_options(
            nats_url,
            ConnectOptions::new().retry_on_initial_connect(),
        ).await.context("Failed to connect to NATS")?;
        Ok(NatsTransport {
            client,
            subject: subject.to_string(),
        })
    }
}

// --- Tools ---
#[async_trait]
pub trait Tool: Send + Sync + 'static {
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn input_schema(&self) -> Value;
    async fn call(&self, input: Option<Value>) -> Result<Vec<Content>>;
}

#[derive(Default)]
struct Tools {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl Tools {
    fn add<T: Tool>(&mut self, tool: T) {
        self.tools.insert(tool.name(), Box::new(tool));
    }

    fn list(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|tool| ToolDefinition {
            name: tool.name(),
            description: tool.description(),
            input_schema: tool.input_schema(),
        }).collect()
    }

    async fn call(&self, name: &str, args: Option<Value>) -> Result<Vec<Content>> {
        self.tools.get(name)
            .ok_or_else(|| anyhow::anyhow!("Tool {} not found", name))?
            .call(args).await
    }
}

// --- ServerBuilder ---
pub struct ServerBuilder {
    transport: Option<Box<dyn Transport>>,
    tools: Tools,
    server_name: String,
    server_version: String,
    capabilities: ServerCapabilities,
}

impl ServerBuilder {
    pub fn new() -> Self {
        Self {
            transport: None,
            tools: Tools::default(),
            server_name: "mcp-server".to_string(),
            server_version: "0.1.0".to_string(),
            capabilities: ServerCapabilities {
                tools: Some(json!({})),
                prompts: None,
                resources: None,
                notification_options: None,
                experimental_capabilities: None,
            },
        }
    }

    pub fn transport(mut self, transport: impl Transport + 'static) -> Self {
        self.transport = Some(Box::new(transport));
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.server_name = name.into();
        self
    }

    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.server_version = version.into();
        self
    }

    pub fn capabilities(mut self, capabilities: ServerCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn add_tool(mut self, tool: impl Tool) -> Self {
        self.tools.add(tool);
        self
    }

    pub fn build(self) -> Result<Server> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .context("Failed to create Tokio runtime")?;
        
        Ok(Server {
            transport: self.transport.ok_or_else(|| anyhow::anyhow!("Transport is required"))?,
            tools: self.tools,
            server_name: self.server_name,
            server_version: self.server_version,
            capabilities: self.capabilities,
            runtime,
        })
    }
}

// --- Example Usage ---
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

    server.run().await
}