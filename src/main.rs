use anyhow::Result;
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation},
    transport::{StreamableHttpClientTransport, streamable_http_client::StreamableHttpClient},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("info,{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let url = "https://api.githubcopilot.com/mcp";
    let token = "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";

    let transport = StreamableHttpClientTransport::from_uri(url);
    let client_info = ClientInfo {
        ..Default::default()
    };
    let client = client_info.serve(transport).await.inspect_err(|e| {
        tracing::error!("client error: {:?}", e);
    })?;
    let server_info = client.peer_info();
    tracing::info!("Connected to server: {server_info:#?}");
    Ok(())
}
