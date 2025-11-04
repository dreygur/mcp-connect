# API Reference

Complete API reference for MCP Connect's programmatic interfaces.

## Core Types

### ConnectConfig

Main configuration structure for MCP Connect.

```rust
pub struct ConnectConfig {
    pub schema: Option<String>,
    pub version: String,
    pub env_file: Option<String>,
    pub servers: HashMap<String, ServerConfig>,
    pub routing: Option<RoutingConfig>,
    pub registries: Option<HashMap<String, RegistryConfig>>,
    pub default_registry: Option<String>,
}
```

### ServerConfig

Configuration for a single MCP server.

```rust
pub struct ServerConfig {
    pub name: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub remote: RemoteConfig,
    pub timeout: Option<u64>,
    pub retry_attempts: Option<u32>,
    pub fallbacks: Option<Vec<String>>,
}
```

### RemoteConfig

Remote transport configuration.

```rust
pub struct RemoteConfig {
    pub transport_type: RemoteTransportType,
    pub url: String,
    pub headers: Option<Vec<KeyValue>>,
}
```

### RegistryConfig

Custom registry source configuration.

```rust
pub struct RegistryConfig {
    pub base_url: String,
    pub api_version: Option<String>,
}
```

## Configuration Management

### ConfigManager

Manages loading, saving, and validating configuration files.

```rust
use mcp_config::ConfigManager;

// Create manager
let manager = ConfigManager::new()?;

// Load configuration
let mut manager = ConfigManager::new()?;
let config = manager.load()?;

// Save configuration
manager.save(&config)?;

// Check if config exists
if manager.exists() {
    // ...
}
```

### Methods

#### `new() -> Result<Self>`

Create a new ConfigManager with default config path.

```rust
let manager = ConfigManager::new()?;
```

#### `with_path<P: AsRef<Path>>(path: P) -> Result<Self>`

Create a ConfigManager with a specific config file path.

```rust
let manager = ConfigManager::with_path("/custom/path/.mcp-connect.json")?;
```

#### `load() -> Result<ConnectConfig>`

Load configuration from file with environment variable substitution.

```rust
let mut manager = ConfigManager::new()?;
let config = manager.load()?;
```

#### `save(&self, config: &ConnectConfig) -> Result<()>`

Save configuration to file.

```rust
manager.save(&config)?;
```

#### `exists(&self) -> bool`

Check if configuration file exists.

```rust
if manager.exists() {
    // ...
}
```

#### `init(&self) -> Result<ConnectConfig>`

Create a new configuration file with default settings.

```rust
let config = manager.init()?;
```

## Registry Client

### RegistryClient

Client for interacting with MCP registries.

```rust
use mcp_registry::RegistryClient;

// Create default client (official registry)
let client = RegistryClient::new()?;

// Create client with custom base URL
let client = RegistryClient::with_base_url(
    "https://registry.example.com",
    Some("v1")
)?;
```

### Methods

#### `new() -> Result<Self>`

Create a new registry client with default settings (official MCP registry).

```rust
let client = RegistryClient::new()?;
```

#### `with_base_url(base_url: &str, api_version: Option<&str>) -> Result<Self>`

Create a registry client with a custom base URL.

```rust
let client = RegistryClient::with_base_url(
    "https://registry.example.com",
    Some("v1")
)?;
```

#### `fetch_servers_page(&self, cursor: Option<String>, limit: usize, search_query: Option<&str>) -> Result<(Vec<ServerResponse>, Option<String>)>`

Fetch a page of servers from the registry.

```rust
let (servers, next_cursor) = client.fetch_servers_page(None, 50, None).await?;
```

#### `get_server(&self, name: &str, version: &str) -> Result<ServerResponse>`

Get detailed information about a specific server.

```rust
let server = client.get_server("modelcontextprotocol/github-mcp-server", "latest").await?;
```

## MCP Client

### McpRemoteClient

Client for connecting to remote MCP servers.

```rust
use mcp_client::McpRemoteClient;
use mcp_client::transport::TransportConfig;

// Create client with endpoint
let client = McpRemoteClient::new(
    "https://api.example.com/mcp".to_string(),
    vec![]
)?;

// Create with custom config
let config = TransportConfig {
    endpoint: "https://api.example.com/mcp".to_string(),
    timeout: Duration::from_secs(30),
    retry_attempts: 3,
    retry_delay: Duration::from_millis(1000),
    headers: HashMap::new(),
    auth_token: None,
    user_agent: Some("custom-client/1.0".to_string()),
};

let client = McpRemoteClient::new_with_config(config, vec![])?;
```

### Methods

#### `connect(&mut self) -> Result<()>`

Establish connection to the remote server.

```rust
client.connect().await?;
```

#### `initialize(&self) -> Result<InitializeResult>`

Initialize the MCP protocol with the server.

```rust
let init_result = client.initialize().await?;
```

#### `send_request(&mut self, request: &str) -> Result<String>`

Send a JSON-RPC request and wait for response.

```rust
let request = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
let response = client.send_request(request).await?;
```

#### `list_tools(&self) -> Result<Value>`

List available tools from the server.

```rust
let tools = client.list_tools().await?;
```

#### `list_resources(&self) -> Result<Value>`

List available resources from the server.

```rust
let resources = client.list_resources().await?;
```

#### `disconnect(&mut self) -> Result<()>`

Disconnect from the server.

```rust
client.disconnect().await?;
```

## Error Types

### McpError

Main error type for MCP operations.

```rust
pub enum McpError {
    Transport(String),
    Protocol(String),
    Serialization(serde_json::Error),
    Io(std::io::Error),
    Connection(String),
    Timeout,
    Auth(String),
}
```

### Usage

```rust
use mcp_types::Result;

fn example() -> Result<()> {
    // Operations that return Result<T>
    Ok(())
}
```

## Transport Configuration

### TransportConfig

Configuration for transport connections.

```rust
pub struct TransportConfig {
    pub endpoint: String,
    pub timeout: Duration,
    pub retry_attempts: u32,
    pub retry_delay: Duration,
    pub headers: HashMap<String, String>,
    pub auth_token: Option<String>,
    pub user_agent: Option<String>,
}
```

### Builder Methods

```rust
let config = TransportConfig {
    endpoint: "https://api.example.com/mcp".to_string(),
    timeout: Duration::from_secs(30),
    retry_attempts: 3,
    retry_delay: Duration::from_millis(1000),
    headers: HashMap::new(),
    auth_token: None,
    user_agent: None,
}
.with_bearer_token("token".to_string())
.with_api_key("X-API-Key".to_string(), "key".to_string());
```

## Examples

### Load and Validate Configuration

```rust
use mcp_config::ConfigManager;

let mut manager = ConfigManager::new()?;
let config = manager.load()?;

// Validate
mcp_config::ConfigManager::validate(&config)?;

// Use configuration
for (name, server) in config.servers {
    println!("Server: {}", name);
}
```

### Connect to Remote Server

```rust
use mcp_client::McpRemoteClient;

let mut client = McpRemoteClient::new(
    "https://api.example.com/mcp".to_string(),
    vec![]
)?;

client.connect().await?;
let init_result = client.initialize().await?;
println!("Connected to: {}", init_result.server_info.name);

let tools = client.list_tools().await?;
println!("Available tools: {}", tools);

client.disconnect().await?;
```

### Search Registry

```rust
use mcp_registry::RegistryClient;

let client = RegistryClient::new()?;
let (servers, _) = client.fetch_servers_page(None, 10, Some("github")).await?;

for server_response in servers {
    println!("Server: {}", server_response.server.name);
}
```

## See Also

- [Configuration Reference](configuration.md) - Configuration file structure
- [Advanced Usage](advanced.md) - Advanced programming examples
- [Architecture Documentation](../ARCHITECTURE.md) - System architecture

