use anyhow::Result;
use clap::{Parser, Subcommand};
use rmcp::{
    ServiceExt, Service, RoleClient,
    model::{
        CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation,
        ListToolsRequestParam, ListResourcesRequestParam, ReadResourceRequestParam,
        CallToolResult, ToolInfo, ResourceInfo, ResourceContents
    },
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
        TokioChildProcess, child_process::ConfigureCommandExt, sse_client::SseClientTransport
    },
};
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use reqwest::{Client, header};
use url::Url;

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

    // let transport = StreamableHttpClientTransport::from_uri(url);
    let mut headers = header::HeaderMap::new();
    let authorization = format!("Bearer {}", token).to_string();
    println!("{}", authorization.clone());
    headers.insert(header::AUTHORIZATION, header::HeaderValue::from_str(&authorization).unwrap());
    let client = Client::builder().default_headers(headers).build().unwrap();
    let transport = StreamableHttpClientTransport::with_client(client, StreamableHttpClientTransportConfig::with_uri(url));
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
