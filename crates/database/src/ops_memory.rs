use super::ops::*;
use super::store::StoreError;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
#[derive(Debug, Default)]
struct Inner {
    jobs: HashMap<i64, OcrJobRow>,
    bg_jobs: HashMap<i64, JobRow>,
    job_run_at: HashMap<i64, Option<std::time::Instant>>,
    fields: HashMap<i64, Vec<OcrFieldRow>>,
    tpls: HashMap<i64, TemplateRow>,
    clauses: HashMap<i64, ClauseRow>,
    invites: HashMap<i64, (InviteRow, String, String)>,
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
    mkt_leads: HashMap<i64, MarketingLeadRow>,
    mkt_notes: HashMap<i64, Vec<MarketingNoteRow>>,
    mkt_followups: HashMap<i64, MarketingFollowUpRow>,
    mkt_demos: HashMap<i64, MarketingDemoRow>,
    mkt_contacts: HashMap<i64, MarketingContactRow>,
    mkt_subs: HashMap<String, MarketingSubscriberRow>,
    mkt_plans: HashMap<String, MarketingPlanRow>,
    mkt_faqs: HashMap<String, MarketingFaqRow>,
    mkt_articles: HashMap<String, MarketingArticleRow>,
    meetings: HashMap<i64, MeetingRow>,
    crm_tags: HashMap<i64, ClientTagRow>,
    crm_links: HashSet<(i64, i64)>,
    apprs: HashMap<i64, ApprovalReqRow>,
    appr_steps: HashMap<i64, ApprovalStepRow>,
    webhooks: Vec<WebhookDeliveryRow>,
    webhook_eps: HashMap<i64, WebhookEndpointRow>,
    webhook_secrets: HashMap<i64, String>,
    obligations: HashMap<i64, PdpObligationRow>,
    consents: HashMap<i64, ConsentRow>,
    ropa: HashMap<i64, RopaRow>,
    ppat: HashMap<i64, PpatRow>,
    ppat_month: HashMap<i64, String>,
    dpia: HashMap<i64, DpiaRow>,
    breach: HashMap<i64, BreachRow>,
    vendors: HashMap<i64, VendorRow>,
    incidents: HashMap<i64, IncidentRow>,
    capas: HashMap<i64, CapaRow>,
    regchanges: HashMap<i64, RegChangeRow>,
    trainings: HashMap<i64, TrainingRow>,
    risks: HashMap<i64, RiskRow>,
    dsars: HashMap<i64, DsarRow>,
    ethics: HashMap<i64, EthicsRow>,
    gifts: HashMap<i64, GiftRow>,
    tevidence: HashMap<i64, TrainingEvidenceRow>,
    afindings: HashMap<i64, AuditFindingRow>,
    whistle: HashMap<i64, WhistleRow>,
    waivers: HashMap<i64, WaiverRow>,
    policies: HashMap<i64, PolicyRow>,
    policy_acks: HashMap<i64, PolicyAckRow>,
    controls: HashMap<i64, ControlRow>,
    ctests: HashMap<i64, ControlTestRow>,
    aplans: HashMap<i64, AuditPlanRow>,
    clists: HashMap<i64, ChecklistRow>,
    evidence: HashMap<i64, EvidenceRow>,
    pbooks: HashMap<i64, PbookRow>,
    phandovers: HashMap<i64, PhandoverRow>,
    appts: HashMap<i64, AppointmentRow>,
    prices: HashMap<i64, PriceRow>,
    quotes: HashMap<i64, QuotationRow>,
    qitems: HashMap<i64, QuotationItemRow>,
    cash: HashMap<i64, CashEntryRow>,
    advances: HashMap<i64, AdvanceRow>,
    inventory: HashMap<i64, InventoryRow>,
    stock_use: HashMap<i64, UsageRow>,
    aboxes: HashMap<i64, ArchiveBoxRow>,
    visitors: HashMap<i64, VisitorRow>,
    announces: HashMap<i64, AnnouncementRow>,
    dpayments: HashMap<i64, DownPaymentRow>,
    witnesses: HashMap<i64, WitnessRow>,
    rechecks: HashMap<i64, RecheckRow>,
    hours: HashMap<i64, OfficeHoursRow>,
    roster: HashMap<i64, RosterRow>,
    leaves: HashMap<i64, LeaveRow>,
    sboxes: HashMap<i64, StorageBoxRow>,
    mtransfers: HashMap<i64, MpdTransferRow>,
    dclists: HashMap<i64, DocCheckRow>,
    oincidents: HashMap<i64, OfficeIncidentRow>,
    bankf: HashMap<i64, BankFacilityRow>,
    feedback: HashMap<i64, FeedbackRow>,
    sops: HashMap<i64, SopRow>,
    bchecks: HashMap<i64, BackupCheckRow>,
    selfchecks: HashMap<i64, SelfcheckRow>,
    seals: HashMap<i64, YearSealRow>,
    pparties: HashMap<i64, PpatPartyRow>,
    chandovers: HashMap<i64, CertHandoverRow>,
    charges: HashMap<i64, DeedChargeRow>,
    zones: HashMap<i64, ZoneRow>,
    pbb: HashMap<i64, PbbRow>,
    lsplits: HashMap<i64, LandSplitRow>,
    lmerges: HashMap<i64, LandMergeRow>,
    ldisputes: HashMap<i64, LandDisputeRow>,
    boundaries: HashMap<i64, BoundaryRow>,
    dconsents: HashMap<i64, DeedConsentRow>,
    lhistory: HashMap<i64, LandHistoryRow>,
    uhandovers: HashMap<i64, UnitHandoverRow>,
    nhoffs: HashMap<i64, NotaryHandoffRow>,
    girik: HashMap<i64, GirikRow>,
    leases: HashMap<i64, LeaseTermRow>,
    dbatches: HashMap<i64, DevBatchRow>,
    dunits: HashMap<i64, DevUnitRow>,
    kprs: HashMap<i64, KprPackRow>,
    pregs: HashMap<i64, PregRow>,
    ajbds: HashMap<i64, AjbDraftRow>,
    delegs: HashMap<i64, DelegationRow>,
    prefs: HashMap<(String, String), bool>,
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
fn month_ok(month: &str) -> Result<(), StoreError> {
    let ok = month.len() == 7
        && month.as_bytes()[4] == b'-'
        && month[..4].chars().all(|c| c.is_ascii_digit())
        && month[5..].chars().all(|c| c.is_ascii_digit());
    if !ok {
        return Err(StoreError::Conflict("month YYYY-MM".into()));
    }
    Ok(())
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
    async fn ocr_store_result(
        &self,
        org: &str,
        id: i64,
        status: &str,
        fields: &[OcrFieldRow],
    ) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        m.jobs
            .get_mut(&id)
            .filter(|j| j.org == org)
            .map(|j| j.status = status.into())
            .ok_or(StoreError::NotFound)?;
        m.fields.insert(id, fields.to_vec());
        Ok(())
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
    async fn clause_create(
        &self,
        org: &str,
        name: &str,
        category: &str,
        body: &str,
    ) -> Result<ClauseRow, StoreError> {
        let name = name.trim();
        if name.is_empty() || name.len() > 200 || body.trim().is_empty() {
            return Err(StoreError::Conflict("name/body".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        let r = ClauseRow {
            id,
            org: org.into(),
            name: name.into(),
            category: if category.trim().is_empty() {
                "umum".into()
            } else {
                category.trim().into()
            },
            body: body.into(),
            version: 1,
        };
        m.clauses.insert(id, r.clone());
        Ok(r)
    }
    async fn clause_list(&self, org: &str) -> Result<Vec<ClauseRow>, StoreError> {
        let mut v: Vec<ClauseRow> = self
            .inner
            .lock()
            .unwrap()
            .clauses
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn clause_update(&self, org: &str, id: i64, body: &str) -> Result<ClauseRow, StoreError> {
        if body.trim().is_empty() {
            return Err(StoreError::Conflict("body".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let r = m
            .clauses
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        r.body = body.into();
        r.version += 1;
        Ok(r.clone())
    }
    async fn clause_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.clauses.get(&id) {
            Some(r) if r.org == org => {
                m.clauses.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
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
            created: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
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
    async fn inv_list_open(&self, org: &str) -> Result<Vec<ArOpenRow>, StoreError> {
        let m = self.inner.lock().unwrap();
        let mut v: Vec<ArOpenRow> = m
            .invs
            .values()
            .filter(|v| v.org == org)
            .map(|v| {
                let paid: i64 = m
                    .pays
                    .get(&v.id)
                    .map(|ps| ps.iter().map(|p| p.amount).sum())
                    .unwrap_or(0);
                ArOpenRow {
                    id: v.id,
                    number: v.number.clone(),
                    matter_id: v.matter_id,
                    total: v.total,
                    paid,
                    balance: v.total - paid,
                }
            })
            .filter(|r| r.balance > 0)
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
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
    async fn notify_list(
        &self,
        user: &str,
        kind: Option<&str>,
        unread_only: bool,
        limit: i64,
    ) -> Result<Vec<NotifRow>, StoreError> {
        let mut v: Vec<NotifRow> = self
            .inner
            .lock()
            .unwrap()
            .notifs
            .iter()
            .filter(|(_, (u, k, _, _, r))| {
                u == user && kind.is_none_or(|x| k == x) && (!unread_only || !r)
            })
            .map(|(id, (_, k, t, _, r))| NotifRow {
                id: *id,
                kind: k.clone(),
                title: t.clone(),
                read: *r,
            })
            .collect();
        v.sort_by_key(|r| -r.id);
        v.truncate(limit.clamp(0, 100) as usize);
        Ok(v)
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
        self.job_enqueue_at(org, kind, payload, 0).await
    }
    async fn job_enqueue_at(
        &self,
        org: &str,
        kind: &str,
        payload: &str,
        delay_secs: u64,
    ) -> Result<i64, StoreError> {
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
        m.job_run_at.insert(
            id,
            if delay_secs == 0 {
                None
            } else {
                Some(std::time::Instant::now() + std::time::Duration::from_secs(delay_secs))
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
        let now = std::time::Instant::now();
        let id = m
            .bg_jobs
            .values()
            .filter(|j| {
                j.status == "queued"
                    && m.job_run_at
                        .get(&j.id)
                        .copied()
                        .flatten()
                        .is_none_or(|t| t <= now)
            })
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
        let mut m = self.inner.lock().unwrap();
        m.bg_jobs
            .get_mut(&id)
            .map(|j| j.status = if ok { "done".into() } else { "failed".into() })
            .ok_or(StoreError::NotFound)?;
        m.job_run_at.remove(&id);
        Ok(())
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
    async fn inv_owner_org(&self, inv: i64) -> Result<Option<String>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .invs
            .get(&inv)
            .map(|v| v.org.clone()))
    }
    async fn appr_create(
        &self,
        org: &str,
        entity: &str,
        entity_id: i64,
        title: &str,
        created_by: &str,
        steps: &[(i64, String)],
    ) -> Result<i64, StoreError> {
        appr_check(entity, steps)?;
        if title.trim().is_empty() {
            return Err(StoreError::Conflict("title".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.apprs.insert(
            id,
            ApprovalReqRow {
                id,
                org: org.into(),
                entity: entity.into(),
                entity_id,
                title: title.into(),
                status: "pending".into(),
                created_by: created_by.into(),
                created: chrono::Utc::now()
                    .naive_utc()
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
            },
        );
        for (no, role) in steps {
            let sid = self.next(&mut m);
            m.appr_steps.insert(
                sid,
                ApprovalStepRow {
                    id: sid,
                    req: id,
                    step_no: *no,
                    role: role.trim().into(),
                    user: None,
                    status: "pending".into(),
                    comment: None,
                },
            );
        }
        Ok(id)
    }
    async fn appr_list(
        &self,
        org: &str,
        status: Option<&str>,
    ) -> Result<Vec<ApprovalReqRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .apprs
            .values()
            .filter(|r| r.org == org && status.is_none_or(|s| r.status == s))
            .cloned()
            .collect())
    }
    async fn appr_get(
        &self,
        org: &str,
        id: i64,
    ) -> Result<(ApprovalReqRow, Vec<ApprovalStepRow>), StoreError> {
        let m = self.inner.lock().unwrap();
        let r = m
            .apprs
            .get(&id)
            .filter(|r| r.org == org)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let mut steps: Vec<ApprovalStepRow> = m
            .appr_steps
            .values()
            .filter(|s| s.req == id)
            .cloned()
            .collect();
        steps.sort_by_key(|s| s.step_no);
        Ok((r, steps))
    }
    async fn appr_decide(
        &self,
        org: &str,
        id: i64,
        step_no: i64,
        user: &str,
        decision: &str,
        comment: Option<&str>,
    ) -> Result<ApprovalStepRow, StoreError> {
        if !["approve", "reject", "rework"].contains(&decision) {
            return Err(StoreError::Conflict("decision".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let req = m
            .apprs
            .get(&id)
            .filter(|r| r.org == org)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        if !["pending", "needs_rework"].contains(&req.status.as_str()) {
            return Err(StoreError::Conflict("request closed".into()));
        }
        let mut steps: Vec<i64> = m
            .appr_steps
            .values()
            .filter(|s| s.req == id)
            .map(|s| s.id)
            .collect();
        steps.sort_by_key(|sid| m.appr_steps[sid].step_no);
        let target = *steps
            .iter()
            .find(|sid| m.appr_steps[sid].step_no == step_no)
            .ok_or(StoreError::NotFound)?;
        if m.appr_steps[&target].status != "pending" {
            return Err(StoreError::Conflict("step already decided".into()));
        }
        for sid in &steps {
            let s = &m.appr_steps[sid];
            if s.step_no < step_no && s.status != "approved" {
                return Err(StoreError::Conflict("earlier step pending".into()));
            }
        }
        {
            let s = m.appr_steps.get_mut(&target).unwrap();
            s.status = match decision {
                "approve" => "approved",
                "reject" => "rejected",
                _ => "pending",
            }
            .into();
            s.user = Some(user.into());
            s.comment = comment.map(str::to_string);
        }
        let max_no = steps
            .iter()
            .map(|sid| m.appr_steps[sid].step_no)
            .max()
            .unwrap_or(0);
        let new_status = match decision {
            "approve" if step_no == max_no => "approved",
            "approve" => req.status.as_str(),
            "reject" => "rejected",
            _ => "needs_rework",
        };
        if new_status == "needs_rework" {
            for sid in &steps {
                let s = m.appr_steps.get_mut(sid).unwrap();
                s.status = "pending".into();
                s.comment = None;
            }
        }
        m.apprs.get_mut(&id).unwrap().status = new_status.into();
        Ok(m.appr_steps[&target].clone())
    }
    async fn appr_revoke(&self, org: &str, id: i64, _user: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .apprs
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        if !["pending", "needs_rework"].contains(&r.status.as_str()) {
            return Err(StoreError::Conflict("request closed".into()));
        }
        r.status = "revoked".into();
        Ok(())
    }
    async fn webhook_log(
        &self,
        org: Option<&str>,
        provider: &str,
        event: &str,
        status: &str,
        detail: &str,
    ) -> Result<i64, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.webhooks.push(WebhookDeliveryRow {
            id,
            org: org.map(str::to_string),
            provider: provider.into(),
            event: event.into(),
            status: status.into(),
            detail: detail.chars().take(255).collect(),
            direction: "in".into(),
        });
        Ok(id)
    }
    async fn webhook_list(
        &self,
        org: &str,
        provider: Option<&str>,
    ) -> Result<Vec<WebhookDeliveryRow>, StoreError> {
        let mut v: Vec<WebhookDeliveryRow> = self
            .inner
            .lock()
            .unwrap()
            .webhooks
            .iter()
            .filter(|r| r.org.as_deref() == Some(org) && provider.is_none_or(|p| r.provider == p))
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        v.truncate(200);
        Ok(v)
    }
    async fn webhook_ep_create(
        &self,
        org: &str,
        url: &str,
        secret: &str,
        events: &str,
    ) -> Result<i64, StoreError> {
        if !(url.starts_with("http://") || url.starts_with("https://")) || url.len() > 500 {
            return Err(StoreError::Conflict("url".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.webhook_eps.insert(
            id,
            WebhookEndpointRow {
                id,
                org: org.into(),
                url: url.trim().into(),
                events: if events.trim().is_empty() {
                    "*".into()
                } else {
                    events.trim().into()
                },
                active: true,
            },
        );
        let _ = secret;
        m.webhook_secrets.insert(id, secret.into());
        Ok(id)
    }
    async fn webhook_ep_list(&self, org: &str) -> Result<Vec<WebhookEndpointRow>, StoreError> {
        let mut v: Vec<WebhookEndpointRow> = self
            .inner
            .lock()
            .unwrap()
            .webhook_eps
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn webhook_ep_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .webhook_eps
            .get(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let _ = r;
        m.webhook_eps.remove(&id);
        m.webhook_secrets.remove(&id);
        Ok(())
    }
    async fn webhook_ep_secret(&self, org: &str, id: i64) -> Result<Option<String>, StoreError> {
        let m = self.inner.lock().unwrap();
        m.webhook_eps
            .get(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        Ok(m.webhook_secrets.get(&id).cloned())
    }
    async fn pdp_obligation_create(
        &self,
        org: &str,
        title: &str,
        category: &str,
        owner: &str,
    ) -> Result<i64, StoreError> {
        if title.trim().is_empty() || title.len() > 200 {
            return Err(StoreError::Conflict("title".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.obligations.insert(
            id,
            PdpObligationRow {
                id,
                org: org.into(),
                title: title.trim().into(),
                category: if category.trim().is_empty() {
                    "general".into()
                } else {
                    category.trim().into()
                },
                status: "open".into(),
                owner: owner.trim().into(),
            },
        );
        Ok(id)
    }
    async fn pdp_obligation_list(
        &self,
        org: &str,
        status: Option<&str>,
    ) -> Result<Vec<PdpObligationRow>, StoreError> {
        let mut v: Vec<PdpObligationRow> = self
            .inner
            .lock()
            .unwrap()
            .obligations
            .values()
            .filter(|r| r.org == org && status.is_none_or(|s| r.status == s))
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn pdp_obligation_set_status(
        &self,
        org: &str,
        id: i64,
        status: &str,
    ) -> Result<(), StoreError> {
        if !["open", "in_progress", "done"].contains(&status) {
            return Err(StoreError::Conflict("status".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let r = m
            .obligations
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        if r.status == "done" {
            return Err(StoreError::Conflict("obligation closed".into()));
        }
        if r.status == "open" && status == "open" {
            return Err(StoreError::Conflict("no transition".into()));
        }
        r.status = status.into();
        Ok(())
    }
    async fn consent_grant(
        &self,
        org: &str,
        subject: &str,
        purpose: &str,
    ) -> Result<i64, StoreError> {
        if subject.trim().is_empty() || purpose.trim().is_empty() {
            return Err(StoreError::Conflict("subject/purpose".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if m.consents.values().any(|c| {
            c.org == org
                && c.subject == subject.trim()
                && c.purpose == purpose.trim()
                && c.status == "granted"
        }) {
            return Err(StoreError::Conflict("consent already granted".into()));
        }
        let id = self.next(&mut m);
        m.consents.insert(
            id,
            ConsentRow {
                id,
                org: org.into(),
                subject: subject.trim().into(),
                purpose: purpose.trim().into(),
                status: "granted".into(),
            },
        );
        Ok(id)
    }
    async fn consent_withdraw(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .consents
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        if r.status != "granted" {
            return Err(StoreError::Conflict("consent not active".into()));
        }
        r.status = "withdrawn".into();
        Ok(())
    }
    async fn consent_list(
        &self,
        org: &str,
        subject: Option<&str>,
    ) -> Result<Vec<ConsentRow>, StoreError> {
        let mut v: Vec<ConsentRow> = self
            .inner
            .lock()
            .unwrap()
            .consents
            .values()
            .filter(|r| r.org == org && subject.is_none_or(|s| r.subject == s))
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn ropa_create(
        &self,
        org: &str,
        activity: &str,
        purpose: &str,
        data_categories: &str,
        legal_basis: &str,
        retention: &str,
    ) -> Result<i64, StoreError> {
        if activity.trim().is_empty() || activity.len() > 200 || purpose.trim().is_empty() {
            return Err(StoreError::Conflict("activity/purpose".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.ropa.insert(
            id,
            RopaRow {
                id,
                org: org.into(),
                activity: activity.trim().into(),
                purpose: purpose.trim().into(),
                data_categories: data_categories.trim().into(),
                legal_basis: if legal_basis.trim().is_empty() {
                    "consent".into()
                } else {
                    legal_basis.trim().into()
                },
                retention: retention.trim().into(),
            },
        );
        Ok(id)
    }
    async fn ropa_list(&self, org: &str) -> Result<Vec<RopaRow>, StoreError> {
        let mut v: Vec<RopaRow> = self
            .inner
            .lock()
            .unwrap()
            .ropa
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn ropa_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.ropa.get(&id) {
            Some(r) if r.org == org => {
                m.ropa.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn ppat_create(
        &self,
        org: &str,
        number: &str,
        deed_kind: &str,
        seller: &str,
        buyer: &str,
        object: &str,
    ) -> Result<i64, StoreError> {
        let number = number.trim();
        if number.is_empty() || number.len() > 60 {
            return Err(StoreError::Conflict("number".into()));
        }
        if !matches!(
            deed_kind,
            "ajb" | "hibah" | "tukar_menukar" | "apht" | "skmht" | "sewa" | "lainnya"
        ) {
            return Err(StoreError::Conflict("deed_kind".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if m.ppat.values().any(|r| r.org == org && r.number == number) {
            return Err(StoreError::Conflict("number taken".into()));
        }
        let id = self.next(&mut m);
        m.ppat.insert(
            id,
            PpatRow {
                id,
                org: org.into(),
                number: number.into(),
                deed_kind: deed_kind.into(),
                party_seller: seller.trim().into(),
                party_buyer: buyer.trim().into(),
                object_desc: object.trim().into(),
                status: "draft".into(),
            },
        );
        m.ppat_month
            .insert(id, chrono::Utc::now().format("%Y-%m").to_string());
        Ok(id)
    }
    async fn ppat_list(&self, org: &str) -> Result<Vec<PpatRow>, StoreError> {
        let mut v: Vec<PpatRow> = self
            .inner
            .lock()
            .unwrap()
            .ppat
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn ppat_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .ppat
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("draft", "signed") | ("signed", "registered")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn ppat_monthly(
        &self,
        org: &str,
        month: &str,
    ) -> Result<Vec<PpatMonthlyRow>, StoreError> {
        month_ok(month)?;
        let m = self.inner.lock().unwrap();
        let mut buckets: std::collections::HashMap<(String, String), i64> =
            std::collections::HashMap::new();
        for r in m.ppat.values().filter(|r| r.org == org) {
            if m.ppat_month.get(&r.id).map(|s| s.as_str()) == Some(month) {
                *buckets
                    .entry((r.deed_kind.clone(), r.status.clone()))
                    .or_insert(0) += 1;
            }
        }
        let mut v: Vec<PpatMonthlyRow> = buckets
            .into_iter()
            .map(|((kind, status), n)| PpatMonthlyRow { kind, status, n })
            .collect();
        v.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.status.cmp(&b.status)));
        Ok(v)
    }
    async fn ppat_dashboard(&self, org: &str) -> Result<PpatDashboard, StoreError> {
        let m = self.inner.lock().unwrap();
        let mut by: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        let mut total = 0i64;
        for r in m.ppat.values().filter(|r| r.org == org) {
            total += 1;
            *by.entry(r.deed_kind.clone()).or_insert(0) += 1;
        }
        let mut by_kind: Vec<PpatMonthlyRow> = by
            .into_iter()
            .map(|(kind, n)| PpatMonthlyRow {
                kind,
                status: String::new(),
                n,
            })
            .collect();
        by_kind.sort_by(|a, b| a.kind.cmp(&b.kind));
        let horizon = (chrono::Utc::now().naive_utc().date() + chrono::Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();
        let charges_due_30 = m
            .charges
            .values()
            .filter(|r| {
                r.org == org
                    && !r.valid_until.trim().is_empty()
                    && r.valid_until.as_str() <= horizon.as_str()
            })
            .count() as i64;
        Ok(PpatDashboard {
            total_deeds: total,
            by_kind,
            unverified_consents: m
                .dconsents
                .values()
                .filter(|r| r.org == org && !r.verified)
                .count() as i64,
            charges_due_30,
            open_disputes: m
                .ldisputes
                .values()
                .filter(|r| r.org == org && r.status != "cleared")
                .count() as i64,
            pending_handovers: m
                .uhandovers
                .values()
                .filter(|r| r.org == org && r.status != "delivered")
                .count() as i64,
        })
    }
    async fn nhoff_add(
        &self,
        org: &str,
        ppat_id: i64,
        doc_name: &str,
        note: &str,
    ) -> Result<i64, StoreError> {
        let doc_name = doc_name.trim();
        if doc_name.is_empty() || doc_name.len() > 160 {
            return Err(StoreError::Conflict("doc_name".into()));
        }
        if note.len() > 500 {
            return Err(StoreError::Conflict("note".into()));
        }
        let mut m = self.inner.lock().unwrap();
        match m.ppat.get(&ppat_id) {
            Some(p) if p.org == org => {}
            _ => return Err(StoreError::NotFound),
        }
        let id = self.next(&mut m);
        m.nhoffs.insert(
            id,
            NotaryHandoffRow {
                id,
                org: org.into(),
                ppat_id,
                doc_name: doc_name.into(),
                received: false,
                note: note.trim().into(),
            },
        );
        Ok(id)
    }
    async fn nhoff_list(
        &self,
        org: &str,
        ppat_id: i64,
    ) -> Result<Vec<NotaryHandoffRow>, StoreError> {
        let mut v: Vec<NotaryHandoffRow> = self
            .inner
            .lock()
            .unwrap()
            .nhoffs
            .values()
            .filter(|r| r.org == org && r.ppat_id == ppat_id)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn nhoff_set_received(
        &self,
        org: &str,
        id: i64,
        received: bool,
    ) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.nhoffs.get_mut(&id) {
            Some(r) if r.org == org => {
                r.received = received;
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn nhoff_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.nhoffs.get(&id) {
            Some(r) if r.org == org => {
                m.nhoffs.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn girik_add(
        &self,
        org: &str,
        letter_c_ref: &str,
        holder: &str,
        area_minor: i64,
        village: &str,
        note: &str,
    ) -> Result<i64, StoreError> {
        let letter_c_ref = letter_c_ref.trim();
        let holder = holder.trim();
        if letter_c_ref.is_empty() || letter_c_ref.len() > 120 {
            return Err(StoreError::Conflict("letter_c_ref".into()));
        }
        if holder.is_empty() || holder.len() > 160 {
            return Err(StoreError::Conflict("holder".into()));
        }
        if area_minor < 0 {
            return Err(StoreError::Conflict("area".into()));
        }
        if village.len() > 120 || note.len() > 500 {
            return Err(StoreError::Conflict("village/note".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.girik.insert(
            id,
            GirikRow {
                id,
                org: org.into(),
                letter_c_ref: letter_c_ref.into(),
                holder: holder.into(),
                area_minor,
                village: village.trim().into(),
                status: "draft".into(),
                note: note.trim().into(),
            },
        );
        Ok(id)
    }
    async fn girik_list(&self, org: &str) -> Result<Vec<GirikRow>, StoreError> {
        let mut v: Vec<GirikRow> = self
            .inner
            .lock()
            .unwrap()
            .girik
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn girik_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .girik
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("draft", "verified") | ("verified", "converted")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn girik_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.girik.get(&id) {
            Some(r) if r.org == org => {
                m.girik.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    #[allow(clippy::too_many_arguments)]
    async fn lease_upsert(
        &self,
        org: &str,
        ppat_id: i64,
        monthly_minor: i64,
        duration_months: i64,
        start_date: &str,
        deposit_minor: i64,
        note: &str,
    ) -> Result<i64, StoreError> {
        if monthly_minor < 0 || deposit_minor < 0 {
            return Err(StoreError::Conflict("amount".into()));
        }
        if !(1..=1200).contains(&duration_months) {
            return Err(StoreError::Conflict("duration 1..1200".into()));
        }
        if start_date.len() > 10 || note.len() > 500 {
            return Err(StoreError::Conflict("date/note".into()));
        }
        let mut m = self.inner.lock().unwrap();
        match m.ppat.get(&ppat_id) {
            Some(p) if p.org == org => {}
            _ => return Err(StoreError::NotFound),
        }
        if let Some((id, r)) = m
            .leases
            .iter_mut()
            .find(|(_, r)| r.org == org && r.ppat_id == ppat_id)
        {
            r.monthly_minor = monthly_minor;
            r.duration_months = duration_months;
            r.start_date = start_date.trim().into();
            r.deposit_minor = deposit_minor;
            r.note = note.trim().into();
            return Ok(*id);
        }
        let id = self.next(&mut m);
        m.leases.insert(
            id,
            LeaseTermRow {
                id,
                org: org.into(),
                ppat_id,
                monthly_minor,
                duration_months,
                start_date: start_date.trim().into(),
                deposit_minor,
                note: note.trim().into(),
            },
        );
        Ok(id)
    }
    async fn lease_get(&self, org: &str, ppat_id: i64) -> Result<Option<LeaseTermRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .leases
            .values()
            .find(|r| r.org == org && r.ppat_id == ppat_id)
            .cloned())
    }
    async fn lease_delete(&self, org: &str, ppat_id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let id = m
            .leases
            .iter()
            .find(|(_, r)| r.org == org && r.ppat_id == ppat_id)
            .map(|(id, _)| *id);
        match id {
            Some(id) => {
                m.leases.remove(&id);
                Ok(())
            }
            None => Err(StoreError::NotFound),
        }
    }
    async fn dbatch_add(
        &self,
        org: &str,
        developer: &str,
        batch_no: &str,
        total_units: i64,
        note: &str,
    ) -> Result<i64, StoreError> {
        let developer = developer.trim();
        let batch_no = batch_no.trim();
        if developer.is_empty() || developer.len() > 160 {
            return Err(StoreError::Conflict("developer".into()));
        }
        if batch_no.is_empty() || batch_no.len() > 60 {
            return Err(StoreError::Conflict("batch_no".into()));
        }
        if total_units < 0 {
            return Err(StoreError::Conflict("total_units".into()));
        }
        if note.len() > 500 {
            return Err(StoreError::Conflict("note".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if m.dbatches
            .values()
            .any(|r| r.org == org && r.batch_no == batch_no)
        {
            return Err(StoreError::Conflict("batch_no taken".into()));
        }
        let id = self.next(&mut m);
        m.dbatches.insert(
            id,
            DevBatchRow {
                id,
                org: org.into(),
                developer: developer.into(),
                batch_no: batch_no.into(),
                total_units,
                status: "planned".into(),
                note: note.trim().into(),
            },
        );
        Ok(id)
    }
    async fn dbatch_list(&self, org: &str) -> Result<Vec<DevBatchRow>, StoreError> {
        let mut v: Vec<DevBatchRow> = self
            .inner
            .lock()
            .unwrap()
            .dbatches
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn dbatch_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .dbatches
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("planned", "processing") | ("processing", "done")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn dbatch_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.dbatches.get(&id) {
            Some(r) if r.org == org => {
                m.dbatches.remove(&id);
                // Detach units (mirror ON DELETE SET NULL).
                for u in m
                    .dunits
                    .values_mut()
                    .filter(|u| u.org == org && u.batch_id == Some(id))
                {
                    u.batch_id = None;
                }
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn dunit_add(
        &self,
        org: &str,
        batch_id: Option<i64>,
        unit_code: &str,
        unit_type: &str,
        price_minor: i64,
    ) -> Result<i64, StoreError> {
        let unit_code = unit_code.trim();
        if unit_code.is_empty() || unit_code.len() > 60 {
            return Err(StoreError::Conflict("unit_code".into()));
        }
        if !matches!(
            unit_type,
            "rumah" | "ruko" | "apartemen" | "kavling" | "lainnya"
        ) {
            return Err(StoreError::Conflict("unit_type".into()));
        }
        if price_minor < 0 {
            return Err(StoreError::Conflict("price".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if let Some(bid) = batch_id {
            match m.dbatches.get(&bid) {
                Some(b) if b.org == org => {}
                _ => return Err(StoreError::NotFound),
            }
        }
        if m.dunits
            .values()
            .any(|r| r.org == org && r.unit_code == unit_code)
        {
            return Err(StoreError::Conflict("unit_code taken".into()));
        }
        let id = self.next(&mut m);
        m.dunits.insert(
            id,
            DevUnitRow {
                id,
                org: org.into(),
                batch_id,
                unit_code: unit_code.into(),
                unit_type: unit_type.into(),
                price_minor,
                status: "available".into(),
            },
        );
        Ok(id)
    }
    async fn dunit_list(
        &self,
        org: &str,
        batch_id: Option<i64>,
    ) -> Result<Vec<DevUnitRow>, StoreError> {
        let mut v: Vec<DevUnitRow> = self
            .inner
            .lock()
            .unwrap()
            .dunits
            .values()
            .filter(|r| r.org == org && batch_id.map(|b| r.batch_id == Some(b)).unwrap_or(true))
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn dunit_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .dunits
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("available", "booked") | ("booked", "sold") | ("booked", "available")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn dunit_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.dunits.get(&id) {
            Some(r) if r.org == org => {
                m.dunits.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn kpr_add(
        &self,
        org: &str,
        ppat_id: i64,
        bank: &str,
        amount_minor: i64,
        note: &str,
    ) -> Result<i64, StoreError> {
        let bank = bank.trim();
        if bank.is_empty() || bank.len() > 120 {
            return Err(StoreError::Conflict("bank".into()));
        }
        if amount_minor < 0 {
            return Err(StoreError::Conflict("amount".into()));
        }
        if note.len() > 500 {
            return Err(StoreError::Conflict("note".into()));
        }
        let mut m = self.inner.lock().unwrap();
        match m.ppat.get(&ppat_id) {
            Some(p) if p.org == org => {}
            _ => return Err(StoreError::NotFound),
        }
        let id = self.next(&mut m);
        m.kprs.insert(
            id,
            KprPackRow {
                id,
                org: org.into(),
                ppat_id,
                bank: bank.into(),
                amount_minor,
                status: "applied".into(),
                note: note.trim().into(),
            },
        );
        Ok(id)
    }
    async fn kpr_list(&self, org: &str, ppat_id: i64) -> Result<Vec<KprPackRow>, StoreError> {
        let mut v: Vec<KprPackRow> = self
            .inner
            .lock()
            .unwrap()
            .kprs
            .values()
            .filter(|r| r.org == org && r.ppat_id == ppat_id)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn kpr_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .kprs
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("applied", "approved")
                | ("applied", "rejected")
                | ("approved", "disbursed")
                | ("approved", "rejected")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn kpr_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.kprs.get(&id) {
            Some(r) if r.org == org => {
                m.kprs.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn preg_add(
        &self,
        org: &str,
        entry_date: &str,
        book_ref: &str,
        deed_no: &str,
        party: &str,
        direction: &str,
    ) -> Result<i64, StoreError> {
        if entry_date.len() > 10 {
            return Err(StoreError::Conflict("entry_date".into()));
        }
        if book_ref.trim().is_empty() || book_ref.len() > 120 {
            return Err(StoreError::Conflict("book_ref".into()));
        }
        if deed_no.len() > 120 || party.trim().is_empty() || party.len() > 160 {
            return Err(StoreError::Conflict("deed_no/party".into()));
        }
        if !matches!(direction, "in" | "out") {
            return Err(StoreError::Conflict("direction".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.pregs.insert(
            id,
            PregRow {
                id,
                org: org.into(),
                entry_date: entry_date.trim().into(),
                book_ref: book_ref.trim().into(),
                deed_no: deed_no.trim().into(),
                party: party.trim().into(),
                direction: direction.into(),
            },
        );
        Ok(id)
    }
    async fn preg_list(&self, org: &str) -> Result<Vec<PregRow>, StoreError> {
        let mut v: Vec<PregRow> = self
            .inner
            .lock()
            .unwrap()
            .pregs
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn preg_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.pregs.get(&id) {
            Some(r) if r.org == org => {
                m.pregs.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn ajbd_add(
        &self,
        org: &str,
        ppat_id: i64,
        template_name: &str,
        body: &str,
        price_minor: i64,
    ) -> Result<i64, StoreError> {
        let body = body.trim();
        if body.len() < 20 || body.len() > 20000 {
            return Err(StoreError::Conflict("body 20..20000".into()));
        }
        if template_name.len() > 120 {
            return Err(StoreError::Conflict("template_name".into()));
        }
        if price_minor < 0 {
            return Err(StoreError::Conflict("price".into()));
        }
        let mut m = self.inner.lock().unwrap();
        match m.ppat.get(&ppat_id) {
            Some(p) if p.org == org => {}
            _ => return Err(StoreError::NotFound),
        }
        let id = self.next(&mut m);
        m.ajbds.insert(
            id,
            AjbDraftRow {
                id,
                org: org.into(),
                ppat_id,
                template_name: template_name.trim().into(),
                body: body.into(),
                price_minor,
            },
        );
        Ok(id)
    }
    async fn ajbd_list(&self, org: &str, ppat_id: i64) -> Result<Vec<AjbDraftRow>, StoreError> {
        let mut v: Vec<AjbDraftRow> = self
            .inner
            .lock()
            .unwrap()
            .ajbds
            .values()
            .filter(|r| r.org == org && r.ppat_id == ppat_id)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn ajbd_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.ajbds.get(&id) {
            Some(r) if r.org == org => {
                m.ajbds.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn dpia_create(
        &self,
        org: &str,
        title: &str,
        data_types: &str,
        risk: &str,
    ) -> Result<i64, StoreError> {
        let title = title.trim();
        if title.is_empty() || title.len() > 200 {
            return Err(StoreError::Conflict("title".into()));
        }
        if !matches!(risk, "low" | "medium" | "high") {
            return Err(StoreError::Conflict("risk_level".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.dpia.insert(
            id,
            DpiaRow {
                id,
                org: org.into(),
                title: title.into(),
                data_types: data_types.trim().into(),
                risk_level: risk.into(),
                status: "draft".into(),
            },
        );
        Ok(id)
    }
    async fn dpia_list(&self, org: &str) -> Result<Vec<DpiaRow>, StoreError> {
        let mut v: Vec<DpiaRow> = self
            .inner
            .lock()
            .unwrap()
            .dpia
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn dpia_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .dpia
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("draft", "review") | ("review", "approved")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn breach_create(
        &self,
        org: &str,
        title: &str,
        discovered: &str,
        affected: i64,
    ) -> Result<i64, StoreError> {
        let title = title.trim();
        if title.is_empty() || title.len() > 200 {
            return Err(StoreError::Conflict("title".into()));
        }
        if affected < 0 {
            return Err(StoreError::Conflict("affected".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.breach.insert(
            id,
            BreachRow {
                id,
                org: org.into(),
                title: title.into(),
                discovered_at: discovered.trim().into(),
                affected_count: affected,
                status: "draft".into(),
            },
        );
        Ok(id)
    }
    async fn breach_list(&self, org: &str) -> Result<Vec<BreachRow>, StoreError> {
        let mut v: Vec<BreachRow> = self
            .inner
            .lock()
            .unwrap()
            .breach
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn breach_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .breach
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("draft", "assessing") | ("assessing", "notified") | ("notified", "closed")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn vendor_create(&self, org: &str, name: &str, service: &str) -> Result<i64, StoreError> {
        let name = name.trim();
        if name.is_empty() || name.len() > 200 {
            return Err(StoreError::Conflict("name".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.vendors.insert(
            id,
            VendorRow {
                id,
                org: org.into(),
                name: name.into(),
                service: service.trim().into(),
                dpa_status: "draft".into(),
            },
        );
        Ok(id)
    }
    async fn vendor_list(&self, org: &str) -> Result<Vec<VendorRow>, StoreError> {
        let mut v: Vec<VendorRow> = self
            .inner
            .lock()
            .unwrap()
            .vendors
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn vendor_set_dpa(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "draft" | "review" | "signed") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let r = m
            .vendors
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        r.dpa_status = to.into();
        Ok(())
    }
    async fn vendor_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.vendors.get(&id) {
            Some(r) if r.org == org => {
                m.vendors.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn incident_create(
        &self,
        org: &str,
        title: &str,
        severity: &str,
    ) -> Result<i64, StoreError> {
        let title = title.trim();
        if title.is_empty() || title.len() > 200 {
            return Err(StoreError::Conflict("title".into()));
        }
        if !matches!(severity, "low" | "medium" | "high" | "critical") {
            return Err(StoreError::Conflict("severity".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.incidents.insert(
            id,
            IncidentRow {
                id,
                org: org.into(),
                title: title.into(),
                severity: severity.into(),
                status: "open".into(),
            },
        );
        Ok(id)
    }
    async fn incident_list(&self, org: &str) -> Result<Vec<IncidentRow>, StoreError> {
        let mut v: Vec<IncidentRow> = self
            .inner
            .lock()
            .unwrap()
            .incidents
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn incident_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .incidents
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("open", "investigating") | ("investigating", "resolved") | ("resolved", "closed")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn incident_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.incidents.get(&id) {
            Some(r) if r.org == org => {
                m.incidents.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn capa_create(
        &self,
        org: &str,
        title: &str,
        cause: &str,
        action: &str,
    ) -> Result<i64, StoreError> {
        let title = title.trim();
        if title.is_empty() || title.len() > 200 {
            return Err(StoreError::Conflict("title".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.capas.insert(
            id,
            CapaRow {
                id,
                org: org.into(),
                title: title.into(),
                cause: cause.trim().into(),
                action: action.trim().into(),
                status: "open".into(),
            },
        );
        Ok(id)
    }
    async fn capa_list(&self, org: &str) -> Result<Vec<CapaRow>, StoreError> {
        let mut v: Vec<CapaRow> = self
            .inner
            .lock()
            .unwrap()
            .capas
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn capa_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .capas
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("open", "in_progress") | ("in_progress", "done") | ("done", "verified")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn capa_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.capas.get(&id) {
            Some(r) if r.org == org => {
                m.capas.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn regchange_create(
        &self,
        org: &str,
        title: &str,
        source: &str,
        effective: &str,
    ) -> Result<i64, StoreError> {
        let title = title.trim();
        if title.is_empty() || title.len() > 200 {
            return Err(StoreError::Conflict("title".into()));
        }
        if effective.len() > 10 {
            return Err(StoreError::Conflict("effective_date".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.regchanges.insert(
            id,
            RegChangeRow {
                id,
                org: org.into(),
                title: title.into(),
                source: source.trim().chars().take(120).collect(),
                effective_date: effective.trim().into(),
                status: "open".into(),
            },
        );
        Ok(id)
    }
    async fn regchange_list(&self, org: &str) -> Result<Vec<RegChangeRow>, StoreError> {
        let mut v: Vec<RegChangeRow> = self
            .inner
            .lock()
            .unwrap()
            .regchanges
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn regchange_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .regchanges
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("open", "acknowledged") | ("acknowledged", "implemented")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn regchange_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.regchanges.get(&id) {
            Some(r) if r.org == org => {
                m.regchanges.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn training_create(
        &self,
        org: &str,
        title: &str,
        assignee: &str,
        due: &str,
    ) -> Result<i64, StoreError> {
        let title = title.trim();
        if title.is_empty() || title.len() > 200 {
            return Err(StoreError::Conflict("title".into()));
        }
        if due.len() > 10 {
            return Err(StoreError::Conflict("due_date".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.trainings.insert(
            id,
            TrainingRow {
                id,
                org: org.into(),
                title: title.into(),
                assignee: assignee.trim().chars().take(120).collect(),
                due_date: due.trim().into(),
                status: "assigned".into(),
            },
        );
        Ok(id)
    }
    async fn training_list(&self, org: &str) -> Result<Vec<TrainingRow>, StoreError> {
        let mut v: Vec<TrainingRow> = self
            .inner
            .lock()
            .unwrap()
            .trainings
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn training_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .trainings
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("assigned", "in_progress") | ("in_progress", "completed")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn training_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.trainings.get(&id) {
            Some(r) if r.org == org => {
                m.trainings.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn risk_create(
        &self,
        org: &str,
        title: &str,
        category: &str,
        likelihood: i64,
        impact: i64,
        owner: &str,
    ) -> Result<i64, StoreError> {
        let title = title.trim();
        if title.is_empty() || title.len() > 200 {
            return Err(StoreError::Conflict("title".into()));
        }
        if !(1..=5).contains(&likelihood) || !(1..=5).contains(&impact) {
            return Err(StoreError::Conflict("likelihood/impact 1-5".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.risks.insert(
            id,
            RiskRow {
                id,
                org: org.into(),
                title: title.into(),
                category: category.trim().chars().take(60).collect(),
                likelihood,
                impact,
                owner: owner.trim().chars().take(120).collect(),
                status: "open".into(),
            },
        );
        Ok(id)
    }
    async fn risk_list(&self, org: &str) -> Result<Vec<RiskRow>, StoreError> {
        let mut v: Vec<RiskRow> = self
            .inner
            .lock()
            .unwrap()
            .risks
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -(r.likelihood * r.impact * 1_000_000 + r.id));
        Ok(v)
    }
    async fn risk_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .risks
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("open", "mitigating") | ("mitigating", "mitigated") | ("mitigated", "closed")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn risk_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.risks.get(&id) {
            Some(r) if r.org == org => {
                m.risks.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn dsar_create(
        &self,
        org: &str,
        subject: &str,
        kind: &str,
        detail: &str,
        due: &str,
    ) -> Result<i64, StoreError> {
        let subject = subject.trim();
        if subject.is_empty() || subject.len() > 160 {
            return Err(StoreError::Conflict("subject".into()));
        }
        if !matches!(kind, "access" | "deletion" | "rectification") {
            return Err(StoreError::Conflict("kind".into()));
        }
        if detail.trim().is_empty() || detail.len() > 2000 {
            return Err(StoreError::Conflict("detail".into()));
        }
        if due.len() > 10 {
            return Err(StoreError::Conflict("due_date".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.dsars.insert(
            id,
            DsarRow {
                id,
                org: org.into(),
                subject: subject.into(),
                kind: kind.into(),
                detail: detail.trim().into(),
                due_date: due.trim().into(),
                status: "open".into(),
            },
        );
        Ok(id)
    }
    async fn dsar_list(&self, org: &str) -> Result<Vec<DsarRow>, StoreError> {
        let mut v: Vec<DsarRow> = self
            .inner
            .lock()
            .unwrap()
            .dsars
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn dsar_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .dsars
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("open", "in_review") | ("in_review", "fulfilled") | ("in_review", "rejected")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn dsar_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.dsars.get(&id) {
            Some(r) if r.org == org => {
                m.dsars.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn ethics_create(
        &self,
        org: &str,
        declarant: &str,
        year: &str,
        statement: &str,
    ) -> Result<i64, StoreError> {
        let declarant = declarant.trim();
        if declarant.is_empty() || declarant.len() > 120 {
            return Err(StoreError::Conflict("declarant".into()));
        }
        if statement.trim().is_empty() || statement.len() > 2000 {
            return Err(StoreError::Conflict("statement".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.ethics.insert(
            id,
            EthicsRow {
                id,
                org: org.into(),
                declarant: declarant.into(),
                year: year.trim().chars().take(4).collect(),
                statement: statement.trim().into(),
                status: "submitted".into(),
            },
        );
        Ok(id)
    }
    async fn ethics_list(&self, org: &str) -> Result<Vec<EthicsRow>, StoreError> {
        let mut v: Vec<EthicsRow> = self
            .inner
            .lock()
            .unwrap()
            .ethics
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn ethics_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .ethics
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        if !matches!((r.status.as_str(), to), ("submitted", "acknowledged")) {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn ethics_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.ethics.get(&id) {
            Some(r) if r.org == org => {
                m.ethics.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn gift_create(
        &self,
        org: &str,
        direction: &str,
        description: &str,
        counterpart: &str,
        given_at: &str,
    ) -> Result<i64, StoreError> {
        let description = description.trim();
        if description.is_empty() || description.len() > 255 {
            return Err(StoreError::Conflict("description".into()));
        }
        if !matches!(direction, "given" | "received") {
            return Err(StoreError::Conflict("direction".into()));
        }
        if given_at.len() > 10 {
            return Err(StoreError::Conflict("given_at".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.gifts.insert(
            id,
            GiftRow {
                id,
                org: org.into(),
                direction: direction.into(),
                description: description.into(),
                counterpart: counterpart.trim().chars().take(120).collect(),
                given_at: given_at.trim().into(),
                status: "logged".into(),
            },
        );
        Ok(id)
    }
    async fn gift_list(&self, org: &str) -> Result<Vec<GiftRow>, StoreError> {
        let mut v: Vec<GiftRow> = self
            .inner
            .lock()
            .unwrap()
            .gifts
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn gift_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .gifts
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("logged", "reviewed") | ("reviewed", "cleared") | ("reviewed", "returned")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn gift_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.gifts.get(&id) {
            Some(r) if r.org == org => {
                m.gifts.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn tevidence_add(
        &self,
        org: &str,
        training_id: i64,
        note: &str,
    ) -> Result<i64, StoreError> {
        let note = note.trim();
        if note.is_empty() || note.len() > 500 {
            return Err(StoreError::Conflict("note".into()));
        }
        let mut m = self.inner.lock().unwrap();
        match m.trainings.get(&training_id) {
            Some(t) if t.org == org => {}
            _ => return Err(StoreError::NotFound),
        }
        let id = self.next(&mut m);
        m.tevidence.insert(
            id,
            TrainingEvidenceRow {
                id,
                org: org.into(),
                training_id,
                note: note.into(),
            },
        );
        Ok(id)
    }
    async fn tevidence_list(
        &self,
        org: &str,
        training_id: i64,
    ) -> Result<Vec<TrainingEvidenceRow>, StoreError> {
        let m = self.inner.lock().unwrap();
        match m.trainings.get(&training_id) {
            Some(t) if t.org == org => {}
            _ => return Err(StoreError::NotFound),
        }
        let mut v: Vec<TrainingEvidenceRow> = m
            .tevidence
            .values()
            .filter(|r| r.org == org && r.training_id == training_id)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn tevidence_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.tevidence.get(&id) {
            Some(r) if r.org == org => {
                m.tevidence.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn afinding_create(
        &self,
        org: &str,
        title: &str,
        area: &str,
        severity: &str,
    ) -> Result<i64, StoreError> {
        let title = title.trim();
        if title.is_empty() || title.len() > 200 {
            return Err(StoreError::Conflict("title".into()));
        }
        if !matches!(severity, "low" | "medium" | "high" | "critical") {
            return Err(StoreError::Conflict("severity".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.afindings.insert(
            id,
            AuditFindingRow {
                id,
                org: org.into(),
                title: title.into(),
                area: area.trim().chars().take(60).collect(),
                severity: severity.into(),
                status: "open".into(),
            },
        );
        Ok(id)
    }
    async fn afinding_list(&self, org: &str) -> Result<Vec<AuditFindingRow>, StoreError> {
        let mut v: Vec<AuditFindingRow> = self
            .inner
            .lock()
            .unwrap()
            .afindings
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn afinding_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .afindings
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("open", "in_remediation") | ("in_remediation", "closed")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn afinding_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.afindings.get(&id) {
            Some(r) if r.org == org => {
                m.afindings.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn whistle_create(
        &self,
        org: &str,
        channel: &str,
        category: &str,
        summary: &str,
        contact: &str,
    ) -> Result<i64, StoreError> {
        let summary = summary.trim();
        if summary.is_empty() || summary.len() > 2000 {
            return Err(StoreError::Conflict("summary".into()));
        }
        if !matches!(channel, "web" | "phone" | "box" | "email") {
            return Err(StoreError::Conflict("channel".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.whistle.insert(
            id,
            WhistleRow {
                id,
                org: org.into(),
                channel: channel.into(),
                category: category.trim().chars().take(60).collect(),
                summary: summary.into(),
                contact: contact.trim().chars().take(160).collect(),
                status: "received".into(),
            },
        );
        Ok(id)
    }
    async fn whistle_list(&self, org: &str) -> Result<Vec<WhistleRow>, StoreError> {
        let mut v: Vec<WhistleRow> = self
            .inner
            .lock()
            .unwrap()
            .whistle
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn whistle_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .whistle
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("received", "under_review")
                | ("under_review", "resolved")
                | ("under_review", "dismissed")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn whistle_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.whistle.get(&id) {
            Some(r) if r.org == org => {
                m.whistle.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn waiver_create(
        &self,
        org: &str,
        title: &str,
        justification: &str,
        requested_by: &str,
        expires_at: &str,
    ) -> Result<i64, StoreError> {
        let title = title.trim();
        if title.is_empty() || title.len() > 200 {
            return Err(StoreError::Conflict("title".into()));
        }
        if justification.trim().is_empty() || justification.len() > 2000 {
            return Err(StoreError::Conflict("justification".into()));
        }
        if expires_at.len() > 10 {
            return Err(StoreError::Conflict("expires_at".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.waivers.insert(
            id,
            WaiverRow {
                id,
                org: org.into(),
                title: title.into(),
                justification: justification.trim().into(),
                requested_by: requested_by.trim().chars().take(120).collect(),
                expires_at: expires_at.trim().into(),
                status: "requested".into(),
            },
        );
        Ok(id)
    }
    async fn waiver_list(&self, org: &str) -> Result<Vec<WaiverRow>, StoreError> {
        let mut v: Vec<WaiverRow> = self
            .inner
            .lock()
            .unwrap()
            .waivers
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn waiver_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .waivers
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("requested", "approved") | ("requested", "rejected") | ("approved", "revoked")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn waiver_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.waivers.get(&id) {
            Some(r) if r.org == org => {
                m.waivers.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn policy_create(
        &self,
        org: &str,
        title: &str,
        version: &str,
        body: &str,
    ) -> Result<i64, StoreError> {
        let title = title.trim();
        if title.is_empty() || title.len() > 200 {
            return Err(StoreError::Conflict("title".into()));
        }
        if body.trim().is_empty() || body.len() > 200_000 {
            return Err(StoreError::Conflict("body".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.policies.insert(
            id,
            PolicyRow {
                id,
                org: org.into(),
                title: title.into(),
                version: version.trim().chars().take(20).collect(),
                body: body.trim().into(),
                status: "draft".into(),
            },
        );
        Ok(id)
    }
    async fn policy_list(&self, org: &str) -> Result<Vec<PolicyRow>, StoreError> {
        let mut v: Vec<PolicyRow> = self
            .inner
            .lock()
            .unwrap()
            .policies
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn policy_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .policies
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("draft", "published") | ("published", "archived")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn policy_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.policies.get(&id) {
            Some(r) if r.org == org => {
                m.policies.remove(&id);
                m.policy_acks.retain(|_, a| a.policy_id != id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn policy_ack(&self, org: &str, policy_id: i64, user: &str) -> Result<i64, StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.policies.get(&policy_id) {
            Some(p) if p.org == org && p.status == "published" => {}
            Some(_) => return Err(StoreError::Conflict("not published".into())),
            None => return Err(StoreError::NotFound),
        }
        if m.policy_acks
            .values()
            .any(|a| a.policy_id == policy_id && a.user_id == user)
        {
            return Err(StoreError::Conflict("already acked".into()));
        }
        let id = self.next(&mut m);
        m.policy_acks.insert(
            id,
            PolicyAckRow {
                id,
                org: org.into(),
                policy_id,
                user_id: user.into(),
            },
        );
        Ok(id)
    }
    async fn policy_ack_list(
        &self,
        org: &str,
        policy_id: i64,
    ) -> Result<Vec<PolicyAckRow>, StoreError> {
        let m = self.inner.lock().unwrap();
        match m.policies.get(&policy_id) {
            Some(p) if p.org == org => {}
            _ => return Err(StoreError::NotFound),
        }
        let mut v: Vec<PolicyAckRow> = m
            .policy_acks
            .values()
            .filter(|a| a.org == org && a.policy_id == policy_id)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn control_create(
        &self,
        org: &str,
        code: &str,
        title: &str,
        area: &str,
        frequency: &str,
    ) -> Result<i64, StoreError> {
        let code = code.trim();
        let title = title.trim();
        if code.is_empty() || code.len() > 40 {
            return Err(StoreError::Conflict("code".into()));
        }
        if title.is_empty() || title.len() > 200 {
            return Err(StoreError::Conflict("title".into()));
        }
        if !matches!(
            frequency,
            "daily" | "weekly" | "monthly" | "quarterly" | "annual"
        ) {
            return Err(StoreError::Conflict("frequency".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if m.controls.values().any(|c| c.org == org && c.code == code) {
            return Err(StoreError::Conflict("duplicate code".into()));
        }
        let id = self.next(&mut m);
        m.controls.insert(
            id,
            ControlRow {
                id,
                org: org.into(),
                code: code.into(),
                title: title.into(),
                area: area.trim().chars().take(60).collect(),
                frequency: frequency.into(),
                active: true,
            },
        );
        Ok(id)
    }
    async fn control_list(&self, org: &str) -> Result<Vec<ControlRow>, StoreError> {
        let mut v: Vec<ControlRow> = self
            .inner
            .lock()
            .unwrap()
            .controls
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn control_set_active(&self, org: &str, id: i64, active: bool) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .controls
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        r.active = active;
        Ok(())
    }
    async fn control_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.controls.get(&id) {
            Some(r) if r.org == org => {
                m.controls.remove(&id);
                m.ctests.retain(|_, t| t.control_id != id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn ctest_add(
        &self,
        org: &str,
        control_id: i64,
        tested_at: &str,
        result: &str,
        note: &str,
    ) -> Result<i64, StoreError> {
        if tested_at.len() > 10 {
            return Err(StoreError::Conflict("tested_at".into()));
        }
        if !matches!(result, "pass" | "fail" | "partial") {
            return Err(StoreError::Conflict("result".into()));
        }
        if note.len() > 500 {
            return Err(StoreError::Conflict("note".into()));
        }
        let mut m = self.inner.lock().unwrap();
        match m.controls.get(&control_id) {
            Some(c) if c.org == org => {}
            _ => return Err(StoreError::NotFound),
        }
        let id = self.next(&mut m);
        m.ctests.insert(
            id,
            ControlTestRow {
                id,
                org: org.into(),
                control_id,
                tested_at: tested_at.trim().into(),
                result: result.into(),
                note: note.trim().into(),
            },
        );
        Ok(id)
    }
    async fn ctest_list(
        &self,
        org: &str,
        control_id: i64,
    ) -> Result<Vec<ControlTestRow>, StoreError> {
        let m = self.inner.lock().unwrap();
        match m.controls.get(&control_id) {
            Some(c) if c.org == org => {}
            _ => return Err(StoreError::NotFound),
        }
        let mut v: Vec<ControlTestRow> = m
            .ctests
            .values()
            .filter(|t| t.org == org && t.control_id == control_id)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn ctest_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.ctests.get(&id) {
            Some(r) if r.org == org => {
                m.ctests.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn aplan_create(
        &self,
        org: &str,
        title: &str,
        period: &str,
        scope: &str,
    ) -> Result<i64, StoreError> {
        let title = title.trim();
        if title.is_empty() || title.len() > 200 {
            return Err(StoreError::Conflict("title".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.aplans.insert(
            id,
            AuditPlanRow {
                id,
                org: org.into(),
                title: title.into(),
                period: period.trim().chars().take(20).collect(),
                scope: scope.trim().chars().take(255).collect(),
                status: "planned".into(),
            },
        );
        Ok(id)
    }
    async fn aplan_list(&self, org: &str) -> Result<Vec<AuditPlanRow>, StoreError> {
        let mut v: Vec<AuditPlanRow> = self
            .inner
            .lock()
            .unwrap()
            .aplans
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn aplan_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .aplans
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("planned", "in_progress") | ("in_progress", "completed")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn aplan_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.aplans.get(&id) {
            Some(r) if r.org == org => {
                m.aplans.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn clist_add(&self, org: &str, matter_id: i64, item: &str) -> Result<i64, StoreError> {
        let item = item.trim();
        if item.is_empty() || item.len() > 200 {
            return Err(StoreError::Conflict("item".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.clists.insert(
            id,
            ChecklistRow {
                id,
                org: org.into(),
                matter_id,
                item: item.into(),
                done: false,
            },
        );
        Ok(id)
    }
    async fn clist_list(&self, org: &str, matter_id: i64) -> Result<Vec<ChecklistRow>, StoreError> {
        let mut v: Vec<ChecklistRow> = self
            .inner
            .lock()
            .unwrap()
            .clists
            .values()
            .filter(|r| r.org == org && r.matter_id == matter_id)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn clist_toggle(&self, org: &str, id: i64, done: bool) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .clists
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        r.done = done;
        Ok(())
    }
    async fn clist_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.clists.get(&id) {
            Some(r) if r.org == org => {
                m.clists.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn evidence_add(
        &self,
        org: &str,
        title: &str,
        register_name: &str,
        register_id: i64,
        note: &str,
    ) -> Result<i64, StoreError> {
        let title = title.trim();
        if title.is_empty() || title.len() > 200 {
            return Err(StoreError::Conflict("title".into()));
        }
        if note.len() > 500 {
            return Err(StoreError::Conflict("note".into()));
        }
        if register_id <= 0 {
            return Err(StoreError::Conflict("register_id".into()));
        }
        if !self
            .evidence_register_exists(org, register_name, register_id)
            .await?
        {
            return Err(StoreError::NotFound);
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.evidence.insert(
            id,
            EvidenceRow {
                id,
                org: org.into(),
                title: title.into(),
                register_name: register_name.into(),
                register_id,
                note: note.trim().into(),
            },
        );
        Ok(id)
    }
    async fn evidence_list(
        &self,
        org: &str,
        register_name: Option<&str>,
        register_id: Option<i64>,
    ) -> Result<Vec<EvidenceRow>, StoreError> {
        let mut v: Vec<EvidenceRow> = self
            .inner
            .lock()
            .unwrap()
            .evidence
            .values()
            .filter(|r| {
                r.org == org
                    && register_name.map(|n| r.register_name == n).unwrap_or(true)
                    && register_id.map(|i| r.register_id == i).unwrap_or(true)
            })
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn evidence_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.evidence.get(&id) {
            Some(r) if r.org == org => {
                m.evidence.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn pbook_create(&self, org: &str, book_no: &str, year: &str) -> Result<i64, StoreError> {
        let book_no = book_no.trim();
        if book_no.is_empty() || book_no.len() > 40 {
            return Err(StoreError::Conflict("book_no".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if m.pbooks
            .values()
            .any(|b| b.org == org && b.book_no == book_no)
        {
            return Err(StoreError::Conflict("duplicate book_no".into()));
        }
        let id = self.next(&mut m);
        m.pbooks.insert(
            id,
            PbookRow {
                id,
                org: org.into(),
                book_no: book_no.into(),
                book_year: year.trim().chars().take(4).collect(),
                status: "open".into(),
            },
        );
        Ok(id)
    }
    async fn pbook_list(&self, org: &str) -> Result<Vec<PbookRow>, StoreError> {
        let mut v: Vec<PbookRow> = self
            .inner
            .lock()
            .unwrap()
            .pbooks
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn pbook_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .pbooks
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("open", "closed") | ("closed", "sealed")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn pbook_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.pbooks.get(&id) {
            Some(r) if r.org == org => {
                m.pbooks.remove(&id);
                m.phandovers.retain(|_, h| h.book_id != id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn phandover_add(
        &self,
        org: &str,
        book_id: i64,
        from_holder: &str,
        to_holder: &str,
        handed_at: &str,
        note: &str,
    ) -> Result<i64, StoreError> {
        if from_holder.trim().is_empty() || to_holder.trim().is_empty() {
            return Err(StoreError::Conflict("holder".into()));
        }
        if from_holder.trim() == to_holder.trim() {
            return Err(StoreError::Conflict("same holder".into()));
        }
        if handed_at.len() > 10 {
            return Err(StoreError::Conflict("handed_at".into()));
        }
        if note.len() > 500 {
            return Err(StoreError::Conflict("note".into()));
        }
        let mut m = self.inner.lock().unwrap();
        match m.pbooks.get(&book_id) {
            Some(b) if b.org == org => {}
            _ => return Err(StoreError::NotFound),
        }
        let id = self.next(&mut m);
        m.phandovers.insert(
            id,
            PhandoverRow {
                id,
                org: org.into(),
                book_id,
                from_holder: from_holder.trim().chars().take(120).collect(),
                to_holder: to_holder.trim().chars().take(120).collect(),
                handed_at: handed_at.trim().into(),
                note: note.trim().into(),
            },
        );
        Ok(id)
    }
    async fn phandover_list(
        &self,
        org: &str,
        book_id: i64,
    ) -> Result<Vec<PhandoverRow>, StoreError> {
        let m = self.inner.lock().unwrap();
        match m.pbooks.get(&book_id) {
            Some(b) if b.org == org => {}
            _ => return Err(StoreError::NotFound),
        }
        let mut v: Vec<PhandoverRow> = m
            .phandovers
            .values()
            .filter(|r| r.org == org && r.book_id == book_id)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn phandover_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.phandovers.get(&id) {
            Some(r) if r.org == org => {
                m.phandovers.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn appt_create(
        &self,
        org: &str,
        client_name: &str,
        purpose: &str,
        scheduled_at: &str,
    ) -> Result<i64, StoreError> {
        let client_name = client_name.trim();
        if client_name.is_empty() || client_name.len() > 160 {
            return Err(StoreError::Conflict("client_name".into()));
        }
        if purpose.len() > 255 {
            return Err(StoreError::Conflict("purpose".into()));
        }
        // Scheduler integrity: strict slot format, no silent coercion.
        if chrono::NaiveDateTime::parse_from_str(scheduled_at.trim(), "%Y-%m-%d %H:%M").is_err() {
            return Err(StoreError::Conflict("scheduled_at YYYY-MM-DD HH:MM".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.appts.insert(
            id,
            AppointmentRow {
                id,
                org: org.into(),
                client_name: client_name.into(),
                purpose: purpose.trim().chars().take(255).collect(),
                scheduled_at: scheduled_at.trim().into(),
                status: "scheduled".into(),
            },
        );
        Ok(id)
    }
    async fn appt_list(&self, org: &str) -> Result<Vec<AppointmentRow>, StoreError> {
        let mut v: Vec<AppointmentRow> = self
            .inner
            .lock()
            .unwrap()
            .appts
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by(|a, b| a.scheduled_at.cmp(&b.scheduled_at).then(b.id.cmp(&a.id)));
        Ok(v)
    }
    async fn appt_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .appts
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("scheduled", "checked_in") | ("checked_in", "done") | ("scheduled", "cancelled")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn appt_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.appts.get(&id) {
            Some(r) if r.org == org => {
                m.appts.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn price_create(
        &self,
        org: &str,
        code: &str,
        name: &str,
        amount_minor: i64,
    ) -> Result<i64, StoreError> {
        let code = code.trim();
        let name = name.trim();
        if code.is_empty() || code.len() > 40 {
            return Err(StoreError::Conflict("code".into()));
        }
        if name.is_empty() || name.len() > 200 {
            return Err(StoreError::Conflict("name".into()));
        }
        if amount_minor < 0 {
            return Err(StoreError::Conflict("amount".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if m.prices.values().any(|p| p.org == org && p.code == code) {
            return Err(StoreError::Conflict("duplicate code".into()));
        }
        let id = self.next(&mut m);
        m.prices.insert(
            id,
            PriceRow {
                id,
                org: org.into(),
                code: code.into(),
                name: name.into(),
                amount_minor,
                active: true,
            },
        );
        Ok(id)
    }
    async fn price_list(&self, org: &str) -> Result<Vec<PriceRow>, StoreError> {
        let mut v: Vec<PriceRow> = self
            .inner
            .lock()
            .unwrap()
            .prices
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn price_set_active(&self, org: &str, id: i64, active: bool) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .prices
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        r.active = active;
        Ok(())
    }
    async fn price_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.prices.get(&id) {
            Some(r) if r.org == org => {
                m.prices.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn quote_create(
        &self,
        org: &str,
        client_name: &str,
        deed_kind: &str,
        items: &[(String, i64)],
    ) -> Result<i64, StoreError> {
        let client_name = client_name.trim();
        if client_name.is_empty() || client_name.len() > 160 {
            return Err(StoreError::Conflict("client_name".into()));
        }
        if items.is_empty() || items.len() > 50 {
            return Err(StoreError::Conflict("items".into()));
        }
        let mut m = self.inner.lock().unwrap();
        // Snapshot amounts from ACTIVE price codes only — inactive/unknown rejected.
        let mut lines: Vec<QuotationItemRow> = Vec::with_capacity(items.len());
        let mut total: i64 = 0;
        for (code, qty) in items {
            if *qty <= 0 || *qty > 1000 {
                return Err(StoreError::Conflict("qty".into()));
            }
            let p = m
                .prices
                .values()
                .find(|p| p.org == org && p.code == *code && p.active)
                .ok_or_else(|| StoreError::Conflict(format!("unknown/inactive price {code}")))?;
            let line_total = p.amount_minor.saturating_mul(*qty);
            total = total.saturating_add(line_total);
            lines.push(QuotationItemRow {
                id: 0,
                org: org.into(),
                quotation_id: 0,
                price_code: code.clone(),
                qty: *qty,
                amount_minor: line_total,
            });
        }
        let id = self.next(&mut m);
        m.quotes.insert(
            id,
            QuotationRow {
                id,
                org: org.into(),
                client_name: client_name.into(),
                deed_kind: deed_kind.trim().chars().take(60).collect(),
                total_minor: total,
                status: "draft".into(),
            },
        );
        for line in lines {
            let lid = self.next(&mut m);
            m.qitems.insert(
                lid,
                QuotationItemRow {
                    id: lid,
                    org: org.into(),
                    quotation_id: id,
                    price_code: line.price_code,
                    qty: line.qty,
                    amount_minor: line.amount_minor,
                },
            );
        }
        Ok(id)
    }
    async fn quote_list(&self, org: &str) -> Result<Vec<QuotationRow>, StoreError> {
        let mut v: Vec<QuotationRow> = self
            .inner
            .lock()
            .unwrap()
            .quotes
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn quote_items(
        &self,
        org: &str,
        quotation_id: i64,
    ) -> Result<Vec<QuotationItemRow>, StoreError> {
        let m = self.inner.lock().unwrap();
        match m.quotes.get(&quotation_id) {
            Some(q) if q.org == org => {}
            _ => return Err(StoreError::NotFound),
        }
        let mut v: Vec<QuotationItemRow> = m
            .qitems
            .values()
            .filter(|r| r.org == org && r.quotation_id == quotation_id)
            .cloned()
            .collect();
        v.sort_by_key(|r| r.id);
        Ok(v)
    }
    async fn quote_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .quotes
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("draft", "issued") | ("issued", "accepted") | ("issued", "rejected")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn quote_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.quotes.get(&id) {
            Some(r) if r.org == org => {
                m.quotes.remove(&id);
                m.qitems.retain(|_, i| i.quotation_id != id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn cash_add(
        &self,
        org: &str,
        kind: &str,
        amount_minor: i64,
        note: &str,
        occurred_on: &str,
    ) -> Result<i64, StoreError> {
        if !matches!(kind, "in" | "out") {
            return Err(StoreError::Conflict("kind".into()));
        }
        if amount_minor <= 0 {
            return Err(StoreError::Conflict("amount".into()));
        }
        if note.len() > 255 || occurred_on.len() > 10 {
            return Err(StoreError::Conflict("note/date".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.cash.insert(
            id,
            CashEntryRow {
                id,
                org: org.into(),
                kind: kind.into(),
                amount_minor,
                note: note.trim().into(),
                occurred_on: occurred_on.trim().into(),
            },
        );
        Ok(id)
    }
    async fn cash_list(&self, org: &str) -> Result<Vec<CashEntryRow>, StoreError> {
        let mut v: Vec<CashEntryRow> = self
            .inner
            .lock()
            .unwrap()
            .cash
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn cash_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.cash.get(&id) {
            Some(r) if r.org == org => {
                m.cash.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn advance_create(
        &self,
        org: &str,
        holder: &str,
        amount_minor: i64,
        purpose: &str,
    ) -> Result<i64, StoreError> {
        let holder = holder.trim();
        if holder.is_empty() || holder.len() > 120 {
            return Err(StoreError::Conflict("holder".into()));
        }
        if amount_minor <= 0 {
            return Err(StoreError::Conflict("amount".into()));
        }
        if purpose.len() > 255 {
            return Err(StoreError::Conflict("purpose".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.advances.insert(
            id,
            AdvanceRow {
                id,
                org: org.into(),
                holder: holder.into(),
                amount_minor,
                purpose: purpose.trim().into(),
                status: "open".into(),
            },
        );
        Ok(id)
    }
    async fn advance_list(&self, org: &str) -> Result<Vec<AdvanceRow>, StoreError> {
        let mut v: Vec<AdvanceRow> = self
            .inner
            .lock()
            .unwrap()
            .advances
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn advance_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .advances
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("open", "settled") | ("open", "cancelled")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn advance_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.advances.get(&id) {
            Some(r) if r.org == org => {
                m.advances.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    #[allow(clippy::too_many_arguments)]
    async fn stock_create(
        &self,
        org: &str,
        code: &str,
        name: &str,
        qty: i64,
        unit: &str,
        min_stock: i64,
        kind: &str,
    ) -> Result<i64, StoreError> {
        let code = code.trim();
        let name = name.trim();
        if code.is_empty() || code.len() > 40 {
            return Err(StoreError::Conflict("code".into()));
        }
        if name.is_empty() || name.len() > 200 {
            return Err(StoreError::Conflict("name".into()));
        }
        if qty < 0 || min_stock < 0 {
            return Err(StoreError::Conflict("qty".into()));
        }
        if !matches!(kind, "general" | "atk" | "blanko" | "meterai") {
            return Err(StoreError::Conflict("kind".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if m.inventory.values().any(|p| p.org == org && p.code == code) {
            return Err(StoreError::Conflict("duplicate code".into()));
        }
        let id = self.next(&mut m);
        m.inventory.insert(
            id,
            InventoryRow {
                id,
                org: org.into(),
                kind: kind.into(),
                code: code.into(),
                name: name.into(),
                qty,
                unit: unit.trim().chars().take(20).collect(),
                min_stock,
            },
        );
        Ok(id)
    }
    async fn stock_list(
        &self,
        org: &str,
        kind: Option<&str>,
    ) -> Result<Vec<InventoryRow>, StoreError> {
        let mut v: Vec<InventoryRow> = self
            .inner
            .lock()
            .unwrap()
            .inventory
            .values()
            .filter(|r| r.org == org && kind.map(|k| r.kind == k).unwrap_or(true))
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn stock_adjust(&self, org: &str, id: i64, delta: i64) -> Result<i64, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .inventory
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let next = r.qty.saturating_add(delta);
        if next < 0 {
            return Err(StoreError::Conflict("insufficient stock".into()));
        }
        r.qty = next;
        Ok(next)
    }
    async fn stock_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.inventory.get(&id) {
            Some(r) if r.org == org => {
                m.inventory.remove(&id);
                m.stock_use.retain(|_, u| u.item_id != id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn stock_use_add(
        &self,
        org: &str,
        item_id: i64,
        qty: i64,
        need: &str,
        used_on: &str,
    ) -> Result<i64, StoreError> {
        if qty <= 0 {
            return Err(StoreError::Conflict("qty".into()));
        }
        if need.len() > 255 || used_on.len() > 10 {
            return Err(StoreError::Conflict("need/date".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let item = m
            .inventory
            .get_mut(&item_id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        if item.qty < qty {
            return Err(StoreError::Conflict("insufficient stock".into()));
        }
        item.qty -= qty;
        let id = self.next(&mut m);
        m.stock_use.insert(
            id,
            UsageRow {
                id,
                org: org.into(),
                item_id,
                qty,
                need: need.trim().into(),
                used_on: used_on.trim().into(),
            },
        );
        Ok(id)
    }
    async fn stock_use_list(&self, org: &str, item_id: i64) -> Result<Vec<UsageRow>, StoreError> {
        let m = self.inner.lock().unwrap();
        match m.inventory.get(&item_id) {
            Some(p) if p.org == org => {}
            _ => return Err(StoreError::NotFound),
        }
        let mut v: Vec<UsageRow> = m
            .stock_use
            .values()
            .filter(|r| r.org == org && r.item_id == item_id)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn stock_use_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.stock_use.get(&id) {
            Some(r) if r.org == org => {
                m.stock_use.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn abox_create(
        &self,
        org: &str,
        box_code: &str,
        location: &str,
        description: &str,
    ) -> Result<i64, StoreError> {
        let box_code = box_code.trim();
        if box_code.is_empty() || box_code.len() > 40 {
            return Err(StoreError::Conflict("box_code".into()));
        }
        if location.len() > 120 || description.len() > 255 {
            return Err(StoreError::Conflict("location/description".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if m.aboxes
            .values()
            .any(|b| b.org == org && b.box_code == box_code)
        {
            return Err(StoreError::Conflict("duplicate box_code".into()));
        }
        let id = self.next(&mut m);
        m.aboxes.insert(
            id,
            ArchiveBoxRow {
                id,
                org: org.into(),
                box_code: box_code.into(),
                location: location.trim().into(),
                description: description.trim().chars().take(255).collect(),
            },
        );
        Ok(id)
    }
    async fn abox_list(&self, org: &str) -> Result<Vec<ArchiveBoxRow>, StoreError> {
        let mut v: Vec<ArchiveBoxRow> = self
            .inner
            .lock()
            .unwrap()
            .aboxes
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn abox_move(&self, org: &str, id: i64, location: &str) -> Result<(), StoreError> {
        if location.len() > 120 {
            return Err(StoreError::Conflict("location".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let r = m
            .aboxes
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        r.location = location.trim().into();
        Ok(())
    }
    async fn abox_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.aboxes.get(&id) {
            Some(r) if r.org == org => {
                m.aboxes.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn visitor_checkin(
        &self,
        org: &str,
        name: &str,
        need: &str,
        check_in: &str,
    ) -> Result<i64, StoreError> {
        let name = name.trim();
        if name.is_empty() || name.len() > 160 {
            return Err(StoreError::Conflict("name".into()));
        }
        if need.len() > 255 || check_in.len() > 16 {
            return Err(StoreError::Conflict("need/check_in".into()));
        }
        if chrono::NaiveDateTime::parse_from_str(check_in.trim(), "%Y-%m-%d %H:%M").is_err() {
            return Err(StoreError::Conflict("check_in YYYY-MM-DD HH:MM".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.visitors.insert(
            id,
            VisitorRow {
                id,
                org: org.into(),
                name: name.into(),
                need: need.trim().chars().take(255).collect(),
                check_in: check_in.trim().into(),
                check_out: String::new(),
                status: "in".into(),
            },
        );
        Ok(id)
    }
    async fn visitor_list(&self, org: &str) -> Result<Vec<VisitorRow>, StoreError> {
        let mut v: Vec<VisitorRow> = self
            .inner
            .lock()
            .unwrap()
            .visitors
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn visitor_checkout(
        &self,
        org: &str,
        id: i64,
        check_out: &str,
    ) -> Result<(), StoreError> {
        if chrono::NaiveDateTime::parse_from_str(check_out.trim(), "%Y-%m-%d %H:%M").is_err() {
            return Err(StoreError::Conflict("check_out YYYY-MM-DD HH:MM".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let r = m
            .visitors
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        if r.status != "in" {
            return Err(StoreError::Conflict("already out".into()));
        }
        r.status = "out".into();
        r.check_out = check_out.trim().into();
        Ok(())
    }
    async fn visitor_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.visitors.get(&id) {
            Some(r) if r.org == org => {
                m.visitors.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn announce_create(
        &self,
        org: &str,
        title: &str,
        body: &str,
        pinned: bool,
    ) -> Result<i64, StoreError> {
        let title = title.trim();
        if title.is_empty() || title.len() > 200 {
            return Err(StoreError::Conflict("title".into()));
        }
        if body.len() > 5000 {
            return Err(StoreError::Conflict("body".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.announces.insert(
            id,
            AnnouncementRow {
                id,
                org: org.into(),
                title: title.into(),
                body: body.trim().into(),
                pinned,
                active: true,
            },
        );
        Ok(id)
    }
    async fn announce_list(&self, org: &str) -> Result<Vec<AnnouncementRow>, StoreError> {
        let mut v: Vec<AnnouncementRow> = self
            .inner
            .lock()
            .unwrap()
            .announces
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by(|a, b| b.pinned.cmp(&a.pinned).then(b.id.cmp(&a.id)));
        Ok(v)
    }
    async fn announce_set(
        &self,
        org: &str,
        id: i64,
        pinned: Option<bool>,
        active: Option<bool>,
    ) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .announces
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        if let Some(p) = pinned {
            r.pinned = p;
        }
        if let Some(a) = active {
            r.active = a;
        }
        Ok(())
    }
    async fn announce_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.announces.get(&id) {
            Some(r) if r.org == org => {
                m.announces.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn dp_create(
        &self,
        org: &str,
        receipt_no: &str,
        client_name: &str,
        matter_id: Option<i64>,
        amount_minor: i64,
        issued_on: &str,
    ) -> Result<i64, StoreError> {
        let receipt_no = receipt_no.trim();
        let client_name = client_name.trim();
        if receipt_no.is_empty() || receipt_no.len() > 40 {
            return Err(StoreError::Conflict("receipt_no".into()));
        }
        if client_name.is_empty() || client_name.len() > 160 {
            return Err(StoreError::Conflict("client_name".into()));
        }
        if amount_minor <= 0 {
            return Err(StoreError::Conflict("amount".into()));
        }
        if issued_on.len() > 10 {
            return Err(StoreError::Conflict("issued_on".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if m.dpayments
            .values()
            .any(|p| p.org == org && p.receipt_no == receipt_no)
        {
            return Err(StoreError::Conflict("duplicate receipt_no".into()));
        }
        let id = self.next(&mut m);
        m.dpayments.insert(
            id,
            DownPaymentRow {
                id,
                org: org.into(),
                receipt_no: receipt_no.into(),
                client_name: client_name.into(),
                matter_id,
                amount_minor,
                issued_on: issued_on.trim().into(),
                status: "issued".into(),
            },
        );
        Ok(id)
    }
    async fn dp_list(&self, org: &str) -> Result<Vec<DownPaymentRow>, StoreError> {
        let mut v: Vec<DownPaymentRow> = self
            .inner
            .lock()
            .unwrap()
            .dpayments
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn dp_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .dpayments
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("issued", "applied") | ("issued", "refunded")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn dp_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.dpayments.get(&id) {
            Some(r) if r.org == org => {
                m.dpayments.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn witness_create(
        &self,
        org: &str,
        name: &str,
        id_number: &str,
        phone: &str,
    ) -> Result<i64, StoreError> {
        let name = name.trim();
        if name.is_empty() || name.len() > 160 {
            return Err(StoreError::Conflict("name".into()));
        }
        if id_number.len() > 40 || phone.len() > 40 {
            return Err(StoreError::Conflict("id/phone".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.witnesses.insert(
            id,
            WitnessRow {
                id,
                org: org.into(),
                name: name.into(),
                id_number: id_number.trim().into(),
                phone: phone.trim().into(),
                active: true,
            },
        );
        Ok(id)
    }
    async fn witness_list(&self, org: &str) -> Result<Vec<WitnessRow>, StoreError> {
        let mut v: Vec<WitnessRow> = self
            .inner
            .lock()
            .unwrap()
            .witnesses
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn witness_set_active(&self, org: &str, id: i64, active: bool) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .witnesses
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        r.active = active;
        Ok(())
    }
    async fn witness_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.witnesses.get(&id) {
            Some(r) if r.org == org => {
                m.witnesses.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn recheck_add(
        &self,
        org: &str,
        subject: &str,
        method: &str,
        result: &str,
        verified_on: &str,
        note: &str,
    ) -> Result<i64, StoreError> {
        let subject = subject.trim();
        if subject.is_empty() || subject.len() > 160 {
            return Err(StoreError::Conflict("subject".into()));
        }
        if !matches!(method, "document" | "physical" | "video") {
            return Err(StoreError::Conflict("method".into()));
        }
        if !matches!(result, "pass" | "fail") {
            return Err(StoreError::Conflict("result".into()));
        }
        if verified_on.len() > 10 || note.len() > 500 {
            return Err(StoreError::Conflict("note/date".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.rechecks.insert(
            id,
            RecheckRow {
                id,
                org: org.into(),
                subject: subject.into(),
                method: method.into(),
                result: result.into(),
                verified_on: verified_on.trim().into(),
                note: note.trim().into(),
            },
        );
        Ok(id)
    }
    async fn recheck_list(&self, org: &str) -> Result<Vec<RecheckRow>, StoreError> {
        let mut v: Vec<RecheckRow> = self
            .inner
            .lock()
            .unwrap()
            .rechecks
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn recheck_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.rechecks.get(&id) {
            Some(r) if r.org == org => {
                m.rechecks.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn hours_set(
        &self,
        org: &str,
        weekday: i64,
        open_time: &str,
        close_time: &str,
        active: bool,
    ) -> Result<i64, StoreError> {
        if !(0..=6).contains(&weekday) {
            return Err(StoreError::Conflict("weekday 0-6".into()));
        }
        if !valid_hhmm(open_time.trim()) || !valid_hhmm(close_time.trim()) {
            return Err(StoreError::Conflict("HH:MM".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if let Some((id, r)) = m
            .hours
            .iter_mut()
            .find(|(_, r)| r.org == org && r.weekday == weekday)
        {
            r.open_time = open_time.trim().into();
            r.close_time = close_time.trim().into();
            r.active = active;
            return Ok(*id);
        }
        let id = self.next(&mut m);
        m.hours.insert(
            id,
            OfficeHoursRow {
                id,
                org: org.into(),
                weekday,
                open_time: open_time.trim().into(),
                close_time: close_time.trim().into(),
                active,
            },
        );
        Ok(id)
    }
    async fn hours_list(&self, org: &str) -> Result<Vec<OfficeHoursRow>, StoreError> {
        let mut v: Vec<OfficeHoursRow> = self
            .inner
            .lock()
            .unwrap()
            .hours
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| r.weekday);
        Ok(v)
    }
    async fn roster_add(
        &self,
        org: &str,
        name: &str,
        role: &str,
        shift: &str,
    ) -> Result<i64, StoreError> {
        let name = name.trim();
        if name.is_empty() || name.len() > 160 {
            return Err(StoreError::Conflict("name".into()));
        }
        if !matches!(shift, "pagi" | "siang" | "penuh") {
            return Err(StoreError::Conflict("shift".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.roster.insert(
            id,
            RosterRow {
                id,
                org: org.into(),
                name: name.into(),
                role: role.trim().chars().take(60).collect(),
                shift: shift.into(),
                active: true,
            },
        );
        Ok(id)
    }
    async fn roster_list(&self, org: &str) -> Result<Vec<RosterRow>, StoreError> {
        let mut v: Vec<RosterRow> = self
            .inner
            .lock()
            .unwrap()
            .roster
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn roster_set_active(&self, org: &str, id: i64, active: bool) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .roster
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        r.active = active;
        Ok(())
    }
    async fn roster_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.roster.get(&id) {
            Some(r) if r.org == org => {
                m.roster.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn leave_request(
        &self,
        org: &str,
        holder: &str,
        kind: &str,
        from_date: &str,
        to_date: &str,
        substitute: &str,
    ) -> Result<i64, StoreError> {
        let holder = holder.trim();
        if holder.is_empty() || holder.len() > 120 {
            return Err(StoreError::Conflict("holder".into()));
        }
        if !matches!(kind, "leave" | "sick" | "duty") {
            return Err(StoreError::Conflict("kind".into()));
        }
        if from_date.len() > 10 || to_date.len() > 10 {
            return Err(StoreError::Conflict("date".into()));
        }
        if !from_date.trim().is_empty() && !to_date.trim().is_empty() && from_date > to_date {
            return Err(StoreError::Conflict("range".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.leaves.insert(
            id,
            LeaveRow {
                id,
                org: org.into(),
                holder: holder.into(),
                kind: kind.into(),
                from_date: from_date.trim().into(),
                to_date: to_date.trim().into(),
                substitute: substitute.trim().chars().take(120).collect(),
                status: "requested".into(),
            },
        );
        Ok(id)
    }
    async fn leave_list(&self, org: &str) -> Result<Vec<LeaveRow>, StoreError> {
        let mut v: Vec<LeaveRow> = self
            .inner
            .lock()
            .unwrap()
            .leaves
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn leave_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .leaves
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("requested", "approved") | ("requested", "rejected")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn leave_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.leaves.get(&id) {
            Some(r) if r.org == org => {
                m.leaves.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn sbox_create(
        &self,
        org: &str,
        box_code: &str,
        location: &str,
        capacity: i64,
    ) -> Result<i64, StoreError> {
        let box_code = box_code.trim();
        if box_code.is_empty() || box_code.len() > 40 {
            return Err(StoreError::Conflict("box_code".into()));
        }
        if location.len() > 120 {
            return Err(StoreError::Conflict("location".into()));
        }
        if capacity < 0 {
            return Err(StoreError::Conflict("capacity".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if m.sboxes
            .values()
            .any(|b| b.org == org && b.box_code == box_code)
        {
            return Err(StoreError::Conflict("duplicate box_code".into()));
        }
        let id = self.next(&mut m);
        m.sboxes.insert(
            id,
            StorageBoxRow {
                id,
                org: org.into(),
                box_code: box_code.into(),
                location: location.trim().into(),
                capacity,
                status: "available".into(),
            },
        );
        Ok(id)
    }
    async fn sbox_list(&self, org: &str) -> Result<Vec<StorageBoxRow>, StoreError> {
        let mut v: Vec<StorageBoxRow> = self
            .inner
            .lock()
            .unwrap()
            .sboxes
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn sbox_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .sboxes
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        if !matches!(
            (r.status.as_str(), to),
            ("available", "full") | ("full", "sealed") | ("available", "sealed")
        ) {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn sbox_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.sboxes.get(&id) {
            Some(r) if r.org == org => {
                m.sboxes.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn mtransfer_create(
        &self,
        org: &str,
        book_id: Option<i64>,
        recipient: &str,
        transfer_date: &str,
        receipt_no: &str,
    ) -> Result<i64, StoreError> {
        let recipient = recipient.trim();
        if recipient.is_empty() || recipient.len() > 160 {
            return Err(StoreError::Conflict("recipient".into()));
        }
        if transfer_date.len() > 10 || receipt_no.len() > 60 {
            return Err(StoreError::Conflict("date/receipt".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if let Some(bid) = book_id {
            match m.pbooks.get(&bid) {
                Some(b) if b.org == org => {}
                _ => return Err(StoreError::NotFound),
            }
        }
        let id = self.next(&mut m);
        m.mtransfers.insert(
            id,
            MpdTransferRow {
                id,
                org: org.into(),
                book_id,
                recipient: recipient.into(),
                transfer_date: transfer_date.trim().into(),
                receipt_no: receipt_no.trim().chars().take(60).collect(),
                status: "prepared".into(),
            },
        );
        Ok(id)
    }
    async fn mtransfer_list(&self, org: &str) -> Result<Vec<MpdTransferRow>, StoreError> {
        let mut v: Vec<MpdTransferRow> = self
            .inner
            .lock()
            .unwrap()
            .mtransfers
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn mtransfer_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .mtransfers
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        if !matches!((r.status.as_str(), to), ("prepared", "transferred")) {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn mtransfer_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.mtransfers.get(&id) {
            Some(r) if r.org == org => {
                m.mtransfers.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn dclist_add(&self, org: &str, doc_id: i64, item: &str) -> Result<i64, StoreError> {
        let item = item.trim();
        if item.is_empty() || item.len() > 200 {
            return Err(StoreError::Conflict("item".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.dclists.insert(
            id,
            DocCheckRow {
                id,
                org: org.into(),
                doc_id,
                item: item.into(),
                done: false,
            },
        );
        Ok(id)
    }
    async fn dclist_list(&self, org: &str, doc_id: i64) -> Result<Vec<DocCheckRow>, StoreError> {
        let mut v: Vec<DocCheckRow> = self
            .inner
            .lock()
            .unwrap()
            .dclists
            .values()
            .filter(|r| r.org == org && r.doc_id == doc_id)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn dclist_toggle(&self, org: &str, id: i64, done: bool) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .dclists
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        r.done = done;
        Ok(())
    }
    async fn dclist_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.dclists.get(&id) {
            Some(r) if r.org == org => {
                m.dclists.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn oincident_create(
        &self,
        org: &str,
        title: &str,
        severity: &str,
    ) -> Result<i64, StoreError> {
        let title = title.trim();
        if title.is_empty() || title.len() > 200 {
            return Err(StoreError::Conflict("title".into()));
        }
        if !matches!(severity, "low" | "medium" | "high" | "critical") {
            return Err(StoreError::Conflict("severity".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.oincidents.insert(
            id,
            OfficeIncidentRow {
                id,
                org: org.into(),
                title: title.into(),
                severity: severity.into(),
                status: "open".into(),
            },
        );
        Ok(id)
    }
    async fn oincident_list(&self, org: &str) -> Result<Vec<OfficeIncidentRow>, StoreError> {
        let mut v: Vec<OfficeIncidentRow> = self
            .inner
            .lock()
            .unwrap()
            .oincidents
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn oincident_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .oincidents
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("open", "investigating") | ("investigating", "resolved") | ("resolved", "closed")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn oincident_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.oincidents.get(&id) {
            Some(r) if r.org == org => {
                m.oincidents.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn bankf_create(
        &self,
        org: &str,
        bank_name: &str,
        facility: &str,
        rate_text: &str,
        tenor: &str,
    ) -> Result<i64, StoreError> {
        let bank_name = bank_name.trim();
        if bank_name.is_empty() || bank_name.len() > 120 {
            return Err(StoreError::Conflict("bank_name".into()));
        }
        if facility.trim().is_empty() || facility.len() > 120 {
            return Err(StoreError::Conflict("facility".into()));
        }
        if rate_text.len() > 20 || tenor.len() > 40 {
            return Err(StoreError::Conflict("rate/tenor".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.bankf.insert(
            id,
            BankFacilityRow {
                id,
                org: org.into(),
                bank_name: bank_name.into(),
                facility: facility.trim().into(),
                rate_text: rate_text.trim().into(),
                tenor: tenor.trim().chars().take(40).collect(),
                active: true,
            },
        );
        Ok(id)
    }
    async fn bankf_list(&self, org: &str) -> Result<Vec<BankFacilityRow>, StoreError> {
        let mut v: Vec<BankFacilityRow> = self
            .inner
            .lock()
            .unwrap()
            .bankf
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn bankf_set_active(&self, org: &str, id: i64, active: bool) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .bankf
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        r.active = active;
        Ok(())
    }
    async fn bankf_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.bankf.get(&id) {
            Some(r) if r.org == org => {
                m.bankf.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn feedback_add(
        &self,
        org: &str,
        name: &str,
        message: &str,
        rating: i64,
    ) -> Result<i64, StoreError> {
        let message = message.trim();
        if message.is_empty() || message.len() > 2000 {
            return Err(StoreError::Conflict("message".into()));
        }
        if !(0..=5).contains(&rating) {
            return Err(StoreError::Conflict("rating 0-5".into()));
        }
        if name.len() > 120 {
            return Err(StoreError::Conflict("name".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.feedback.insert(
            id,
            FeedbackRow {
                id,
                org: org.into(),
                name: name.trim().chars().take(120).collect(),
                message: message.into(),
                rating,
                status: "new".into(),
            },
        );
        Ok(id)
    }
    async fn feedback_list(&self, org: &str) -> Result<Vec<FeedbackRow>, StoreError> {
        let mut v: Vec<FeedbackRow> = self
            .inner
            .lock()
            .unwrap()
            .feedback
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn feedback_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .feedback
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("new", "reviewed") | ("reviewed", "archived")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn feedback_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.feedback.get(&id) {
            Some(r) if r.org == org => {
                m.feedback.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn sop_create(
        &self,
        org: &str,
        code: &str,
        title: &str,
        version: &str,
        body: &str,
    ) -> Result<i64, StoreError> {
        let code = code.trim();
        let title = title.trim();
        if code.is_empty() || code.len() > 40 {
            return Err(StoreError::Conflict("code".into()));
        }
        if title.is_empty() || title.len() > 200 {
            return Err(StoreError::Conflict("title".into()));
        }
        if body.trim().is_empty() || body.len() > 200_000 {
            return Err(StoreError::Conflict("body".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if m.sops.values().any(|p| p.org == org && p.code == code) {
            return Err(StoreError::Conflict("duplicate code".into()));
        }
        let id = self.next(&mut m);
        m.sops.insert(
            id,
            SopRow {
                id,
                org: org.into(),
                code: code.into(),
                title: title.into(),
                version: version.trim().chars().take(20).collect(),
                body: body.trim().into(),
                status: "draft".into(),
            },
        );
        Ok(id)
    }
    async fn sop_list(&self, org: &str) -> Result<Vec<SopRow>, StoreError> {
        let mut v: Vec<SopRow> = self
            .inner
            .lock()
            .unwrap()
            .sops
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn sop_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .sops
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("draft", "published") | ("published", "archived")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn sop_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.sops.get(&id) {
            Some(r) if r.org == org => {
                m.sops.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn bcheck_add(
        &self,
        org: &str,
        checked_at: &str,
        method: &str,
        result: &str,
        note: &str,
    ) -> Result<i64, StoreError> {
        if checked_at.len() != 16
            || chrono::NaiveDateTime::parse_from_str(checked_at.trim(), "%Y-%m-%d %H:%M").is_err()
        {
            return Err(StoreError::Conflict("checked_at YYYY-MM-DD HH:MM".into()));
        }
        if !matches!(method, "dump" | "files" | "remote") {
            return Err(StoreError::Conflict("method".into()));
        }
        if !matches!(result, "pass" | "fail") {
            return Err(StoreError::Conflict("result".into()));
        }
        if note.len() > 500 {
            return Err(StoreError::Conflict("note".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.bchecks.insert(
            id,
            BackupCheckRow {
                id,
                org: org.into(),
                checked_at: checked_at.trim().into(),
                method: method.into(),
                result: result.into(),
                note: note.trim().into(),
            },
        );
        Ok(id)
    }
    async fn bcheck_list(&self, org: &str) -> Result<Vec<BackupCheckRow>, StoreError> {
        let mut v: Vec<BackupCheckRow> = self
            .inner
            .lock()
            .unwrap()
            .bchecks
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn bcheck_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.bchecks.get(&id) {
            Some(r) if r.org == org => {
                m.bchecks.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn selfcheck_add(
        &self,
        org: &str,
        period: &str,
        passed: i64,
        total: i64,
        note: &str,
    ) -> Result<i64, StoreError> {
        let period = period.trim();
        if period.is_empty() || period.len() > 20 {
            return Err(StoreError::Conflict("period".into()));
        }
        if !(0..=total).contains(&passed) || total <= 0 || total > 1000 {
            return Err(StoreError::Conflict("passed/total".into()));
        }
        if note.len() > 500 {
            return Err(StoreError::Conflict("note".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if m.selfchecks
            .values()
            .any(|r| r.org == org && r.period == period)
        {
            return Err(StoreError::Conflict("duplicate period".into()));
        }
        let id = self.next(&mut m);
        m.selfchecks.insert(
            id,
            SelfcheckRow {
                id,
                org: org.into(),
                period: period.into(),
                passed_items: passed,
                total_items: total,
                note: note.trim().into(),
            },
        );
        Ok(id)
    }
    async fn selfcheck_list(&self, org: &str) -> Result<Vec<SelfcheckRow>, StoreError> {
        let mut v: Vec<SelfcheckRow> = self
            .inner
            .lock()
            .unwrap()
            .selfchecks
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn selfcheck_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.selfchecks.get(&id) {
            Some(r) if r.org == org => {
                m.selfchecks.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn sealy_prepare(&self, org: &str, year: &str, note: &str) -> Result<i64, StoreError> {
        let year = year.trim();
        if year.len() != 4 || year.parse::<u32>().is_err() {
            return Err(StoreError::Conflict("year YYYY".into()));
        }
        if note.len() > 500 {
            return Err(StoreError::Conflict("note".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if m.seals.values().any(|r| r.org == org && r.year == year) {
            return Err(StoreError::Conflict("duplicate year".into()));
        }
        let count = m
            .pbooks
            .values()
            .filter(|b| b.org == org && b.book_year == year)
            .count() as i64;
        let id = self.next(&mut m);
        m.seals.insert(
            id,
            YearSealRow {
                id,
                org: org.into(),
                year: year.into(),
                book_count: count,
                note: note.trim().into(),
                status: "prepared".into(),
            },
        );
        Ok(id)
    }
    async fn sealy_list(&self, org: &str) -> Result<Vec<YearSealRow>, StoreError> {
        let mut v: Vec<YearSealRow> = self
            .inner
            .lock()
            .unwrap()
            .seals
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn sealy_seal(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .seals
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        if r.status != "prepared" {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = "sealed".into();
        Ok(())
    }
    async fn sealy_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.seals.get(&id) {
            Some(r) if r.org == org => {
                m.seals.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    #[allow(clippy::too_many_arguments)]
    async fn charge_upsert(
        &self,
        org: &str,
        ppat_id: i64,
        kind: &str,
        creditor: &str,
        ceiling_minor: i64,
        valid_until: &str,
        note: &str,
    ) -> Result<i64, StoreError> {
        if !matches!(kind, "apht" | "skmht") {
            return Err(StoreError::Conflict("kind apht|skmht".into()));
        }
        let creditor = creditor.trim();
        if creditor.is_empty() || creditor.len() > 160 {
            return Err(StoreError::Conflict("creditor".into()));
        }
        if ceiling_minor < 0 {
            return Err(StoreError::Conflict("ceiling".into()));
        }
        if valid_until.len() > 10 || note.len() > 500 {
            return Err(StoreError::Conflict("date/note".into()));
        }
        let mut m = self.inner.lock().unwrap();
        match m.ppat.get(&ppat_id) {
            Some(p) if p.org == org => {}
            _ => return Err(StoreError::NotFound),
        }
        if let Some((id, r)) = m
            .charges
            .iter_mut()
            .find(|(_, r)| r.org == org && r.ppat_id == ppat_id && r.kind == kind)
        {
            r.creditor = creditor.into();
            r.ceiling_minor = ceiling_minor;
            r.valid_until = valid_until.trim().into();
            r.note = note.trim().into();
            return Ok(*id);
        }
        let id = self.next(&mut m);
        m.charges.insert(
            id,
            DeedChargeRow {
                id,
                org: org.into(),
                ppat_id,
                kind: kind.into(),
                creditor: creditor.into(),
                ceiling_minor,
                valid_until: valid_until.trim().into(),
                note: note.trim().into(),
            },
        );
        Ok(id)
    }
    async fn charge_list(&self, org: &str, ppat_id: i64) -> Result<Vec<DeedChargeRow>, StoreError> {
        let mut v: Vec<DeedChargeRow> = self
            .inner
            .lock()
            .unwrap()
            .charges
            .values()
            .filter(|r| r.org == org && r.ppat_id == ppat_id)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn charge_due(&self, org: &str, days: i64) -> Result<Vec<DeedChargeRow>, StoreError> {
        let days = days.clamp(0, 365);
        let horizon = (chrono::Utc::now().naive_utc().date() + chrono::Duration::days(days))
            .format("%Y-%m-%d")
            .to_string();
        let mut v: Vec<DeedChargeRow> = self
            .inner
            .lock()
            .unwrap()
            .charges
            .values()
            .filter(|r| {
                r.org == org
                    && !r.valid_until.trim().is_empty()
                    && r.valid_until.as_str() <= horizon.as_str()
            })
            .cloned()
            .collect();
        v.sort_by(|a, b| a.valid_until.cmp(&b.valid_until));
        Ok(v)
    }
    async fn charge_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.charges.get(&id) {
            Some(r) if r.org == org => {
                m.charges.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn pparty_add(
        &self,
        org: &str,
        ppat_id: i64,
        role: &str,
        name: &str,
        id_number: &str,
    ) -> Result<i64, StoreError> {
        let name = name.trim();
        if name.is_empty() || name.len() > 160 {
            return Err(StoreError::Conflict("name".into()));
        }
        if !matches!(role, "seller" | "buyer" | "witness" | "spouse" | "heir") {
            return Err(StoreError::Conflict("role".into()));
        }
        if id_number.len() > 40 {
            return Err(StoreError::Conflict("id_number".into()));
        }
        let mut m = self.inner.lock().unwrap();
        match m.ppat.get(&ppat_id) {
            Some(p) if p.org == org => {}
            _ => return Err(StoreError::NotFound),
        }
        let id = self.next(&mut m);
        m.pparties.insert(
            id,
            PpatPartyRow {
                id,
                org: org.into(),
                ppat_id,
                role: role.into(),
                name: name.into(),
                id_number: id_number.trim().into(),
                verified: false,
            },
        );
        Ok(id)
    }
    async fn pparty_list(&self, org: &str, ppat_id: i64) -> Result<Vec<PpatPartyRow>, StoreError> {
        let mut v: Vec<PpatPartyRow> = self
            .inner
            .lock()
            .unwrap()
            .pparties
            .values()
            .filter(|r| r.org == org && r.ppat_id == ppat_id)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn pparty_set_verified(
        &self,
        org: &str,
        id: i64,
        verified: bool,
    ) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .pparties
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        r.verified = verified;
        Ok(())
    }
    async fn pparty_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.pparties.get(&id) {
            Some(r) if r.org == org => {
                m.pparties.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn chandover_add(
        &self,
        org: &str,
        ppat_id: i64,
        handed_to: &str,
        handed_at: &str,
        receipt_no: &str,
    ) -> Result<i64, StoreError> {
        let handed_to = handed_to.trim();
        if handed_to.is_empty() || handed_to.len() > 160 {
            return Err(StoreError::Conflict("handed_to".into()));
        }
        if handed_at.len() > 10 || receipt_no.len() > 60 {
            return Err(StoreError::Conflict("date/receipt".into()));
        }
        let mut m = self.inner.lock().unwrap();
        match m.ppat.get(&ppat_id) {
            Some(p) if p.org == org => {}
            _ => return Err(StoreError::NotFound),
        }
        let id = self.next(&mut m);
        m.chandovers.insert(
            id,
            CertHandoverRow {
                id,
                org: org.into(),
                ppat_id,
                handed_to: handed_to.into(),
                handed_at: handed_at.trim().into(),
                receipt_no: receipt_no.trim().chars().take(60).collect(),
                status: "prepared".into(),
            },
        );
        Ok(id)
    }
    async fn chandover_list(
        &self,
        org: &str,
        ppat_id: i64,
    ) -> Result<Vec<CertHandoverRow>, StoreError> {
        let mut v: Vec<CertHandoverRow> = self
            .inner
            .lock()
            .unwrap()
            .chandovers
            .values()
            .filter(|r| r.org == org && r.ppat_id == ppat_id)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn chandover_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .chandovers
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        if !matches!((r.status.as_str(), to), ("prepared", "handed")) {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn chandover_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.chandovers.get(&id) {
            Some(r) if r.org == org => {
                m.chandovers.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn zone_create(
        &self,
        org: &str,
        area_code: &str,
        area_name: &str,
        njop_minor: i64,
    ) -> Result<i64, StoreError> {
        let area_code = area_code.trim();
        let area_name = area_name.trim();
        if area_code.is_empty() || area_code.len() > 40 {
            return Err(StoreError::Conflict("area_code".into()));
        }
        if area_name.is_empty() || area_name.len() > 200 {
            return Err(StoreError::Conflict("area_name".into()));
        }
        if njop_minor < 0 {
            return Err(StoreError::Conflict("njop".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if m.zones
            .values()
            .any(|z| z.org == org && z.area_code == area_code)
        {
            return Err(StoreError::Conflict("duplicate area_code".into()));
        }
        let id = self.next(&mut m);
        m.zones.insert(
            id,
            ZoneRow {
                id,
                org: org.into(),
                area_code: area_code.into(),
                area_name: area_name.into(),
                njop_minor,
            },
        );
        Ok(id)
    }
    async fn zone_list(&self, org: &str) -> Result<Vec<ZoneRow>, StoreError> {
        let mut v: Vec<ZoneRow> = self
            .inner
            .lock()
            .unwrap()
            .zones
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn zone_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.zones.get(&id) {
            Some(r) if r.org == org => {
                m.zones.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn pbb_add(
        &self,
        org: &str,
        nop: &str,
        tax_year: &str,
        result: &str,
        checked_on: &str,
        note: &str,
    ) -> Result<i64, StoreError> {
        let nop = nop.trim();
        if nop.is_empty() || nop.len() > 40 {
            return Err(StoreError::Conflict("nop".into()));
        }
        if tax_year.len() != 4 || tax_year.parse::<u32>().is_err() {
            return Err(StoreError::Conflict("tax_year YYYY".into()));
        }
        if !matches!(result, "paid" | "unpaid" | "unknown") {
            return Err(StoreError::Conflict("result".into()));
        }
        if checked_on.len() > 10 || note.len() > 500 {
            return Err(StoreError::Conflict("note/date".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.pbb.insert(
            id,
            PbbRow {
                id,
                org: org.into(),
                nop: nop.into(),
                tax_year: tax_year.into(),
                result: result.into(),
                checked_on: checked_on.trim().into(),
                note: note.trim().into(),
            },
        );
        Ok(id)
    }
    async fn pbb_list(&self, org: &str) -> Result<Vec<PbbRow>, StoreError> {
        let mut v: Vec<PbbRow> = self
            .inner
            .lock()
            .unwrap()
            .pbb
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn pbb_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.pbb.get(&id) {
            Some(r) if r.org == org => {
                m.pbb.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn lsplit_create(
        &self,
        org: &str,
        source_parcel: &str,
        new_parcels: &str,
        split_date: &str,
    ) -> Result<i64, StoreError> {
        let sp = source_parcel.trim();
        let np = new_parcels.trim();
        if sp.is_empty() || sp.len() > 120 || np.is_empty() || np.len() > 1000 {
            return Err(StoreError::Conflict("parcels".into()));
        }
        if split_date.len() > 10 {
            return Err(StoreError::Conflict("split_date".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.lsplits.insert(
            id,
            LandSplitRow {
                id,
                org: org.into(),
                source_parcel: sp.into(),
                new_parcels: np.into(),
                split_date: split_date.trim().into(),
                status: "planned".into(),
            },
        );
        Ok(id)
    }
    async fn lsplit_list(&self, org: &str) -> Result<Vec<LandSplitRow>, StoreError> {
        let mut v: Vec<LandSplitRow> = self
            .inner
            .lock()
            .unwrap()
            .lsplits
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn lsplit_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .lsplits
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("planned", "surveyed") | ("surveyed", "registered")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn lsplit_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.lsplits.get(&id) {
            Some(r) if r.org == org => {
                m.lsplits.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn lmerge_create(
        &self,
        org: &str,
        source_parcels: &str,
        result_parcel: &str,
        merge_date: &str,
    ) -> Result<i64, StoreError> {
        let sp = source_parcels.trim();
        let rp = result_parcel.trim();
        if sp.is_empty() || sp.len() > 1000 || rp.is_empty() || rp.len() > 120 {
            return Err(StoreError::Conflict("parcels".into()));
        }
        if merge_date.len() > 10 {
            return Err(StoreError::Conflict("merge_date".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.lmerges.insert(
            id,
            LandMergeRow {
                id,
                org: org.into(),
                source_parcels: sp.into(),
                result_parcel: rp.into(),
                merge_date: merge_date.trim().into(),
                status: "planned".into(),
            },
        );
        Ok(id)
    }
    async fn lmerge_list(&self, org: &str) -> Result<Vec<LandMergeRow>, StoreError> {
        let mut v: Vec<LandMergeRow> = self
            .inner
            .lock()
            .unwrap()
            .lmerges
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn lmerge_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .lmerges
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("planned", "surveyed") | ("surveyed", "registered")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn lmerge_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.lmerges.get(&id) {
            Some(r) if r.org == org => {
                m.lmerges.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn ldispute_create(
        &self,
        org: &str,
        parcel_ref: &str,
        kind: &str,
        description: &str,
    ) -> Result<i64, StoreError> {
        let parcel_ref = parcel_ref.trim();
        let description = description.trim();
        if parcel_ref.is_empty() || parcel_ref.len() > 120 {
            return Err(StoreError::Conflict("parcel_ref".into()));
        }
        if !matches!(kind, "sengketa" | "batas" | "ganda" | "lainnya") {
            return Err(StoreError::Conflict("kind".into()));
        }
        if description.is_empty() || description.len() > 2000 {
            return Err(StoreError::Conflict("description".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.ldisputes.insert(
            id,
            LandDisputeRow {
                id,
                org: org.into(),
                parcel_ref: parcel_ref.into(),
                kind: kind.into(),
                description: description.into(),
                status: "flagged".into(),
            },
        );
        Ok(id)
    }
    async fn ldispute_list(&self, org: &str) -> Result<Vec<LandDisputeRow>, StoreError> {
        let mut v: Vec<LandDisputeRow> = self
            .inner
            .lock()
            .unwrap()
            .ldisputes
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn ldispute_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .ldisputes
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("flagged", "in_review") | ("in_review", "cleared")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn ldispute_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.ldisputes.get(&id) {
            Some(r) if r.org == org => {
                m.ldisputes.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn bndry_add(
        &self,
        org: &str,
        parcel_ref: &str,
        declared_area: i64,
        measured_area: i64,
        checked_on: &str,
    ) -> Result<i64, StoreError> {
        let parcel_ref = parcel_ref.trim();
        if parcel_ref.is_empty() || parcel_ref.len() > 120 {
            return Err(StoreError::Conflict("parcel_ref".into()));
        }
        if declared_area < 0 || measured_area < 0 {
            return Err(StoreError::Conflict("area".into()));
        }
        if checked_on.len() > 10 {
            return Err(StoreError::Conflict("checked_on".into()));
        }
        let result = if declared_area == 0 || measured_area == 0 {
            "pending"
        } else if declared_area == measured_area {
            "match"
        } else {
            "mismatch"
        };
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.boundaries.insert(
            id,
            BoundaryRow {
                id,
                org: org.into(),
                parcel_ref: parcel_ref.into(),
                declared_area,
                measured_area,
                result: result.into(),
                checked_on: checked_on.trim().into(),
            },
        );
        Ok(id)
    }
    async fn bndry_list(&self, org: &str) -> Result<Vec<BoundaryRow>, StoreError> {
        let mut v: Vec<BoundaryRow> = self
            .inner
            .lock()
            .unwrap()
            .boundaries
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn bndry_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.boundaries.get(&id) {
            Some(r) if r.org == org => {
                m.boundaries.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn dconsent_add(
        &self,
        org: &str,
        ppat_id: i64,
        kind: &str,
        party: &str,
        note: &str,
    ) -> Result<i64, StoreError> {
        if !matches!(kind, "marital" | "heir" | "board" | "power") {
            return Err(StoreError::Conflict("kind".into()));
        }
        let party = party.trim();
        if party.is_empty() || party.len() > 160 {
            return Err(StoreError::Conflict("party".into()));
        }
        if note.len() > 500 {
            return Err(StoreError::Conflict("note".into()));
        }
        let mut m = self.inner.lock().unwrap();
        match m.ppat.get(&ppat_id) {
            Some(p) if p.org == org => {}
            _ => return Err(StoreError::NotFound),
        }
        let id = self.next(&mut m);
        m.dconsents.insert(
            id,
            DeedConsentRow {
                id,
                org: org.into(),
                ppat_id,
                kind: kind.into(),
                party: party.into(),
                verified: false,
                note: note.trim().into(),
            },
        );
        Ok(id)
    }
    async fn dconsent_list(
        &self,
        org: &str,
        ppat_id: i64,
    ) -> Result<Vec<DeedConsentRow>, StoreError> {
        let mut v: Vec<DeedConsentRow> = self
            .inner
            .lock()
            .unwrap()
            .dconsents
            .values()
            .filter(|r| r.org == org && r.ppat_id == ppat_id)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn dconsent_verify(&self, org: &str, id: i64, verified: bool) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.dconsents.get_mut(&id) {
            Some(r) if r.org == org => {
                r.verified = verified;
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn dconsent_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.dconsents.get(&id) {
            Some(r) if r.org == org => {
                m.dconsents.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn lhist_add(
        &self,
        org: &str,
        parcel_ref: &str,
        event: &str,
        detail: &str,
        event_date: &str,
    ) -> Result<i64, StoreError> {
        let parcel_ref = parcel_ref.trim();
        let detail = detail.trim();
        if parcel_ref.is_empty() || parcel_ref.len() > 120 {
            return Err(StoreError::Conflict("parcel_ref".into()));
        }
        if !matches!(
            event,
            "alas_hak"
                | "jual_beli"
                | "hibah"
                | "waris"
                | "split"
                | "merge"
                | "sengketa"
                | "lainnya"
        ) {
            return Err(StoreError::Conflict("event".into()));
        }
        if detail.is_empty() || detail.len() > 2000 {
            return Err(StoreError::Conflict("detail".into()));
        }
        if event_date.len() > 10 {
            return Err(StoreError::Conflict("event_date".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.lhistory.insert(
            id,
            LandHistoryRow {
                id,
                org: org.into(),
                parcel_ref: parcel_ref.into(),
                event: event.into(),
                detail: detail.into(),
                event_date: event_date.trim().into(),
            },
        );
        Ok(id)
    }
    async fn lhist_list(
        &self,
        org: &str,
        parcel_ref: &str,
    ) -> Result<Vec<LandHistoryRow>, StoreError> {
        let mut v: Vec<LandHistoryRow> = self
            .inner
            .lock()
            .unwrap()
            .lhistory
            .values()
            .filter(|r| r.org == org && r.parcel_ref == parcel_ref)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn lhist_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.lhistory.get(&id) {
            Some(r) if r.org == org => {
                m.lhistory.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn uho_add(
        &self,
        org: &str,
        unit_ref: &str,
        handed_from: &str,
        handed_to: &str,
        bast_no: &str,
        handed_on: &str,
    ) -> Result<i64, StoreError> {
        let unit_ref = unit_ref.trim();
        if unit_ref.is_empty() || unit_ref.len() > 120 {
            return Err(StoreError::Conflict("unit_ref".into()));
        }
        for (name, v, max) in [
            ("handed_from", handed_from, 160),
            ("handed_to", handed_to, 160),
            ("bast_no", bast_no, 60),
        ] {
            if v.trim().is_empty() || v.len() > max {
                return Err(StoreError::Conflict(name.into()));
            }
        }
        if handed_on.len() > 10 {
            return Err(StoreError::Conflict("handed_on".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.uhandovers.insert(
            id,
            UnitHandoverRow {
                id,
                org: org.into(),
                unit_ref: unit_ref.into(),
                handed_from: handed_from.trim().into(),
                handed_to: handed_to.trim().into(),
                bast_no: bast_no.trim().into(),
                handed_on: handed_on.trim().into(),
                status: "draft".into(),
            },
        );
        Ok(id)
    }
    async fn uho_list(&self, org: &str) -> Result<Vec<UnitHandoverRow>, StoreError> {
        let mut v: Vec<UnitHandoverRow> = self
            .inner
            .lock()
            .unwrap()
            .uhandovers
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn uho_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        let r = m
            .uhandovers
            .get_mut(&id)
            .filter(|r| r.org == org)
            .ok_or(StoreError::NotFound)?;
        let ok = matches!(
            (r.status.as_str(), to),
            ("draft", "signed") | ("signed", "delivered")
        );
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        r.status = to.into();
        Ok(())
    }
    async fn uho_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.uhandovers.get(&id) {
            Some(r) if r.org == org => {
                m.uhandovers.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn delegation_set(
        &self,
        org: &str,
        delegator: &str,
        delegate: &str,
        active: bool,
    ) -> Result<i64, StoreError> {
        if delegator == delegate {
            return Err(StoreError::Conflict("self delegation".into()));
        }
        let mut m = self.inner.lock().unwrap();
        // Upsert on the org pair (idempotent toggle).
        if let Some((id, r)) = m
            .delegs
            .iter_mut()
            .find(|(_, r)| r.org == org && r.delegator == delegator && r.delegate == delegate)
        {
            r.active = active;
            return Ok(*id);
        }
        let id = self.next(&mut m);
        m.delegs.insert(
            id,
            DelegationRow {
                id,
                org: org.into(),
                delegator: delegator.into(),
                delegate: delegate.into(),
                active,
            },
        );
        Ok(id)
    }
    async fn delegation_list(&self, org: &str) -> Result<Vec<DelegationRow>, StoreError> {
        let mut v: Vec<DelegationRow> = self
            .inner
            .lock()
            .unwrap()
            .delegs
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn delegation_delegators(
        &self,
        org: &str,
        user: &str,
    ) -> Result<Vec<String>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .delegs
            .values()
            .filter(|r| r.org == org && r.delegate == user && r.active)
            .map(|r| r.delegator.clone())
            .collect())
    }
    async fn invite_create(
        &self,
        org: &str,
        email: &str,
        role: &str,
        hash: &str,
        expires_at: &str,
    ) -> Result<i64, StoreError> {
        let email = email.trim().to_lowercase();
        if !email.contains('@') || role.trim().is_empty() || role.len() > 60 {
            return Err(StoreError::Conflict("email/role".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        m.invites.insert(
            id,
            (
                InviteRow {
                    id,
                    org: org.into(),
                    email: email.clone(),
                    role: role.trim().into(),
                    accepted: false,
                },
                hash.into(),
                expires_at.into(),
            ),
        );
        Ok(id)
    }
    async fn invite_list(&self, org: &str) -> Result<Vec<InviteRow>, StoreError> {
        let mut v: Vec<InviteRow> = self
            .inner
            .lock()
            .unwrap()
            .invites
            .values()
            .filter(|(r, _, _)| r.org == org)
            .map(|(r, _, _)| r.clone())
            .collect();
        v.sort_by_key(|r| -r.id);
        Ok(v)
    }
    async fn invite_peek(&self, hash: &str) -> Result<Option<InviteRow>, StoreError> {
        let now = chrono::Utc::now()
            .naive_utc()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        Ok(self
            .inner
            .lock()
            .unwrap()
            .invites
            .values()
            .find(|(r, h, exp)| h == hash && exp.as_str() > now.as_str() && !r.accepted)
            .map(|(r, _, _)| r.clone()))
    }
    async fn invite_redeem(&self, hash: &str) -> Result<InviteRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let now = chrono::Utc::now()
            .naive_utc()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let e = m
            .invites
            .values_mut()
            .find(|(_, h, exp)| h == hash && exp.as_str() > now.as_str())
            .ok_or(StoreError::NotFound)?;
        if e.0.accepted {
            return Err(StoreError::Conflict("already accepted".into()));
        }
        e.0.accepted = true;
        Ok(e.0.clone())
    }
    async fn invite_revoke(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.invites.get(&id) {
            Some((r, _, _)) if r.org == org => {
                m.invites.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn notif_pref_set(
        &self,
        user: &str,
        kind: &str,
        enabled: bool,
    ) -> Result<(), StoreError> {
        if kind.trim().is_empty() || kind.len() > 64 {
            return Err(StoreError::Conflict("kind".into()));
        }
        self.inner
            .lock()
            .unwrap()
            .prefs
            .insert((user.into(), kind.trim().into()), enabled);
        Ok(())
    }
    async fn notif_pref_list(&self, user: &str) -> Result<Vec<(String, bool)>, StoreError> {
        let mut v: Vec<(String, bool)> = self
            .inner
            .lock()
            .unwrap()
            .prefs
            .iter()
            .filter(|((u, _), _)| u == user)
            .map(|((_, k), e)| (k.clone(), *e))
            .collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(v)
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
}

#[async_trait::async_trait]
impl super::ops::MarketingStore for MemoryOps {
    async fn mkt_lead_create(
        &self,
        org: Option<&str>,
        name: &str,
        email: &str,
        phone: Option<&str>,
        company: Option<&str>,
        source: &str,
    ) -> Result<MarketingLeadRow, StoreError> {
        if name.trim().is_empty() || !email.contains('@') {
            return Err(StoreError::Conflict("name and valid email required".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        let r = MarketingLeadRow {
            id,
            org: org.map(str::to_string),
            name: name.into(),
            email: email.into(),
            phone: phone.map(str::to_string),
            company: company.map(str::to_string),
            source: source.into(),
            status: "new".into(),
            assigned_to: None,
        };
        m.mkt_leads.insert(id, r.clone());
        Ok(r)
    }
    async fn mkt_lead_list(&self, org: Option<&str>) -> Result<Vec<MarketingLeadRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .mkt_leads
            .values()
            .filter(|l| l.org.as_deref() == org)
            .cloned()
            .collect())
    }
    async fn mkt_lead_get(&self, id: i64) -> Result<MarketingLeadRow, StoreError> {
        self.inner
            .lock()
            .unwrap()
            .mkt_leads
            .get(&id)
            .cloned()
            .ok_or(StoreError::NotFound)
    }
    async fn mkt_lead_set_status(
        &self,
        id: i64,
        status: &str,
    ) -> Result<MarketingLeadRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let l = m.mkt_leads.get_mut(&id).ok_or(StoreError::NotFound)?;
        if !super::ops::marketing_allowed_transition(&l.status, status) {
            return Err(StoreError::Conflict(format!(
                "illegal transition {} -> {status}",
                l.status
            )));
        }
        l.status = status.into();
        Ok(l.clone())
    }
    async fn mkt_lead_assign(
        &self,
        id: i64,
        assignee: Option<&str>,
    ) -> Result<MarketingLeadRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let l = m.mkt_leads.get_mut(&id).ok_or(StoreError::NotFound)?;
        l.assigned_to = assignee.map(str::to_string);
        Ok(l.clone())
    }
    async fn mkt_note_add(
        &self,
        lead: i64,
        author: &str,
        note: &str,
    ) -> Result<MarketingNoteRow, StoreError> {
        if note.trim().is_empty() {
            return Err(StoreError::Conflict("note required".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if !m.mkt_leads.contains_key(&lead) {
            return Err(StoreError::NotFound);
        }
        let id = m.seq + 1;
        m.seq = id;
        let r = MarketingNoteRow {
            id,
            lead_id: lead,
            author: author.into(),
            note: note.into(),
        };
        m.mkt_notes.entry(lead).or_default().push(r.clone());
        Ok(r)
    }
    async fn mkt_note_list(&self, lead: i64) -> Result<Vec<MarketingNoteRow>, StoreError> {
        let m = self.inner.lock().unwrap();
        if !m.mkt_leads.contains_key(&lead) {
            return Err(StoreError::NotFound);
        }
        Ok(m.mkt_notes.get(&lead).cloned().unwrap_or_default())
    }
    async fn mkt_followup_add(
        &self,
        lead: i64,
        due_at: &str,
        note: &str,
    ) -> Result<MarketingFollowUpRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        if !m.mkt_leads.contains_key(&lead) {
            return Err(StoreError::NotFound);
        }
        let id = m.seq + 1;
        m.seq = id;
        let r = MarketingFollowUpRow {
            id,
            lead_id: lead,
            due_at: due_at.into(),
            done: false,
            note: note.into(),
        };
        m.mkt_followups.insert(id, r.clone());
        Ok(r)
    }
    async fn mkt_followup_list(&self, lead: i64) -> Result<Vec<MarketingFollowUpRow>, StoreError> {
        let m = self.inner.lock().unwrap();
        if !m.mkt_leads.contains_key(&lead) {
            return Err(StoreError::NotFound);
        }
        Ok(m.mkt_followups
            .values()
            .filter(|f| f.lead_id == lead)
            .cloned()
            .collect())
    }
    async fn mkt_followup_done(&self, id: i64) -> Result<MarketingFollowUpRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let f = m.mkt_followups.get_mut(&id).ok_or(StoreError::NotFound)?;
        f.done = true;
        Ok(f.clone())
    }
    async fn mkt_demo_create(
        &self,
        lead: Option<i64>,
        name: &str,
        email: &str,
        company: &str,
        preferred_at: Option<&str>,
    ) -> Result<MarketingDemoRow, StoreError> {
        if !email.contains('@') {
            return Err(StoreError::Conflict("valid email required".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if let Some(lid) = lead {
            if !m.mkt_leads.contains_key(&lid) {
                return Err(StoreError::NotFound);
            }
        }
        let id = self.next(&mut m);
        let r = MarketingDemoRow {
            id,
            lead_id: lead,
            name: name.into(),
            email: email.into(),
            company: company.into(),
            preferred_at: preferred_at.map(str::to_string),
            status: "new".into(),
        };
        m.mkt_demos.insert(id, r.clone());
        Ok(r)
    }
    async fn mkt_demo_list(&self) -> Result<Vec<MarketingDemoRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .mkt_demos
            .values()
            .cloned()
            .collect())
    }
    async fn mkt_contact_create(
        &self,
        name: &str,
        email: &str,
        subject: &str,
        message: &str,
    ) -> Result<MarketingContactRow, StoreError> {
        if !email.contains('@') || message.trim().is_empty() {
            return Err(StoreError::Conflict(
                "valid email and message required".into(),
            ));
        }
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        let r = MarketingContactRow {
            id,
            name: name.into(),
            email: email.into(),
            subject: subject.into(),
            message: message.into(),
            status: "new".into(),
        };
        m.mkt_contacts.insert(id, r.clone());
        Ok(r)
    }
    async fn mkt_contact_list(&self) -> Result<Vec<MarketingContactRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .mkt_contacts
            .values()
            .cloned()
            .collect())
    }
    async fn mkt_subscribe(&self, email: &str) -> Result<MarketingSubscriberRow, StoreError> {
        let key = email.trim().to_lowercase();
        if !key.contains('@') {
            return Err(StoreError::Conflict("valid email required".into()));
        }
        let mut m = self.inner.lock().unwrap();
        if m.mkt_subs.contains_key(&key) {
            return Err(StoreError::Conflict("already subscribed".into()));
        }
        let r = MarketingSubscriberRow {
            email: key.clone(),
            confirmed: false,
        };
        m.mkt_subs.insert(key, r.clone());
        Ok(r)
    }
    async fn mkt_unsubscribe(&self, email: &str) -> Result<(), StoreError> {
        let key = email.trim().to_lowercase();
        self.inner
            .lock()
            .unwrap()
            .mkt_subs
            .remove(&key)
            .map(|_| ())
            .ok_or(StoreError::NotFound)
    }
    async fn mkt_pricing_upsert(
        &self,
        code: &str,
        name: &str,
        monthly_minor: i64,
        annual_minor: i64,
        features_json: &str,
        active: bool,
    ) -> Result<MarketingPlanRow, StoreError> {
        if code.trim().is_empty() {
            return Err(StoreError::Conflict("code required".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = m.mkt_plans.get(code).map(|p| p.id).unwrap_or_else(|| {
            m.seq += 1;
            m.seq
        });
        let r = MarketingPlanRow {
            id,
            code: code.into(),
            name: name.into(),
            monthly_minor,
            annual_minor,
            features_json: features_json.into(),
            active,
        };
        m.mkt_plans.insert(code.into(), r.clone());
        Ok(r)
    }
    async fn mkt_pricing_list(&self) -> Result<Vec<MarketingPlanRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .mkt_plans
            .values()
            .cloned()
            .collect())
    }
    async fn mkt_faq_upsert(
        &self,
        slug: &str,
        question: &str,
        answer: &str,
        sort: i64,
    ) -> Result<MarketingFaqRow, StoreError> {
        if slug.trim().is_empty() {
            return Err(StoreError::Conflict("slug required".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = m.mkt_faqs.get(slug).map(|f| f.id).unwrap_or_else(|| {
            m.seq += 1;
            m.seq
        });
        let r = MarketingFaqRow {
            id,
            slug: slug.into(),
            question: question.into(),
            answer: answer.into(),
            sort,
        };
        m.mkt_faqs.insert(slug.into(), r.clone());
        Ok(r)
    }
    async fn mkt_faq_list(&self) -> Result<Vec<MarketingFaqRow>, StoreError> {
        let mut v: Vec<MarketingFaqRow> = self
            .inner
            .lock()
            .unwrap()
            .mkt_faqs
            .values()
            .cloned()
            .collect();
        v.sort_by_key(|f| f.sort);
        Ok(v)
    }
    async fn mkt_article_upsert(
        &self,
        slug: &str,
        title: &str,
        body_md: &str,
        published: bool,
    ) -> Result<MarketingArticleRow, StoreError> {
        if slug.trim().is_empty() {
            return Err(StoreError::Conflict("slug required".into()));
        }
        let mut m = self.inner.lock().unwrap();
        let id = m.mkt_articles.get(slug).map(|a| a.id).unwrap_or_else(|| {
            m.seq += 1;
            m.seq
        });
        let r = MarketingArticleRow {
            id,
            slug: slug.into(),
            title: title.into(),
            body_md: body_md.into(),
            published,
        };
        m.mkt_articles.insert(slug.into(), r.clone());
        Ok(r)
    }
    async fn mkt_article_get(&self, slug: &str) -> Result<MarketingArticleRow, StoreError> {
        self.inner
            .lock()
            .unwrap()
            .mkt_articles
            .get(slug)
            .cloned()
            .ok_or(StoreError::NotFound)
    }
    async fn mkt_article_list(
        &self,
        published_only: bool,
    ) -> Result<Vec<MarketingArticleRow>, StoreError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .mkt_articles
            .values()
            .filter(|a| !published_only || a.published)
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod marketing_tests {
    use super::*;
    use crate::ops::{MarketingStore, OpsStore};

    #[tokio::test]
    async fn pipeline_transition_ok_and_illegal() {
        let ops = MemoryOps::default();
        let l = ops
            .mkt_lead_create(None, "Acme", "a@acme.id", None, None, "website")
            .await
            .unwrap();
        assert_eq!(l.status, "new");
        let l = ops.mkt_lead_set_status(l.id, "contacted").await.unwrap();
        assert_eq!(l.status, "contacted");
        // skip-ahead is illegal
        let err = ops.mkt_lead_set_status(l.id, "demo").await.unwrap_err();
        assert!(matches!(err, StoreError::Conflict(_)));
        // status unchanged after illegal move
        assert_eq!(ops.mkt_lead_get(l.id).await.unwrap().status, "contacted");
        // unknown status is illegal
        let err = ops.mkt_lead_set_status(l.id, "bogus").await.unwrap_err();
        assert!(matches!(err, StoreError::Conflict(_)));
    }

    #[tokio::test]
    async fn newsletter_duplicate_is_conflict() {
        let ops = MemoryOps::default();
        ops.mkt_subscribe("user@example.com").await.unwrap();
        let err = ops.mkt_subscribe("user@example.com").await.unwrap_err();
        assert!(matches!(err, StoreError::Conflict(_)));
        ops.mkt_unsubscribe("user@example.com").await.unwrap();
        // resubscribe after unsubscribe works
        ops.mkt_subscribe("user@example.com").await.unwrap();
    }

    #[tokio::test]
    async fn followup_done_flow() {
        let ops = MemoryOps::default();
        let l = ops
            .mkt_lead_create(None, "Acme", "a@acme.id", None, None, "website")
            .await
            .unwrap();
        let f = ops
            .mkt_followup_add(l.id, "2026-12-01 10:00:00", "call back")
            .await
            .unwrap();
        assert!(!f.done);
        let list = ops.mkt_followup_list(l.id).await.unwrap();
        assert_eq!(list.len(), 1);
        let done = ops.mkt_followup_done(f.id).await.unwrap();
        assert!(done.done);
    }

    #[tokio::test]
    async fn notes_require_lead_and_content() {
        let ops = MemoryOps::default();
        let err = ops.mkt_note_add(999, "me", "hi").await.unwrap_err();
        assert!(matches!(err, StoreError::NotFound));
        let l = ops
            .mkt_lead_create(None, "Acme", "a@acme.id", None, None, "website")
            .await
            .unwrap();
        let err = ops.mkt_note_add(l.id, "me", "  ").await.unwrap_err();
        assert!(matches!(err, StoreError::Conflict(_)));
        let n = ops.mkt_note_add(l.id, "me", "called").await.unwrap();
        assert_eq!(ops.mkt_note_list(l.id).await.unwrap().len(), 1);
        assert_eq!(n.note, "called");
    }

    #[tokio::test]
    async fn catalog_upsert_flows() {
        let ops = MemoryOps::default();
        let p = ops
            .mkt_pricing_upsert("pro", "Pro", 99000, 990000, "[]", true)
            .await
            .unwrap();
        let p2 = ops
            .mkt_pricing_upsert("pro", "Pro+", 149000, 1490000, "[]", true)
            .await
            .unwrap();
        assert_eq!(p.id, p2.id);
        assert_eq!(ops.mkt_pricing_list().await.unwrap().len(), 1);
        ops.mkt_faq_upsert("s1", "Q?", "A.", 1).await.unwrap();
        assert_eq!(ops.mkt_faq_list().await.unwrap().len(), 1);
        ops.mkt_article_upsert("hello", "Hello", "# hi", false)
            .await
            .unwrap();
        assert!(ops.mkt_article_list(true).await.unwrap().is_empty());
        assert_eq!(ops.mkt_article_list(false).await.unwrap().len(), 1);
        ops.mkt_article_upsert("hello", "Hello", "# hi", true)
            .await
            .unwrap();
        assert_eq!(ops.mkt_article_list(true).await.unwrap().len(), 1);
        assert_eq!(ops.mkt_article_get("hello").await.unwrap().title, "Hello");
        let err = ops.mkt_article_get("missing").await.unwrap_err();
        assert!(matches!(err, StoreError::NotFound));
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

fn crm_check_meeting(title: &str, happened_at: &str) -> Result<(), StoreError> {
    if title.trim().chars().count() < 3 {
        return Err(StoreError::Conflict(
            "title must be at least 3 chars".into(),
        ));
    }
    if chrono::NaiveDateTime::parse_from_str(happened_at, "%Y-%m-%d %H:%M:%S").is_err() {
        return Err(StoreError::Conflict(
            "happened_at must be YYYY-MM-DD HH:MM:SS".into(),
        ));
    }
    Ok(())
}

fn crm_check_tag(name: &str) -> Result<String, StoreError> {
    let clean = name.trim();
    if clean.is_empty() {
        return Err(StoreError::Conflict("tag name required".into()));
    }
    if clean.chars().count() > 64 {
        return Err(StoreError::Conflict(
            "tag name must be at most 64 chars".into(),
        ));
    }
    Ok(clean.to_string())
}

#[async_trait::async_trait]
impl super::ops::CrmStore for MemoryOps {
    async fn meeting_create(
        &self,
        org: &str,
        client_id: i64,
        title: &str,
        notes: Option<&str>,
        happened_at: &str,
        created_by: &str,
    ) -> Result<MeetingRow, StoreError> {
        crm_check_meeting(title, happened_at)?;
        let mut m = self.inner.lock().unwrap();
        let id = self.next(&mut m);
        let r = MeetingRow {
            id,
            org: org.into(),
            client_id,
            title: title.into(),
            notes: notes.map(str::to_string),
            happened_at: happened_at.into(),
            created_by: created_by.into(),
        };
        m.meetings.insert(id, r.clone());
        Ok(r)
    }
    async fn meeting_list_by_client(
        &self,
        org: &str,
        client_id: i64,
    ) -> Result<Vec<MeetingRow>, StoreError> {
        let mut v: Vec<MeetingRow> = self
            .inner
            .lock()
            .unwrap()
            .meetings
            .values()
            .filter(|r| r.org == org && r.client_id == client_id)
            .cloned()
            .collect();
        v.sort_by_key(|r| r.id);
        Ok(v)
    }
    async fn meeting_list_by_org(&self, org: &str) -> Result<Vec<MeetingRow>, StoreError> {
        let mut v: Vec<MeetingRow> = self
            .inner
            .lock()
            .unwrap()
            .meetings
            .values()
            .filter(|r| r.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|r| r.id);
        Ok(v)
    }
    async fn meeting_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.meetings.get(&id) {
            Some(r) if r.org == org => {
                m.meetings.remove(&id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn tag_create(&self, org: &str, name: &str) -> Result<ClientTagRow, StoreError> {
        let clean = crm_check_tag(name)?;
        let mut m = self.inner.lock().unwrap();
        if m.crm_tags.values().any(|t| t.org == org && t.name == clean) {
            return Err(StoreError::Conflict("tag taken".into()));
        }
        let id = self.next(&mut m);
        let r = ClientTagRow {
            id,
            org: org.into(),
            name: clean,
        };
        m.crm_tags.insert(id, r.clone());
        Ok(r)
    }
    async fn tag_list(&self, org: &str) -> Result<Vec<ClientTagRow>, StoreError> {
        let mut v: Vec<ClientTagRow> = self
            .inner
            .lock()
            .unwrap()
            .crm_tags
            .values()
            .filter(|t| t.org == org)
            .cloned()
            .collect();
        v.sort_by_key(|t| t.id);
        Ok(v)
    }
    async fn tag_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        match m.crm_tags.get(&id) {
            Some(t) if t.org == org => {
                m.crm_tags.remove(&id);
                // Cascade: drop links the way ON DELETE CASCADE does in MySQL.
                m.crm_links.retain(|(tid, _)| *tid != id);
                Ok(())
            }
            _ => Err(StoreError::NotFound),
        }
    }
    async fn tag_attach(&self, org: &str, tag_id: i64, client_id: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        if !m.crm_tags.get(&tag_id).is_some_and(|t| t.org == org) {
            return Err(StoreError::NotFound);
        }
        // Idempotent: re-attaching an existing link is a no-op.
        m.crm_links.insert((tag_id, client_id));
        Ok(())
    }
    async fn tag_detach(&self, _org: &str, tag_id: i64, client_id: i64) -> Result<(), StoreError> {
        // Idempotent: detaching a missing link is a no-op.
        self.inner
            .lock()
            .unwrap()
            .crm_links
            .remove(&(tag_id, client_id));
        Ok(())
    }
    async fn tags_for_client(
        &self,
        org: &str,
        client_id: i64,
    ) -> Result<Vec<ClientTagRow>, StoreError> {
        let m = self.inner.lock().unwrap();
        let mut v: Vec<ClientTagRow> = m
            .crm_tags
            .values()
            .filter(|t| t.org == org && m.crm_links.contains(&(t.id, client_id)))
            .cloned()
            .collect();
        v.sort_by_key(|t| t.id);
        Ok(v)
    }
}

#[cfg(test)]
mod crm_tests {
    use super::*;
    use crate::ops::CrmStore;

    #[tokio::test]
    async fn meeting_crud_scoped_by_client() {
        let ops = MemoryOps::default();
        let a = ops
            .meeting_create(
                "org1",
                7,
                "Kickoff call",
                Some("scope notes"),
                "2026-11-18 10:00:00",
                "alice",
            )
            .await
            .unwrap();
        assert_eq!(a.client_id, 7);
        ops.meeting_create("org1", 8, "Site visit", None, "2026-11-19 09:00:00", "bob")
            .await
            .unwrap();
        let mine = ops.meeting_list_by_client("org1", 7).await.unwrap();
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].title, "Kickoff call");
        assert_eq!(ops.meeting_list_by_org("org1").await.unwrap().len(), 2);
        ops.meeting_delete("org1", a.id).await.unwrap();
        assert!(ops
            .meeting_list_by_client("org1", 7)
            .await
            .unwrap()
            .is_empty());
        assert_eq!(ops.meeting_list_by_org("org1").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn meeting_rejects_short_title() {
        let ops = MemoryOps::default();
        for bad in ["", "  ", "ab"] {
            let err = ops
                .meeting_create("org1", 1, bad, None, "2026-11-18 10:00:00", "alice")
                .await
                .unwrap_err();
            assert!(matches!(err, StoreError::Conflict(_)));
        }
    }

    #[tokio::test]
    async fn meeting_rejects_bad_timestamp() {
        let ops = MemoryOps::default();
        for bad in ["", "2026-11-18", "2026-13-40 99:99:99"] {
            let err = ops
                .meeting_create("org1", 1, "Kickoff call", None, bad, "alice")
                .await
                .unwrap_err();
            assert!(matches!(err, StoreError::Conflict(_)));
        }
    }

    #[tokio::test]
    async fn meeting_delete_missing_or_foreign_org_is_not_found() {
        let ops = MemoryOps::default();
        let err = ops.meeting_delete("org1", 999).await.unwrap_err();
        assert!(matches!(err, StoreError::NotFound));
        let a = ops
            .meeting_create(
                "org1",
                1,
                "Kickoff call",
                None,
                "2026-11-18 10:00:00",
                "alice",
            )
            .await
            .unwrap();
        let err = ops.meeting_delete("org2", a.id).await.unwrap_err();
        assert!(matches!(err, StoreError::NotFound));
        // Row untouched by the foreign-org delete.
        assert_eq!(ops.meeting_list_by_org("org1").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn tag_validation_and_dedup() {
        let ops = MemoryOps::default();
        for bad in ["", "   ", &"x".repeat(65)] {
            let err = ops.tag_create("org1", bad).await.unwrap_err();
            assert!(matches!(err, StoreError::Conflict(_)));
        }
        let t = ops.tag_create("org1", "  vip  ").await.unwrap();
        assert_eq!(t.name, "vip");
        let err = ops.tag_create("org1", "vip").await.unwrap_err();
        assert!(matches!(err, StoreError::Conflict(_)));
        // Same name in another org is fine.
        let other = ops.tag_create("org2", "vip").await.unwrap();
        assert_ne!(t.id, other.id);
        assert_eq!(ops.tag_list("org1").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn tag_attach_detach_idempotent() {
        let ops = MemoryOps::default();
        let t = ops.tag_create("org1", "vip").await.unwrap();
        // Unknown tag cannot be attached.
        let err = ops.tag_attach("org1", 999, 7).await.unwrap_err();
        assert!(matches!(err, StoreError::NotFound));
        ops.tag_attach("org1", t.id, 7).await.unwrap();
        // Re-attach is a no-op, not a conflict.
        ops.tag_attach("org1", t.id, 7).await.unwrap();
        let tags = ops.tags_for_client("org1", 7).await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "vip");
        ops.tag_detach("org1", t.id, 7).await.unwrap();
        // Re-detach is a no-op.
        ops.tag_detach("org1", t.id, 7).await.unwrap();
        assert!(ops.tags_for_client("org1", 7).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn tag_delete_cascades_links() {
        let ops = MemoryOps::default();
        let keep = ops.tag_create("org1", "keep").await.unwrap();
        let drop = ops.tag_create("org1", "drop").await.unwrap();
        ops.tag_attach("org1", keep.id, 7).await.unwrap();
        ops.tag_attach("org1", drop.id, 7).await.unwrap();
        assert_eq!(ops.tags_for_client("org1", 7).await.unwrap().len(), 2);
        ops.tag_delete("org1", drop.id).await.unwrap();
        // Link rows vanish with the tag (mirrors ON DELETE CASCADE).
        let tags = ops.tags_for_client("org1", 7).await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].id, keep.id);
        let err = ops.tag_delete("org1", drop.id).await.unwrap_err();
        assert!(matches!(err, StoreError::NotFound));
    }

    #[tokio::test]
    async fn crm_state_scoped_by_org() {
        let ops = MemoryOps::default();
        let t = ops.tag_create("org1", "vip").await.unwrap();
        ops.tag_attach("org1", t.id, 7).await.unwrap();
        ops.meeting_create(
            "org1",
            7,
            "Kickoff call",
            None,
            "2026-11-18 10:00:00",
            "alice",
        )
        .await
        .unwrap();
        // Other org sees nothing.
        assert!(ops.meeting_list_by_org("org2").await.unwrap().is_empty());
        assert!(ops
            .meeting_list_by_client("org2", 7)
            .await
            .unwrap()
            .is_empty());
        assert!(ops.tags_for_client("org2", 7).await.unwrap().is_empty());
        assert!(ops.tag_list("org2").await.unwrap().is_empty());
        // Cross-org attach is rejected.
        let err = ops.tag_attach("org2", t.id, 7).await.unwrap_err();
        assert!(matches!(err, StoreError::NotFound));
    }
}

#[cfg(test)]
mod compliance_tests {
    use super::*;
    use crate::ops::OpsStore;

    #[tokio::test]
    async fn ropa_crud_and_tenant_isolation() {
        let ops = MemoryOps::default();
        assert!(ops
            .ropa_create("org1", "  ", "p", "", "", "")
            .await
            .is_err());
        let id = ops
            .ropa_create("org1", "ROPA A", "Pemasaran", "nik", "consent", "2y")
            .await
            .unwrap();
        let v = ops.ropa_list("org1").await.unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].legal_basis, "consent");
        assert!(ops.ropa_list("org2").await.unwrap().is_empty());
        ops.ropa_delete("org2", id).await.unwrap_err();
        ops.ropa_delete("org1", id).await.unwrap();
        assert!(ops.ropa_list("org1").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn dpia_breach_guards_and_isolation() {
        let ops = MemoryOps::default();
        assert!(ops.dpia_create("o1", "  ", "", "high").await.is_err());
        assert!(ops.dpia_create("o1", "D", "", "extreme").await.is_err());
        let d = ops.dpia_create("o1", "D", "nik", "high").await.unwrap();
        assert!(ops.dpia_list("o2").await.unwrap().is_empty());
        assert!(ops.dpia_set_status("o1", d, "approved").await.is_err());
        assert!(ops.dpia_set_status("o2", d, "review").await.is_err());
        ops.dpia_set_status("o1", d, "review").await.unwrap();
        ops.dpia_set_status("o1", d, "approved").await.unwrap();
        assert!(ops.breach_create("o1", "  ", "", 0).await.is_err());
        assert!(ops.breach_create("o1", "B", "", -1).await.is_err());
        let b = ops.breach_create("o1", "B", "2026-11-18", 5).await.unwrap();
        assert!(ops.breach_set_status("o1", b, "closed").await.is_err());
        for st in ["assessing", "notified", "closed"] {
            ops.breach_set_status("o1", b, st).await.unwrap();
        }
        assert!(ops.breach_list("o2").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn vendor_crud_and_dpa_isolation() {
        let ops = MemoryOps::default();
        assert!(ops.vendor_create("o1", "  ", "").await.is_err());
        let id = ops.vendor_create("o1", "Cloud", "storage").await.unwrap();
        assert!(ops.vendor_list("o2").await.unwrap().is_empty());
        assert!(ops.vendor_set_dpa("o1", id, "bogus").await.is_err());
        assert!(ops.vendor_set_dpa("o2", id, "review").await.is_err());
        ops.vendor_set_dpa("o1", id, "review").await.unwrap();
        ops.vendor_set_dpa("o1", id, "signed").await.unwrap();
        assert!(ops.vendor_delete("o2", id).await.is_err());
        ops.vendor_delete("o1", id).await.unwrap();
        assert!(ops.vendor_delete("o1", id).await.is_err());
        assert!(ops.vendor_list("o1").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn delegation_upsert_and_resolve() {
        let ops = MemoryOps::default();
        assert!(ops.delegation_set("o1", "u1", "u1", true).await.is_err());
        let id = ops.delegation_set("o1", "u1", "u2", true).await.unwrap();
        assert_eq!(
            ops.delegation_delegators("o1", "u2").await.unwrap(),
            vec!["u1".to_string()]
        );
        assert!(ops
            .delegation_delegators("o2", "u2")
            .await
            .unwrap()
            .is_empty());
        // Toggle off is an upsert on the same pair (same id).
        assert_eq!(
            ops.delegation_set("o1", "u1", "u2", false).await.unwrap(),
            id
        );
        assert!(ops
            .delegation_delegators("o1", "u2")
            .await
            .unwrap()
            .is_empty());
        let v = ops.delegation_list("o1").await.unwrap();
        assert_eq!(v.len(), 1);
        assert!(!v[0].active);
        assert!(ops.delegation_list("o2").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn ppat_register_guards_and_transitions() {
        let ops = MemoryOps::default();
        assert!(ops
            .ppat_create("o1", "  ", "ajb", "", "", "")
            .await
            .is_err());
        assert!(ops
            .ppat_create("o1", "1/26", "gadai", "", "", "")
            .await
            .is_err());
        let id = ops
            .ppat_create("o1", "1/26", "ajb", "A", "B", "SHM 1")
            .await
            .unwrap();
        assert!(ops
            .ppat_create("o1", "1/26", "hibah", "", "", "")
            .await
            .is_err());
        assert!(ops.ppat_list("o2").await.unwrap().is_empty());
        // Skip-ahead rejected; step through draft -> signed -> registered.
        assert!(ops.ppat_set_status("o1", id, "registered").await.is_err());
        ops.ppat_set_status("o1", id, "signed").await.unwrap();
        assert!(ops.ppat_set_status("o2", id, "registered").await.is_err());
        ops.ppat_set_status("o1", id, "registered").await.unwrap();
        assert!(ops.ppat_set_status("o1", id, "draft").await.is_err());
        let v = ops.ppat_list("o1").await.unwrap();
        assert_eq!(v[0].status, "registered");
    }

    #[tokio::test]
    async fn obligation_lifecycle_and_tenant_isolation() {
        let ops = MemoryOps::default();
        let id = ops
            .pdp_obligation_create("org1", "ROPA review", "ropa", "dpo")
            .await
            .unwrap();
        assert!(ops
            .pdp_obligation_create("org1", "  ", "ropa", "")
            .await
            .is_err());
        let v = ops.pdp_obligation_list("org1", None).await.unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].status, "open");
        ops.pdp_obligation_set_status("org1", id, "in_progress")
            .await
            .unwrap();
        assert!(ops
            .pdp_obligation_set_status("org2", id, "done")
            .await
            .is_err());
        ops.pdp_obligation_set_status("org1", id, "done")
            .await
            .unwrap();
        assert!(ops
            .pdp_obligation_set_status("org1", id, "open")
            .await
            .is_err());
        assert!(ops
            .pdp_obligation_list("org2", None)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn consent_grant_withdraw_regrant() {
        let ops = MemoryOps::default();
        let id = ops.consent_grant("org1", "nik-1", "kyc").await.unwrap();
        assert!(ops.consent_grant("org1", "nik-1", "kyc").await.is_err());
        ops.consent_withdraw("org1", id).await.unwrap();
        assert!(ops.consent_withdraw("org1", id).await.is_err());
        let id2 = ops.consent_grant("org1", "nik-1", "kyc").await.unwrap();
        assert_ne!(id, id2);
        let v = ops.consent_list("org1", Some("nik-1")).await.unwrap();
        assert_eq!(v.len(), 2);
    }
}

#[cfg(test)]
mod webhook_tests {
    use super::*;
    use crate::ops::OpsStore;

    #[tokio::test]
    async fn log_and_list_is_tenant_scoped() {
        let ops = MemoryOps::default();
        ops.webhook_log(Some("org1"), "tripay", "PAID", "accepted", "T-1")
            .await
            .unwrap();
        ops.webhook_log(None, "psre", "signed", "accepted", "")
            .await
            .unwrap();
        let v = ops.webhook_list("org1", None).await.unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].provider, "tripay");
        let v = ops.webhook_list("org1", Some("psre")).await.unwrap();
        assert!(v.is_empty(), "unattributed rows stay invisible");
        assert!(ops.webhook_list("org2", None).await.unwrap().is_empty());
    }
}
