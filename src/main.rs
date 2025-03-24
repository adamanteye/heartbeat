use std::sync::Arc;
use tokio::sync::Mutex;

use clap::Parser;
use rusqlite::Connection;

mod config;
mod view;

use config::Config;
use view::AppState;

#[derive(Parser, Debug)]
#[command(version)]
pub struct Args {
    #[arg(short, long, default_value = "/etc/heartbeat.toml")]
    pub config: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = env_logger::Env::default().filter_or("HEARTBEAT_LOG_LEVEL", "info");
    env_logger::init_from_env(env);
    let args = Args::parse();
    let config_path = std::path::Path::new(args.config.as_str());
    let config = std::fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(config.as_str())?;
    log::info!("connecting to sqlite database");
    let conn = Connection::open(config.store.data)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS record (
            id INTEGER PRIMARY KEY,
            time TEXT,
            source TEXT,
            event TEXT,
            note TEXT
        )",
        (),
    )?;
    let conn = Arc::new(Mutex::new(conn));
    let token = {
        if let Ok(t) = std::env::var("HEARTBEAT_TOKEN") {
            t
        } else {
            config.store.token
        }
    };
    let token = Arc::new(token);
    let state = AppState { conn, token };
    let app = view::router(state);
    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", config.listen.address, config.listen.port))
            .await?;
    log::info!(
        "listening at {}:{}",
        config.listen.address,
        config.listen.port
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    log::info!("received terminate signal");
}
