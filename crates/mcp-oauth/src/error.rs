pub type Result<T> = std::result::Result<T, OAuthError>;

#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("OAuth2 library error: {0}")]
    OAuth2(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("URL parsing error: {0}")]
    Url(#[from] url::ParseError),

    #[error("Token storage error: {0}")]
    TokenStorage(String),

    #[error("Browser launch error: {0}")]
    BrowserLaunch(String),

    #[error("Callback server error: {0}")]
    CallbackServer(String),

    #[error("Dynamic client registration failed: {0}")]
    ClientRegistration(String),

    #[error("PKCE verification failed: {0}")]
    PkceVerification(String),

    #[error("Token exchange failed: {0}")]
    TokenExchange(String),

    #[error("Token refresh failed: {0}")]
    TokenRefresh(String),

    #[error("Authentication timeout")]
    AuthTimeout,

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Missing required parameter: {0}")]
    MissingParameter(String),
}

impl<T> From<oauth2::RequestTokenError<reqwest::Error, T>> for OAuthError
where
    T: oauth2::ErrorResponseType + oauth2::ErrorResponse + 'static,
{
    fn from(err: oauth2::RequestTokenError<reqwest::Error, T>) -> Self {
        OAuthError::OAuth2(format!("{:?}", err))
    }
}

impl From<keyring::Error> for OAuthError {
    fn from(err: keyring::Error) -> Self {
        OAuthError::TokenStorage(err.to_string())
    }
}
