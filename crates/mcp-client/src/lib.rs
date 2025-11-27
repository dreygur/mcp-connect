pub mod client;
pub mod transport;
pub mod error;
pub mod auth;
pub mod oauth_discovery;

pub use client::McpRemoteClient;
pub use error::ClientError;
pub use transport::{HttpTransport, StdioTransport, TcpTransport};
pub use auth::{
    OAuthClient, OAuthClientConfig, ClientToken, TokenCache, CachedToken,
    TokenProvider, StaticTokenProvider, OAuthTokenProvider,
};
pub use oauth_discovery::{OAuthDiscovery, OAuthMetadata, OAuthRequirement};
