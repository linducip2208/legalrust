use legalrust_database as db;
use legalrust_infrastructure::{init_tracing, Config};
use legalrust_server::{FullState, Stores};
use legalrust_storage::LocalStore;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let config = Config::from_env();
    let secret = std::env::var("JWT_SECRET").unwrap_or_default().into_bytes();
    if secret.is_empty() {
        tracing::warn!("JWT_SECRET is not set; auth endpoints return 501 until configured");
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
            tracing::warn!("database unreachable; ephemeral dev store active (non-production)");
            None
        }
    };
    let (users, core, ops): (
        Arc<dyn legalrust_database::UserStore>,
        Arc<dyn legalrust_database::CoreStore>,
        Arc<dyn legalrust_database::OpsStore>,
    ) = match &pool {
        Some(p) => {
            let m = Arc::new(legalrust_database::mysql::MysqlStores::new(p.clone()));
            let o = Arc::new(legalrust_database::ops_mysql::MysqlOps::new(p.clone()));
            (m.clone(), m, o)
        }
        None => {
            let m = Arc::new(legalrust_database::memory::MemoryStores::default());
            let o = Arc::new(legalrust_database::ops_memory::MemoryOps::default());
            (m.clone(), m, o)
        }
    };
    let mut stores = Stores::new(
        users,
        core,
        ops,
        Arc::new(LocalStore::new(std::path::PathBuf::from("./var/objects"))),
        secret,
    );
    stores.integration_secret = std::env::var("INTEGRATION_SECRET")
        .unwrap_or_default()
        .into_bytes();
    let state = FullState {
        version: "0.1.0",
        pool,
        stores,
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
