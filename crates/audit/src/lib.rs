use serde::Serialize;
#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    pub tenant_id: String,
    pub actor_id: Option<String>,
    pub action: String,
    pub entity: String,
    pub entity_id: i64,
    pub ip: Option<String>,
}
pub async fn record(pool: &sqlx::MySqlPool, e: &AuditEvent) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO audit_events (tenant_id, actor_id, action, entity, entity_id, ip) VALUES (?,?,?,?,?,?)")
        .bind(&e.tenant_id).bind(&e.actor_id).bind(&e.action).bind(&e.entity).bind(e.entity_id).bind(&e.ip)
        .execute(pool).await.map(|_| ())
}
