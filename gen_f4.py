#!/usr/bin/env python3
"""Final round 4: server dual-auth, Stores events, auth-service roles."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust")
def edit(rel, old, new):
    p = R/rel; t = p.read_text(encoding="utf-8")
    assert old in t, f"anchor missing in {rel}: {old[:60]}"
    p.write_text(t.replace(old, new, 1), encoding="utf-8", newline="\n")

# auth service: real roles from store
edit("crates/auth/src/lib.rs",
"""    let (exp_dt, exp) = expiry();
    let sid = store.create_session(&uid, exp_dt).await.map_err(map_store)?;
    let token = issue_token(secret, &Claims { sub: uid.clone(), org: org_id.clone(), roles: vec!["org_owner".into()], sid: sid.clone(), exp }).map_err(|e| AppError::Internal(anyhow::Error::msg(e.to_string())))?;
    Ok(Session { user_id: uid, org_id, token, session_id: sid })
}""",
"""    let (exp_dt, exp) = expiry();
    let sid = store.create_session(&uid, exp_dt).await.map_err(map_store)?;
    let roles = store.user_roles(&uid).await.map_err(map_store)?;
    let token = issue_token(secret, &Claims { sub: uid.clone(), org: org_id.clone(), roles, sid: sid.clone(), exp }).map_err(|e| AppError::Internal(anyhow::Error::msg(e.to_string())))?;
    Ok(Session { user_id: uid, org_id, token, session_id: sid })
}""")
edit("crates/auth/src/lib.rs",
"""    let (exp_dt, exp) = expiry();
    let sid = store.create_session(&u.id, exp_dt).await.map_err(map_store)?;
    let token = issue_token(secret, &Claims { sub: u.id.clone(), org: u.org_id.clone(), roles: vec!["org_owner".into()], sid: sid.clone(), exp }).map_err(|e| AppError::Internal(anyhow::Error::msg(e.to_string())))?;
    Ok(Session { user_id: u.id, org_id: u.org_id, token, session_id: sid })""",
"""    let (exp_dt, exp) = expiry();
    let sid = store.create_session(&u.id, exp_dt).await.map_err(map_store)?;
    let roles = store.user_roles(&u.id).await.map_err(map_store)?;
    let token = issue_token(secret, &Claims { sub: u.id.clone(), org: u.org_id.clone(), roles, sid: sid.clone(), exp }).map_err(|e| AppError::Internal(anyhow::Error::msg(e.to_string())))?;
    Ok(Session { user_id: u.id, org_id: u.org_id, token, session_id: sid })""")

print("F4a OK")
