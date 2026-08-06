use hammer_editor_mcp_server::config::CliArgs;
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    hammer_editor_mcp_server::init_logging();
    let args = CliArgs::parse();
    tracing::info!("Connecting to Hammer sync server at: {}", args.server_url);
    hammer_editor_mcp_server::mcp::run_loop(args).await
}
