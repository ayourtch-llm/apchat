use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::cli::Cli;
use apchat_vty::{print_heart_red};
use crate::config::ClientConfig;
use apchat_policy::PolicyManager;
use crate::mspc::MspcChannel;
use crate::web::server::{WebServer, WebServerConfig};

/// Expand ~ to home directory
fn expand_tilde(path: &str) -> Result<PathBuf> {
    if path.starts_with("~/") {
        let home = std::env::var("HOME")
            .context("HOME environment variable not set")?;
        Ok(PathBuf::from(home).join(&path[2..]))
    } else if path == "~" {
        let home = std::env::var("HOME")
            .context("HOME environment variable not set")?;
        Ok(PathBuf::from(home))
    } else {
        Ok(PathBuf::from(path))
    }
}

/// Run the web server
pub async fn run_web_server(
    cli: &Cli,
    client_config: ClientConfig,
    work_dir: PathBuf,
    policy_manager: PolicyManager,
) -> Result<()> {
    // Parse bind address
    let addr: SocketAddr = format!("{}:{}", cli.web_bind, cli.web_port).parse()?;

    print_heart_red(&format!("🌐 Starting APChat web server..."), true);
    print_heart_red(&format!("   Address: {}", addr), true);
    print_heart_red(&format!("   Working directory: {}", work_dir.display()), true);

    // Determine web directory (relative to work_dir)
    let web_dir = work_dir.join("web");

    // Expand sessions directory path (handles ~ expansion)
    let sessions_dir = expand_tilde(&cli.sessions_dir)?;

    // Create web server config
    let config = WebServerConfig {
        bind_addr: addr,
        work_dir,
        client_config,
        policy_manager,
        web_dir: Some(web_dir),
        sessions_dir,
    };

    // Create and start server
    let server = WebServer::new(config, None);
    server.start().await?;

    Ok(())
}
