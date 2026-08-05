use hammer_editor_mcp_server::mcp;

use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    hammer_editor_mcp_server::init_logging();

    let server_url =
        std::env::var("HAMMER_SERVER_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    info!("Connecting to Hammer sync server at: {}", server_url);

    mcp::run_loop(server_url).await
}
