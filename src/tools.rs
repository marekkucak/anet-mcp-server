use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use crate::types::{Content, ToolDefinition};

#[async_trait]
pub trait Tool: Send + Sync + 'static {
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn input_schema(&self) -> Value;
    async fn call(&self, input: Option<Value>) -> Result<Vec<Content>>;
}

#[derive(Default)]
pub struct Tools {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl Tools {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
    
    pub fn add<T: Tool>(&mut self, tool: T) {
        self.tools.insert(tool.name(), Box::new(tool));
    }

    pub fn list(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|tool| ToolDefinition {
            name: tool.name(),
            description: tool.description(),
            input_schema: tool.input_schema(),
        }).collect()
    }

    pub async fn call(&self, name: &str, args: Option<Value>) -> Result<Vec<Content>> {
        self.tools.get(name)
            .ok_or_else(|| anyhow::anyhow!("Tool {} not found", name))?
            .call(args).await
    }
}
