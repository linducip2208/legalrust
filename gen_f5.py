#!/usr/bin/env python3
"""Final round 5: server dual-auth, broadcast, all Phase-11/12 routes, wiring."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust")
def edit(rel, old, new):
    p = R/rel; t = p.read_text(encoding="utf-8")
    assert old in t, f"anchor missing in {rel}: {old[:70]}"
    p.write_text(t.replace(old, new, 1), encoding="utf-8", newline="\n")

# 1. dual auth (JWT session OR api token)
edit("apps/server/src/lib.rs",
"""    let tok = v.strip_prefix("Bearer ").ok_or(AppError::Unauthorized)?;
    let claims = legalrust_auth::validate_token(&state.stores.jwt_secret, tok)
        .map_err(|_| AppError::Unauthorized)?;
    if !state
        .stores
        .users
        .session_live(&claims.sid)
        .await
        .unwrap_or(false)
    {
        return Err(AppError::Unauthorized);
    }
    Ok(AuthUser { claims })""",
"""    let tok = v.strip_prefix("Bearer ").ok_or(AppError::Unauthorized)?;
    match legalrust_auth::validate_token(&state.stores.jwt_secret, tok) {
        Ok(claims) => {
            if !state
                .stores
                .users
                .session_live(&claims.sid)
                .await
                .unwrap_or(false)
            {
                return Err(AppError::Unauthorized);
            }
            Ok(AuthUser { claims })
        }
        Err(_) => {
            let h = legalrust_document::sha256_hex(tok.as_bytes());
            let (uid, org) = state
                .stores
                .users
                .token_owner(&h)
                .await
                .map_err(|_| AppError::Unauthorized)?
                .ok_or(AppError::Unauthorized)?;
            let roles = state
                .stores
                .users
                .user_roles(&uid)
                .await
                .map_err(|_| AppError::Unauthorized)?;
            Ok(AuthUser {
                claims: Claims {
                    sub: uid,
                    org,
                    roles,
                    sid: "api-token".into(),
                    exp: usize::MAX,
                },
            })
        }
    }""")

# 2. broadcast channel in Stores
edit("apps/server/src/lib.rs",
"""    pub jwt_secret: Vec<u8>,
    pub integration_secret: Vec<u8>,
    pub integrations: Vec<String>,
    pub limiter: RateLimiter,
}""",
"""    pub jwt_secret: Vec<u8>,
    pub integration_secret: Vec<u8>,
    pub integrations: Vec<String>,
    pub limiter: RateLimiter,
    pub events: tokio::sync::broadcast::Sender<String>,
}""")
edit("apps/server/src/lib.rs",
"""    pub fn new(
        users: Arc<dyn UserStore>,
        core: Arc<dyn CoreStore>,
        ops: Arc<dyn OpsStore>,
        objects: Arc<dyn ObjectStore>,
        jwt_secret: Vec<u8>,
    ) -> Self {""",
"""    pub fn new(
        users: Arc<dyn UserStore>,
        core: Arc<dyn CoreStore>,
        ops: Arc<dyn OpsStore>,
        objects: Arc<dyn ObjectStore>,
        jwt_secret: Vec<u8>,
    ) -> Self {
        let (events, _) = tokio::sync::broadcast::channel(64);""")
edit("apps/server/src/lib.rs",
"""            integration_secret: Vec::new(),
            integrations: vec!["dukcapil".into(), "bpn".into(), "psre".into()],
            limiter: RateLimiter::new(20),""",
"""            integration_secret: Vec::new(),
            integrations: vec!["dukcapil".into(), "bpn".into(), "psre".into()],
            limiter: RateLimiter::new(20),
            events,""")
print("F5a OK")
