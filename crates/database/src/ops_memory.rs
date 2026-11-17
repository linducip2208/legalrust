use super::ops::*;
use super::store::StoreError;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
#[derive(Debug, Default)]
struct Inner {
    jobs: HashMap<i64, OcrJobRow>,
    bg_jobs: HashMap<i64, JobRow>,
    fields: HashMap<i64, Vec<OcrFieldRow>>,
    tpls: HashMap<i64, TemplateRow>,
    deeds: HashMap<i64, DeedRow>,
    convs: HashMap<i64, String>,
    msgs: HashMap<i64, Vec<(String, String)>>,
    usage: HashMap<String, (i64, i64)>,
    rag: HashMap<i64, Vec<String>>,
    invs: HashMap<i64, InvoiceRow>,
    pays: HashMap<i64, Vec<PaymentRow>>,
    trust_accts: HashMap<i64, TrustAccountRow>,
    trust_entries: HashMap<i64, Vec<TrustEntryRow>>,
    notifs: HashMap<i64, (String, String, String, String, bool)>,
    shares: HashMap<i64, ShareRow>,
    audits: Vec<AuditRow>,
    seq: i64,
}
#[derive(Debug, Clone, Default)]
pub struct MemoryOps {
    inner: Arc<Mutex<Inner>>,
}
impl MemoryOps {
    fn next(&self, m: &mut Inner) -> i64 {
        m.seq += 1;
        m.seq
    }
}
#[async_trait::async_trait]
impl OpsStore for MemoryOps {
    async fn ocr_create(&self, org: &str, doc: i64, doc_type: &str) -> Result<i64, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.jobs.insert(
            id,
            OcrJobRow {
                id,
                org: org.into(),
                doc,
                doc_type: doc_type.into(),
                status: "queued".into(),
            },
        );
        Ok(id)
    }
    async fn ocr_list(&self, org: &str) -> Result<Vec<OcrJobRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .jobs
            .values()
            .filter(|j| j.org == org)
            .cloned()
            .collect())
    }
    async fn ocr_get(
        &self,
        org: &str,
        id: i64,
    ) -> Result<(OcrJobRow, Vec<OcrFieldRow>), StoreError> {
        let m = self.inner.lock().unwrap();
        let j = m
            .jobs
            .get(&id)
            .filter(|j| j.org == org)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        Ok((j, m.fields.get(&id).cloned().unwrap_or_default()))
    }
    async fn ocr_set_status(&self, org: &str, id: i64, status: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        m.jobs
            .get_mut(&id)
            .filter(|j| j.org == org)
            .map(|j| j.status = status.into())
            .ok_or(StoreError::NotFound)
    }
    async fn ocr_review(
        &self,
        org: &str,
        id: i64,
        _reviewer: &str,
        decision: &str,
        _note: Option<&str>,
    ) -> Result<(), StoreError> {
        self.ocr_set_status(
            org,
            id,
            if decision == "approve" {
                "reviewed_approved"
            } else {
                "reviewed_rejected"
            },
        )
        .await
    }
    async fn tpl_create(
        &self,
        org: &str,
        name: &str,
        body: &str,
    ) -> Result<TemplateRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        let t = TemplateRow {
            id,
            org: org.into(),
            name: name.into(),
            body: body.into(),
            version: 1,
        };
        m.tpls.insert(id, t.clone());
        Ok(t)
    }
    async fn tpl_list(&self, org: &str) -> Result<Vec<TemplateRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .tpls
            .values()
            .filter(|t| t.org == org)
            .cloned()
            .collect())
    }
    async fn tpl_get(&self, org: &str, id: i64) -> Result<TemplateRow, StoreError> {
        self.inner
            .lock()
            .unwrap()
            .tpls
            .get(&id)
            .filter(|t| t.org == org)
            .cloned()
            .ok_or(StoreError::NotFound)
    }
    async fn tpl_update(&self, org: &str, id: i64, body: &str) -> Result<TemplateRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let t = m
            .tpls
            .get_mut(&id)
            .filter(|t| t.org == org)
            .ok_or(StoreError::NotFound)?;
        t.body = body.into();
        t.version += 1;
        Ok(t.clone())
    }
    async fn deed_create(
        &self,
        org: &str,
        matter: i64,
        number: &str,
        deed_type: &str,
        _rendered: &str,
    ) -> Result<DeedRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        if m.deeds.values().any(|d| d.org == org && d.number == number) {
            return Err(StoreError::Conflict("deed number taken".into()));
        }
        let d = DeedRow {
            id,
            org: org.into(),
            matter_id: matter,
            number: number.into(),
            deed_type: deed_type.into(),
            state: "draft".into(),
        };
        m.deeds.insert(id, d.clone());
        Ok(d)
    }
    async fn deed_list(&self, org: &str) -> Result<Vec<DeedRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .deeds
            .values()
            .filter(|d| d.org == org)
            .cloned()
            .collect())
    }
    async fn deed_set_state(&self, org: &str, id: i64, state: &str) -> Result<DeedRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let d = m
            .deeds
            .get_mut(&id)
            .filter(|d| d.org == org)
            .ok_or(StoreError::NotFound)?;
        d.state = state.into();
        Ok(d.clone())
    }
    async fn conv_create(&self, org: &str, title: &str) -> Result<i64, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.convs.insert(id, org.into());
        m.msgs.insert(id, Vec::new());
        let _ = title;
        Ok(id)
    }
    async fn msg_add(&self, conv: i64, role: &str, content: &str) -> Result<(), StoreError> {
        self.inner
            .lock()
            .unwrap()
            .msgs
            .get_mut(&conv)
            .ok_or(StoreError::NotFound)?
            .push((role.into(), content.into()));
        Ok(())
    }
    async fn usage_add(&self, org: &str, input: i64, output: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let e = m.usage.entry(org.into()).or_insert((0, 0));
        e.0 += input;
        e.1 += output;
        Ok(())
    }
    async fn usage_sum(&self, org: &str) -> Result<(i64, i64), StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .usage
            .get(org)
            .cloned()
            .unwrap_or((0, 0)))
    }
    async fn rag_put(&self, doc: i64, chunks: &[String]) -> Result<(), StoreError> {
        self.inner.lock().unwrap().rag.insert(doc, chunks.to_vec());
        Ok(())
    }
    async fn rag_search(&self, _org: &str, q: &str, limit: i64) -> Result<Vec<RagHit>, StoreError> {
        let ql = q.to_lowercase();
        let mut hits: Vec<RagHit> = self
            .inner
            .lock()
            .unwrap()
            .rag
            .iter()
            .flat_map(|(d, cs)| {
                cs.iter()
                    .enumerate()
                    .filter(|(_, c)| c.to_lowercase().contains(&ql))
                    .map(|(i, c)| RagHit {
                        doc_id: *d,
                        idx: i as i64,
                        text: c.clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        hits.truncate(limit.max(0) as usize);
        Ok(hits)
    }
    async fn inv_create(
        &self,
        org: &str,
        matter: i64,
        number: &str,
        total: i64,
        currency: &str,
    ) -> Result<InvoiceRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        if m.invs.values().any(|v| v.org == org && v.number == number) {
            return Err(StoreError::Conflict("invoice number taken".into()));
        }
        let r = InvoiceRow {
            id,
            org: org.into(),
            matter_id: matter,
            number: number.into(),
            total,
            currency: currency.into(),
            status: "issued".into(),
        };
        m.invs.insert(id, r.clone());
        Ok(r)
    }
    async fn inv_list(&self, org: &str, matter: i64) -> Result<Vec<InvoiceRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .invs
            .values()
            .filter(|v| v.org == org && v.matter_id == matter)
            .cloned()
            .collect())
    }
    async fn inv_get(
        &self,
        org: &str,
        id: i64,
    ) -> Result<(InvoiceRow, Vec<PaymentRow>), StoreError> {
        let m = self.inner.lock().unwrap();
        let v = m
            .invs
            .get(&id)
            .filter(|v| v.org == org)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        Ok((v, m.pays.get(&id).cloned().unwrap_or_default()))
    }
    async fn notify(
        &self,
        org: &str,
        user: &str,
        kind: &str,
        title: &str,
        body: Option<&str>,
    ) -> Result<i64, StoreError> {
        let _ = org;
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.notifs.insert(
            id,
            (
                user.into(),
                kind.into(),
                title.into(),
                body.unwrap_or("").into(),
                false,
            ),
        );
        Ok(id)
    }
    async fn notify_list(&self, user: &str) -> Result<Vec<NotifRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .notifs
            .iter()
            .filter(|(_, (u, _, _, _, _))| u == user)
            .map(|(id, (_, k, t, _, r))| NotifRow {
                id: *id,
                kind: k.clone(),
                title: t.clone(),
                read: *r,
            })
            .collect())
    }
    async fn notify_read(&self, user: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        m.notifs
            .get_mut(&id)
            .filter(|(u, _, _, _, _)| u == user)
            .map(|n| n.4 = true)
            .ok_or(StoreError::NotFound)
    }
    async fn share_create(
        &self,
        org: &str,
        doc: i64,
        token: &str,
        hours: i64,
    ) -> Result<i64, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        let exp = (chrono::Utc::now() + chrono::Duration::hours(hours))
            .naive_utc()
            .to_string();
        m.shares.insert(
            id,
            ShareRow {
                id,
                org: org.into(),
                doc,
                token: token.into(),
                expires: exp,
                revoked: false,
            },
        );
        Ok(id)
    }
    async fn share_list(&self, org: &str, doc: i64) -> Result<Vec<ShareRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .shares
            .values()
            .filter(|s| s.org == org && s.doc == doc)
            .cloned()
            .collect())
    }
    async fn share_get(&self, token: &str) -> Result<Option<ShareRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .shares
            .values()
            .find(|s| s.token == token)
            .cloned())
    }
    async fn share_revoke(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        m.shares
            .get_mut(&id)
            .filter(|s| s.org == org)
            .map(|s| s.revoked = true)
            .ok_or(StoreError::NotFound)
    }
    async fn job_enqueue(&self, org: &str, kind: &str, payload: &str) -> Result<i64, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.bg_jobs.insert(
            id,
            JobRow {
                id,
                org: org.into(),
                kind: kind.into(),
                payload: payload.into(),
                status: "queued".into(),
                attempts: 0,
            },
        );
        Ok(id)
    }
    async fn job_list(&self, org: &str) -> Result<Vec<JobRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .bg_jobs
            .values()
            .filter(|j| j.org == org)
            .cloned()
            .collect())
    }
    async fn job_claim(&self) -> Result<Option<JobRow>, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let id = m
            .bg_jobs
            .values()
            .filter(|j| j.status == "queued")
            .map(|j| j.id)
            .min();
        Ok(match id {
            Some(i) => {
                let j = m.bg_jobs.get_mut(&i).unwrap();
                j.status = "running".into();
                j.attempts += 1;
                Some(j.clone())
            }
            None => None,
        })
    }
    async fn job_done(&self, id: i64, ok: bool, _note: &str) -> Result<(), StoreError> {
        self.inner
            .lock()
            .unwrap()
            .bg_jobs
            .get_mut(&id)
            .map(|j| j.status = if ok { "done".into() } else { "failed".into() })
            .ok_or(StoreError::NotFound)
    }
    async fn audit_list(
        &self,
        org: &str,
        entity: Option<&str>,
        limit: i64,
    ) -> Result<Vec<AuditRow>, StoreError> {
        let _ = org;
        Ok(self
            .inner
            .lock()
            .unwrap()
            .audits
            .iter()
            .rev()
            .filter(|a| entity.is_none_or(|e| a.entity == e))
            .take(limit.max(0) as usize)
            .cloned()
            .collect())
    }
    async fn audit_add(
        &self,
        tenant: &str,
        actor: Option<&str>,
        action: &str,
        entity: &str,
        entity_id: i64,
    ) -> Result<(), StoreError> {
        let _ = tenant;
        let mut m = self.inner.lock().unwrap();
        let id = m.seq + 1;
        m.seq = id;
        m.audits.push(AuditRow {
            id,
            actor: actor.map(str::to_string),
            action: action.into(),
            entity: entity.into(),
            entity_id,
            created: chrono::Utc::now().naive_utc().to_string(),
        });
        Ok(())
    }
    async fn pay_add(
        &self,
        org: &str,
        inv: i64,
        amount: i64,
        method: &str,
    ) -> Result<PaymentRow, StoreError> {
        if amount <= 0 {
            return Err(StoreError::Conflict("amount must be positive".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let v = m
            .invs
            .get(&inv)
            .filter(|v| v.org == org)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let paid: i64 = m
            .pays
            .get(&inv)
            .map(|ps| ps.iter().map(|p| p.amount).sum())
            .unwrap_or(0);
        if paid + amount > v.total {
            return Err(StoreError::Conflict("overpayment".into()));
        }
        let id = m.seq + 1;
        m.seq = id;
        let p = PaymentRow {
            id,
            invoice_id: inv,
            amount,
            method: method.into(),
        };
        m.pays.entry(inv).or_default().push(p.clone());
        Ok(p)
    }
    async fn trust_create_account(
        &self,
        org: &str,
        client_id: i64,
        currency: &str,
    ) -> Result<TrustAccountRow, StoreError> {
        if currency.trim().is_empty() {
            return Err(StoreError::Conflict("currency required".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        let a = TrustAccountRow {
            id,
            org: org.into(),
            client_id,
            currency: currency.into(),
            balance: 0,
        };
        m.trust_accts.insert(id, a.clone());
        Ok(a)
    }
    async fn trust_get(
        &self,
        org: &str,
        id: i64,
    ) -> Result<(TrustAccountRow, Vec<TrustEntryRow>), StoreError> {
        let m = self.inner.lock().unwrap();
        let a = m
            .trust_accts
            .get(&id)
            .filter(|a| a.org == org)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        Ok((a, m.trust_entries.get(&id).cloned().unwrap_or_default()))
    }
    async fn trust_list(&self, org: &str) -> Result<Vec<TrustAccountRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .trust_accts
            .values()
            .filter(|a| a.org == org)
            .cloned()
            .collect())
    }
    async fn trust_post(
        &self,
        org: &str,
        account: i64,
        matter: Option<i64>,
        kind: &str,
        amount: i64,
        ref_type: &str,
        ref_id: i64,
        created_by: &str,
    ) -> Result<TrustEntryRow, StoreError> {
        if amount <= 0 {
            return Err(StoreError::Conflict("amount must be positive".into()));
        }
        if kind != "debit" && kind != "credit" {
            return Err(StoreError::Conflict("kind must be debit or credit".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let a = m
            .trust_accts
            .get_mut(&account)
            .filter(|a| a.org == org)
            .ok_or(StoreError::NotFound)?;
        if kind == "debit" {
            if a.balance < amount {
                return Err(StoreError::Conflict("insufficient trust balance".into()));
            }
            a.balance -= amount;
        } else {
            a.balance += amount;
        }
        let id = m.seq + 1;
        m.seq = id;
        let e = TrustEntryRow {
            id,
            org: org.into(),
            account_id: account,
            matter_id: matter,
            kind: kind.into(),
            amount,
            ref_type: ref_type.into(),
            ref_id,
            created_by: created_by.into(),
        };
        m.trust_entries.entry(account).or_default().push(e.clone());
        Ok(e)
    }
    async fn trust_transfer(
        &self,
        org: &str,
        from: i64,
        to: i64,
        amount: i64,
        matter: Option<i64>,
        ref_type: &str,
        ref_id: i64,
        created_by: &str,
    ) -> Result<(TrustEntryRow, TrustEntryRow), StoreError> {
        if amount <= 0 {
            return Err(StoreError::Conflict("amount must be positive".into()));
        }
        if from == to {
            return Err(StoreError::Conflict("cannot transfer to self".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let src = m
            .trust_accts
            .get(&from)
            .filter(|a| a.org == org)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let dst = m
            .trust_accts
            .get(&to)
            .filter(|a| a.org == org)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if src.currency != dst.currency {
            return Err(StoreError::Conflict("currency mismatch".into()));
        }
        if src.balance < amount {
            return Err(StoreError::Conflict("insufficient trust balance".into()));
        }
        // Atomic under the single mutex: both legs post or neither does.
        // Debit leg (source) + credit leg (destination): sum(lines) == amount per side.
        m.trust_accts.get_mut(&from).unwrap().balance -= amount;
        m.trust_accts.get_mut(&to).unwrap().balance += amount;
        let id1 = m.seq + 1;
        let id2 = m.seq + 2;
        m.seq = id2;
        let debit = TrustEntryRow {
            id: id1,
            org: org.into(),
            account_id: from,
            matter_id: matter,
            kind: "debit".into(),
            amount,
            ref_type: ref_type.into(),
            ref_id,
            created_by: created_by.into(),
        };
        let credit = TrustEntryRow {
            id: id2,
            org: org.into(),
            account_id: to,
            matter_id: matter,
            kind: "credit".into(),
            amount,
            ref_type: ref_type.into(),
            ref_id,
            created_by: created_by.into(),
        };
        m.trust_entries.entry(from).or_default().push(debit.clone());
        m.trust_entries.entry(to).or_default().push(credit.clone());
        Ok((debit, credit))
    }
}

#[cfg(test)]
mod trust_tests {
    use super::*;
    use crate::ops::OpsStore;

    #[tokio::test]
    async fn create_and_post_balanced_ok() {
        let ops = MemoryOps::default();
        let a = ops.trust_create_account("org1", 7, "IDR").await.unwrap();
        assert_eq!(a.balance, 0);
        ops.trust_post("org1", a.id, None, "credit", 1_000, "deposit", 1, "teller")
            .await
            .unwrap();
        let (a2, entries) = ops.trust_get("org1", a.id).await.unwrap();
        assert_eq!(a2.balance, 1_000);
        assert_eq!(entries.len(), 1);
        ops.trust_post("org1", a.id, None, "debit", 400, "payout", 2, "teller")
            .await
            .unwrap();
        let (a3, _) = ops.trust_get("org1", a.id).await.unwrap();
        assert_eq!(a3.balance, 600);
    }

    #[tokio::test]
    async fn transfer_is_double_entry_atomic() {
        let ops = MemoryOps::default();
        let from = ops.trust_create_account("org1", 1, "IDR").await.unwrap();
        let to = ops.trust_create_account("org1", 2, "IDR").await.unwrap();
        ops.trust_post(
            "org1", from.id, None, "credit", 1_000, "deposit", 1, "teller",
        )
        .await
        .unwrap();
        let (debit, credit) = ops
            .trust_transfer("org1", from.id, to.id, 400, None, "xfer", 9, "teller")
            .await
            .unwrap();
        // Double-entry guard: both legs carry the full amount.
        assert_eq!(debit.amount, 400);
        assert_eq!(credit.amount, 400);
        assert_eq!(debit.kind, "debit");
        assert_eq!(credit.kind, "credit");
        let (f, _) = ops.trust_get("org1", from.id).await.unwrap();
        let (t, _) = ops.trust_get("org1", to.id).await.unwrap();
        assert_eq!(f.balance, 600);
        assert_eq!(t.balance, 400);
    }

    #[tokio::test]
    async fn insufficient_balance_is_conflict() {
        let ops = MemoryOps::default();
        let a = ops.trust_create_account("org1", 1, "IDR").await.unwrap();
        ops.trust_post("org1", a.id, None, "credit", 100, "deposit", 1, "teller")
            .await
            .unwrap();
        let err = ops
            .trust_post("org1", a.id, None, "debit", 500, "payout", 2, "teller")
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::Conflict(_)));
        // Balance unchanged.
        let (kept, _) = ops.trust_get("org1", a.id).await.unwrap();
        assert_eq!(kept.balance, 100);
        // Transfer with insufficient funds also conflicts and posts nothing.
        let b = ops.trust_create_account("org1", 2, "IDR").await.unwrap();
        let err = ops
            .trust_transfer("org1", a.id, b.id, 500, None, "xfer", 3, "teller")
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::Conflict(_)));
        let (_, entries) = ops.trust_get("org1", b.id).await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn zero_and_negative_amounts_conflict() {
        let ops = MemoryOps::default();
        let a = ops.trust_create_account("org1", 1, "IDR").await.unwrap();
        for bad in [0, -50] {
            let err = ops
                .trust_post("org1", a.id, None, "credit", bad, "deposit", 1, "teller")
                .await
                .unwrap_err();
            assert!(matches!(err, StoreError::Conflict(_)));
        }
        let b = ops.trust_create_account("org1", 2, "IDR").await.unwrap();
        let err = ops
            .trust_transfer("org1", a.id, b.id, -10, None, "xfer", 1, "teller")
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::Conflict(_)));
    }
}
