pub mod client;
pub mod transport;
pub mod error;
pub mod auth;

pub use client::McpRemoteClient;
pub use error::ClientError;
pub use transport::{HttpTransport, StdioTransport, TcpTransport};
pub use auth::{OAuthClient, OAuthClientConfig, ClientToken};
