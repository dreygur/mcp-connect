pub mod client;
pub mod transport;
pub mod error;
pub mod auth;
pub mod oauth_discovery;
mod templates;

pub use client::McpRemoteClient;
pub use error::ClientError;
pub use transport::{HttpTransport, StdioTransport, TcpTransport};
pub use auth::{
    OAuthClient, OAuthClientConfig, ClientToken,
    TokenProvider, StaticTokenProvider,
};
pub use oauth_discovery::{OAuthDiscovery, OAuthMetadata, OAuthRequirement};
