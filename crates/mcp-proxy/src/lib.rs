//! MCP Proxy - Forwards requests between local MCP server and remote MCP client
//!
//! This crate implements a proxy that bridges local IDE/LLM clients (via STDIO)
//! with remote MCP servers (via HTTP/SSE), providing seamless bidirectional communication.

pub mod proxy;
pub mod error;
pub mod strategy;

pub use proxy::McpProxy;
pub use error::{ProxyError, Result};
pub use strategy::{TransportStrategy, TransportType};
