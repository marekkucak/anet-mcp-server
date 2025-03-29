use anyhow::Result;
use async_trait::async_trait;

pub mod nats;

use crate::server::Server;

#[async_trait]
pub trait Transport: Send + Sync + 'static {
    async fn run(&self, server: &Server) -> Result<()>;
}
