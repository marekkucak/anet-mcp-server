use anyhow::{Result, Context};
use async_nats::{Client, ConnectOptions};
use async_trait::async_trait;
use futures_util::stream::StreamExt;
use tracing::{info, error};
use serde_json;

use crate::server::Server;
use crate::transport::Transport;

pub struct NatsTransport {
    client: Client,
    subject: String,
}

impl NatsTransport {
    pub async fn new(nats_url: &str, subject: &str) -> Result<Self> {
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

#[async_trait]
impl Transport for NatsTransport {
    async fn run(&self, server: &Server) -> Result<()> {
        info!("Connecting to NATS on subject '{}'", self.subject);
        let mut subscription = self.client.subscribe(self.subject.clone()).await?;
        
        while let Some(message) = subscription.next().await {
            info!("Received message on subject '{}'", self.subject);
            
            match serde_json::from_slice(&message.payload) {
                Ok(request) => {
                    let response = server.handle_request(request).await;
                    let serialized = serde_json::to_vec(&response)?;
                    
                    if let Some(reply) = message.reply {
                        info!("Sending response to reply subject '{}'", reply);
                        self.client.publish(reply, serialized.into()).await?;
                    } else {
                        info!("No reply subject provided, response not sent");
                    }
                },
                Err(e) => {
                    error!("Failed to parse JSON-RPC request from NATS: {}", e);
                }
            }
        }
        Ok(())
    }
}
