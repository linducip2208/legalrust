use legalrust_database as db;
use legalrust_infrastructure::init_tracing;
use std::time::Duration;
use tokio::time::interval;

async fn run_once(pool: &sqlx::MySqlPool) {
    use legalrust_database::OpsStore;
    let ops = legalrust_database::ops_mysql::MysqlOps::new(pool.clone());
    let Some(job) = ops.job_claim().await.unwrap_or(None) else {
        return;
    };
    tracing::info!("claimed job {} kind={}", job.id, job.kind);
    let (ok, note): (bool, String) = match job.kind.as_str() {
        "notify.digest" => (true, "digest marked".into()),
        "ocr.process" => (
            false,
            "no OCR engine available: native libtesseract missing".into(),
        ),
        "webhook.dispatch" => match legalrust_integrations::process_webhook_job(&ops, &job).await {
            Ok(legalrust_integrations::DispatchOutcome::Delivered(code)) => {
                (true, format!("delivered HTTP {code}"))
            }
            Ok(legalrust_integrations::DispatchOutcome::RetryScheduled(secs)) => {
                (false, format!("retry scheduled in {secs}s"))
            }
            Ok(legalrust_integrations::DispatchOutcome::DeadLetter(reason)) => {
                (false, format!("dead-letter: {reason}"))
            }
            Err(e) => (false, format!("dispatch error: {e}")),
        },
        _ => (false, "no executor for kind".into()),
    };
    if let Err(e) = ops.job_done(job.id, ok, &note).await {
        tracing::error!("job_done failed: {e}");
    }
}

#[tokio::main]
async fn main() {
    init_tracing();
    tracing::info!("worker started (durable DB queue; no in-memory critical tasks)");
    let url = std::env::var("DATABASE_URL").unwrap_or_default();
    let pool = if url.is_empty() {
        None
    } else {
        match tokio::time::timeout(Duration::from_secs(3), db::connect(&url)).await {
            Ok(Ok(p)) => Some(p),
            _ => {
                tracing::warn!("database unreachable; worker idles");
                None
            }
        }
    };
    let mut tick = interval(Duration::from_secs(5));
    loop {
        tick.tick().await;
        match &pool {
            Some(p) => run_once(p).await,
            None => tracing::info!("worker heartbeat: no broker configured"),
        }
    }
}
