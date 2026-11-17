use legalrust_database as db;
use legalrust_infrastructure::{init_tracing, AppState, Config};
use std::time::Duration;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let config = Config::from_env();
    if !config.jwt_secret_set {
        tracing::warn!("JWT_SECRET is not set; auth tokens are disabled until configured");
    }
    let pool = match tokio::time::timeout(Duration::from_secs(3), db::connect(&config.database_url))
        .await
    {
        Ok(Ok(p)) => {
            tracing::info!("database connected; running migrations");
            if let Err(e) = db::migrate(&p).await {
                tracing::error!("migration failed: {e}");
            }
            Some(p)
        }
        _ => {
            tracing::warn!("database unreachable; serving without live pool (see /readyz)");
            None
        }
    };
    let state = AppState {
        version: "0.1.0",
        pool,
    };
    let app = legalrust_server::build_router(state);
    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!("listening on {}", config.port);
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
        })
        .await?;
    Ok(())
}
