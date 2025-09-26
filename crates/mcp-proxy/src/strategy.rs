//! Transport strategy for connecting to remote MCP servers using rmcp SDK

use crate::error::{ProxyError, Result};
use rmcp::service::{serve_client, ServiceExt, DynService, RoleClient};

#[derive(Debug, Clone)]
pub enum TransportStrategy {
    HttpFirst,
    SseFirst,
    HttpOnly,
    SseOnly,
}

#[derive(Debug, Clone)]
pub enum TransportType {
    Http,
    Sse,
}

impl Default for TransportStrategy {
    fn default() -> Self {
        TransportStrategy::SseFirst
    }
}

/// Create a remote service using the specified transport strategy
///
/// Note: HTTP and SSE transports are not yet implemented in this version.
/// This is a placeholder that will be completed once we have proper rmcp transport imports.
pub async fn create_remote_service(
    server_url: &str,
    strategy: TransportStrategy,
    headers: &[String],
) -> Result<(Box<dyn DynService<RoleClient>>, TransportType)> {
    match strategy {
        TransportStrategy::HttpFirst => {
            // Try HTTP first, fallback to SSE
            match try_http_connection(server_url, headers).await {
                Ok((service, transport_type)) => Ok((service, transport_type)),
                Err(_) => {
                    tracing::warn!("HTTP connection failed, trying SSE");
                    try_sse_connection(server_url).await
                }
            }
        }
        TransportStrategy::SseFirst => {
            // Try SSE first, fallback to HTTP
            match try_sse_connection(server_url).await {
                Ok((service, transport_type)) => Ok((service, transport_type)),
                Err(_) => {
                    tracing::warn!("SSE connection failed, trying HTTP");
                    try_http_connection(server_url, headers).await
                }
            }
        }
        TransportStrategy::HttpOnly => {
            try_http_connection(server_url, headers).await
        }
        TransportStrategy::SseOnly => {
            try_sse_connection(server_url).await
        }
    }
}

/// Try to establish an HTTP connection
///
/// TODO: Implement proper HTTP transport once rmcp HTTP transport is available
async fn try_http_connection(
    server_url: &str,
    _headers: &[String],
) -> Result<(Box<dyn DynService<RoleClient>>, TransportType)> {
    tracing::debug!("Attempting HTTP connection to: {}", server_url);

    // TODO: Once rmcp HTTP transport is properly imported, implement:
    // let mut http_transport = rmcp::transport::Http::new(server_url)?;
    //
    // // Apply custom headers
    // for header in headers {
    //     if let Some((key, value)) = header.split_once(':') {
    //         http_transport = http_transport.with_header(key.trim(), value.trim());
    //     }
    // }
    //
    // let service = serve_client((), http_transport).await?;
    // Ok((service.into_dyn(), TransportType::Http))

    Err(ProxyError::Transport("HTTP transport not yet implemented".into()))
}

/// Try to establish an SSE connection
///
/// TODO: Implement proper SSE transport once rmcp SSE transport is available
async fn try_sse_connection(
    server_url: &str,
) -> Result<(Box<dyn DynService<RoleClient>>, TransportType)> {
    tracing::debug!("Attempting SSE connection to: {}", server_url);

    // TODO: Once rmcp SSE transport is properly imported, implement:
    // let sse_transport = rmcp::transport::Sse::new(server_url)?;
    // let service = serve_client((), sse_transport).await?;
    // Ok((service.into_dyn(), TransportType::Sse))

    Err(ProxyError::Transport("SSE transport not yet implemented".into()))
}
