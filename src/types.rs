use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<String>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Serialize, Debug, Clone)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<String>,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}

#[derive(Serialize, Debug, Clone)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ServerInfo {
    pub server_name: String,
    pub server_version: String,
    pub capabilities: ServerCapabilities,
}

#[derive(Serialize, Debug, Clone)]
pub struct ServerCapabilities {
    pub tools: Option<Value>,
    pub prompts: Option<Value>,
    pub resources: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_options: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental_capabilities: Option<Value>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

#[derive(Serialize, Debug, Clone)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct Prompt {
    pub name: String,
    pub description: String,
    pub arguments: Vec<PromptArgument>,
}

#[derive(Serialize, Debug, Clone)]
pub struct PromptArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Content {
    #[serde(rename = "text")]
    Text { text: String },
}
