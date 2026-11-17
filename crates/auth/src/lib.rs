use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use legalrust_application::AppError;
use legalrust_database::{StoreError, UserStore};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("hash failed")]
    Hash,
    #[error("invalid credentials")]
    Invalid,
    #[error("password does not meet policy")]
    WeakPassword,
    #[error("token error")]
    Token,
}

/// Minimum accepted password length (self-audit hardening 2026-11-17:
/// raised from 8 to 12 to match NIST 800-63B guidance for user-chosen secrets).
pub const MIN_PASSWORD_LEN: usize = 12;
/// Session lifetime in hours (self-audit hardening 2026-11-17: reduced 24h -> 8h
/// so a stolen JWT is useful for at most one work shift).
pub const SESSION_TTL_HOURS: i64 = 8;
/// Minimum JWT secret length in bytes (256-bit key material).
pub const MIN_SECRET_LEN: usize = 32;

/// Shared password-policy gate used by `register` (and mirrored by
/// `password_change` callers). Length-only on purpose: complexity rules push
/// users toward predictable `P@ssw0rd1!` patterns; length is what matters.
pub fn validate_password(pw: &str) -> Result<(), AuthError> {
    if pw.len() < MIN_PASSWORD_LEN {
        return Err(AuthError::WeakPassword);
    }
    Ok(())
}
pub fn hash_password(plain: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(plain.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| AuthError::Hash)
}
pub fn verify_password(hash: &str, plain: &str) -> Result<(), AuthError> {
    let parsed = PasswordHash::new(hash).map_err(|_| AuthError::Invalid)?;
    Argon2::default()
        .verify_password(plain.as_bytes(), &parsed)
        .map_err(|_| AuthError::Invalid)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub org: String,
    pub roles: Vec<String>,
    pub sid: String,
    pub exp: usize,
}
pub fn issue_token(secret: &[u8], claims: &Claims) -> Result<String, AuthError> {
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        claims,
        &jsonwebtoken::EncodingKey::from_secret(secret),
    )
    .map_err(|_| AuthError::Token)
}
pub fn validate_token(secret: &[u8], token: &str) -> Result<Claims, AuthError> {
    jsonwebtoken::decode::<Claims>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(secret),
        &jsonwebtoken::Validation::default(),
    )
    .map(|d| d.claims)
    .map_err(|_| AuthError::Token)
}
pub fn recovery_codes(n: usize) -> Vec<String> {
    use rand_core::RngCore;
    (0..n)
        .map(|_| {
            let mut b = [0u8; 10];
            OsRng.fill_bytes(&mut b);
            hex::encode(b)
        })
        .collect()
}
fn map_store(e: StoreError) -> AppError {
    match e {
        StoreError::NotFound => AppError::NotFound("record".into()),
        StoreError::Conflict(m) => AppError::Conflict(m),
        StoreError::Db(m) => AppError::Internal(anyhow::Error::msg(m)),
    }
}
#[derive(Clone, Serialize)]
pub struct Session {
    pub user_id: String,
    pub org_id: String,
    pub token: String,
    pub session_id: String,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("user_id", &self.user_id)
            .field("org_id", &self.org_id)
            .field("token", &"[redacted]")
            .field("session_id", &self.session_id)
            .finish()
    }
}

fn expiry() -> (chrono::NaiveDateTime, usize) {
    let dt = chrono::Utc::now() + chrono::Duration::hours(SESSION_TTL_HOURS);
    (dt.naive_utc(), dt.timestamp() as usize)
}

pub fn check_secret(secret: &[u8]) -> Result<(), AppError> {
    if secret.is_empty() {
        return Err(AppError::BlockedExternal(
            "JWT_SECRET not configured".into(),
        ));
    }
    if secret.len() < MIN_SECRET_LEN {
        return Err(AppError::BlockedExternal(
            "JWT_SECRET too short; provide at least 32 bytes".into(),
        ));
    }
    Ok(())
}
pub async fn register(
    store: &dyn UserStore,
    secret: &[u8],
    org: &str,
    email: &str,
    name: &str,
    pw: &str,
) -> Result<Session, AppError> {
    check_secret(secret)?;
    if !email.contains('@') {
        return Err(AppError::Validation("email invalid".into()));
    }
    validate_password(pw).map_err(|_| AppError::Validation("password invalid".into()))?;
    let hash = hash_password(pw).map_err(|_| AppError::Internal(anyhow::Error::msg("hash")))?;
    let (org_id, uid) = store
        .create_org_owner(org, email, name, &hash)
        .await
        .map_err(map_store)?;
    let (exp_dt, exp) = expiry();
    let sid = store
        .create_session(&uid, exp_dt)
        .await
        .map_err(map_store)?;
    let roles = store.user_roles(&uid).await.map_err(map_store)?;
    let token = issue_token(
        secret,
        &Claims {
            sub: uid.clone(),
            org: org_id.clone(),
            roles,
            sid: sid.clone(),
            exp,
        },
    )
    .map_err(|e| AppError::Internal(anyhow::Error::msg(e.to_string())))?;
    Ok(Session {
        user_id: uid,
        org_id,
        token,
        session_id: sid,
    })
}
pub async fn login(
    store: &dyn UserStore,
    secret: &[u8],
    email: &str,
    pw: &str,
) -> Result<Session, AppError> {
    check_secret(secret)?;
    let u = store
        .find_by_email(email)
        .await
        .map_err(map_store)?
        .ok_or(AppError::Unauthorized)?;
    if u.status != "active" {
        return Err(AppError::Unauthorized);
    }
    verify_password(&u.pw_hash, pw).map_err(|_| AppError::Unauthorized)?;
    let (exp_dt, exp) = expiry();
    let sid = store
        .create_session(&u.id, exp_dt)
        .await
        .map_err(map_store)?;
    let roles = store.user_roles(&u.id).await.map_err(map_store)?;
    let token = issue_token(
        secret,
        &Claims {
            sub: u.id.clone(),
            org: u.org_id.clone(),
            roles,
            sid: sid.clone(),
            exp,
        },
    )
    .map_err(|e| AppError::Internal(anyhow::Error::msg(e.to_string())))?;
    Ok(Session {
        user_id: u.id,
        org_id: u.org_id,
        token,
        session_id: sid,
    })
}
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// In-memory brute-force guard keyed by IP (or any caller-chosen key).
#[derive(Debug)]
pub struct LoginGuard {
    max_attempts: u32,
    window: Duration,
    lock_for: Duration,
    failures: HashMap<String, Vec<Instant>>,
    locked_until: HashMap<String, Instant>,
}

impl Default for LoginGuard {
    fn default() -> Self {
        Self::new(5, 300, 900)
    }
}

impl LoginGuard {
    pub fn new(max_attempts: u32, window_secs: u64, lock_secs: u64) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            window: Duration::from_secs(window_secs),
            lock_for: Duration::from_secs(lock_secs),
            failures: HashMap::new(),
            locked_until: HashMap::new(),
        }
    }

    fn prune(&mut self, key: &str, now: Instant) {
        if let Some(v) = self.failures.get_mut(key) {
            v.retain(|t| now.duration_since(*t) < self.window);
        }
    }

    /// Record a failure for `key`. Returns `true` when the key is (now) locked.
    pub fn register_failure(&mut self, key: &str) -> bool {
        let now = Instant::now();
        if self.is_locked(key) {
            return true;
        }
        self.prune(key, now);
        let entry = self.failures.entry(key.to_string()).or_default();
        entry.push(now);
        if entry.len() as u32 >= self.max_attempts {
            self.locked_until
                .insert(key.to_string(), now + self.lock_for);
            return true;
        }
        false
    }

    /// Returns `true` while the key remains locked. Expired locks are evicted.
    pub fn is_locked(&mut self, key: &str) -> bool {
        let now = Instant::now();
        match self.locked_until.get(key).copied() {
            Some(until) if now < until => true,
            Some(_) => {
                self.locked_until.remove(key);
                self.failures.remove(key);
                false
            }
            None => false,
        }
    }

    /// Clear failures and any lock for `key`.
    pub fn reset(&mut self, key: &str) {
        self.failures.remove(key);
        self.locked_until.remove(key);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use legalrust_database::memory::MemoryStores;
    #[test]
    fn password_roundtrip() {
        let h = hash_password("s3cret-pass").unwrap();
        assert!(verify_password(&h, "s3cret-pass").is_ok());
        assert!(verify_password(&h, "wrong").is_err());
    }
    #[test]
    fn jwt_roundtrip() {
        let c = Claims {
            sub: "u1".into(),
            org: "o1".into(),
            roles: vec!["notary".into()],
            sid: "s1".into(),
            exp: 9_999_999_999,
        };
        let t = issue_token(b"test-secret", &c).unwrap();
        let back = validate_token(b"test-secret", &t).unwrap();
        assert_eq!(back.sub, "u1");
        assert!(validate_token(b"other", &t).is_err());
    }
    #[tokio::test]
    async fn register_then_login_memory() {
        let s = MemoryStores::default();
        let secret = b"0123456789abcdef0123456789abcdef";
        let r = register(&s, secret, "Acme", "a@x.id", "Ayu", "password1-2-3")
            .await
            .unwrap();
        assert_eq!(r.org_id.chars().count(), 5);
        let l = login(&s, secret, "a@x.id", "password1-2-3").await.unwrap();
        assert_eq!(l.org_id, r.org_id);
        assert!(login(&s, secret, "a@x.id", "nope-nope-nope").await.is_err());
        assert!(register(&s, b"", "Acme", "b@x.id", "Bud", "password1-2-3")
            .await
            .is_err());
    }
    #[test]
    fn guard_locks_after_max_attempts() {
        let mut g = LoginGuard::new(3, 300, 900);
        assert!(!g.register_failure("1.1.1.1"));
        assert!(!g.register_failure("1.1.1.1"));
        assert!(g.register_failure("1.1.1.1"));
        assert!(g.is_locked("1.1.1.1"));
        assert!(!g.is_locked("2.2.2.2"));
    }
    #[test]
    fn guard_unlocks_after_lock_expiry() {
        let mut g = LoginGuard::new(1, 300, 0);
        assert!(g.register_failure("9.9.9.9"));
        assert!(!g.is_locked("9.9.9.9"));
    }
    #[test]
    fn guard_reset_clears_lock() {
        let mut g = LoginGuard::new(2, 300, 900);
        assert!(!g.register_failure("3.3.3.3"));
        assert!(g.register_failure("3.3.3.3"));
        assert!(g.is_locked("3.3.3.3"));
        g.reset("3.3.3.3");
        assert!(!g.is_locked("3.3.3.3"));
        assert!(!g.register_failure("3.3.3.3"));
    }
    // --- self-audit hardening tests (2026-11-17) ---
    #[test]
    fn password_policy_rejects_short() {
        assert_eq!(MIN_PASSWORD_LEN, 12);
        assert!(validate_password("short").is_err());
        assert!(validate_password("12345678").is_err()); // old 8-char minimum no longer enough
        assert!(validate_password("password1-2-3").is_ok());
    }
    #[test]
    fn secret_must_be_32_bytes() {
        assert!(check_secret(b"").is_err());
        assert!(check_secret(b"short").is_err());
        assert!(check_secret(b"0123456789abcdef0123456789abcdef").is_ok());
    }
    #[test]
    fn session_ttl_is_eight_hours() {
        assert_eq!(SESSION_TTL_HOURS, 8);
        let (dt, exp) = expiry();
        let now = chrono::Utc::now().naive_utc();
        let delta = (dt - now).num_minutes();
        assert!((450..=490).contains(&delta), "ttl ~8h, got {delta}m");
        let _ = exp;
    }
    #[test]
    fn session_debug_redacts_token() {
        let s = Session {
            user_id: "u".into(),
            org_id: "o".into(),
            token: "super-secret-jwt".into(),
            session_id: "sid".into(),
        };
        let dbg = format!("{s:?}");
        assert!(!dbg.contains("super-secret-jwt"), "token leaked in Debug");
        assert!(dbg.contains("[redacted]"));
    }
    #[test]
    fn token_error_carries_no_detail() {
        let err = validate_token(b"0123456789abcdef0123456789abcdef", "bogus").unwrap_err();
        assert_eq!(err.to_string(), "token error");
        assert!(!err.to_string().contains("Invalid"));
    }
    #[tokio::test]
    async fn register_rejects_weak_password() {
        let s = MemoryStores::default();
        let secret = b"0123456789abcdef0123456789abcdef";
        assert!(register(&s, secret, "Acme", "w@x.id", "W", "short-pw")
            .await
            .is_err());
        assert!(
            register(&s, b"tiny", "Acme", "w2@x.id", "W", "password1-2-3")
                .await
                .is_err()
        );
    }
}
