pub mod http;
pub mod sse;

use crate::error::Result;
use crate::types::JsonRpcMessage;
use async_trait::async_trait;

#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(&mut self, message: JsonRpcMessage) -> Result<()>;
    async fn receive(&mut self) -> Result<JsonRpcMessage>;
    async fn close(&mut self) -> Result<()>;
    fn is_connected(&self) -> bool;
}

pub use http::HttpTransport;
pub use sse::SseTransport;
