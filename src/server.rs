use anyhow::{Result, Context};
use serde_json::{json, Value};
use tokio::runtime::Runtime;
use tracing::{info, error};

use crate::tools::{Tool, Tools};
use crate::transport::Transport;
use crate::types::{JsonRpcRequest, JsonRpcResponse, JsonRpcError, ServerCapabilities};

pub struct Server {
    transport: Box<dyn Transport>,
    tools: Tools,
    server_name: String,
    server_version: String,
    capabilities: ServerCapabilities,
    runtime: Runtime,
}

impl Server {
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
            tools: Tools::new(),
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
