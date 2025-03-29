# Anet MCP Server

A Rust implementation of the **Model Control Protocol (MCP)** server that enables communication between clients and AI models via a standardized protocol.

This project provides a scalable and asynchronous framework for building AI services using **Rust**, **Tokio**, and **NATS**. It is designed for developers building **AI agent systems**, **LLM-based tools**, or custom **JSON-RPC 2.0** service layers. The architecture supports real-time message passing, making it ideal for **microservices**, **AI orchestration**, and **tool-based model interaction**.

---

## Features

- ✅ JSON-RPC 2.0 compatible API  
- 🔄 NATS transport layer for message passing  
- 🛠️ Extensible tool system  
- 🧠 Support for prompts and resources  
- ⚡ Asynchronous request handling with Tokio  

---

## Requirements

- **Rust** 1.70+  
- **NATS** server running locally or accessible via network  

---

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
anet_mcp_server = "0.1.0"
```

---

## Getting Started

### Running the Example Server

The repository includes a basic example server that demonstrates core functionality:

```bash
# Start a NATS server in another terminal or ensure one is already running
# Example:
nats-server

# Run the example server
cargo run --example basic_server
```

### Testing the Server

You can test the server using the included test client:

```bash
cargo run --example test_client
```

This will send various requests to the server and print the responses.

---

## Usage

### Creating a Server

```rust
use anet_mcp_server::{
    ServerBuilder, ServerCapabilities, 
    transport::nats::NatsTransport,
};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let transport = NatsTransport::new("nats://localhost:4222", "mcp.requests").await?;

    let server = ServerBuilder::new()
        .transport(transport)
        .name("my-mcp-server")
        .version("0.1.0")
        .capabilities(ServerCapabilities {
            tools: Some(json!({})),
            prompts: Some(json!({})),
            resources: Some(json!({})),
            notification_options: None,
            experimental_capabilities: None,
        })
        .build()?;

    server.run().await
}
```

---

### Implementing a Custom Tool

```rust
use anet_mcp_server::{Content, Tool};
use async_trait::async_trait;
use serde_json::{json, Value};

struct MyTool;

#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> String {
        "my_tool".to_string()
    }

    fn description(&self) -> String {
        "A custom tool".to_string()
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "input": { "type": "string" }
            }
        })
    }

    async fn call(&self, input: Option<Value>) -> anyhow::Result<Vec<Content>> {
        Ok(vec![Content::Text {
            text: "Tool response".to_string()
        }])
    }
}
```

---

## API Reference

The server implements the following **JSON-RPC** methods:

- `initialize` – Initialize the connection and get server information  
- `listTools` – Get a list of available tools  
- `callTool` – Call a specific tool with arguments  
- `listResources` – Get a list of available resources  
- `readResource` – Read a specific resource  
- `listPrompts` – Get a list of available prompts  
- `getPrompt` – Get a specific prompt with arguments  

---

## Architecture

The server follows a modular design:

- **server** – Core server logic and request handling  
- **transport** – Message transport layer (currently NATS)  
- **tools** – Tool interfaces and implementations  
- **types** – Common data structures  

---

## License

MIT License
```

Let me know if you want badges, contribution guidelines, or example JSON-RPC payloads added to the README as well.
