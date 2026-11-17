use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    pub tenant_id: String,
    pub actor_id: Option<String>,
    pub action: String,
    pub entity: String,
    pub entity_id: i64,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditError {
    EmptyTenant,
    InvalidAction(String),
}

impl fmt::Display for AuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditError::EmptyTenant => write!(f, "tenant_id must not be empty"),
            AuditError::InvalidAction(a) => {
                write!(f, "action must be namespaced with a dot: {a}")
            }
        }
    }
}

impl std::error::Error for AuditError {}

impl AuditEvent {
    pub fn validate(&self) -> Result<(), AuditError> {
        if self.tenant_id.trim().is_empty() {
            return Err(AuditError::EmptyTenant);
        }
        if !self.action.contains('.') {
            return Err(AuditError::InvalidAction(self.action.clone()));
        }
        Ok(())
    }
}

pub async fn record(pool: &sqlx::MySqlPool, e: &AuditEvent) -> Result<(), sqlx::Error> {
    e.validate()
        .map_err(|err| sqlx::Error::Protocol(err.to_string()))?;
    sqlx::query("INSERT INTO audit_events (tenant_id, actor_id, action, entity, entity_id, ip) VALUES (?,?,?,?,?,?)")
        .bind(&e.tenant_id).bind(&e.actor_id).bind(&e.action).bind(&e.entity).bind(e.entity_id).bind(&e.ip)
        .execute(pool).await.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(tenant: &str, action: &str) -> AuditEvent {
        AuditEvent {
            tenant_id: tenant.into(),
            actor_id: None,
            action: action.into(),
            entity: "matter".into(),
            entity_id: 1,
            ip: None,
        }
    }

    #[test]
    fn valid_event_ok() {
        assert!(event("t-1", "matter.created").validate().is_ok());
    }

    #[test]
    fn empty_tenant_rejected() {
        assert_eq!(
            event("", "matter.created").validate(),
            Err(AuditError::EmptyTenant)
        );
        assert_eq!(
            event("   ", "matter.created").validate(),
            Err(AuditError::EmptyTenant)
        );
    }

    #[test]
    fn action_without_dot_rejected() {
        assert!(event("t-1", "created").validate().is_err());
    }
}
