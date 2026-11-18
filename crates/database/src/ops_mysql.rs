use super::ops::*;
use super::store::StoreError;
use sqlx::{MySqlPool, Row};
#[derive(Debug, Clone)]
pub struct MysqlOps {
    pub pool: MySqlPool,
}
impl MysqlOps {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}
fn db_err(e: sqlx::Error) -> StoreError {
    if let Some(dbe) = e.as_database_error() {
        if dbe.code().as_deref() == Some("23000") {
            return StoreError::Conflict(dbe.message().into());
        }
    }
    StoreError::Db(e.to_string())
}
fn appr_req_row(x: sqlx::mysql::MySqlRow) -> ApprovalReqRow {
    ApprovalReqRow {
        id: x.get::<u64, _>("id") as i64,
        org: x.get::<String, _>("org_id"),
        entity: x.get::<String, _>("entity"),
        entity_id: x.get::<u64, _>("entity_id") as i64,
        title: x.get::<String, _>("title"),
        status: x.get::<String, _>("status"),
        created_by: x.get::<String, _>("created_by"),
    }
}
fn appr_step_row(x: sqlx::mysql::MySqlRow) -> ApprovalStepRow {
    ApprovalStepRow {
        id: x.get::<u64, _>("id") as i64,
        req: x.get::<u64, _>("request_id") as i64,
        step_no: x.get::<i32, _>("step_no") as i64,
        role: x.get::<String, _>("approver_role"),
        user: x
            .try_get::<Option<String>, _>("approver_user")
            .ok()
            .flatten(),
        status: x.get::<String, _>("status"),
        comment: x.try_get::<Option<String>, _>("comment").ok().flatten(),
    }
}
/// 9-arg helper mirrors the `OpsStore::trust_post`/`trust_transfer` signatures so both
/// legs of a transfer share one guarded insert path; grouping would churn the trait.
#[allow(clippy::too_many_arguments)]
async fn post_entry(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
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
    if !["debit", "credit"].contains(&kind) {
        return Err(StoreError::Conflict("bad kind".into()));
    }
    let cur: i64 =
        sqlx::query("SELECT balance FROM trust_accounts WHERE id=? AND org_id=? FOR UPDATE")
            .bind(account as u64)
            .bind(org)
            .fetch_optional(&mut **tx)
            .await
            .map_err(db_err)?
            .ok_or(StoreError::NotFound)?
            .try_get("balance")
            .map_err(|e| StoreError::Db(e.to_string()))?;
    let next = if kind == "debit" {
        cur - amount
    } else {
        cur + amount
    };
    if next < 0 {
        return Err(StoreError::Conflict("insufficient trust balance".into()));
    }
    let r = sqlx::query("INSERT INTO trust_entries (org_id, account_id, matter_id, kind, amount_minor, ref_type, ref_id, created_by) VALUES (?,?,?,?,?,?,?,?)").bind(org).bind(account as u64).bind(matter.map(|v| v as u64)).bind(kind).bind(amount).bind(ref_type).bind(ref_id as u64).bind(created_by).execute(&mut **tx).await.map_err(db_err)?;
    sqlx::query("UPDATE trust_accounts SET balance=? WHERE id=?")
        .bind(next)
        .bind(account as u64)
        .execute(&mut **tx)
        .await
        .map_err(db_err)?;
    Ok(TrustEntryRow {
        id: r.last_insert_id() as i64,
        org: org.into(),
        account_id: account,
        matter_id: matter,
        kind: kind.into(),
        amount,
        ref_type: ref_type.into(),
        ref_id,
        created_by: created_by.into(),
    })
}

#[async_trait::async_trait]
impl OpsStore for MysqlOps {
    async fn ocr_create(&self, org: &str, doc: i64, doc_type: &str) -> Result<i64, StoreError> {
        let r = sqlx::query("INSERT INTO ocr_jobs (org_id, document_id, doc_type) VALUES (?,?,?)")
            .bind(org)
            .bind(doc as u64)
            .bind(doc_type)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn ocr_list(&self, org: &str) -> Result<Vec<OcrJobRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, document_id, doc_type, status FROM ocr_jobs WHERE org_id=? ORDER BY id DESC LIMIT 200").bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| OcrJobRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                doc: x.get::<u64, _>("document_id") as i64,
                doc_type: x.get::<String, _>("doc_type"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn ocr_get(
        &self,
        org: &str,
        id: i64,
    ) -> Result<(OcrJobRow, Vec<OcrFieldRow>), StoreError> {
        let x = sqlx::query("SELECT id, org_id, document_id, doc_type, status FROM ocr_jobs WHERE id=? AND org_id=?").bind(id as u64).bind(org).fetch_optional(&self.pool).await.map_err(db_err)?.ok_or(StoreError::NotFound)?;
        let j = OcrJobRow {
            id: x.get::<u64, _>("id") as i64,
            org: x.get::<String, _>("org_id"),
            doc: x.get::<u64, _>("document_id") as i64,
            doc_type: x.get::<String, _>("doc_type"),
            status: x.get::<String, _>("status"),
        };
        let fs = sqlx::query("SELECT name, value, confidence, page FROM ocr_fields WHERE job_id=?")
            .bind(id as u64)
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;
        Ok((
            j,
            fs.into_iter()
                .map(|f| OcrFieldRow {
                    name: f.get::<String, _>("name"),
                    value: f.get::<String, _>("value"),
                    confidence: f.get::<f32, _>("confidence"),
                    page: f.get::<i32, _>("page") as i64,
                })
                .collect(),
        ))
    }
    async fn ocr_set_status(&self, org: &str, id: i64, status: &str) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE ocr_jobs SET status=? WHERE id=? AND org_id=?")
            .bind(status)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }
    async fn ocr_store_result(
        &self,
        org: &str,
        id: i64,
        status: &str,
        fields: &[OcrFieldRow],
    ) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let n = sqlx::query("UPDATE ocr_jobs SET status=? WHERE id=? AND org_id=?")
            .bind(status)
            .bind(id as u64)
            .bind(org)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        sqlx::query("DELETE FROM ocr_fields WHERE job_id=?")
            .bind(id as u64)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        for f in fields {
            sqlx::query(
                "INSERT INTO ocr_fields (job_id, name, value, confidence, page) VALUES (?,?,?,?,?)",
            )
            .bind(id as u64)
            .bind(&f.name)
            .bind(&f.value)
            .bind(f.confidence)
            .bind(f.page as i32)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }
    async fn ocr_review(
        &self,
        org: &str,
        id: i64,
        reviewer: &str,
        decision: &str,
        note: Option<&str>,
    ) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let n = sqlx::query("UPDATE ocr_jobs SET status=? WHERE id=? AND org_id=?")
            .bind(if decision == "approve" {
                "reviewed_approved"
            } else {
                "reviewed_rejected"
            })
            .bind(id as u64)
            .bind(org)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        sqlx::query(
            "INSERT INTO ocr_reviews (job_id, reviewer_id, decision, note) VALUES (?,?,?,?)",
        )
        .bind(id as u64)
        .bind(reviewer)
        .bind(decision)
        .bind(note)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }
    async fn tpl_create(
        &self,
        org: &str,
        name: &str,
        body: &str,
    ) -> Result<TemplateRow, StoreError> {
        let r = sqlx::query("INSERT INTO deed_templates (org_id, name, body) VALUES (?,?,?)")
            .bind(org)
            .bind(name)
            .bind(body)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(TemplateRow {
            id: r.last_insert_id() as i64,
            org: org.into(),
            name: name.into(),
            body: body.into(),
            version: 1,
        })
    }
    async fn tpl_list(&self, org: &str) -> Result<Vec<TemplateRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, name, body, version FROM deed_templates WHERE org_id=? ORDER BY id DESC LIMIT 200").bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| TemplateRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                name: x.get::<String, _>("name"),
                body: x.get::<String, _>("body"),
                version: x.get::<i32, _>("version") as i64,
            })
            .collect())
    }
    async fn tpl_get(&self, org: &str, id: i64) -> Result<TemplateRow, StoreError> {
        let x = sqlx::query(
            "SELECT id, org_id, name, body, version FROM deed_templates WHERE id=? AND org_id=?",
        )
        .bind(id as u64)
        .bind(org)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .ok_or(StoreError::NotFound)?;
        Ok(TemplateRow {
            id: x.get::<u64, _>("id") as i64,
            org: x.get::<String, _>("org_id"),
            name: x.get::<String, _>("name"),
            body: x.get::<String, _>("body"),
            version: x.get::<i32, _>("version") as i64,
        })
    }
    async fn tpl_update(&self, org: &str, id: i64, body: &str) -> Result<TemplateRow, StoreError> {
        let n = sqlx::query(
            "UPDATE deed_templates SET body=?, version=version+1 WHERE id=? AND org_id=?",
        )
        .bind(body)
        .bind(id as u64)
        .bind(org)
        .execute(&self.pool)
        .await
        .map_err(db_err)?
        .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        self.tpl_get(org, id).await
    }
    async fn deed_create(
        &self,
        org: &str,
        matter: i64,
        number: &str,
        deed_type: &str,
        rendered: &str,
    ) -> Result<DeedRow, StoreError> {
        let r = sqlx::query(
            "INSERT INTO deeds (org_id, matter_id, number, deed_type, rendered) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(matter as u64)
        .bind(number)
        .bind(deed_type)
        .bind(rendered)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if let Some(dbe) = e.as_database_error() {
                if dbe.code().as_deref() == Some("23000") {
                    return StoreError::Conflict("deed number taken".into());
                }
            }
            StoreError::Db(e.to_string())
        })?;
        Ok(DeedRow {
            id: r.last_insert_id() as i64,
            org: org.into(),
            matter_id: matter,
            number: number.into(),
            deed_type: deed_type.into(),
            state: "draft".into(),
            created: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        })
    }
    async fn deed_list(&self, org: &str) -> Result<Vec<DeedRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, matter_id, number, deed_type, state, DATE_FORMAT(created_at,'%Y-%m-%d %H:%i:%s') AS created_at FROM deeds WHERE org_id=? ORDER BY id DESC LIMIT 200").bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| DeedRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                matter_id: x.get::<u64, _>("matter_id") as i64,
                number: x.get::<String, _>("number"),
                deed_type: x.get::<String, _>("deed_type"),
                state: x.get::<String, _>("state"),
                created: x.get::<String, _>("created_at"),
            })
            .collect())
    }
    async fn deed_set_state(&self, org: &str, id: i64, state: &str) -> Result<DeedRow, StoreError> {
        let n = sqlx::query("UPDATE deeds SET state=? WHERE id=? AND org_id=?")
            .bind(state)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        let x = sqlx::query(
            "SELECT id, org_id, matter_id, number, deed_type, state, DATE_FORMAT(created_at,'%Y-%m-%d %H:%i:%s') AS created_at FROM deeds WHERE id=?",
        )
        .bind(id as u64)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(DeedRow {
            id: x.get::<u64, _>("id") as i64,
            org: x.get::<String, _>("org_id"),
            matter_id: x.get::<u64, _>("matter_id") as i64,
            number: x.get::<String, _>("number"),
            deed_type: x.get::<String, _>("deed_type"),
            state: x.get::<String, _>("state"),
            created: x.get::<String, _>("created_at"),
        })
    }
    async fn conv_create(&self, org: &str, title: &str) -> Result<i64, StoreError> {
        let r = sqlx::query("INSERT INTO ai_conversations (org_id, title) VALUES (?,?)")
            .bind(org)
            .bind(title)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn msg_add(&self, conv: i64, role: &str, content: &str) -> Result<(), StoreError> {
        sqlx::query("INSERT INTO ai_messages (conv_id, role, content) VALUES (?,?,?)")
            .bind(conv as u64)
            .bind(role)
            .bind(content)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn usage_add(&self, org: &str, input: i64, output: i64) -> Result<(), StoreError> {
        sqlx::query("INSERT INTO ai_usage (org_id, input_tokens, output_tokens) VALUES (?,?,?)")
            .bind(org)
            .bind(input)
            .bind(output)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn usage_sum(&self, org: &str) -> Result<(i64, i64), StoreError> {
        let x = sqlx::query("SELECT COALESCE(SUM(input_tokens),0) AS i, COALESCE(SUM(output_tokens),0) AS o FROM ai_usage WHERE org_id=?").bind(org).fetch_one(&self.pool).await.map_err(db_err)?;
        Ok((
            x.try_get::<i64, _>("i").unwrap_or(0),
            x.try_get::<i64, _>("o").unwrap_or(0),
        ))
    }
    async fn rag_put(&self, doc: i64, chunks: &[String]) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        sqlx::query("DELETE FROM rag_chunks WHERE doc_id=?")
            .bind(doc as u64)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        for (i, c) in chunks.iter().enumerate() {
            sqlx::query("INSERT INTO rag_chunks (doc_id, idx, text) VALUES (?,?,?)")
                .bind(doc as u64)
                .bind(i as i32)
                .bind(c)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
        }
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }
    async fn rag_search(&self, org: &str, q: &str, limit: i64) -> Result<Vec<RagHit>, StoreError> {
        let like = format!("%{q}%");
        let rows = sqlx::query("SELECT r.doc_id, r.idx, r.text FROM rag_chunks r JOIN documents d ON d.id=r.doc_id WHERE d.org_id=? AND r.text LIKE ? ORDER BY r.doc_id, r.idx LIMIT ?")
            .bind(org).bind(like).bind(limit).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| RagHit {
                doc_id: x.get::<u64, _>("doc_id") as i64,
                idx: x.get::<i32, _>("idx") as i64,
                text: x.get::<String, _>("text"),
            })
            .collect())
    }
    async fn inv_create(
        &self,
        org: &str,
        matter: i64,
        number: &str,
        total: i64,
        currency: &str,
    ) -> Result<InvoiceRow, StoreError> {
        let r = sqlx::query("INSERT INTO invoices (org_id, matter_id, number, total_minor, currency) VALUES (?,?,?,?,?)").bind(org).bind(matter as u64).bind(number).bind(total).bind(currency).execute(&self.pool).await.map_err(|e| {
            if let Some(dbe) = e.as_database_error() { if dbe.code().as_deref() == Some("23000") { return StoreError::Conflict("invoice number taken".into()); } }
            StoreError::Db(e.to_string())
        })?;
        Ok(InvoiceRow {
            id: r.last_insert_id() as i64,
            org: org.into(),
            matter_id: matter,
            number: number.into(),
            total,
            currency: currency.into(),
            status: "issued".into(),
        })
    }
    async fn inv_list(&self, org: &str, matter: i64) -> Result<Vec<InvoiceRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, matter_id, number, total_minor, currency, status FROM invoices WHERE org_id=? AND matter_id=? ORDER BY id DESC").bind(org).bind(matter as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| InvoiceRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                matter_id: x.get::<u64, _>("matter_id") as i64,
                number: x.get::<String, _>("number"),
                total: x.get::<i64, _>("total_minor"),
                currency: x.get::<String, _>("currency"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn inv_list_open(&self, org: &str) -> Result<Vec<ArOpenRow>, StoreError> {
        let rows = sqlx::query(
            "SELECT i.id, i.number, i.matter_id, i.total_minor, COALESCE(SUM(p.amount_minor),0) AS paid \
             FROM invoices i LEFT JOIN payments p ON p.invoice_id=i.id \
             WHERE i.org_id=? GROUP BY i.id HAVING paid < i.total_minor ORDER BY i.id DESC LIMIT 200",
        )
        .bind(org)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| {
                let total = x.get::<i64, _>("total_minor");
                // SUM over BIGINT UNSIGNED may decode as u64.
                let paid: i64 = x
                    .try_get::<i64, _>("paid")
                    .unwrap_or_else(|_| x.get::<u64, _>("paid") as i64);
                ArOpenRow {
                    id: x.get::<u64, _>("id") as i64,
                    number: x.get::<String, _>("number"),
                    matter_id: x.get::<u64, _>("matter_id") as i64,
                    total,
                    paid,
                    balance: total - paid,
                }
            })
            .collect())
    }
    async fn inv_get(
        &self,
        org: &str,
        id: i64,
    ) -> Result<(InvoiceRow, Vec<PaymentRow>), StoreError> {
        let x = sqlx::query("SELECT id, org_id, matter_id, number, total_minor, currency, status FROM invoices WHERE id=? AND org_id=?").bind(id as u64).bind(org).fetch_optional(&self.pool).await.map_err(db_err)?.ok_or(StoreError::NotFound)?;
        let v = InvoiceRow {
            id: x.get::<u64, _>("id") as i64,
            org: x.get::<String, _>("org_id"),
            matter_id: x.get::<u64, _>("matter_id") as i64,
            number: x.get::<String, _>("number"),
            total: x.get::<i64, _>("total_minor"),
            currency: x.get::<String, _>("currency"),
            status: x.get::<String, _>("status"),
        };
        let ps = sqlx::query(
            "SELECT id, invoice_id, amount_minor, method FROM payments WHERE invoice_id=?",
        )
        .bind(id as u64)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        Ok((
            v,
            ps.into_iter()
                .map(|p| PaymentRow {
                    id: p.get::<u64, _>("id") as i64,
                    invoice_id: p.get::<u64, _>("invoice_id") as i64,
                    amount: p.get::<i64, _>("amount_minor"),
                    method: p.get::<String, _>("method"),
                })
                .collect(),
        ))
    }
    async fn inv_owner_org(&self, inv: i64) -> Result<Option<String>, StoreError> {
        let x = sqlx::query("SELECT org_id FROM invoices WHERE id=?")
            .bind(inv as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(x.map(|r| r.get::<String, _>("org_id")))
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
        super::ops::appr_check(entity, steps)?;
        if title.trim().is_empty() {
            return Err(StoreError::Conflict("title".into()));
        }
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let r = sqlx::query(
            "INSERT INTO approval_requests (org_id, entity, entity_id, title, created_by) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(entity)
        .bind(entity_id as u64)
        .bind(title)
        .bind(created_by)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        let id = r.last_insert_id() as i64;
        for (no, role) in steps {
            sqlx::query(
                "INSERT INTO approval_steps (request_id, step_no, approver_role) VALUES (?,?,?)",
            )
            .bind(id as u64)
            .bind(*no)
            .bind(role.trim())
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }
        tx.commit().await.map_err(db_err)?;
        Ok(id)
    }
    async fn appr_list(
        &self,
        org: &str,
        status: Option<&str>,
    ) -> Result<Vec<ApprovalReqRow>, StoreError> {
        let rows = if let Some(s) = status {
            sqlx::query("SELECT id, org_id, entity, entity_id, title, status, created_by FROM approval_requests WHERE org_id=? AND status=? ORDER BY id DESC LIMIT 200")
                .bind(org).bind(s).fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query("SELECT id, org_id, entity, entity_id, title, status, created_by FROM approval_requests WHERE org_id=? ORDER BY id DESC LIMIT 200")
                .bind(org).fetch_all(&self.pool).await.map_err(db_err)?
        };
        Ok(rows.into_iter().map(appr_req_row).collect())
    }
    async fn appr_get(
        &self,
        org: &str,
        id: i64,
    ) -> Result<(ApprovalReqRow, Vec<ApprovalStepRow>), StoreError> {
        let x = sqlx::query("SELECT id, org_id, entity, entity_id, title, status, created_by FROM approval_requests WHERE id=? AND org_id=?")
            .bind(id as u64).bind(org).fetch_optional(&self.pool).await.map_err(db_err)?.ok_or(StoreError::NotFound)?;
        let fs = sqlx::query("SELECT id, request_id, step_no, approver_role, approver_user, status, comment FROM approval_steps WHERE request_id=? ORDER BY step_no")
            .bind(id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok((appr_req_row(x), fs.into_iter().map(appr_step_row).collect()))
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
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let x = sqlx::query("SELECT status FROM approval_requests WHERE id=? AND org_id=?")
            .bind(id as u64)
            .bind(org)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?
            .ok_or(StoreError::NotFound)?;
        let cur: String = x.get::<String, _>("status");
        if !["pending", "needs_rework"].contains(&cur.as_str()) {
            return Err(StoreError::Conflict("request closed".into()));
        }
        let steps = sqlx::query(
            "SELECT id, request_id, step_no, approver_role, approver_user, status, comment FROM approval_steps WHERE request_id=? ORDER BY step_no",
        )
        .bind(id as u64)
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
        let target = steps
            .iter()
            .find(|r| r.get::<i32, _>("step_no") as i64 == step_no)
            .ok_or(StoreError::NotFound)?;
        if target.get::<String, _>("status") != "pending" {
            return Err(StoreError::Conflict("step already decided".into()));
        }
        for r in &steps {
            if (r.get::<i32, _>("step_no") as i64) < step_no
                && r.get::<String, _>("status") != "approved"
            {
                return Err(StoreError::Conflict("earlier step pending".into()));
            }
        }
        let max_no = steps
            .iter()
            .map(|r| r.get::<i32, _>("step_no") as i64)
            .max()
            .unwrap_or(0);
        let step_status = match decision {
            "approve" => "approved",
            "reject" => "rejected",
            _ => "pending",
        };
        sqlx::query("UPDATE approval_steps SET status=?, approver_user=?, comment=? WHERE request_id=? AND step_no=?")
            .bind(step_status)
            .bind(user)
            .bind(comment)
            .bind(id as u64)
            .bind(step_no)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        let new_status = match decision {
            "approve" if step_no == max_no => "approved",
            "approve" => cur.as_str(),
            "reject" => "rejected",
            _ => "needs_rework",
        };
        if new_status == "needs_rework" {
            sqlx::query(
                "UPDATE approval_steps SET status='pending', comment=NULL WHERE request_id=?",
            )
            .bind(id as u64)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }
        sqlx::query("UPDATE approval_requests SET status=? WHERE id=?")
            .bind(new_status)
            .bind(id as u64)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        let fx = sqlx::query("SELECT id, request_id, step_no, approver_role, approver_user, status, comment FROM approval_steps WHERE request_id=? AND step_no=?")
            .bind(id as u64).bind(step_no).fetch_one(&self.pool).await.map_err(db_err)?;
        Ok(appr_step_row(fx))
    }
    async fn appr_revoke(&self, org: &str, id: i64, _user: &str) -> Result<(), StoreError> {
        let n = sqlx::query(
            "UPDATE approval_requests SET status='revoked' WHERE id=? AND org_id=? AND status IN ('pending','needs_rework')",
        )
        .bind(id as u64)
        .bind(org)
        .execute(&self.pool)
        .await
        .map_err(db_err)?
        .rows_affected();
        if n == 0 {
            let exists = sqlx::query("SELECT id FROM approval_requests WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .is_some();
            if exists {
                return Err(StoreError::Conflict("request closed".into()));
            }
            return Err(StoreError::NotFound);
        }
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
        let detail: String = detail.chars().take(255).collect();
        let r = sqlx::query(
            "INSERT INTO webhook_deliveries (org_id, provider, event, status, detail) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(provider)
        .bind(event)
        .bind(status)
        .bind(detail)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn webhook_list(
        &self,
        org: &str,
        provider: Option<&str>,
    ) -> Result<Vec<WebhookDeliveryRow>, StoreError> {
        let rows = if let Some(p) = provider {
            sqlx::query("SELECT id, org_id, provider, event, status, detail, direction FROM webhook_deliveries WHERE org_id=? AND provider=? ORDER BY id DESC LIMIT 200")
                .bind(org).bind(p).fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query("SELECT id, org_id, provider, event, status, detail, direction FROM webhook_deliveries WHERE org_id=? ORDER BY id DESC LIMIT 200")
                .bind(org).fetch_all(&self.pool).await.map_err(db_err)?
        };
        Ok(rows
            .into_iter()
            .map(|x| WebhookDeliveryRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.try_get::<Option<String>, _>("org_id").ok().flatten(),
                provider: x.get::<String, _>("provider"),
                event: x.get::<String, _>("event"),
                status: x.get::<String, _>("status"),
                detail: x.get::<String, _>("detail"),
                direction: x
                    .try_get::<Option<String>, _>("direction")
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "in".into()),
            })
            .collect())
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
        let ev = if events.trim().is_empty() {
            "*"
        } else {
            events.trim()
        };
        let r = sqlx::query(
            "INSERT INTO webhook_endpoints (org_id, url, secret, events) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(url.trim())
        .bind(secret)
        .bind(ev)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn webhook_ep_list(&self, org: &str) -> Result<Vec<WebhookEndpointRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, url, events, active FROM webhook_endpoints WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| WebhookEndpointRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                url: x.get::<String, _>("url"),
                events: x.get::<String, _>("events"),
                active: x.get::<i8, _>("active") != 0,
            })
            .collect())
    }
    async fn webhook_ep_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM webhook_endpoints WHERE id=? AND org_id=?")
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }
    async fn webhook_ep_secret(&self, org: &str, id: i64) -> Result<Option<String>, StoreError> {
        let x = sqlx::query("SELECT secret FROM webhook_endpoints WHERE id=? AND org_id=?")
            .bind(id as u64)
            .bind(org)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .ok_or(StoreError::NotFound)?;
        Ok(Some(x.get::<String, _>("secret")))
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
        let cat = if category.trim().is_empty() {
            "general"
        } else {
            category.trim()
        };
        let r = sqlx::query(
            "INSERT INTO pdp_obligations (org_id, title, category, owner) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(title.trim())
        .bind(cat)
        .bind(owner.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn pdp_obligation_list(
        &self,
        org: &str,
        status: Option<&str>,
    ) -> Result<Vec<PdpObligationRow>, StoreError> {
        let rows = if let Some(s) = status {
            sqlx::query("SELECT id, org_id, title, category, status, owner FROM pdp_obligations WHERE org_id=? AND status=? ORDER BY id DESC LIMIT 200")
                .bind(org).bind(s).fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query("SELECT id, org_id, title, category, status, owner FROM pdp_obligations WHERE org_id=? ORDER BY id DESC LIMIT 200")
                .bind(org).fetch_all(&self.pool).await.map_err(db_err)?
        };
        Ok(rows
            .into_iter()
            .map(|x| PdpObligationRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                title: x.get::<String, _>("title"),
                category: x.get::<String, _>("category"),
                status: x.get::<String, _>("status"),
                owner: x.get::<String, _>("owner"),
            })
            .collect())
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
        let x = sqlx::query("SELECT status FROM pdp_obligations WHERE id=? AND org_id=?")
            .bind(id as u64)
            .bind(org)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .ok_or(StoreError::NotFound)?;
        let cur: String = x.get::<String, _>("status");
        if cur == "done" {
            return Err(StoreError::Conflict("obligation closed".into()));
        }
        if cur == "open" && status == "open" {
            return Err(StoreError::Conflict("no transition".into()));
        }
        sqlx::query("UPDATE pdp_obligations SET status=? WHERE id=?")
            .bind(status)
            .bind(id as u64)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
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
        let dup = sqlx::query("SELECT id FROM consent_records WHERE org_id=? AND subject=? AND purpose=? AND status='granted' LIMIT 1")
            .bind(org).bind(subject.trim()).bind(purpose.trim()).fetch_optional(&self.pool).await.map_err(db_err)?;
        if dup.is_some() {
            return Err(StoreError::Conflict("consent already granted".into()));
        }
        let r =
            sqlx::query("INSERT INTO consent_records (org_id, subject, purpose) VALUES (?,?,?)")
                .bind(org)
                .bind(subject.trim())
                .bind(purpose.trim())
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn consent_withdraw(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query(
            "UPDATE consent_records SET status='withdrawn', withdrawn_at=CURRENT_TIMESTAMP WHERE id=? AND org_id=? AND status='granted'",
        )
        .bind(id as u64)
        .bind(org)
        .execute(&self.pool)
        .await
        .map_err(db_err)?
        .rows_affected();
        if n == 0 {
            let exists = sqlx::query("SELECT id FROM consent_records WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .is_some();
            if exists {
                return Err(StoreError::Conflict("consent not active".into()));
            }
            return Err(StoreError::NotFound);
        }
        Ok(())
    }
    async fn consent_list(
        &self,
        org: &str,
        subject: Option<&str>,
    ) -> Result<Vec<ConsentRow>, StoreError> {
        let rows = if let Some(s) = subject {
            sqlx::query("SELECT id, org_id, subject, purpose, status FROM consent_records WHERE org_id=? AND subject=? ORDER BY id DESC LIMIT 200")
                .bind(org).bind(s).fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query("SELECT id, org_id, subject, purpose, status FROM consent_records WHERE org_id=? ORDER BY id DESC LIMIT 200")
                .bind(org).fetch_all(&self.pool).await.map_err(db_err)?
        };
        Ok(rows
            .into_iter()
            .map(|x| ConsentRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                subject: x.get::<String, _>("subject"),
                purpose: x.get::<String, _>("purpose"),
                status: x.get::<String, _>("status"),
            })
            .collect())
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
        let lb = if legal_basis.trim().is_empty() {
            "consent"
        } else {
            legal_basis.trim()
        };
        let r = sqlx::query(
            "INSERT INTO ropa_entries (org_id, activity, purpose, data_categories, legal_basis, retention) VALUES (?,?,?,?,?,?)",
        )
        .bind(org)
        .bind(activity.trim())
        .bind(purpose.trim())
        .bind(data_categories.trim())
        .bind(lb)
        .bind(retention.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn ropa_list(&self, org: &str) -> Result<Vec<RopaRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, activity, purpose, data_categories, legal_basis, retention FROM ropa_entries WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| RopaRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                activity: x.get::<String, _>("activity"),
                purpose: x.get::<String, _>("purpose"),
                data_categories: x.get::<String, _>("data_categories"),
                legal_basis: x.get::<String, _>("legal_basis"),
                retention: x.get::<String, _>("retention"),
            })
            .collect())
    }
    async fn ropa_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM ropa_entries WHERE id=? AND org_id=?")
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
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
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let x = sqlx::query("SELECT total_minor FROM invoices WHERE id=? AND org_id=?")
            .bind(inv as u64)
            .bind(org)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?
            .ok_or(StoreError::NotFound)?;
        let total: i64 = x
            .try_get("total_minor")
            .map_err(|e| StoreError::Db(e.to_string()))?;
        let paid: Option<i64> = sqlx::query(
            "SELECT COALESCE(SUM(amount_minor),0) AS s FROM payments WHERE invoice_id=?",
        )
        .bind(inv as u64)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_err)?
        .try_get("s")
        .ok();
        if paid.unwrap_or(0) + amount > total {
            return Err(StoreError::Conflict("overpayment".into()));
        }
        let r =
            sqlx::query("INSERT INTO payments (invoice_id, amount_minor, method) VALUES (?,?,?)")
                .bind(inv as u64)
                .bind(amount)
                .bind(method)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(PaymentRow {
            id: r.last_insert_id() as i64,
            invoice_id: inv,
            amount,
            method: method.into(),
        })
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
        let r = sqlx::query(
            "INSERT INTO notifications (org_id, user_id, kind, title, body) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(user)
        .bind(kind)
        .bind(title)
        .bind(body)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn notify_list(
        &self,
        user: &str,
        kind: Option<&str>,
    ) -> Result<Vec<NotifRow>, StoreError> {
        let rows = if let Some(k) = kind {
            sqlx::query("SELECT id, kind, title, UNIX_TIMESTAMP(read_at) AS read_unix FROM notifications WHERE user_id=? AND kind=? ORDER BY id DESC LIMIT 100").bind(user).bind(k).fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query("SELECT id, kind, title, UNIX_TIMESTAMP(read_at) AS read_unix FROM notifications WHERE user_id=? ORDER BY id DESC LIMIT 100").bind(user).fetch_all(&self.pool).await.map_err(db_err)?
        };
        Ok(rows
            .into_iter()
            .map(|x| NotifRow {
                id: x.get::<u64, _>("id") as i64,
                kind: x.get::<String, _>("kind"),
                title: x.get::<String, _>("title"),
                read: x.get::<Option<i64>, _>("read_unix").is_some(),
            })
            .collect())
    }
    async fn notify_read(&self, user: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE notifications SET read_at=NOW() WHERE id=? AND user_id=?")
            .bind(id as u64)
            .bind(user)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }
    async fn share_create(
        &self,
        org: &str,
        doc: i64,
        token: &str,
        hours: i64,
    ) -> Result<i64, StoreError> {
        let r = sqlx::query("INSERT INTO document_shares (org_id, document_id, token, expires_at) VALUES (?,?,?,DATE_ADD(NOW(), INTERVAL ? HOUR))").bind(org).bind(doc as u64).bind(token).bind(hours).execute(&self.pool).await.map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn share_list(&self, org: &str, doc: i64) -> Result<Vec<ShareRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, document_id, token, DATE_FORMAT(expires_at,'%Y-%m-%d %H:%i:%s') AS expires_at, revoked FROM document_shares WHERE org_id=? AND document_id=? ORDER BY id DESC").bind(org).bind(doc as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| ShareRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                doc: x.get::<u64, _>("document_id") as i64,
                token: x.get::<String, _>("token"),
                expires: x.get::<String, _>("expires_at"),
                revoked: x.get::<bool, _>("revoked"),
            })
            .collect())
    }
    async fn share_get(&self, token: &str) -> Result<Option<ShareRow>, StoreError> {
        let x = sqlx::query("SELECT id, org_id, document_id, token, DATE_FORMAT(expires_at,'%Y-%m-%d %H:%i:%s') AS expires_at, revoked FROM document_shares WHERE token=?").bind(token).fetch_optional(&self.pool).await.map_err(db_err)?;
        Ok(x.map(|x| ShareRow {
            id: x.get::<u64, _>("id") as i64,
            org: x.get::<String, _>("org_id"),
            doc: x.get::<u64, _>("document_id") as i64,
            token: x.get::<String, _>("token"),
            expires: x.get::<String, _>("expires_at"),
            revoked: x.get::<bool, _>("revoked"),
        }))
    }
    async fn share_revoke(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE document_shares SET revoked=1 WHERE id=? AND org_id=?")
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
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
        let run_at: Option<String> = if delay_secs == 0 {
            None
        } else {
            // Resolved in SQL so DB time (not app time) governs the lease.
            let row: (String,) = sqlx::query_as(
                "SELECT DATE_FORMAT(DATE_ADD(NOW(), INTERVAL ? SECOND),'%Y-%m-%d %H:%i:%s')",
            )
            .bind(delay_secs)
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?;
            Some(row.0)
        };
        let r = if let Some(ts) = run_at {
            sqlx::query("INSERT INTO jobs (org_id, kind, payload, run_at) VALUES (?,?,?,?)")
                .bind(org)
                .bind(kind)
                .bind(payload)
                .bind(ts)
                .execute(&self.pool)
                .await
                .map_err(db_err)?
        } else {
            sqlx::query("INSERT INTO jobs (org_id, kind, payload) VALUES (?,?,?)")
                .bind(org)
                .bind(kind)
                .bind(payload)
                .execute(&self.pool)
                .await
                .map_err(db_err)?
        };
        Ok(r.last_insert_id() as i64)
    }
    async fn job_list(&self, org: &str) -> Result<Vec<JobRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, kind, payload, status, attempts FROM jobs WHERE org_id=? ORDER BY id DESC LIMIT 200").bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| JobRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                kind: x.get::<String, _>("kind"),
                payload: x.get::<Option<String>, _>("payload").unwrap_or_default(),
                status: x.get::<String, _>("status"),
                attempts: x.get::<i32, _>("attempts") as i64,
            })
            .collect())
    }
    async fn job_claim(&self) -> Result<Option<JobRow>, StoreError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let x = sqlx::query("SELECT id, org_id, kind, payload, status, attempts FROM jobs WHERE status='queued' AND (run_at IS NULL OR run_at <= NOW()) ORDER BY id LIMIT 1 FOR UPDATE SKIP LOCKED").fetch_optional(&mut *tx).await.map_err(db_err)?;
        match x {
            None => Ok(None),
            Some(r) => {
                let id: u64 = r.get("id");
                sqlx::query("UPDATE jobs SET status='running', attempts=attempts+1 WHERE id=?")
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_err)?;
                tx.commit().await.map_err(db_err)?;
                Ok(Some(JobRow {
                    id: id as i64,
                    org: r.get::<String, _>("org_id"),
                    kind: r.get::<String, _>("kind"),
                    payload: r.get::<Option<String>, _>("payload").unwrap_or_default(),
                    status: "running".into(),
                    attempts: r.get::<i32, _>("attempts") as i64 + 1,
                }))
            }
        }
    }
    async fn job_done(&self, id: i64, ok: bool, note: &str) -> Result<(), StoreError> {
        sqlx::query("UPDATE jobs SET status=?, note=? WHERE id=?")
            .bind(if ok { "done" } else { "failed" })
            .bind(note)
            .bind(id as u64)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn audit_list(
        &self,
        org: &str,
        entity: Option<&str>,
        limit: i64,
    ) -> Result<Vec<AuditRow>, StoreError> {
        let rows = if let Some(e) = entity {
            sqlx::query("SELECT id, actor_id, action, entity, entity_id, DATE_FORMAT(created_at,'%Y-%m-%d %H:%i:%s') AS created_at FROM audit_events WHERE tenant_id=? AND entity=? ORDER BY id DESC LIMIT ?").bind(org).bind(e).bind(limit).fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query("SELECT id, actor_id, action, entity, entity_id, DATE_FORMAT(created_at,'%Y-%m-%d %H:%i:%s') AS created_at FROM audit_events WHERE tenant_id=? ORDER BY id DESC LIMIT ?").bind(org).bind(limit).fetch_all(&self.pool).await.map_err(db_err)?
        };
        Ok(rows
            .into_iter()
            .map(|x| AuditRow {
                id: x.get::<u64, _>("id") as i64,
                actor: x.get::<Option<String>, _>("actor_id"),
                action: x.get::<String, _>("action"),
                entity: x.get::<String, _>("entity"),
                entity_id: x.get::<i64, _>("entity_id"),
                created: x.get::<String, _>("created_at"),
            })
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
        sqlx::query("INSERT INTO audit_events (tenant_id, actor_id, action, entity, entity_id) VALUES (?,?,?,?,?)").bind(tenant).bind(actor).bind(action).bind(entity).bind(entity_id).execute(&self.pool).await.map_err(db_err)?;
        Ok(())
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
        let r =
            sqlx::query("INSERT INTO trust_accounts (org_id, client_id, currency) VALUES (?,?,?)")
                .bind(org)
                .bind(client_id as u64)
                .bind(currency)
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(TrustAccountRow {
            id: r.last_insert_id() as i64,
            org: org.into(),
            client_id,
            currency: currency.into(),
            balance: 0,
        })
    }
    async fn trust_get(
        &self,
        org: &str,
        id: i64,
    ) -> Result<(TrustAccountRow, Vec<TrustEntryRow>), StoreError> {
        let x = sqlx::query("SELECT id, org_id, client_id, currency, balance FROM trust_accounts WHERE id=? AND org_id=?").bind(id as u64).bind(org).fetch_optional(&self.pool).await.map_err(db_err)?.ok_or(StoreError::NotFound)?;
        let a = TrustAccountRow {
            id: x.get::<u64, _>("id") as i64,
            org: x.get::<String, _>("org_id"),
            client_id: x.get::<u64, _>("client_id") as i64,
            currency: x.get::<String, _>("currency"),
            balance: x.get::<i64, _>("balance"),
        };
        let es = sqlx::query("SELECT id, org_id, account_id, matter_id, kind, amount_minor, ref_type, ref_id, created_by FROM trust_entries WHERE account_id=? ORDER BY id").bind(id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok((
            a,
            es.into_iter()
                .map(|e| TrustEntryRow {
                    id: e.get::<u64, _>("id") as i64,
                    org: e.get::<String, _>("org_id"),
                    account_id: e.get::<u64, _>("account_id") as i64,
                    matter_id: e.get::<Option<u64>, _>("matter_id").map(|v| v as i64),
                    kind: e.get::<String, _>("kind"),
                    amount: e.get::<i64, _>("amount_minor"),
                    ref_type: e.get::<String, _>("ref_type"),
                    ref_id: e.get::<u64, _>("ref_id") as i64,
                    created_by: e.get::<String, _>("created_by"),
                })
                .collect(),
        ))
    }
    async fn trust_list(&self, org: &str) -> Result<Vec<TrustAccountRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, client_id, currency, balance FROM trust_accounts WHERE org_id=? ORDER BY id DESC LIMIT 200").bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| TrustAccountRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                client_id: x.get::<u64, _>("client_id") as i64,
                currency: x.get::<String, _>("currency"),
                balance: x.get::<i64, _>("balance"),
            })
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
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let e = post_entry(
            &mut tx, org, account, matter, kind, amount, ref_type, ref_id, created_by,
        )
        .await?;
        tx.commit().await.map_err(db_err)?;
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
        if from == to {
            return Err(StoreError::Conflict("same account".into()));
        }
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let a = post_entry(
            &mut tx, org, from, matter, "debit", amount, ref_type, ref_id, created_by,
        )
        .await?;
        let b = post_entry(
            &mut tx, org, to, matter, "credit", amount, ref_type, ref_id, created_by,
        )
        .await?;
        tx.commit().await.map_err(db_err)?;
        Ok((a, b))
    }
}

fn mkt_lead_row(x: &sqlx::mysql::MySqlRow) -> Result<MarketingLeadRow, StoreError> {
    use sqlx::Row;
    Ok(MarketingLeadRow {
        id: x.get::<u64, _>("id") as i64,
        org: x.get::<Option<String>, _>("org_id"),
        name: x.get::<String, _>("name"),
        email: x.get::<String, _>("email"),
        phone: x.get::<Option<String>, _>("phone"),
        company: x.get::<Option<String>, _>("company"),
        source: x.get::<String, _>("source"),
        status: x.get::<String, _>("status"),
        assigned_to: x.get::<Option<String>, _>("assigned_to"),
    })
}

#[async_trait::async_trait]
impl MarketingStore for MysqlOps {
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
        let r = sqlx::query("INSERT INTO marketing_leads (org_id, name, email, phone, company, source) VALUES (?,?,?,?,?,?)").bind(org).bind(name).bind(email).bind(phone).bind(company).bind(source).execute(&self.pool).await.map_err(db_err)?;
        Ok(MarketingLeadRow {
            id: r.last_insert_id() as i64,
            org: org.map(str::to_string),
            name: name.into(),
            email: email.into(),
            phone: phone.map(str::to_string),
            company: company.map(str::to_string),
            source: source.into(),
            status: "new".into(),
            assigned_to: None,
        })
    }
    async fn mkt_lead_list(&self, org: Option<&str>) -> Result<Vec<MarketingLeadRow>, StoreError> {
        let rows = if let Some(o) = org {
            sqlx::query("SELECT id, org_id, name, email, phone, company, source, status, assigned_to FROM marketing_leads WHERE org_id=? ORDER BY id DESC LIMIT 200").bind(o).fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query("SELECT id, org_id, name, email, phone, company, source, status, assigned_to FROM marketing_leads WHERE org_id IS NULL ORDER BY id DESC LIMIT 200").fetch_all(&self.pool).await.map_err(db_err)?
        };
        rows.iter().map(mkt_lead_row).collect()
    }
    async fn mkt_lead_get(&self, id: i64) -> Result<MarketingLeadRow, StoreError> {
        let x = sqlx::query("SELECT id, org_id, name, email, phone, company, source, status, assigned_to FROM marketing_leads WHERE id=?").bind(id as u64).fetch_optional(&self.pool).await.map_err(db_err)?.ok_or(StoreError::NotFound)?;
        mkt_lead_row(&x)
    }
    async fn mkt_lead_set_status(
        &self,
        id: i64,
        status: &str,
    ) -> Result<MarketingLeadRow, StoreError> {
        use sqlx::Row;
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let cur: String = sqlx::query("SELECT status FROM marketing_leads WHERE id=? FOR UPDATE")
            .bind(id as u64)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?
            .ok_or(StoreError::NotFound)?
            .try_get("status")
            .map_err(|e| StoreError::Db(e.to_string()))?;
        if !marketing_allowed_transition(&cur, status) {
            return Err(StoreError::Conflict(format!(
                "illegal transition {cur} -> {status}"
            )));
        }
        sqlx::query("UPDATE marketing_leads SET status=? WHERE id=?")
            .bind(status)
            .bind(id as u64)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        self.mkt_lead_get(id).await
    }
    async fn mkt_lead_assign(
        &self,
        id: i64,
        assignee: Option<&str>,
    ) -> Result<MarketingLeadRow, StoreError> {
        let n = sqlx::query("UPDATE marketing_leads SET assigned_to=? WHERE id=?")
            .bind(assignee)
            .bind(id as u64)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        self.mkt_lead_get(id).await
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
        let r =
            sqlx::query("INSERT INTO marketing_lead_notes (lead_id, author, note) VALUES (?,?,?)")
                .bind(lead as u64)
                .bind(author)
                .bind(note)
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(MarketingNoteRow {
            id: r.last_insert_id() as i64,
            lead_id: lead,
            author: author.into(),
            note: note.into(),
        })
    }
    async fn mkt_note_list(&self, lead: i64) -> Result<Vec<MarketingNoteRow>, StoreError> {
        use sqlx::Row;
        // Surface NotFound for notes on a missing lead instead of an empty list.
        self.mkt_lead_get(lead).await?;
        let rows = sqlx::query("SELECT id, lead_id, author, note FROM marketing_lead_notes WHERE lead_id=? ORDER BY id").bind(lead as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| MarketingNoteRow {
                id: x.get::<u64, _>("id") as i64,
                lead_id: x.get::<u64, _>("lead_id") as i64,
                author: x.get::<String, _>("author"),
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn mkt_followup_add(
        &self,
        lead: i64,
        due_at: &str,
        note: &str,
    ) -> Result<MarketingFollowUpRow, StoreError> {
        let r =
            sqlx::query("INSERT INTO marketing_follow_ups (lead_id, due_at, note) VALUES (?,?,?)")
                .bind(lead as u64)
                .bind(due_at)
                .bind(note)
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(MarketingFollowUpRow {
            id: r.last_insert_id() as i64,
            lead_id: lead,
            due_at: due_at.into(),
            done: false,
            note: note.into(),
        })
    }
    async fn mkt_followup_list(&self, lead: i64) -> Result<Vec<MarketingFollowUpRow>, StoreError> {
        use sqlx::Row;
        self.mkt_lead_get(lead).await?;
        let rows = sqlx::query("SELECT id, lead_id, DATE_FORMAT(due_at,'%Y-%m-%d %H:%i:%s') AS due_at, done, note FROM marketing_follow_ups WHERE lead_id=? ORDER BY id").bind(lead as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| MarketingFollowUpRow {
                id: x.get::<u64, _>("id") as i64,
                lead_id: x.get::<u64, _>("lead_id") as i64,
                due_at: x.get::<String, _>("due_at"),
                done: x.get::<bool, _>("done"),
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn mkt_followup_done(&self, id: i64) -> Result<MarketingFollowUpRow, StoreError> {
        use sqlx::Row;
        let n = sqlx::query("UPDATE marketing_follow_ups SET done=1 WHERE id=?")
            .bind(id as u64)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        let x = sqlx::query(
            "SELECT id, lead_id, DATE_FORMAT(due_at,'%Y-%m-%d %H:%i:%s') AS due_at, done, note FROM marketing_follow_ups WHERE id=?",
        )
        .bind(id as u64)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(MarketingFollowUpRow {
            id: x.get::<u64, _>("id") as i64,
            lead_id: x.get::<u64, _>("lead_id") as i64,
            due_at: x.get::<String, _>("due_at"),
            done: x.get::<bool, _>("done"),
            note: x.get::<String, _>("note"),
        })
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
        let r = sqlx::query("INSERT INTO marketing_demo_requests (lead_id, name, email, company, preferred_at) VALUES (?,?,?,?,?)").bind(lead.map(|v| v as u64)).bind(name).bind(email).bind(company).bind(preferred_at).execute(&self.pool).await.map_err(db_err)?;
        Ok(MarketingDemoRow {
            id: r.last_insert_id() as i64,
            lead_id: lead,
            name: name.into(),
            email: email.into(),
            company: company.into(),
            preferred_at: preferred_at.map(str::to_string),
            status: "new".into(),
        })
    }
    async fn mkt_demo_list(&self) -> Result<Vec<MarketingDemoRow>, StoreError> {
        use sqlx::Row;
        let rows = sqlx::query("SELECT id, lead_id, name, email, company, preferred_at, status FROM marketing_demo_requests ORDER BY id DESC LIMIT 200").fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| MarketingDemoRow {
                id: x.get::<u64, _>("id") as i64,
                lead_id: x.get::<Option<u64>, _>("lead_id").map(|v| v as i64),
                name: x.get::<String, _>("name"),
                email: x.get::<String, _>("email"),
                company: x.get::<String, _>("company"),
                preferred_at: x
                    .get::<Option<chrono::NaiveDateTime>, _>("preferred_at")
                    .map(|v| v.to_string()),
                status: x.get::<String, _>("status"),
            })
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
        let r = sqlx::query("INSERT INTO marketing_contact_messages (name, email, subject, message) VALUES (?,?,?,?)").bind(name).bind(email).bind(subject).bind(message).execute(&self.pool).await.map_err(db_err)?;
        Ok(MarketingContactRow {
            id: r.last_insert_id() as i64,
            name: name.into(),
            email: email.into(),
            subject: subject.into(),
            message: message.into(),
            status: "new".into(),
        })
    }
    async fn mkt_contact_list(&self) -> Result<Vec<MarketingContactRow>, StoreError> {
        use sqlx::Row;
        let rows = sqlx::query("SELECT id, name, email, subject, message, status FROM marketing_contact_messages ORDER BY id DESC LIMIT 200").fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| MarketingContactRow {
                id: x.get::<u64, _>("id") as i64,
                name: x.get::<String, _>("name"),
                email: x.get::<String, _>("email"),
                subject: x.get::<String, _>("subject"),
                message: x.get::<String, _>("message"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn mkt_subscribe(&self, email: &str) -> Result<MarketingSubscriberRow, StoreError> {
        let key = email.trim().to_lowercase();
        if !key.contains('@') {
            return Err(StoreError::Conflict("valid email required".into()));
        }
        sqlx::query("INSERT INTO marketing_newsletter_subscribers (email) VALUES (?)")
            .bind(&key)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(MarketingSubscriberRow {
            email: key,
            confirmed: false,
        })
    }
    async fn mkt_unsubscribe(&self, email: &str) -> Result<(), StoreError> {
        let key = email.trim().to_lowercase();
        let n = sqlx::query("DELETE FROM marketing_newsletter_subscribers WHERE email=?")
            .bind(&key)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
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
        sqlx::query("INSERT INTO marketing_pricing_plans (code, name, monthly_minor, annual_minor, features_json, active) VALUES (?,?,?,?,?,?) ON DUPLICATE KEY UPDATE name=VALUES(name), monthly_minor=VALUES(monthly_minor), annual_minor=VALUES(annual_minor), features_json=VALUES(features_json), active=VALUES(active)").bind(code).bind(name).bind(monthly_minor).bind(annual_minor).bind(features_json).bind(active).execute(&self.pool).await.map_err(db_err)?;
        use sqlx::Row;
        let x = sqlx::query("SELECT id, code, name, monthly_minor, annual_minor, CAST(features_json AS CHAR) AS features_json, active FROM marketing_pricing_plans WHERE code=?").bind(code).fetch_one(&self.pool).await.map_err(db_err)?;
        Ok(MarketingPlanRow {
            id: x.get::<u64, _>("id") as i64,
            code: x.get::<String, _>("code"),
            name: x.get::<String, _>("name"),
            monthly_minor: x.get::<i64, _>("monthly_minor"),
            annual_minor: x.get::<i64, _>("annual_minor"),
            features_json: x.get::<String, _>("features_json"),
            active: x.get::<bool, _>("active"),
        })
    }
    async fn mkt_pricing_list(&self) -> Result<Vec<MarketingPlanRow>, StoreError> {
        use sqlx::Row;
        let rows = sqlx::query("SELECT id, code, name, monthly_minor, annual_minor, CAST(features_json AS CHAR) AS features_json, active FROM marketing_pricing_plans ORDER BY id").fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| MarketingPlanRow {
                id: x.get::<u64, _>("id") as i64,
                code: x.get::<String, _>("code"),
                name: x.get::<String, _>("name"),
                monthly_minor: x.get::<i64, _>("monthly_minor"),
                annual_minor: x.get::<i64, _>("annual_minor"),
                features_json: x.get::<String, _>("features_json"),
                active: x.get::<bool, _>("active"),
            })
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
        sqlx::query("INSERT INTO marketing_faqs (slug, question, answer, sort) VALUES (?,?,?,?) ON DUPLICATE KEY UPDATE question=VALUES(question), answer=VALUES(answer), sort=VALUES(sort)").bind(slug).bind(question).bind(answer).bind(sort as i32).execute(&self.pool).await.map_err(db_err)?;
        use sqlx::Row;
        let x =
            sqlx::query("SELECT id, slug, question, answer, sort FROM marketing_faqs WHERE slug=?")
                .bind(slug)
                .fetch_one(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(MarketingFaqRow {
            id: x.get::<u64, _>("id") as i64,
            slug: x.get::<String, _>("slug"),
            question: x.get::<String, _>("question"),
            answer: x.get::<String, _>("answer"),
            sort: x.get::<i32, _>("sort") as i64,
        })
    }
    async fn mkt_faq_list(&self) -> Result<Vec<MarketingFaqRow>, StoreError> {
        use sqlx::Row;
        let rows = sqlx::query(
            "SELECT id, slug, question, answer, sort FROM marketing_faqs ORDER BY sort, id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| MarketingFaqRow {
                id: x.get::<u64, _>("id") as i64,
                slug: x.get::<String, _>("slug"),
                question: x.get::<String, _>("question"),
                answer: x.get::<String, _>("answer"),
                sort: x.get::<i32, _>("sort") as i64,
            })
            .collect())
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
        sqlx::query("INSERT INTO marketing_articles (slug, title, body_md, published) VALUES (?,?,?,?) ON DUPLICATE KEY UPDATE title=VALUES(title), body_md=VALUES(body_md), published=VALUES(published)").bind(slug).bind(title).bind(body_md).bind(published).execute(&self.pool).await.map_err(db_err)?;
        self.mkt_article_get(slug).await
    }
    async fn mkt_article_get(&self, slug: &str) -> Result<MarketingArticleRow, StoreError> {
        use sqlx::Row;
        let x = sqlx::query(
            "SELECT id, slug, title, body_md, published FROM marketing_articles WHERE slug=?",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .ok_or(StoreError::NotFound)?;
        Ok(MarketingArticleRow {
            id: x.get::<u64, _>("id") as i64,
            slug: x.get::<String, _>("slug"),
            title: x.get::<String, _>("title"),
            body_md: x.get::<String, _>("body_md"),
            published: x.get::<bool, _>("published"),
        })
    }
    async fn mkt_article_list(
        &self,
        published_only: bool,
    ) -> Result<Vec<MarketingArticleRow>, StoreError> {
        use sqlx::Row;
        let rows = if published_only {
            sqlx::query("SELECT id, slug, title, body_md, published FROM marketing_articles WHERE published=1 ORDER BY id DESC LIMIT 200").fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query("SELECT id, slug, title, body_md, published FROM marketing_articles ORDER BY id DESC LIMIT 200").fetch_all(&self.pool).await.map_err(db_err)?
        };
        Ok(rows
            .into_iter()
            .map(|x| MarketingArticleRow {
                id: x.get::<u64, _>("id") as i64,
                slug: x.get::<String, _>("slug"),
                title: x.get::<String, _>("title"),
                body_md: x.get::<String, _>("body_md"),
                published: x.get::<bool, _>("published"),
            })
            .collect())
    }
}

fn crm_meeting_row(x: &sqlx::mysql::MySqlRow) -> Result<MeetingRow, StoreError> {
    use sqlx::Row;
    Ok(MeetingRow {
        id: x.get::<u64, _>("id") as i64,
        org: x.get::<String, _>("org_id"),
        client_id: x.get::<u64, _>("client_id") as i64,
        title: x.get::<String, _>("title"),
        notes: x.get::<Option<String>, _>("notes"),
        happened_at: x
            .get::<chrono::NaiveDateTime, _>("happened_at")
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        created_by: x.get::<String, _>("created_by"),
    })
}

fn crm_tag_row(x: &sqlx::mysql::MySqlRow) -> Result<ClientTagRow, StoreError> {
    use sqlx::Row;
    Ok(ClientTagRow {
        id: x.get::<u64, _>("id") as i64,
        org: x.get::<String, _>("org_id"),
        name: x.get::<String, _>("name"),
    })
}

#[async_trait::async_trait]
impl super::ops::CrmStore for MysqlOps {
    async fn meeting_create(
        &self,
        org: &str,
        client_id: i64,
        title: &str,
        notes: Option<&str>,
        happened_at: &str,
        created_by: &str,
    ) -> Result<MeetingRow, StoreError> {
        if title.trim().chars().count() < 3 {
            return Err(StoreError::Conflict(
                "title must be at least 3 chars".into(),
            ));
        }
        let ts = chrono::NaiveDateTime::parse_from_str(happened_at, "%Y-%m-%d %H:%M:%S")
            .map_err(|_| StoreError::Conflict("happened_at must be YYYY-MM-DD HH:MM:SS".into()))?;
        let r = sqlx::query("INSERT INTO client_meetings (org_id, client_id, title, notes, happened_at, created_by) VALUES (?,?,?,?,?,?)").bind(org).bind(client_id as u64).bind(title).bind(notes).bind(ts).bind(created_by).execute(&self.pool).await.map_err(db_err)?;
        Ok(MeetingRow {
            id: r.last_insert_id() as i64,
            org: org.into(),
            client_id,
            title: title.into(),
            notes: notes.map(str::to_string),
            happened_at: ts.format("%Y-%m-%d %H:%M:%S").to_string(),
            created_by: created_by.into(),
        })
    }
    async fn meeting_list_by_client(
        &self,
        org: &str,
        client_id: i64,
    ) -> Result<Vec<MeetingRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, client_id, title, notes, happened_at, created_by FROM client_meetings WHERE org_id=? AND client_id=? ORDER BY id").bind(org).bind(client_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        rows.iter().map(crm_meeting_row).collect()
    }
    async fn meeting_list_by_org(&self, org: &str) -> Result<Vec<MeetingRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, client_id, title, notes, happened_at, created_by FROM client_meetings WHERE org_id=? ORDER BY id DESC LIMIT 200").bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        rows.iter().map(crm_meeting_row).collect()
    }
    async fn meeting_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM client_meetings WHERE id=? AND org_id=?")
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }
    async fn tag_create(&self, org: &str, name: &str) -> Result<ClientTagRow, StoreError> {
        let clean = name.trim();
        if clean.is_empty() {
            return Err(StoreError::Conflict("tag name required".into()));
        }
        if clean.chars().count() > 64 {
            return Err(StoreError::Conflict(
                "tag name must be at most 64 chars".into(),
            ));
        }
        let r = sqlx::query("INSERT INTO client_tags (org_id, name) VALUES (?,?)")
            .bind(org)
            .bind(clean)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(ClientTagRow {
            id: r.last_insert_id() as i64,
            org: org.into(),
            name: clean.into(),
        })
    }
    async fn tag_list(&self, org: &str) -> Result<Vec<ClientTagRow>, StoreError> {
        let rows =
            sqlx::query("SELECT id, org_id, name FROM client_tags WHERE org_id=? ORDER BY id")
                .bind(org)
                .fetch_all(&self.pool)
                .await
                .map_err(db_err)?;
        rows.iter().map(crm_tag_row).collect()
    }
    async fn tag_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM client_tags WHERE id=? AND org_id=?")
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }
    async fn tag_attach(&self, org: &str, tag_id: i64, client_id: i64) -> Result<(), StoreError> {
        let tag: Option<sqlx::mysql::MySqlRow> =
            sqlx::query("SELECT id FROM client_tags WHERE id=? AND org_id=?")
                .bind(tag_id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;
        if tag.is_none() {
            return Err(StoreError::NotFound);
        }
        // Idempotent: re-attaching an existing link is a no-op.
        sqlx::query("INSERT IGNORE INTO client_tag_links (tag_id, client_id) VALUES (?,?)")
            .bind(tag_id as u64)
            .bind(client_id as u64)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn tag_detach(&self, _org: &str, tag_id: i64, client_id: i64) -> Result<(), StoreError> {
        // Idempotent: detaching a missing link is a no-op.
        sqlx::query("DELETE FROM client_tag_links WHERE tag_id=? AND client_id=?")
            .bind(tag_id as u64)
            .bind(client_id as u64)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn tags_for_client(
        &self,
        org: &str,
        client_id: i64,
    ) -> Result<Vec<ClientTagRow>, StoreError> {
        let rows = sqlx::query("SELECT t.id, t.org_id, t.name FROM client_tags t JOIN client_tag_links l ON l.tag_id=t.id WHERE t.org_id=? AND l.client_id=? ORDER BY t.id").bind(org).bind(client_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        rows.iter().map(crm_tag_row).collect()
    }
}
