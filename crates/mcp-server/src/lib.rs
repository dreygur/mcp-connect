//! MCP Server - Provides STDIO interface for IDE/LLM communication
//!
//! This crate implements an MCP server that communicates with clients (IDEs/LLMs)
//! via standard input/output streams using the MCP protocol.

pub mod transport;
pub mod server;
pub mod error;
pub mod types;

pub use server::McpServer;
pub use error::{ServerError, Result};
pub use transport::{Transport, StdioTransport};
pub use types::*;
