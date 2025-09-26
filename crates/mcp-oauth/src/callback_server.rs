use crate::{OAuthError, Result};
use crate::types::AuthorizationResponse;
use std::collections::HashMap;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn, error};
use warp::{Filter, Reply};

/// OAuth callback server for handling authorization code redirects
///
/// This server runs temporarily during the OAuth flow to receive the
/// authorization code from the OAuth provider's redirect.
pub struct CallbackServer {
    port: u16,
    sender: Arc<mpsc::UnboundedSender<AuthorizationResponse>>,
    receiver: mpsc::UnboundedReceiver<AuthorizationResponse>,
}

impl CallbackServer {
    /// Create a new callback server on the specified port
    ///
    /// # Arguments
    /// * `port` - Port to bind the server to, or 0 for automatic port selection
    ///
    /// # Returns
    /// New CallbackServer instance with the actual port it will bind to
    pub fn new(port: u16) -> Result<Self> {
        let (sender, receiver) = mpsc::unbounded_channel();

        Ok(Self {
            port,
            sender: Arc::new(sender),
            receiver,
        })
    }

    /// Get the port the server will bind to
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get the callback URL for this server
    pub fn callback_url(&self, host: &str) -> String {
        format!("http://{}:{}/callback", host, self.port)
    }

    /// Start the callback server and wait for the OAuth redirect
    ///
    /// # Arguments
    /// * `timeout_duration` - Maximum time to wait for the callback
    ///
    /// # Returns
    /// Authorization response containing the code and state
    pub async fn wait_for_callback(
        mut self,
        timeout_duration: Duration,
    ) -> Result<AuthorizationResponse> {
        info!("Starting OAuth callback server on port {}", self.port);

        // Clone sender for the warp handler
        let sender_clone = Arc::clone(&self.sender);

        // Create the callback route
        let callback_route = warp::path("callback")
            .and(warp::query::<HashMap<String, String>>())
            .map(move |params: HashMap<String, String>| {
                let sender = Arc::clone(&sender_clone);
                tokio::spawn(async move {
                    let _ = Self::handle_callback_simple(sender, params).await;
                });
                Self::callback_response()
            });

        // Create a success page route
        let success_route = warp::path("success")
            .map(|| {
                warp::reply::html(Self::success_page())
            });

        // Create the complete routes
        let routes = callback_route.or(success_route);

        // Start the server
        let server_addr: SocketAddr = ([127, 0, 0, 1], self.port).into();
        let (addr, server) = warp::serve(routes)
            .try_bind_ephemeral(server_addr)
            .map_err(|e| OAuthError::CallbackServer(format!("Failed to bind to port {}: {}", self.port, e)))?;

        // Update the actual port if we used 0 (auto-select)
        self.port = addr.port();
        info!("OAuth callback server listening on {}", addr);

        // Start server in background
        let server_handle = tokio::spawn(server);

        // Wait for either the callback or timeout
        let result = timeout(timeout_duration, self.receiver.recv()).await;

        // Shutdown the server
        server_handle.abort();

        match result {
            Ok(Some(auth_response)) => {
                info!("Received OAuth authorization response");
                debug!("Authorization response: {:?}", auth_response);
                Ok(auth_response)
            }
            Ok(None) => {
                error!("Callback server channel closed unexpectedly");
                Err(OAuthError::CallbackServer("Server channel closed".to_string()))
            }
            Err(_) => {
                warn!("OAuth authorization timed out after {:?}", timeout_duration);
                Err(OAuthError::AuthTimeout)
            }
        }
    }

    /// Simplified callback handler that doesn't return warp types
    async fn handle_callback_simple(
        sender: Arc<mpsc::UnboundedSender<AuthorizationResponse>>,
        params: HashMap<String, String>,
    ) -> Result<()> {
        debug!("Received OAuth callback with parameters: {:?}", params);

        // Check for error parameter first
        if let Some(error) = params.get("error") {
            let error_description = params.get("error_description")
                .map(|s| s.as_str())
                .unwrap_or("No description provided");

            error!("OAuth authorization error: {} - {}", error, error_description);
            return Err(OAuthError::CallbackServer(format!("Authorization error: {}", error)));
        }

        // Extract authorization code and state
        let code = params.get("code")
            .ok_or_else(|| {
                warn!("Missing authorization code in callback");
                OAuthError::CallbackServer("Missing authorization code".to_string())
            })?;

        let state = params.get("state")
            .ok_or_else(|| {
                warn!("Missing state parameter in callback");
                OAuthError::CallbackServer("Missing state parameter".to_string())
            })?;

        // Create authorization response
        let auth_response = AuthorizationResponse {
            code: code.clone(),
            state: state.clone(),
        };

        // Send the response through the channel
        if let Err(e) = sender.send(auth_response) {
            error!("Failed to send authorization response: {}", e);
            return Err(OAuthError::CallbackServer("Failed to process authorization".to_string()));
        }

        info!("Successfully processed OAuth callback");
        Ok(())
    }

    /// Generate callback response HTML
    fn callback_response() -> impl Reply {
        let success_page = r#"
            <html>
            <head>
                <title>Authorization Successful</title>
                <meta http-equiv="refresh" content="2;url=/success">
                <style>
                    body { font-family: Arial, sans-serif; text-align: center; margin-top: 50px; }
                    .success { color: #28a745; }
                    .loading { color: #6c757d; }
                </style>
            </head>
            <body>
                <h2 class="success">✅ Authorization Successful!</h2>
                <p class="loading">You can now close this window and return to the MCP Remote application.</p>
                <p><small>Redirecting to success page...</small></p>
            </body>
            </html>
        "#;

        warp::reply::html(success_page)
    }

    /// Generate HTML success page
    fn success_page() -> String {
        r#"
        <html>
        <head>
            <title>MCP Remote - Authorization Complete</title>
            <style>
                body { font-family: Arial, sans-serif; text-align: center; margin-top: 50px; }
                .success { color: #28a745; }
                .info { color: #17a2b8; }
                .container { max-width: 500px; margin: 0 auto; }
            </style>
        </head>
        <body>
            <div class="container">
                <h1 class="success">🎉 Authorization Complete!</h1>
                <p class="info">Your MCP Remote application has been successfully authorized.</p>
                <p>You can now close this browser window and return to your terminal.</p>
                <hr>
                <p><small>MCP Remote OAuth Callback Server</small></p>
            </div>
        </body>
        </html>
        "#.to_string()
    }

    /// Generate HTML error page
    fn error_page(error: &str, description: &str) -> String {
        format!(
            r#"
            <html>
            <head>
                <title>MCP Remote - Authorization Error</title>
                <style>
                    body {{ font-family: Arial, sans-serif; text-align: center; margin-top: 50px; }}
                    .error {{ color: #dc3545; }}
                    .info {{ color: #6c757d; }}
                    .container {{ max-width: 500px; margin: 0 auto; }}
                </style>
            </head>
            <body>
                <div class="container">
                    <h1 class="error">❌ Authorization Failed</h1>
                    <p class="error"><strong>Error:</strong> {}</p>
                    <p class="info">{}</p>
                    <p>Please return to your terminal and try again.</p>
                    <hr>
                    <p><small>MCP Remote OAuth Callback Server</small></p>
                </div>
            </body>
            </html>
            "#,
            html_escape(error),
            html_escape(description)
        )
    }
}

/// Simple HTML escaping for security
fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Find an available port for the callback server
///
/// # Arguments
/// * `preferred_port` - Preferred port to try first, or 0 for any available port
///
/// # Returns
/// An available port number
pub fn find_available_port(preferred_port: u16) -> Result<u16> {
    use std::net::TcpListener;

    if preferred_port != 0 {
        // Try the preferred port first
        if let Ok(listener) = TcpListener::bind(("127.0.0.1", preferred_port)) {
            let port = listener.local_addr()?.port();
            return Ok(port);
        }
    }

    // Find any available port
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("Hello World"), "Hello World");
        assert_eq!(html_escape("<script>alert('xss')</script>"),
                   "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
        assert_eq!(html_escape("AT&T"), "AT&amp;T");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }

    #[tokio::test]
    async fn test_find_available_port() {
        let port = find_available_port(0).unwrap();
        assert!(port > 0);
        assert!(port < 65535);
    }

    #[tokio::test]
    async fn test_callback_server_creation() {
        let server = CallbackServer::new(0).unwrap();
        assert!(server.callback_url("localhost").starts_with("http://localhost:"));
        assert!(server.callback_url("localhost").ends_with("/callback"));
    }
}
