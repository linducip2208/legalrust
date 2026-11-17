#!/usr/bin/env python3
"""Final push part 2: OpsStore memory + mysql impls."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust")
def w(rel, text):
    p = R/rel; p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text.strip()+"\n", encoding="utf-8", newline="\n")

w("crates/database/src/ops_memory.rs", """use super::ops::*;
use super::store::StoreError;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
#[derive(Debug, Default)]
struct Inner {
    jobs: HashMap<i64, OcrJobRow>, fields: HashMap<i64, Vec<OcrFieldRow>>,
    tpls: HashMap<i64, TemplateRow>, deeds: HashMap<i64, DeedRow>,
    convs: HashMap<i64, String>, msgs: HashMap<i64, Vec<(String, String)>>,
    usage: HashMap<String, (i64, i64)>, rag: HashMap<i64, Vec<String>>,
    invs: HashMap<i64, InvoiceRow>, pays: HashMap<i64, Vec<PaymentRow>>,
    seq: i64,
}
#[derive(Debug, Clone, Default)]
pub struct MemoryOps { inner: Arc<Mutex<Inner>> }
impl MemoryOps { fn next(&self, m: &mut Inner) -> i64 { m.seq += 1; m.seq } }
#[async_trait::async_trait]
impl OpsStore for MemoryOps {
    async fn ocr_create(&self, org: &str, doc: i64, doc_type: &str) -> Result<i64, StoreError> {
        let mut m = self.inner.lock().unwrap(); let id = self.next(&mut m);
        m.jobs.insert(id, OcrJobRow { id, org: org.into(), doc, doc_type: doc_type.into(), status: "queued".into() }); Ok(id)
    }
    async fn ocr_list(&self, org: &str) -> Result<Vec<OcrJobRow>, StoreError> {
        Ok(self.inner.lock().unwrap().jobs.values().filter(|j| j.org == org).cloned().collect())
    }
    async fn ocr_get(&self, org: &str, id: i64) -> Result<(OcrJobRow, Vec<OcrFieldRow>), StoreError> {
        let m = self.inner.lock().unwrap();
        let j = m.jobs.get(&id).filter(|j| j.org == org).cloned().ok_or(StoreError::NotFound)?;
        Ok((j, m.fields.get(&id).cloned().unwrap_or_default()))
    }
    async fn ocr_set_status(&self, org: &str, id: i64, status: &str) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap();
        m.jobs.get_mut(&id).filter(|j| j.org == org).map(|j| j.status = status.into()).ok_or(StoreError::NotFound)
    }
    async fn ocr_review(&self, org: &str, id: i64, _reviewer: &str, decision: &str, _note: Option<&str>) -> Result<(), StoreError> {
        self.ocr_set_status(org, id, if decision == "approve" { "reviewed_approved" } else { "reviewed_rejected" }).await
    }
    async fn tpl_create(&self, org: &str, name: &str, body: &str) -> Result<TemplateRow, StoreError> {
        let mut m = self.inner.lock().unwrap(); let id = self.next(&mut m);
        let t = TemplateRow { id, org: org.into(), name: name.into(), body: body.into(), version: 1 };
        m.tpls.insert(id, t.clone()); Ok(t)
    }
    async fn tpl_list(&self, org: &str) -> Result<Vec<TemplateRow>, StoreError> {
        Ok(self.inner.lock().unwrap().tpls.values().filter(|t| t.org == org).cloned().collect())
    }
    async fn tpl_get(&self, org: &str, id: i64) -> Result<TemplateRow, StoreError> {
        self.inner.lock().unwrap().tpls.get(&id).filter(|t| t.org == org).cloned().ok_or(StoreError::NotFound)
    }
    async fn tpl_update(&self, org: &str, id: i64, body: &str) -> Result<TemplateRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let t = m.tpls.get_mut(&id).filter(|t| t.org == org).ok_or(StoreError::NotFound)?;
        t.body = body.into(); t.version += 1; Ok(t.clone())
    }
    async fn deed_create(&self, org: &str, matter: i64, number: &str, deed_type: &str, _rendered: &str) -> Result<DeedRow, StoreError> {
        let mut m = self.inner.lock().unwrap(); let id = self.next(&mut m);
        if m.deeds.values().any(|d| d.org == org && d.number == number) { return Err(StoreError::Conflict("deed number taken".into())); }
        let d = DeedRow { id, org: org.into(), matter_id: matter, number: number.into(), deed_type: deed_type.into(), state: "draft".into() };
        m.deeds.insert(id, d.clone()); Ok(d)
    }
    async fn deed_list(&self, org: &str) -> Result<Vec<DeedRow>, StoreError> {
        Ok(self.inner.lock().unwrap().deeds.values().filter(|d| d.org == org).cloned().collect())
    }
    async fn deed_set_state(&self, org: &str, id: i64, state: &str) -> Result<DeedRow, StoreError> {
        let mut m = self.inner.lock().unwrap();
        let d = m.deeds.get_mut(&id).filter(|d| d.org == org).ok_or(StoreError::NotFound)?;
        d.state = state.into(); Ok(d.clone())
    }
    async fn conv_create(&self, org: &str, title: &str) -> Result<i64, StoreError> {
        let mut m = self.inner.lock().unwrap(); let id = self.next(&mut m);
        m.convs.insert(id, org.into()); m.msgs.insert(id, Vec::new()); let _ = title; Ok(id)
    }
    async fn msg_add(&self, conv: i64, role: &str, content: &str) -> Result<(), StoreError> {
        self.inner.lock().unwrap().msgs.get_mut(&conv).ok_or(StoreError::NotFound)?.push((role.into(), content.into())); Ok(())
    }
    async fn usage_add(&self, org: &str, input: i64, output: i64) -> Result<(), StoreError> {
        let mut m = self.inner.lock().unwrap(); let e = m.usage.entry(org.into()).or_insert((0, 0)); e.0 += input; e.1 += output; Ok(())
    }
    async fn usage_sum(&self, org: &str) -> Result<(i64, i64), StoreError> {
        Ok(self.inner.lock().unwrap().usage.get(org).cloned().unwrap_or((0, 0)))
    }
    async fn rag_put(&self, doc: i64, chunks: &[String]) -> Result<(), StoreError> {
        self.inner.lock().unwrap().rag.insert(doc, chunks.to_vec()); Ok(())
    }
    async fn rag_search(&self, _org: &str, q: &str, limit: i64) -> Result<Vec<RagHit>, StoreError> {
        let ql = q.to_lowercase();
        let mut hits: Vec<RagHit> = self.inner.lock().unwrap().rag.iter().flat_map(|(d, cs)| cs.iter().enumerate().filter(|(_, c)| c.to_lowercase().contains(&ql)).map(|(i, c)| RagHit { doc_id: *d, idx: i as i64, text: c.clone() }).collect::<Vec<_>>()).collect();
        hits.truncate(limit.max(0) as usize); Ok(hits)
    }
    async fn inv_create(&self, org: &str, matter: i64, number: &str, total: i64, currency: &str) -> Result<InvoiceRow, StoreError> {
        let mut m = self.inner.lock().unwrap(); let id = self.next(&mut m);
        if m.invs.values().any(|v| v.org == org && v.number == number) { return Err(StoreError::Conflict("invoice number taken".into())); }
        let r = InvoiceRow { id, org: org.into(), matter_id: matter, number: number.into(), total, currency: currency.into(), status: "issued".into() };
        m.invs.insert(id, r.clone()); Ok(r)
    }
    async fn inv_list(&self, org: &str, matter: i64) -> Result<Vec<InvoiceRow>, StoreError> {
        Ok(self.inner.lock().unwrap().invs.values().filter(|v| v.org == org && v.matter_id == matter).cloned().collect())
    }
    async fn inv_get(&self, org: &str, id: i64) -> Result<(InvoiceRow, Vec<PaymentRow>), StoreError> {
        let m = self.inner.lock().unwrap();
        let v = m.invs.get(&id).filter(|v| v.org == org).cloned().ok_or(StoreError::NotFound)?;
        Ok((v, m.pays.get(&id).cloned().unwrap_or_default()))
    }
    async fn pay_add(&self, org: &str, inv: i64, amount: i64, method: &str) -> Result<PaymentRow, StoreError> {
        if amount <= 0 { return Err(StoreError::Conflict("amount must be positive".into())); }
        let mut m = self.inner.lock().unwrap();
        let v = m.invs.get(&inv).filter(|v| v.org == org).cloned().ok_or(StoreError::NotFound)?;
        let paid: i64 = m.pays.get(&inv).map(|ps| ps.iter().map(|p| p.amount).sum()).unwrap_or(0);
        if paid + amount > v.total { return Err(StoreError::Conflict("overpayment".into())); }
        let id = m.seq + 1; m.seq = id;
        let p = PaymentRow { id, invoice_id: inv, amount, method: method.into() };
        m.pays.entry(inv).or_default().push(p.clone()); Ok(p)
    }
}
""")
print("P2 OK")
