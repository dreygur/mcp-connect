//! MCP Client - Connects to remote MCP servers via HTTP/SSE
//!
//! This crate implements a client that can communicate with remote MCP servers
//! using HTTP POST requests and Server-Sent Events (SSE) for bidirectional communication.

pub mod transport;
pub mod client;
pub mod error;
pub mod types;

pub use client::McpClient;
pub use error::{ClientError, Result};
pub use transport::{Transport, HttpTransport, SseTransport};
pub use types::*;
