//! HTML templates for OAuth authentication status pages.

pub fn success_page() -> &'static str {
    include_str!("templates/success.html")
}

pub fn error_page(error: &str, description: &str) -> String {
    include_str!("templates/error.html")
        .replace("{description}", description)
        .replace("{error}", error)
}
