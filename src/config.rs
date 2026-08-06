//! Command-line configuration for the MCP client.

use clap::Parser;

/// MCP client for the Hammer Editor sync server.
///
/// Connect to a Hammer synchronization server and expose its API as
/// MCP (Model Context Protocol) tools.
#[derive(Parser, Debug, Clone)]
#[command(version, about)]
pub struct CliArgs {
    /// URL of the Hammer sync server (e.g. http://localhost:8080)
    #[arg(long, default_value = "http://localhost:8080", env = "HAMMER_SERVER_URL")]
    pub server_url: String,

    /// Email for automatic login
    #[arg(long, env = "HAMMER_EMAIL")]
    pub email: Option<String>,

    /// Password for automatic login
    #[arg(long, env = "HAMMER_PASSWORD")]
    pub password: Option<String>,

    /// Install ID (auto-generated UUID if not provided)
    #[arg(long, env = "HAMMER_INSTALL_ID")]
    pub install_id: Option<String>,

    /// Pre-set user ID (skips login if provided with email/password)
    #[arg(long, env = "HAMMER_USER_ID")]
    pub user_id: Option<i64>,

    /// Auth token (alternative to email/password login)
    #[arg(long, env = "HAMMER_AUTH_TOKEN")]
    pub auth_token: Option<String>,
}