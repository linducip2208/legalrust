use legalrust_infrastructure::init_tracing;
use std::time::Duration;
#[tokio::main]
async fn main() {
    init_tracing();
    tracing::info!(
        "worker started (durable job wiring lands in Phase 3; no in-memory critical tasks)"
    );
    let mut tick = tokio::time::interval(Duration::from_secs(30));
    loop {
        tick.tick().await;
        tracing::info!("worker heartbeat: 0 due jobs (broker adapter pending configuration)");
    }
}
