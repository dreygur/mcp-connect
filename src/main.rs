use anyhow::Result;
use rmcp::{
    model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation}, transport::{streamable_http_client::StreamableHttpClient, StreamableHttpClientTransport}, ServiceExt
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
        id: "rmcp".to_string(),
        name: "rmcp".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: ClientCapabilities {
            call_tool: true,
            ..Default::default()
        },
    };
let client_info_provider = ClientInfoProvider::new(client_info);
let client = StreamableHttpClient::new(transport, client_info_provider);
    Ok(())
}
