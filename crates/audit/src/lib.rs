use serde::Serialize;
use sqlx::Row;
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

/// Hash-chained audit trail (F-D22-026): every entry commits to the previous
/// entry's hash, so deletion or silent modification breaks verification.
/// Genesis tip for tenants with no chained row yet.
pub const GENESIS_HASH: &str = "GENESIS";

/// Canonical chain hash. Field order is part of the protocol; verify_chain
/// recomputes exactly this over stored row fields (timestamps excluded so
/// formatting can never desync writers and verifiers).
pub fn chain_hash(
    prev: &str,
    tenant: &str,
    actor: Option<&str>,
    action: &str,
    entity: &str,
    entity_id: i64,
) -> String {
    let canonical = format!(
        "{}|{}|{}|{}|{}|{}",
        prev,
        tenant,
        actor.unwrap_or(""),
        action,
        entity,
        entity_id
    );
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(canonical.as_bytes());
    hex::encode(h.finalize())
}

/// One row for chain verification (id-ascending order).
#[derive(Debug, Clone)]
pub struct ChainRow {
    pub id: i64,
    pub tenant: String,
    pub actor: Option<String>,
    pub action: String,
    pub entity: String,
    pub entity_id: i64,
    pub before: Option<String>,
    pub after: Option<String>,
}

/// Verify chains (rows must be id-ascending). Tips are tracked per tenant so
/// a shared store holding several tenants still verifies. Legacy rows without
/// hashes (written before chaining) are skipped and counted; any broken link
/// or recomputation mismatch returns the offending row id.
pub fn verify_chain(rows: &[ChainRow]) -> Result<(usize, usize), i64> {
    let mut expected: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
    let mut checked = 0usize;
    let mut legacy = 0usize;
    for r in rows {
        match (&r.before, &r.after) {
            (None, None) => {
                legacy += 1;
                expected.remove(r.tenant.as_str());
                continue;
            }
            (Some(b), Some(a)) => {
                let want_prev = expected
                    .get(r.tenant.as_str())
                    .map(|x| x.as_str())
                    .unwrap_or(GENESIS_HASH);
                if b != want_prev {
                    return Err(r.id);
                }
                let recomputed = chain_hash(
                    b,
                    &r.tenant,
                    r.actor.as_deref(),
                    &r.action,
                    &r.entity,
                    r.entity_id,
                );
                if &recomputed != a {
                    return Err(r.id);
                }
                checked += 1;
                expected.insert(r.tenant.as_str(), a.clone());
            }
            _ => return Err(r.id),
        }
    }
    Ok((checked, legacy))
}

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
    let mut tx = pool.begin().await?;
    let prev: Option<String> = sqlx::query(
        "SELECT after_hash FROM audit_events WHERE tenant_id=? ORDER BY id DESC LIMIT 1 FOR UPDATE",
    )
    .bind(&e.tenant_id)
    .fetch_optional(&mut *tx)
    .await?
    .map(|x| x.get::<Option<String>, _>("after_hash"))
    .unwrap_or(None);
    let before = prev.as_deref().unwrap_or(GENESIS_HASH);
    let r = sqlx::query("INSERT INTO audit_events (tenant_id, actor_id, action, entity, entity_id, ip, before_hash) VALUES (?,?,?,?,?,?,?)")
        .bind(&e.tenant_id).bind(&e.actor_id).bind(&e.action).bind(&e.entity).bind(e.entity_id).bind(&e.ip).bind(before)
        .execute(&mut *tx).await?;
    let id = r.last_insert_id();
    let after = chain_hash(
        before,
        &e.tenant_id,
        e.actor_id.as_deref(),
        &e.action,
        &e.entity,
        e.entity_id,
    );
    sqlx::query("UPDATE audit_events SET after_hash=? WHERE id=?")
        .bind(&after)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
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

    fn chain(tenant: &str, n: i64) -> Vec<ChainRow> {
        let mut rows = Vec::new();
        let mut prev = GENESIS_HASH.to_string();
        for i in 0..n {
            let after = chain_hash(&prev, tenant, Some("u1"), "matter.created", "matter", i);
            rows.push(ChainRow {
                id: i + 1,
                tenant: tenant.into(),
                actor: Some("u1".into()),
                action: "matter.created".into(),
                entity: "matter".into(),
                entity_id: i,
                before: Some(prev.clone()),
                after: Some(after.clone()),
            });
            prev = after;
        }
        rows
    }

    #[test]
    fn chain_hash_deterministic_and_order_sensitive() {
        let a = chain_hash("GENESIS", "t", Some("u"), "m.created", "matter", 1);
        assert_eq!(
            a,
            chain_hash("GENESIS", "t", Some("u"), "m.created", "matter", 1)
        );
        assert_ne!(
            a,
            chain_hash("GENESIS", "t", Some("u"), "m.created", "matter", 2)
        );
        assert_ne!(
            a,
            chain_hash("OTHER", "t", Some("u"), "m.created", "matter", 1)
        );
        assert_ne!(
            a,
            chain_hash("GENESIS", "t", None, "m.created", "matter", 1)
        );
    }

    #[test]
    fn verify_chain_ok_and_counts() {
        let rows = chain("t-1", 3);
        assert_eq!(verify_chain(&rows), Ok((3, 0)));
    }

    #[test]
    fn verify_chain_detects_tamper() {
        let mut rows = chain("t-1", 3);
        rows[1].action = "matter.deleted".into();
        assert_eq!(verify_chain(&rows), Err(2));
        let mut rows = chain("t-1", 3);
        rows[2].before = Some("FORGED".into());
        assert_eq!(verify_chain(&rows), Err(3));
    }

    #[test]
    fn verify_chain_skips_legacy_prefix() {
        let mut rows = vec![ChainRow {
            id: 1,
            tenant: "t-1".into(),
            actor: None,
            action: "matter.created".into(),
            entity: "matter".into(),
            entity_id: 9,
            before: None,
            after: None,
        }];
        rows.extend(chain("t-1", 2).into_iter().map(|mut r| {
            r.id += 1;
            r
        }));
        // Re-link the first chained row to GENESIS (legacy prefix has no tip).
        rows[1].before = Some(GENESIS_HASH.into());
        rows[1].after = Some(chain_hash(
            GENESIS_HASH,
            "t-1",
            Some("u1"),
            "matter.created",
            "matter",
            0,
        ));
        rows[2].before = rows[1].after.clone();
        rows[2].after = Some(chain_hash(
            rows[1].after.as_deref().unwrap(),
            "t-1",
            Some("u1"),
            "matter.created",
            "matter",
            1,
        ));
        assert_eq!(verify_chain(&rows), Ok((2, 1)));
    }

    #[test]
    fn verify_chain_rejects_half_hashed_row() {
        let mut rows = chain("t-1", 1);
        rows[0].after = None;
        assert_eq!(verify_chain(&rows), Err(1));
    }

    #[test]
    fn verify_chain_tracks_tips_per_tenant() {
        let mut a = chain("t-a", 2);
        let mut b = chain("t-b", 2);
        // Interleave by id like a shared store would return.
        let mut rows = vec![a.remove(0), b.remove(0), a.remove(0), b.remove(0)];
        rows[0].id = 1;
        rows[1].id = 2;
        rows[2].id = 3;
        rows[3].id = 4;
        assert_eq!(verify_chain(&rows), Ok((4, 0)));
        // Tamper one tenant's row: only that id reported.
        rows[2].action = "matter.deleted".into();
        assert_eq!(verify_chain(&rows), Err(3));
    }
}
