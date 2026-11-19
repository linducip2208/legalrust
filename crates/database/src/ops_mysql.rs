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
        created: x
            .try_get::<chrono::NaiveDateTime, _>("created_at")
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default(),
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
        let cat = if category.trim().is_empty() {
            "umum"
        } else {
            category.trim()
        };
        let r =
            sqlx::query("INSERT INTO deed_clauses (org_id, name, category, body) VALUES (?,?,?,?)")
                .bind(org)
                .bind(name)
                .bind(cat)
                .bind(body)
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(ClauseRow {
            id: r.last_insert_id() as i64,
            org: org.into(),
            name: name.into(),
            category: cat.into(),
            body: body.into(),
            version: 1,
        })
    }
    async fn clause_list(&self, org: &str) -> Result<Vec<ClauseRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, name, category, body, version FROM deed_clauses WHERE org_id=? ORDER BY id DESC LIMIT 200").bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| ClauseRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                name: x.get::<String, _>("name"),
                category: x.get::<String, _>("category"),
                body: x.get::<String, _>("body"),
                version: x.get::<i32, _>("version") as i64,
            })
            .collect())
    }
    async fn clause_update(&self, org: &str, id: i64, body: &str) -> Result<ClauseRow, StoreError> {
        if body.trim().is_empty() {
            return Err(StoreError::Conflict("body".into()));
        }
        let n = sqlx::query(
            "UPDATE deed_clauses SET body=?, version=version+1 WHERE id=? AND org_id=?",
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
        let x = sqlx::query(
            "SELECT id, org_id, name, category, body, version FROM deed_clauses WHERE id=? AND org_id=?",
        )
        .bind(id as u64)
        .bind(org)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .ok_or(StoreError::NotFound)?;
        Ok(ClauseRow {
            id: x.get::<u64, _>("id") as i64,
            org: x.get::<String, _>("org_id"),
            name: x.get::<String, _>("name"),
            category: x.get::<String, _>("category"),
            body: x.get::<String, _>("body"),
            version: x.get::<i32, _>("version") as i64,
        })
    }
    async fn clause_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM deed_clauses WHERE id=? AND org_id=?")
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
            sqlx::query("SELECT id, org_id, entity, entity_id, title, status, created_by, created_at FROM approval_requests WHERE org_id=? AND status=? ORDER BY id DESC LIMIT 200")
                .bind(org).bind(s).fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query("SELECT id, org_id, entity, entity_id, title, status, created_by, created_at FROM approval_requests WHERE org_id=? ORDER BY id DESC LIMIT 200")
                .bind(org).fetch_all(&self.pool).await.map_err(db_err)?
        };
        Ok(rows.into_iter().map(appr_req_row).collect())
    }
    async fn appr_get(
        &self,
        org: &str,
        id: i64,
    ) -> Result<(ApprovalReqRow, Vec<ApprovalStepRow>), StoreError> {
        let x = sqlx::query("SELECT id, org_id, entity, entity_id, title, status, created_by, created_at FROM approval_requests WHERE id=? AND org_id=?")
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
    async fn appr_decide_bulk(
        &self,
        org: &str,
        user: &str,
        items: &[(i64, i64, String, Option<String>)],
    ) -> Result<Vec<ApprovalStepRow>, StoreError> {
        if items.is_empty() || items.len() > 50 {
            return Err(StoreError::Conflict("items 1..50".into()));
        }
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let mut out = Vec::with_capacity(items.len());
        for (id, step_no, decision, comment) in items {
            if !["approve", "reject", "rework"].contains(&decision.as_str()) {
                return Err(StoreError::Conflict("decision".into()));
            }
            let x = sqlx::query("SELECT status FROM approval_requests WHERE id=? AND org_id=?")
                .bind(*id as u64)
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
            .bind(*id as u64)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;
            let target = steps
                .iter()
                .find(|r| r.get::<i32, _>("step_no") as i64 == *step_no)
                .ok_or(StoreError::NotFound)?;
            if target.get::<String, _>("status") != "pending" {
                return Err(StoreError::Conflict("step already decided".into()));
            }
            for r in &steps {
                if (r.get::<i32, _>("step_no") as i64) < *step_no
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
            let step_status = match decision.as_str() {
                "approve" => "approved",
                "reject" => "rejected",
                _ => "pending",
            };
            sqlx::query("UPDATE approval_steps SET status=?, approver_user=?, comment=? WHERE request_id=? AND step_no=?")
                .bind(step_status)
                .bind(user)
                .bind(comment.clone())
                .bind(*id as u64)
                .bind(*step_no)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
            let new_status = match decision.as_str() {
                "approve" if *step_no == max_no => "approved",
                "approve" => cur.as_str(),
                "reject" => "rejected",
                _ => "needs_rework",
            };
            if new_status == "needs_rework" {
                sqlx::query(
                    "UPDATE approval_steps SET status='pending', comment=NULL WHERE request_id=?",
                )
                .bind(*id as u64)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
            }
            sqlx::query("UPDATE approval_requests SET status=? WHERE id=?")
                .bind(new_status)
                .bind(*id as u64)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
            let fx = sqlx::query("SELECT id, request_id, step_no, approver_role, approver_user, status, comment FROM approval_steps WHERE request_id=? AND step_no=?")
                .bind(*id as u64).bind(*step_no).fetch_one(&mut *tx).await.map_err(db_err)?;
            out.push(appr_step_row(fx));
        }
        tx.commit().await.map_err(db_err)?;
        Ok(out)
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
    async fn idem_claim(&self, org: &str, key: &str, endpoint: &str) -> Result<(), StoreError> {
        if key.len() > 64 || endpoint.len() > 80 {
            return Err(StoreError::Conflict("key".into()));
        }
        sqlx::query("INSERT INTO idempotency_keys (org_id, idem_key, endpoint) VALUES (?,?,?)")
            .bind(org)
            .bind(key)
            .bind(endpoint)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                if let Some(dbe) = e.as_database_error() {
                    if dbe.code().as_deref() == Some("23000") {
                        return StoreError::Conflict("replayed".into());
                    }
                }
                db_err(e)
            })?;
        Ok(())
    }
    async fn apref_get(&self, user: &str) -> Result<ApprPrefRow, StoreError> {
        let row = sqlx::query(
            "SELECT user_id, digest_mode, escalate_to FROM approval_prefs WHERE user_id=?",
        )
        .bind(user)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(match row {
            Some(x) => ApprPrefRow {
                user: x.get::<String, _>("user_id"),
                digest_mode: x.get::<String, _>("digest_mode"),
                escalate_to: x.get::<String, _>("escalate_to"),
            },
            None => ApprPrefRow {
                user: user.into(),
                digest_mode: "instant".into(),
                escalate_to: String::new(),
            },
        })
    }
    async fn apref_set(
        &self,
        user: &str,
        digest_mode: &str,
        escalate_to: &str,
    ) -> Result<ApprPrefRow, StoreError> {
        if !matches!(digest_mode, "instant" | "daily" | "off") {
            return Err(StoreError::Conflict("digest_mode".into()));
        }
        if escalate_to.len() > 160 {
            return Err(StoreError::Conflict("escalate_to".into()));
        }
        sqlx::query(
            "INSERT INTO approval_prefs (user_id, digest_mode, escalate_to) VALUES (?,?,?) ON DUPLICATE KEY UPDATE digest_mode=VALUES(digest_mode), escalate_to=VALUES(escalate_to)",
        )
        .bind(user)
        .bind(digest_mode)
        .bind(escalate_to.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        self.apref_get(user).await
    }
    async fn freeze_get(&self, org: &str) -> Result<FreezeRow, StoreError> {
        let row = sqlx::query(
            "SELECT org_id, frozen, reason, set_by FROM approval_freeze WHERE org_id=?",
        )
        .bind(org)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(match row {
            Some(x) => FreezeRow {
                org: x.get::<String, _>("org_id"),
                frozen: x.get::<u8, _>("frozen") != 0,
                reason: x.get::<String, _>("reason"),
                set_by: x.get::<String, _>("set_by"),
            },
            None => FreezeRow {
                org: org.into(),
                frozen: false,
                reason: String::new(),
                set_by: String::new(),
            },
        })
    }
    async fn freeze_set(
        &self,
        org: &str,
        frozen: bool,
        reason: &str,
        set_by: &str,
    ) -> Result<FreezeRow, StoreError> {
        if reason.len() > 500 {
            return Err(StoreError::Conflict("reason".into()));
        }
        sqlx::query(
            "INSERT INTO approval_freeze (org_id, frozen, reason, set_by) VALUES (?,?,?,?) ON DUPLICATE KEY UPDATE frozen=VALUES(frozen), reason=VALUES(reason), set_by=VALUES(set_by)",
        )
        .bind(org)
        .bind(if frozen { 1u8 } else { 0u8 })
        .bind(reason.trim())
        .bind(set_by)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        self.freeze_get(org).await
    }
    #[allow(clippy::too_many_arguments)]
    async fn aroute_add(
        &self,
        org: &str,
        kind: &str,
        entity: &str,
        min_value: i64,
        role: &str,
        priority: i64,
        note: &str,
    ) -> Result<i64, StoreError> {
        if !matches!(kind, "amount" | "risk") {
            return Err(StoreError::Conflict("kind".into()));
        }
        if !["matter", "invoice", "deed"].contains(&entity) {
            return Err(StoreError::Conflict("entity".into()));
        }
        if min_value < 0 {
            return Err(StoreError::Conflict("min_value".into()));
        }
        let role = role.trim();
        if role.is_empty() || role.len() > 60 {
            return Err(StoreError::Conflict("role".into()));
        }
        if note.len() > 500 {
            return Err(StoreError::Conflict("note".into()));
        }
        let r = sqlx::query(
            "INSERT INTO approval_routes (org_id, kind, entity, min_value, role, priority, note) VALUES (?,?,?,?,?,?,?)",
        )
        .bind(org)
        .bind(kind)
        .bind(entity)
        .bind(min_value)
        .bind(role)
        .bind(priority)
        .bind(note.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn aroute_list(&self, org: &str) -> Result<Vec<RouteRuleRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, kind, entity, min_value, role, priority, note FROM approval_routes WHERE org_id=? ORDER BY priority DESC, id ASC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| RouteRuleRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                kind: x.get::<String, _>("kind"),
                entity: x.get::<String, _>("entity"),
                min_value: x.get::<i64, _>("min_value"),
                role: x.get::<String, _>("role"),
                priority: x.get::<i32, _>("priority") as i64,
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn aroute_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM approval_routes WHERE id=? AND org_id=?")
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
    async fn pol_snapshot(
        &self,
        org: &str,
        payload: &str,
        created_by: &str,
    ) -> Result<(i64, i64), StoreError> {
        if payload.len() > 200_000 || payload.trim().is_empty() {
            return Err(StoreError::Conflict("payload".into()));
        }
        if created_by.trim().is_empty() {
            return Err(StoreError::Conflict("created_by".into()));
        }
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let maxv: Option<i64> =
            sqlx::query("SELECT MAX(version) AS m FROM approval_policies WHERE org_id=?")
                .bind(org)
                .fetch_one(&mut *tx)
                .await
                .map_err(db_err)?
                .get::<Option<i64>, _>("m");
        let version = maxv.unwrap_or(0) + 1;
        let r = sqlx::query(
            "INSERT INTO approval_policies (org_id, version, payload, created_by) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(version)
        .bind(payload)
        .bind(created_by)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        let id = r.last_insert_id() as i64;
        tx.commit().await.map_err(db_err)?;
        Ok((id, version))
    }
    async fn pol_list(&self, org: &str) -> Result<Vec<PolicySnapRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, version, payload, created_by FROM approval_policies WHERE org_id=? ORDER BY version DESC LIMIT 100")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| PolicySnapRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                version: x.get::<i32, _>("version") as i64,
                payload: x.get::<String, _>("payload"),
                created_by: x.get::<String, _>("created_by"),
            })
            .collect())
    }
    async fn pol_get(&self, org: &str, version: i64) -> Result<PolicySnapRow, StoreError> {
        let x = sqlx::query("SELECT id, org_id, version, payload, created_by FROM approval_policies WHERE org_id=? AND version=?")
            .bind(org).bind(version).fetch_optional(&self.pool).await.map_err(db_err)?
            .ok_or(StoreError::NotFound)?;
        Ok(PolicySnapRow {
            id: x.get::<u64, _>("id") as i64,
            org: x.get::<String, _>("org_id"),
            version: x.get::<i32, _>("version") as i64,
            payload: x.get::<String, _>("payload"),
            created_by: x.get::<String, _>("created_by"),
        })
    }
    #[allow(clippy::too_many_arguments)]
    async fn dlc_set(
        &self,
        org: &str,
        doc_id: i64,
        on_hold: Option<bool>,
        archived: Option<bool>,
        trashed: Option<bool>,
        retain_until: Option<&str>,
        note: Option<&str>,
    ) -> Result<DocLifecycleRow, StoreError> {
        if doc_id <= 0 {
            return Err(StoreError::Conflict("doc_id".into()));
        }
        if let Some(d) = retain_until {
            if d.len() > 10 {
                return Err(StoreError::Conflict("retain_until".into()));
            }
        }
        if let Some(n) = note {
            if n.len() > 500 {
                return Err(StoreError::Conflict("note".into()));
            }
        }
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let cur: Option<(u64, u8, u8, u8, String, String)> =
            sqlx::query("SELECT id, on_hold, archived, trashed, retain_until, note FROM doc_lifecycle WHERE org_id=? AND doc_id=?")
                .bind(org).bind(doc_id as u64).fetch_optional(&mut *tx).await.map_err(db_err)?
                .map(|x| (x.get::<u64, _>("id"), x.get::<u8, _>("on_hold"), x.get::<u8, _>("archived"), x.get::<u8, _>("trashed"), x.get::<String, _>("retain_until"), x.get::<String, _>("note")));
        let row = match cur {
            Some((id, h, a, t, ru, n)) => DocLifecycleRow {
                id: id as i64,
                org: org.into(),
                doc_id,
                on_hold: on_hold.unwrap_or(h != 0),
                archived: archived.unwrap_or(a != 0),
                trashed: trashed.unwrap_or(t != 0),
                retain_until: retain_until.map(|v| v.trim().to_string()).unwrap_or(ru),
                note: note.map(|v| v.trim().to_string()).unwrap_or(n),
            },
            None => DocLifecycleRow {
                id: 0,
                org: org.into(),
                doc_id,
                on_hold: on_hold.unwrap_or(false),
                archived: archived.unwrap_or(false),
                trashed: trashed.unwrap_or(false),
                retain_until: retain_until.unwrap_or("").trim().into(),
                note: note.unwrap_or("").trim().into(),
            },
        };
        let id = if row.id == 0 {
            let r = sqlx::query("INSERT INTO doc_lifecycle (org_id, doc_id, on_hold, archived, trashed, retain_until, note) VALUES (?,?,?,?,?,?,?)")
                .bind(org).bind(doc_id as u64)
                .bind(if row.on_hold { 1u8 } else { 0u8 })
                .bind(if row.archived { 1u8 } else { 0u8 })
                .bind(if row.trashed { 1u8 } else { 0u8 })
                .bind(&row.retain_until).bind(&row.note)
                .execute(&mut *tx).await.map_err(db_err)?;
            r.last_insert_id() as i64
        } else {
            sqlx::query("UPDATE doc_lifecycle SET on_hold=?, archived=?, trashed=?, retain_until=?, note=? WHERE id=?")
                .bind(if row.on_hold { 1u8 } else { 0u8 })
                .bind(if row.archived { 1u8 } else { 0u8 })
                .bind(if row.trashed { 1u8 } else { 0u8 })
                .bind(&row.retain_until).bind(&row.note).bind(row.id as u64)
                .execute(&mut *tx).await.map_err(db_err)?;
            row.id
        };
        tx.commit().await.map_err(db_err)?;
        Ok(DocLifecycleRow { id, ..row })
    }
    async fn dlc_get(&self, org: &str, doc_id: i64) -> Result<Option<DocLifecycleRow>, StoreError> {
        let x = sqlx::query("SELECT id, org_id, doc_id, on_hold, archived, trashed, retain_until, note FROM doc_lifecycle WHERE org_id=? AND doc_id=?")
            .bind(org).bind(doc_id as u64).fetch_optional(&self.pool).await.map_err(db_err)?;
        Ok(x.map(|x| DocLifecycleRow {
            id: x.get::<u64, _>("id") as i64,
            org: x.get::<String, _>("org_id"),
            doc_id: x.get::<u64, _>("doc_id") as i64,
            on_hold: x.get::<u8, _>("on_hold") != 0,
            archived: x.get::<u8, _>("archived") != 0,
            trashed: x.get::<u8, _>("trashed") != 0,
            retain_until: x.get::<String, _>("retain_until"),
            note: x.get::<String, _>("note"),
        }))
    }
    async fn dlc_due(&self, org: &str) -> Result<Vec<DocLifecycleRow>, StoreError> {
        let today = chrono::Utc::now()
            .naive_utc()
            .date()
            .format("%Y-%m-%d")
            .to_string();
        let rows = sqlx::query("SELECT id, org_id, doc_id, on_hold, archived, trashed, retain_until, note FROM doc_lifecycle WHERE org_id=? AND archived=0 AND retain_until<>'' AND retain_until<=? ORDER BY retain_until ASC LIMIT 200")
            .bind(org).bind(today).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| DocLifecycleRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                doc_id: x.get::<u64, _>("doc_id") as i64,
                on_hold: x.get::<u8, _>("on_hold") != 0,
                archived: x.get::<u8, _>("archived") != 0,
                trashed: x.get::<u8, _>("trashed") != 0,
                retain_until: x.get::<String, _>("retain_until"),
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn dlc_clear(&self, org: &str, doc_id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM doc_lifecycle WHERE org_id=? AND doc_id=?")
            .bind(org)
            .bind(doc_id as u64)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }
    async fn mfield_add(
        &self,
        org: &str,
        key: &str,
        kind: &str,
        required: bool,
        note: &str,
    ) -> Result<i64, StoreError> {
        let k = key.trim().to_lowercase();
        if k.len() < 2
            || k.len() > 60
            || !k
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(StoreError::Conflict("key".into()));
        }
        if !matches!(kind, "text" | "number" | "date" | "tag") {
            return Err(StoreError::Conflict("kind".into()));
        }
        if note.len() > 200 {
            return Err(StoreError::Conflict("note".into()));
        }
        let r = sqlx::query(
            "INSERT INTO meta_fields (org_id, field_key, kind, required, note) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(&k)
        .bind(kind)
        .bind(if required { 1u8 } else { 0u8 })
        .bind(note.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn mfield_list(&self, org: &str) -> Result<Vec<MetaFieldRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, field_key, kind, required, note FROM meta_fields WHERE org_id=? ORDER BY field_key LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| MetaFieldRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                key: x.get::<String, _>("field_key"),
                kind: x.get::<String, _>("kind"),
                required: x.get::<u8, _>("required") != 0,
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn mfield_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let key: Option<String> =
            sqlx::query("SELECT field_key FROM meta_fields WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("field_key"));
        let key = match key {
            Some(k) => k,
            None => return Err(StoreError::NotFound),
        };
        let used: Option<u64> =
            sqlx::query("SELECT id FROM doc_meta WHERE org_id=? AND field_key=? LIMIT 1")
                .bind(org)
                .bind(&key)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<u64, _>("id"));
        if used.is_some() {
            return Err(StoreError::Conflict("field in use".into()));
        }
        sqlx::query("DELETE FROM meta_fields WHERE id=?")
            .bind(id as u64)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn dmeta_set(
        &self,
        org: &str,
        doc_id: i64,
        key: &str,
        value: &str,
    ) -> Result<i64, StoreError> {
        if doc_id <= 0 {
            return Err(StoreError::Conflict("doc_id".into()));
        }
        let k = key.trim().to_lowercase();
        if k.len() < 2 || k.len() > 60 {
            return Err(StoreError::Conflict("key".into()));
        }
        let kind: Option<String> =
            sqlx::query("SELECT kind FROM meta_fields WHERE org_id=? AND field_key=?")
                .bind(org)
                .bind(&k)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("kind"));
        let kind = match kind {
            Some(k) => k,
            None => return Err(StoreError::Conflict("unknown field".into())),
        };
        let v = value.trim();
        if v.is_empty() || v.len() > 500 {
            return Err(StoreError::Conflict("value".into()));
        }
        let kind_ok = match kind.as_str() {
            "number" => v.parse::<i64>().is_ok(),
            "date" => v.len() == 10 && v.as_bytes()[4] == b'-',
            _ => true,
        };
        if !kind_ok {
            return Err(StoreError::Conflict("number/date".into()));
        }
        // Non-tag fields upsert (one row per doc+key); tags accumulate rows.
        if kind != "tag" {
            let cur: Option<u64> =
                sqlx::query("SELECT id FROM doc_meta WHERE org_id=? AND doc_id=? AND field_key=?")
                    .bind(org)
                    .bind(doc_id as u64)
                    .bind(&k)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(db_err)?
                    .map(|x| x.get::<u64, _>("id"));
            if let Some(id) = cur {
                sqlx::query("UPDATE doc_meta SET field_value=? WHERE id=?")
                    .bind(v)
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    .map_err(db_err)?;
                return Ok(id as i64);
            }
        }
        let r = sqlx::query(
            "INSERT INTO doc_meta (org_id, doc_id, field_key, field_value) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(doc_id as u64)
        .bind(&k)
        .bind(v)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn dmeta_list(&self, org: &str, doc_id: i64) -> Result<Vec<DocMetaRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, doc_id, field_key, field_value FROM doc_meta WHERE org_id=? AND doc_id=? ORDER BY field_key, id LIMIT 200")
            .bind(org).bind(doc_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| DocMetaRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                doc_id: x.get::<u64, _>("doc_id") as i64,
                key: x.get::<String, _>("field_key"),
                value: x.get::<String, _>("field_value"),
            })
            .collect())
    }
    async fn dmeta_delete(
        &self,
        org: &str,
        doc_id: i64,
        key: &str,
        value: Option<&str>,
    ) -> Result<u64, StoreError> {
        let k = key.trim().to_lowercase();
        let n = if let Some(v) = value {
            sqlx::query("DELETE FROM doc_meta WHERE org_id=? AND doc_id=? AND field_key=? AND field_value=?")
                .bind(org).bind(doc_id as u64).bind(&k).bind(v)
                .execute(&self.pool).await.map_err(db_err)?.rows_affected()
        } else {
            sqlx::query("DELETE FROM doc_meta WHERE org_id=? AND doc_id=? AND field_key=?")
                .bind(org)
                .bind(doc_id as u64)
                .bind(&k)
                .execute(&self.pool)
                .await
                .map_err(db_err)?
                .rows_affected()
        };
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(n)
    }
    async fn dmeta_by_tag(&self, org: &str, tag: &str) -> Result<Vec<DocMetaRow>, StoreError> {
        let tag = tag.trim();
        if tag.is_empty() || tag.len() > 500 {
            return Err(StoreError::Conflict("tag".into()));
        }
        let rows = sqlx::query("SELECT m.id, m.org_id, m.doc_id, m.field_key, m.field_value FROM doc_meta m JOIN meta_fields f ON f.org_id=m.org_id AND f.field_key=m.field_key AND f.kind='tag' WHERE m.org_id=? AND m.field_value=? ORDER BY m.id DESC LIMIT 200")
            .bind(org).bind(tag).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| DocMetaRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                doc_id: x.get::<u64, _>("doc_id") as i64,
                key: x.get::<String, _>("field_key"),
                value: x.get::<String, _>("field_value"),
            })
            .collect())
    }
    async fn dlock_lock(
        &self,
        org: &str,
        doc_id: i64,
        holder: &str,
        note: &str,
    ) -> Result<DocLockRow, StoreError> {
        if doc_id <= 0 || holder.trim().is_empty() {
            return Err(StoreError::Conflict("lock".into()));
        }
        if note.len() > 200 {
            return Err(StoreError::Conflict("note".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT holder FROM doc_locks WHERE doc_id=? AND org_id=?")
                .bind(doc_id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("holder"));
        match cur {
            Some(h) if h != holder => return Err(StoreError::Conflict("locked by other".into())),
            Some(_) => {}
            None => {
                sqlx::query(
                    "INSERT INTO doc_locks (doc_id, org_id, holder, note) VALUES (?,?,?,?)",
                )
                .bind(doc_id as u64)
                .bind(org)
                .bind(holder.trim())
                .bind(note.trim())
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
            }
        }
        Ok(DocLockRow {
            doc_id,
            org: org.into(),
            holder: holder.trim().into(),
            note: note.trim().into(),
        })
    }
    async fn dlock_get(&self, org: &str, doc_id: i64) -> Result<Option<DocLockRow>, StoreError> {
        let x = sqlx::query(
            "SELECT doc_id, org_id, holder, note FROM doc_locks WHERE doc_id=? AND org_id=?",
        )
        .bind(doc_id as u64)
        .bind(org)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(x.map(|x| DocLockRow {
            doc_id: x.get::<u64, _>("doc_id") as i64,
            org: x.get::<String, _>("org_id"),
            holder: x.get::<String, _>("holder"),
            note: x.get::<String, _>("note"),
        }))
    }
    async fn dlock_unlock(
        &self,
        org: &str,
        doc_id: i64,
        by: &str,
        force: bool,
    ) -> Result<(), StoreError> {
        if force {
            let n = sqlx::query("DELETE FROM doc_locks WHERE doc_id=? AND org_id=?")
                .bind(doc_id as u64)
                .bind(org)
                .execute(&self.pool)
                .await
                .map_err(db_err)?
                .rows_affected();
            return if n == 0 {
                Err(StoreError::NotFound)
            } else {
                Ok(())
            };
        }
        let n = sqlx::query("DELETE FROM doc_locks WHERE doc_id=? AND org_id=? AND holder=?")
            .bind(doc_id as u64)
            .bind(org)
            .bind(by)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            let exists: bool =
                sqlx::query("SELECT doc_id FROM doc_locks WHERE doc_id=? AND org_id=?")
                    .bind(doc_id as u64)
                    .bind(org)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(db_err)?
                    .is_some();
            return if exists {
                Err(StoreError::Conflict("not holder".into()))
            } else {
                Err(StoreError::NotFound)
            };
        }
        Ok(())
    }
    async fn dfav_add(&self, org: &str, user: &str, doc_id: i64) -> Result<(), StoreError> {
        if doc_id <= 0 || user.trim().is_empty() {
            return Err(StoreError::Conflict("fav".into()));
        }
        sqlx::query("INSERT IGNORE INTO doc_favs (user_id, org_id, doc_id) VALUES (?,?,?)")
            .bind(user)
            .bind(org)
            .bind(doc_id as u64)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn dfav_list(&self, org: &str, user: &str) -> Result<Vec<i64>, StoreError> {
        let rows = sqlx::query(
            "SELECT doc_id FROM doc_favs WHERE org_id=? AND user_id=? ORDER BY doc_id LIMIT 500",
        )
        .bind(org)
        .bind(user)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| x.get::<u64, _>("doc_id") as i64)
            .collect())
    }
    async fn dfav_remove(&self, org: &str, user: &str, doc_id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM doc_favs WHERE org_id=? AND user_id=? AND doc_id=?")
            .bind(org)
            .bind(user)
            .bind(doc_id as u64)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    async fn qround_add(
        &self,
        org: &str,
        entity: &str,
        entity_id: i64,
        title: &str,
        required: i64,
        members: &[String],
        created_by: &str,
    ) -> Result<i64, StoreError> {
        if !["matter", "invoice", "deed"].contains(&entity) {
            return Err(StoreError::Conflict("entity".into()));
        }
        if entity_id <= 0 {
            return Err(StoreError::Conflict("entity_id".into()));
        }
        let title = title.trim();
        if title.len() < 2 || title.len() > 200 {
            return Err(StoreError::Conflict("title".into()));
        }
        let clean: Vec<String> = members
            .iter()
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect();
        if clean.is_empty() || clean.len() > 20 {
            return Err(StoreError::Conflict("members 1..20".into()));
        }
        if required < 1 || required > clean.len() as i64 {
            return Err(StoreError::Conflict("required".into()));
        }
        let r = sqlx::query(
            "INSERT INTO quorum_rounds (org_id, entity, entity_id, title, required, members, created_by) VALUES (?,?,?,?,?,?,?)",
        )
        .bind(org)
        .bind(entity)
        .bind(entity_id as u64)
        .bind(title)
        .bind(required)
        .bind(clean.join(","))
        .bind(created_by)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn qround_list(&self, org: &str) -> Result<Vec<QuorumRoundRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, entity, entity_id, title, required, members, status, created_by FROM quorum_rounds WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| QuorumRoundRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                entity: x.get::<String, _>("entity"),
                entity_id: x.get::<u64, _>("entity_id") as i64,
                title: x.get::<String, _>("title"),
                required: x.get::<i32, _>("required") as i64,
                members: x.get::<String, _>("members"),
                status: x.get::<String, _>("status"),
                created_by: x.get::<String, _>("created_by"),
            })
            .collect())
    }
    async fn qround_get(
        &self,
        org: &str,
        id: i64,
    ) -> Result<(QuorumRoundRow, Vec<QuorumVoteRow>), StoreError> {
        let r = sqlx::query("SELECT id, org_id, entity, entity_id, title, required, members, status, created_by FROM quorum_rounds WHERE id=? AND org_id=?")
            .bind(id as u64).bind(org).fetch_optional(&self.pool).await.map_err(db_err)?
            .ok_or(StoreError::NotFound)?;
        let round = QuorumRoundRow {
            id: r.get::<u64, _>("id") as i64,
            org: r.get::<String, _>("org_id"),
            entity: r.get::<String, _>("entity"),
            entity_id: r.get::<u64, _>("entity_id") as i64,
            title: r.get::<String, _>("title"),
            required: r.get::<i32, _>("required") as i64,
            members: r.get::<String, _>("members"),
            status: r.get::<String, _>("status"),
            created_by: r.get::<String, _>("created_by"),
        };
        let vs = sqlx::query("SELECT id, org_id, round_id, voter, decision FROM quorum_votes WHERE round_id=? AND org_id=? ORDER BY id")
            .bind(id as u64).bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok((
            round,
            vs.into_iter()
                .map(|x| QuorumVoteRow {
                    id: x.get::<u64, _>("id") as i64,
                    org: x.get::<String, _>("org_id"),
                    round_id: x.get::<u64, _>("round_id") as i64,
                    voter: x.get::<String, _>("voter"),
                    decision: x.get::<String, _>("decision"),
                })
                .collect(),
        ))
    }
    async fn qvote_add(
        &self,
        org: &str,
        round_id: i64,
        voter: &str,
        decision: &str,
    ) -> Result<String, StoreError> {
        if !["approve", "reject"].contains(&decision) {
            return Err(StoreError::Conflict("decision".into()));
        }
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let r = sqlx::query(
            "SELECT required, members, status FROM quorum_rounds WHERE id=? AND org_id=?",
        )
        .bind(round_id as u64)
        .bind(org)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?
        .ok_or(StoreError::NotFound)?;
        let required = r.get::<i32, _>("required") as i64;
        let members: Vec<String> = r
            .get::<String, _>("members")
            .split(',')
            .map(str::to_string)
            .collect();
        if r.get::<String, _>("status") != "open" {
            return Err(StoreError::Conflict("round closed".into()));
        }
        if !members.iter().any(|x| x == voter) {
            return Err(StoreError::Conflict("not a member".into()));
        }
        let dup: Option<u64> =
            sqlx::query("SELECT id FROM quorum_votes WHERE round_id=? AND org_id=? AND voter=?")
                .bind(round_id as u64)
                .bind(org)
                .bind(voter)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<u64, _>("id"));
        if dup.is_some() {
            return Err(StoreError::Conflict("already voted".into()));
        }
        sqlx::query(
            "INSERT INTO quorum_votes (org_id, round_id, voter, decision) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(round_id as u64)
        .bind(voter)
        .bind(decision)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        let approves: i64 = sqlx::query(
            "SELECT COUNT(*) AS n FROM quorum_votes WHERE round_id=? AND decision='approve'",
        )
        .bind(round_id as u64)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_err)?
        .get::<i64, _>("n");
        let rejects: i64 = sqlx::query(
            "SELECT COUNT(*) AS n FROM quorum_votes WHERE round_id=? AND decision='reject'",
        )
        .bind(round_id as u64)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_err)?
        .get::<i64, _>("n");
        let status = if approves >= required {
            "met"
        } else if rejects > members.len() as i64 - required {
            "failed"
        } else {
            "open"
        };
        sqlx::query("UPDATE quorum_rounds SET status=? WHERE id=?")
            .bind(status)
            .bind(round_id as u64)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(status.into())
    }
    async fn qoverride_request(
        &self,
        org: &str,
        round_id: i64,
        requested_by: &str,
    ) -> Result<i64, StoreError> {
        if requested_by.trim().is_empty() {
            return Err(StoreError::Conflict("requested_by".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM quorum_rounds WHERE id=? AND org_id=?")
                .bind(round_id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        match cur.as_deref() {
            Some("open") => {}
            Some(_) => return Err(StoreError::Conflict("round closed".into())),
            None => return Err(StoreError::NotFound),
        }
        let pending: Option<u64> = sqlx::query(
            "SELECT id FROM quorum_overrides WHERE round_id=? AND org_id=? AND status='requested'",
        )
        .bind(round_id as u64)
        .bind(org)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .map(|x| x.get::<u64, _>("id"));
        if pending.is_some() {
            return Err(StoreError::Conflict("override pending".into()));
        }
        let r = sqlx::query(
            "INSERT INTO quorum_overrides (org_id, round_id, requested_by) VALUES (?,?,?)",
        )
        .bind(org)
        .bind(round_id as u64)
        .bind(requested_by)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn qoverride_decide(
        &self,
        org: &str,
        id: i64,
        approve: bool,
        decided_by: &str,
    ) -> Result<String, StoreError> {
        if decided_by.trim().is_empty() {
            return Err(StoreError::Conflict("decided_by".into()));
        }
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let cur: Option<(u64, String, String)> = sqlx::query(
            "SELECT round_id, requested_by, status FROM quorum_overrides WHERE id=? AND org_id=?",
        )
        .bind(id as u64)
        .bind(org)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?
        .map(|x| {
            (
                x.get::<u64, _>("round_id"),
                x.get::<String, _>("requested_by"),
                x.get::<String, _>("status"),
            )
        });
        let (round_id, requested_by, status) = match cur {
            Some(v) => v,
            None => return Err(StoreError::NotFound),
        };
        if status != "requested" {
            return Err(StoreError::Conflict("already decided".into()));
        }
        if decided_by == requested_by {
            return Err(StoreError::Conflict("self-approval forbidden".into()));
        }
        let to = if approve { "approved" } else { "rejected" };
        sqlx::query("UPDATE quorum_overrides SET status=?, approved_by=? WHERE id=?")
            .bind(to)
            .bind(decided_by)
            .bind(id as u64)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        if approve {
            sqlx::query("UPDATE quorum_rounds SET status='overridden' WHERE id=?")
                .bind(round_id)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
        }
        tx.commit().await.map_err(db_err)?;
        Ok(to.into())
    }
    #[allow(clippy::too_many_arguments)]
    async fn receipt_append(
        &self,
        org: &str,
        req_id: i64,
        step_no: i64,
        decision: &str,
        actor: &str,
        hash: &str,
        prev: &str,
    ) -> Result<i64, StoreError> {
        if hash.len() != 64 || prev.len() != 64 {
            return Err(StoreError::Conflict("hash".into()));
        }
        if req_id <= 0 || actor.trim().is_empty() {
            return Err(StoreError::Conflict("receipt".into()));
        }
        let r = sqlx::query(
            "INSERT INTO approval_receipts (org_id, req_id, step_no, decision, actor, prev_hash, hash) VALUES (?,?,?,?,?,?,?)",
        )
        .bind(org)
        .bind(req_id as u64)
        .bind(step_no)
        .bind(decision)
        .bind(actor)
        .bind(prev)
        .bind(hash)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn receipt_list(&self, org: &str, req_id: i64) -> Result<Vec<ReceiptRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, req_id, step_no, decision, actor, prev_hash, hash FROM approval_receipts WHERE org_id=? AND req_id=? ORDER BY id")
            .bind(org).bind(req_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| ReceiptRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                req_id: x.get::<u64, _>("req_id") as i64,
                step_no: x.get::<i32, _>("step_no") as i64,
                decision: x.get::<String, _>("decision"),
                actor: x.get::<String, _>("actor"),
                prev_hash: x.get::<String, _>("prev_hash"),
                hash: x.get::<String, _>("hash"),
            })
            .collect())
    }
    async fn appr_redact_comment(
        &self,
        org: &str,
        id: i64,
        step_no: i64,
    ) -> Result<(), StoreError> {
        let req_ok: bool = sqlx::query("SELECT id FROM approval_requests WHERE id=? AND org_id=?")
            .bind(id as u64)
            .bind(org)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .is_some();
        if !req_ok {
            return Err(StoreError::NotFound);
        }
        let n = sqlx::query(
            "UPDATE approval_steps SET comment='[redacted]' WHERE request_id=? AND step_no=?",
        )
        .bind(id as u64)
        .bind(step_no)
        .execute(&self.pool)
        .await
        .map_err(db_err)?
        .rows_affected();
        if n == 0 {
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
        let r = sqlx::query(
            "INSERT INTO ppat_entries (org_id, number, deed_kind, party_seller, party_buyer, object_desc) VALUES (?,?,?,?,?,?)",
        )
        .bind(org)
        .bind(number)
        .bind(deed_kind)
        .bind(seller.trim())
        .bind(buyer.trim())
        .bind(object.trim())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let s = e.to_string();
            if s.contains("uq_ppat_org_number") || s.contains("Duplicate entry") {
                StoreError::Conflict("number taken".into())
            } else {
                db_err(e)
            }
        })?;
        Ok(r.last_insert_id() as i64)
    }
    async fn ppat_list(&self, org: &str) -> Result<Vec<PpatRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, number, deed_kind, party_seller, party_buyer, object_desc, status FROM ppat_entries WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| PpatRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                number: x.get::<String, _>("number"),
                deed_kind: x.get::<String, _>("deed_kind"),
                party_seller: x.get::<String, _>("party_seller"),
                party_buyer: x.get::<String, _>("party_buyer"),
                object_desc: x.get::<String, _>("object_desc"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn ppat_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "signed" | "registered") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        // Enforce draft->signed->registered in one conditional update.
        let from = if to == "signed" { "draft" } else { "signed" };
        let n =
            sqlx::query("UPDATE ppat_entries SET status=? WHERE id=? AND org_id=? AND status=?")
                .bind(to)
                .bind(id as u64)
                .bind(org)
                .bind(from)
                .execute(&self.pool)
                .await
                .map_err(db_err)?
                .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }
    async fn ppat_monthly(
        &self,
        org: &str,
        month: &str,
    ) -> Result<Vec<PpatMonthlyRow>, StoreError> {
        let ok = month.len() == 7
            && month.as_bytes()[4] == b'-'
            && month[..4].chars().all(|c| c.is_ascii_digit())
            && month[5..].chars().all(|c| c.is_ascii_digit());
        if !ok {
            return Err(StoreError::Conflict("month YYYY-MM".into()));
        }
        let rows = sqlx::query(
            "SELECT deed_kind, status, COUNT(*) AS n FROM ppat_entries WHERE org_id=? AND DATE_FORMAT(created_at,'%Y-%m')=? GROUP BY deed_kind, status ORDER BY deed_kind, status",
        )
        .bind(org)
        .bind(month)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| PpatMonthlyRow {
                kind: x.get::<String, _>("deed_kind"),
                status: x.get::<String, _>("status"),
                n: x.get::<i64, _>("n"),
            })
            .collect())
    }
    async fn ppat_dashboard(&self, org: &str) -> Result<PpatDashboard, StoreError> {
        let total: i64 = sqlx::query("SELECT COUNT(*) AS n FROM ppat_entries WHERE org_id=?")
            .bind(org)
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?
            .get::<i64, _>("n");
        let kinds = sqlx::query("SELECT deed_kind, COUNT(*) AS n FROM ppat_entries WHERE org_id=? GROUP BY deed_kind ORDER BY deed_kind")
            .bind(org)
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;
        let by_kind: Vec<PpatMonthlyRow> = kinds
            .into_iter()
            .map(|x| PpatMonthlyRow {
                kind: x.get::<String, _>("deed_kind"),
                status: String::new(),
                n: x.get::<i64, _>("n"),
            })
            .collect();
        let unverified_consents: i64 =
            sqlx::query("SELECT COUNT(*) AS n FROM deed_consents WHERE org_id=? AND verified=0")
                .bind(org)
                .fetch_one(&self.pool)
                .await
                .map_err(db_err)?
                .get::<i64, _>("n");
        let horizon = (chrono::Utc::now().naive_utc().date() + chrono::Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();
        let charges_due_30: i64 = sqlx::query("SELECT COUNT(*) AS n FROM deed_charges WHERE org_id=? AND valid_until<>'' AND valid_until<=?")
            .bind(org)
            .bind(horizon)
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?
            .get::<i64, _>("n");
        let open_disputes: i64 = sqlx::query(
            "SELECT COUNT(*) AS n FROM land_disputes WHERE org_id=? AND status<>'cleared'",
        )
        .bind(org)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)?
        .get::<i64, _>("n");
        let pending_handovers: i64 = sqlx::query(
            "SELECT COUNT(*) AS n FROM unit_handovers WHERE org_id=? AND status<>'delivered'",
        )
        .bind(org)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)?
        .get::<i64, _>("n");
        Ok(PpatDashboard {
            total_deeds: total,
            by_kind,
            unverified_consents,
            charges_due_30,
            open_disputes,
            pending_handovers,
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
        let own: Option<String> = sqlx::query("SELECT org_id FROM ppat_entries WHERE id=?")
            .bind(ppat_id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        let r = sqlx::query(
            "INSERT INTO notary_handoffs (org_id, ppat_id, doc_name, note) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(ppat_id as u64)
        .bind(doc_name)
        .bind(note.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn nhoff_list(
        &self,
        org: &str,
        ppat_id: i64,
    ) -> Result<Vec<NotaryHandoffRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, ppat_id, doc_name, received, note FROM notary_handoffs WHERE org_id=? AND ppat_id=? ORDER BY id DESC LIMIT 50")
            .bind(org).bind(ppat_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| NotaryHandoffRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                ppat_id: x.get::<u64, _>("ppat_id") as i64,
                doc_name: x.get::<String, _>("doc_name"),
                received: x.get::<u8, _>("received") != 0,
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn nhoff_set_received(
        &self,
        org: &str,
        id: i64,
        received: bool,
    ) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE notary_handoffs SET received=? WHERE id=? AND org_id=?")
            .bind(if received { 1u8 } else { 0u8 })
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
    async fn nhoff_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM notary_handoffs WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO girik_intakes (org_id, letter_c_ref, holder, area_minor, village, note) VALUES (?,?,?,?,?,?)",
        )
        .bind(org)
        .bind(letter_c_ref)
        .bind(holder)
        .bind(area_minor)
        .bind(village.trim())
        .bind(note.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn girik_list(&self, org: &str) -> Result<Vec<GirikRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, letter_c_ref, holder, area_minor, village, status, note FROM girik_intakes WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| GirikRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                letter_c_ref: x.get::<String, _>("letter_c_ref"),
                holder: x.get::<String, _>("holder"),
                area_minor: x.get::<i64, _>("area_minor"),
                village: x.get::<String, _>("village"),
                status: x.get::<String, _>("status"),
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn girik_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "verified" | "converted") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM girik_intakes WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("draft"), "verified") | (Some("verified"), "converted")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE girik_intakes SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn girik_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM girik_intakes WHERE id=? AND org_id=?")
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
        let own: Option<String> = sqlx::query("SELECT org_id FROM ppat_entries WHERE id=?")
            .bind(ppat_id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let cur: Option<u64> =
            sqlx::query("SELECT id FROM lease_terms WHERE org_id=? AND ppat_id=?")
                .bind(org)
                .bind(ppat_id as u64)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<u64, _>("id"));
        let id = match cur {
            Some(id) => {
                sqlx::query("UPDATE lease_terms SET monthly_minor=?, duration_months=?, start_date=?, deposit_minor=?, note=? WHERE id=?")
                    .bind(monthly_minor)
                    .bind(duration_months)
                    .bind(start_date.trim())
                    .bind(deposit_minor)
                    .bind(note.trim())
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_err)?;
                id as i64
            }
            None => {
                let r = sqlx::query(
                    "INSERT INTO lease_terms (org_id, ppat_id, monthly_minor, duration_months, start_date, deposit_minor, note) VALUES (?,?,?,?,?,?,?)",
                )
                .bind(org)
                .bind(ppat_id as u64)
                .bind(monthly_minor)
                .bind(duration_months)
                .bind(start_date.trim())
                .bind(deposit_minor)
                .bind(note.trim())
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
                r.last_insert_id() as i64
            }
        };
        tx.commit().await.map_err(db_err)?;
        Ok(id)
    }
    async fn lease_get(&self, org: &str, ppat_id: i64) -> Result<Option<LeaseTermRow>, StoreError> {
        let row = sqlx::query("SELECT id, org_id, ppat_id, monthly_minor, duration_months, start_date, deposit_minor, note FROM lease_terms WHERE org_id=? AND ppat_id=?")
            .bind(org).bind(ppat_id as u64).fetch_optional(&self.pool).await.map_err(db_err)?;
        Ok(row.map(|x| LeaseTermRow {
            id: x.get::<u64, _>("id") as i64,
            org: x.get::<String, _>("org_id"),
            ppat_id: x.get::<u64, _>("ppat_id") as i64,
            monthly_minor: x.get::<i64, _>("monthly_minor"),
            duration_months: x.get::<i32, _>("duration_months") as i64,
            start_date: x.get::<String, _>("start_date"),
            deposit_minor: x.get::<i64, _>("deposit_minor"),
            note: x.get::<String, _>("note"),
        }))
    }
    async fn lease_delete(&self, org: &str, ppat_id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM lease_terms WHERE org_id=? AND ppat_id=?")
            .bind(org)
            .bind(ppat_id as u64)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
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
        let r = sqlx::query(
            "INSERT INTO dev_batches (org_id, developer, batch_no, total_units, note) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(developer)
        .bind(batch_no)
        .bind(total_units)
        .bind(note.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn dbatch_list(&self, org: &str) -> Result<Vec<DevBatchRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, developer, batch_no, total_units, status, note FROM dev_batches WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| DevBatchRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                developer: x.get::<String, _>("developer"),
                batch_no: x.get::<String, _>("batch_no"),
                total_units: x.get::<i32, _>("total_units") as i64,
                status: x.get::<String, _>("status"),
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn dbatch_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "processing" | "done") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM dev_batches WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("planned"), "processing") | (Some("processing"), "done")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE dev_batches SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn dbatch_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM dev_batches WHERE id=? AND org_id=?")
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
        if let Some(bid) = batch_id {
            let own: Option<String> = sqlx::query("SELECT org_id FROM dev_batches WHERE id=?")
                .bind(bid as u64)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("org_id"));
            if own.as_deref() != Some(org) {
                return Err(StoreError::NotFound);
            }
        }
        let r = sqlx::query(
            "INSERT INTO dev_units (org_id, batch_id, unit_code, unit_type, price_minor) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(batch_id.map(|b| b as u64))
        .bind(unit_code)
        .bind(unit_type)
        .bind(price_minor)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn dunit_list(
        &self,
        org: &str,
        batch_id: Option<i64>,
    ) -> Result<Vec<DevUnitRow>, StoreError> {
        let rows = if let Some(bid) = batch_id {
            sqlx::query("SELECT id, org_id, batch_id, unit_code, unit_type, price_minor, status FROM dev_units WHERE org_id=? AND batch_id=? ORDER BY id DESC LIMIT 500")
                .bind(org).bind(bid as u64).fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query("SELECT id, org_id, batch_id, unit_code, unit_type, price_minor, status FROM dev_units WHERE org_id=? ORDER BY id DESC LIMIT 500")
                .bind(org).fetch_all(&self.pool).await.map_err(db_err)?
        };
        Ok(rows
            .into_iter()
            .map(|x| DevUnitRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                batch_id: x.get::<Option<u64>, _>("batch_id").map(|b| b as i64),
                unit_code: x.get::<String, _>("unit_code"),
                unit_type: x.get::<String, _>("unit_type"),
                price_minor: x.get::<i64, _>("price_minor"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn dunit_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "booked" | "sold" | "available") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM dev_units WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("available"), "booked")
                | (Some("booked"), "sold")
                | (Some("booked"), "available")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE dev_units SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn dunit_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM dev_units WHERE id=? AND org_id=?")
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
        let own: Option<String> = sqlx::query("SELECT org_id FROM ppat_entries WHERE id=?")
            .bind(ppat_id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        let r = sqlx::query(
            "INSERT INTO kpr_packs (org_id, ppat_id, bank, amount_minor, note) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(ppat_id as u64)
        .bind(bank)
        .bind(amount_minor)
        .bind(note.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn kpr_list(&self, org: &str, ppat_id: i64) -> Result<Vec<KprPackRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, ppat_id, bank, amount_minor, status, note FROM kpr_packs WHERE org_id=? AND ppat_id=? ORDER BY id DESC LIMIT 50")
            .bind(org).bind(ppat_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| KprPackRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                ppat_id: x.get::<u64, _>("ppat_id") as i64,
                bank: x.get::<String, _>("bank"),
                amount_minor: x.get::<i64, _>("amount_minor"),
                status: x.get::<String, _>("status"),
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn kpr_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "approved" | "rejected" | "disbursed") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM kpr_packs WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("applied"), "approved")
                | (Some("applied"), "rejected")
                | (Some("approved"), "disbursed")
                | (Some("approved"), "rejected")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE kpr_packs SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn kpr_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM kpr_packs WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO protocol_register (org_id, entry_date, book_ref, deed_no, party, direction) VALUES (?,?,?,?,?,?)",
        )
        .bind(org)
        .bind(entry_date.trim())
        .bind(book_ref.trim())
        .bind(deed_no.trim())
        .bind(party.trim())
        .bind(direction)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn preg_list(&self, org: &str) -> Result<Vec<PregRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, entry_date, book_ref, deed_no, party, direction FROM protocol_register WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| PregRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                entry_date: x.get::<String, _>("entry_date"),
                book_ref: x.get::<String, _>("book_ref"),
                deed_no: x.get::<String, _>("deed_no"),
                party: x.get::<String, _>("party"),
                direction: x.get::<String, _>("direction"),
            })
            .collect())
    }
    async fn preg_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM protocol_register WHERE id=? AND org_id=?")
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
        let own: Option<String> = sqlx::query("SELECT org_id FROM ppat_entries WHERE id=?")
            .bind(ppat_id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        let r = sqlx::query(
            "INSERT INTO ajb_drafts (org_id, ppat_id, template_name, body, price_minor) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(ppat_id as u64)
        .bind(template_name.trim())
        .bind(body)
        .bind(price_minor)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn ajbd_list(&self, org: &str, ppat_id: i64) -> Result<Vec<AjbDraftRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, ppat_id, template_name, body, price_minor FROM ajb_drafts WHERE org_id=? AND ppat_id=? ORDER BY id DESC LIMIT 50")
            .bind(org).bind(ppat_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| AjbDraftRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                ppat_id: x.get::<u64, _>("ppat_id") as i64,
                template_name: x.get::<String, _>("template_name"),
                body: x.get::<String, _>("body"),
                price_minor: x.get::<i64, _>("price_minor"),
            })
            .collect())
    }
    async fn ajbd_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM ajb_drafts WHERE id=? AND org_id=?")
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
    async fn clink_add(
        &self,
        org: &str,
        ppat_id: i64,
        doc_id: i64,
        doc_name: &str,
        label: &str,
    ) -> Result<i64, StoreError> {
        if label.len() > 120 {
            return Err(StoreError::Conflict("label".into()));
        }
        let own: Option<String> = sqlx::query("SELECT org_id FROM ppat_entries WHERE id=?")
            .bind(ppat_id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        let r = sqlx::query(
            "INSERT INTO cert_links (org_id, ppat_id, doc_id, doc_name, label) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(ppat_id as u64)
        .bind(doc_id as u64)
        .bind(doc_name)
        .bind(label.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn clink_list(&self, org: &str, ppat_id: i64) -> Result<Vec<CertLinkRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, ppat_id, doc_id, doc_name, label FROM cert_links WHERE org_id=? AND ppat_id=? ORDER BY id DESC LIMIT 50")
            .bind(org).bind(ppat_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| CertLinkRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                ppat_id: x.get::<u64, _>("ppat_id") as i64,
                doc_id: x.get::<u64, _>("doc_id") as i64,
                doc_name: x.get::<String, _>("doc_name"),
                label: x.get::<String, _>("label"),
            })
            .collect())
    }
    async fn clink_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM cert_links WHERE id=? AND org_id=?")
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
    async fn bpn_add(
        &self,
        org: &str,
        kind: &str,
        ref_no: &str,
        office: &str,
        note: &str,
    ) -> Result<i64, StoreError> {
        if !matches!(kind, "verification" | "queue" | "response") {
            return Err(StoreError::Conflict("kind".into()));
        }
        if ref_no.trim().is_empty() || ref_no.len() > 120 {
            return Err(StoreError::Conflict("ref_no".into()));
        }
        if office.len() > 120 || note.len() > 500 {
            return Err(StoreError::Conflict("office/note".into()));
        }
        let init = if kind == "queue" { "waiting" } else { "draft" };
        let r = sqlx::query(
            "INSERT INTO bpn_tracks (org_id, kind, ref_no, office, status, note) VALUES (?,?,?,?,?,?)",
        )
        .bind(org)
        .bind(kind)
        .bind(ref_no.trim())
        .bind(office.trim())
        .bind(init)
        .bind(note.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn bpn_list(
        &self,
        org: &str,
        kind: Option<&str>,
    ) -> Result<Vec<BpnTrackRow>, StoreError> {
        let rows = if let Some(k) = kind {
            sqlx::query("SELECT id, org_id, kind, ref_no, office, status, note FROM bpn_tracks WHERE org_id=? AND kind=? ORDER BY id DESC LIMIT 200")
                .bind(org).bind(k).fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query("SELECT id, org_id, kind, ref_no, office, status, note FROM bpn_tracks WHERE org_id=? ORDER BY id DESC LIMIT 200")
                .bind(org).fetch_all(&self.pool).await.map_err(db_err)?
        };
        Ok(rows
            .into_iter()
            .map(|x| BpnTrackRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                kind: x.get::<String, _>("kind"),
                ref_no: x.get::<String, _>("ref_no"),
                office: x.get::<String, _>("office"),
                status: x.get::<String, _>("status"),
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn bpn_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        let cur: Option<(String, String)> =
            sqlx::query("SELECT kind, status FROM bpn_tracks WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| (x.get::<String, _>("kind"), x.get::<String, _>("status")));
        let (kind, status) = match cur {
            Some(v) => v,
            None => return Err(StoreError::NotFound),
        };
        let ok = match kind.as_str() {
            "verification" => matches!(
                (status.as_str(), to),
                ("draft", "submitted") | ("submitted", "answered")
            ),
            "queue" => matches!(
                (status.as_str(), to),
                ("waiting", "called") | ("called", "done")
            ),
            _ => matches!((status.as_str(), to), ("draft", "filed")),
        };
        if !ok {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        sqlx::query("UPDATE bpn_tracks SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn bpn_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM bpn_tracks WHERE id=? AND org_id=?")
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
    async fn rrule_upsert(
        &self,
        org: &str,
        entity: &str,
        retain_years: i64,
        action: &str,
        note: &str,
    ) -> Result<i64, StoreError> {
        if !rule_entity_ok(entity) {
            return Err(StoreError::Conflict("entity".into()));
        }
        if !(1..=100).contains(&retain_years) {
            return Err(StoreError::Conflict("retain_years 1..100".into()));
        }
        if !matches!(action, "archive" | "purge") {
            return Err(StoreError::Conflict("action".into()));
        }
        if note.len() > 500 {
            return Err(StoreError::Conflict("note".into()));
        }
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let cur: Option<u64> =
            sqlx::query("SELECT id FROM retention_rules WHERE org_id=? AND entity=?")
                .bind(org)
                .bind(entity)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<u64, _>("id"));
        let id = match cur {
            Some(id) => {
                sqlx::query("UPDATE retention_rules SET retain_years=?, action=?, active=1, note=? WHERE id=?")
                    .bind(retain_years)
                    .bind(action)
                    .bind(note.trim())
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_err)?;
                id as i64
            }
            None => {
                let r = sqlx::query(
                    "INSERT INTO retention_rules (org_id, entity, retain_years, action, note) VALUES (?,?,?,?,?)",
                )
                .bind(org)
                .bind(entity)
                .bind(retain_years)
                .bind(action)
                .bind(note.trim())
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
                r.last_insert_id() as i64
            }
        };
        tx.commit().await.map_err(db_err)?;
        Ok(id)
    }
    async fn rrule_list(&self, org: &str) -> Result<Vec<RetentionRuleRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, entity, retain_years, action, active, note FROM retention_rules WHERE org_id=? ORDER BY entity LIMIT 100")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| RetentionRuleRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                entity: x.get::<String, _>("entity"),
                retain_years: x.get::<i32, _>("retain_years") as i64,
                action: x.get::<String, _>("action"),
                active: x.get::<u8, _>("active") != 0,
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn rrule_set_active(&self, org: &str, id: i64, active: bool) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE retention_rules SET active=? WHERE id=? AND org_id=?")
            .bind(if active { 1u8 } else { 0u8 })
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
    async fn rrule_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM retention_rules WHERE id=? AND org_id=?")
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
    async fn preq_add(
        &self,
        org: &str,
        entity: &str,
        entity_id: i64,
        reason: &str,
        requested_by: &str,
    ) -> Result<i64, StoreError> {
        if !purgeable(entity) {
            return Err(StoreError::Conflict("entity not purgeable".into()));
        }
        if entity_id <= 0 {
            return Err(StoreError::Conflict("entity_id".into()));
        }
        let reason = reason.trim();
        if reason.len() < 5 || reason.len() > 500 {
            return Err(StoreError::Conflict("reason 5..500".into()));
        }
        if requested_by.trim().is_empty() {
            return Err(StoreError::Conflict("requested_by".into()));
        }
        let r = sqlx::query(
            "INSERT INTO purge_requests (org_id, entity, entity_id, reason, requested_by) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(entity)
        .bind(entity_id as u64)
        .bind(reason)
        .bind(requested_by)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn preq_list(&self, org: &str) -> Result<Vec<PurgeRequestRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, entity, entity_id, reason, requested_by, status, decided_by FROM purge_requests WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| PurgeRequestRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                entity: x.get::<String, _>("entity"),
                entity_id: x.get::<u64, _>("entity_id") as i64,
                reason: x.get::<String, _>("reason"),
                requested_by: x.get::<String, _>("requested_by"),
                status: x.get::<String, _>("status"),
                decided_by: x.get::<String, _>("decided_by"),
            })
            .collect())
    }
    async fn preq_decide(
        &self,
        org: &str,
        id: i64,
        approve: bool,
        decided_by: &str,
    ) -> Result<String, StoreError> {
        if decided_by.trim().is_empty() {
            return Err(StoreError::Conflict("decided_by".into()));
        }
        let cur: Option<(String, String)> =
            sqlx::query("SELECT requested_by, status FROM purge_requests WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| {
                    (
                        x.get::<String, _>("requested_by"),
                        x.get::<String, _>("status"),
                    )
                });
        let (requested_by, status) = match cur {
            Some(v) => v,
            None => return Err(StoreError::NotFound),
        };
        if status != "requested" {
            return Err(StoreError::Conflict("already decided".into()));
        }
        if decided_by == requested_by {
            return Err(StoreError::Conflict("self-approval forbidden".into()));
        }
        let to = if approve { "approved" } else { "rejected" };
        sqlx::query("UPDATE purge_requests SET status=?, decided_by=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(decided_by)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(to.into())
    }
    async fn preq_execute(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let cur: Option<(String, u64, String)> = sqlx::query(
            "SELECT entity, entity_id, status FROM purge_requests WHERE id=? AND org_id=?",
        )
        .bind(id as u64)
        .bind(org)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?
        .map(|x| {
            (
                x.get::<String, _>("entity"),
                x.get::<u64, _>("entity_id"),
                x.get::<String, _>("status"),
            )
        });
        let (entity, entity_id, status) = match cur {
            Some(v) => v,
            None => return Err(StoreError::NotFound),
        };
        if status != "approved" {
            return Err(StoreError::Conflict("not approved".into()));
        }
        let rule: Option<String> = sqlx::query(
            "SELECT action FROM retention_rules WHERE org_id=? AND entity=? AND active=1",
        )
        .bind(org)
        .bind(&entity)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?
        .map(|x| x.get::<String, _>("action"));
        if rule.as_deref() != Some("purge") {
            return Err(StoreError::Conflict("no active purge rule".into()));
        }
        let table_ok = purgeable(&entity);
        if !table_ok {
            return Err(StoreError::Conflict("entity not purgeable".into()));
        }
        let n = match entity.as_str() {
            "bpn" => sqlx::query("DELETE FROM bpn_tracks WHERE id=? AND org_id=?"),
            "girik" => sqlx::query("DELETE FROM girik_intakes WHERE id=? AND org_id=?"),
            "lhist" => sqlx::query("DELETE FROM land_history WHERE id=? AND org_id=?"),
            "uho" => sqlx::query("DELETE FROM unit_handovers WHERE id=? AND org_id=?"),
            "preg" => sqlx::query("DELETE FROM protocol_register WHERE id=? AND org_id=?"),
            _ => sqlx::query("DELETE FROM notary_handoffs WHERE id=? AND org_id=?"),
        }
        .bind(entity_id)
        .bind(org)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?
        .rows_affected();
        if n == 0 {
            return Err(StoreError::Conflict("target missing".into()));
        }
        sqlx::query("UPDATE purge_requests SET status='executed' WHERE id=?")
            .bind(id as u64)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(())
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
        let r = sqlx::query(
            "INSERT INTO dpia_assessments (org_id, title, data_types, risk_level) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(title)
        .bind(data_types.trim())
        .bind(risk)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn dpia_list(&self, org: &str) -> Result<Vec<DpiaRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, title, data_types, risk_level, status FROM dpia_assessments WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| DpiaRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                title: x.get::<String, _>("title"),
                data_types: x.get::<String, _>("data_types"),
                risk_level: x.get::<String, _>("risk_level"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn dpia_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "review" | "approved") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let from = if to == "review" { "draft" } else { "review" };
        let n = sqlx::query(
            "UPDATE dpia_assessments SET status=? WHERE id=? AND org_id=? AND status=?",
        )
        .bind(to)
        .bind(id as u64)
        .bind(org)
        .bind(from)
        .execute(&self.pool)
        .await
        .map_err(db_err)?
        .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
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
        let r = sqlx::query(
            "INSERT INTO breach_logs (org_id, title, discovered_at, affected_count) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(title)
        .bind(discovered.trim())
        .bind(affected)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn breach_list(&self, org: &str) -> Result<Vec<BreachRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, title, discovered_at, affected_count, status FROM breach_logs WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| {
                let affected: i64 = x
                    .try_get::<i64, _>("affected_count")
                    .unwrap_or_else(|_| x.get::<u64, _>("affected_count") as i64);
                BreachRow {
                    id: x.get::<u64, _>("id") as i64,
                    org: x.get::<String, _>("org_id"),
                    title: x.get::<String, _>("title"),
                    discovered_at: x.get::<String, _>("discovered_at"),
                    affected_count: affected,
                    status: x.get::<String, _>("status"),
                }
            })
            .collect())
    }
    async fn breach_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "assessing" | "notified" | "closed") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let from = match to {
            "assessing" => "draft",
            "notified" => "assessing",
            _ => "notified",
        };
        let n = sqlx::query("UPDATE breach_logs SET status=? WHERE id=? AND org_id=? AND status=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .bind(from)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
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
        let r = sqlx::query(
            "INSERT INTO approval_delegations (org_id, delegator_id, delegate_id, active) VALUES (?,?,?,?) ON DUPLICATE KEY UPDATE active=VALUES(active)",
        )
        .bind(org)
        .bind(delegator)
        .bind(delegate)
        .bind(active)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        let id = r.last_insert_id() as i64;
        if id == 0 {
            // Upsert hit: fetch existing row id.
            let x = sqlx::query(
                "SELECT id FROM approval_delegations WHERE org_id=? AND delegator_id=? AND delegate_id=?",
            )
            .bind(org)
            .bind(delegator)
            .bind(delegate)
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?;
            return Ok(x.get::<u64, _>("id") as i64);
        }
        Ok(id)
    }
    async fn delegation_list(&self, org: &str) -> Result<Vec<DelegationRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, delegator_id, delegate_id, active FROM approval_delegations WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| DelegationRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                delegator: x.get::<String, _>("delegator_id"),
                delegate: x.get::<String, _>("delegate_id"),
                active: x.get::<bool, _>("active"),
            })
            .collect())
    }
    async fn delegation_delegators(
        &self,
        org: &str,
        user: &str,
    ) -> Result<Vec<String>, StoreError> {
        let rows = sqlx::query("SELECT delegator_id FROM approval_delegations WHERE org_id=? AND delegate_id=? AND active=1")
            .bind(org).bind(user).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| x.get::<String, _>("delegator_id"))
            .collect())
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
        sqlx::query(
            "INSERT INTO notification_prefs (user_id, kind, enabled) VALUES (?,?,?) ON DUPLICATE KEY UPDATE enabled=VALUES(enabled)",
        )
        .bind(user)
        .bind(kind.trim())
        .bind(enabled)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }
    async fn vendor_create(&self, org: &str, name: &str, service: &str) -> Result<i64, StoreError> {
        let name = name.trim();
        if name.is_empty() || name.len() > 200 {
            return Err(StoreError::Conflict("name".into()));
        }
        let r = sqlx::query("INSERT INTO vendor_register (org_id, name, service) VALUES (?,?,?)")
            .bind(org)
            .bind(name)
            .bind(service.trim())
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn vendor_list(&self, org: &str) -> Result<Vec<VendorRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, name, service, dpa_status FROM vendor_register WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| VendorRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                name: x.get::<String, _>("name"),
                service: x.get::<String, _>("service"),
                dpa_status: x.get::<String, _>("dpa_status"),
            })
            .collect())
    }
    async fn vendor_set_dpa(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "draft" | "review" | "signed") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let n = sqlx::query("UPDATE vendor_register SET dpa_status=? WHERE id=? AND org_id=?")
            .bind(to)
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
    async fn vendor_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM vendor_register WHERE id=? AND org_id=?")
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
        let r = sqlx::query("INSERT INTO incident_logs (org_id, title, severity) VALUES (?,?,?)")
            .bind(org)
            .bind(title)
            .bind(severity)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn incident_list(&self, org: &str) -> Result<Vec<IncidentRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, title, severity, status FROM incident_logs WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| IncidentRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                title: x.get::<String, _>("title"),
                severity: x.get::<String, _>("severity"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn incident_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "investigating" | "resolved" | "closed") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM incident_logs WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("open"), "investigating")
                | (Some("investigating"), "resolved")
                | (Some("resolved"), "closed")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE incident_logs SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn incident_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM incident_logs WHERE id=? AND org_id=?")
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
        let r =
            sqlx::query("INSERT INTO capa_items (org_id, title, cause, action) VALUES (?,?,?,?)")
                .bind(org)
                .bind(title)
                .bind(cause.trim())
                .bind(action.trim())
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn capa_list(&self, org: &str) -> Result<Vec<CapaRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, title, cause, action, status FROM capa_items WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| CapaRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                title: x.get::<String, _>("title"),
                cause: x.get::<String, _>("cause"),
                action: x.get::<String, _>("action"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn capa_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "in_progress" | "done" | "verified") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM capa_items WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("open"), "in_progress")
                | (Some("in_progress"), "done")
                | (Some("done"), "verified")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE capa_items SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn capa_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM capa_items WHERE id=? AND org_id=?")
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
        let src: String = source.trim().chars().take(120).collect();
        let r = sqlx::query(
            "INSERT INTO regulatory_changes (org_id, title, source, effective_date) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(title)
        .bind(src)
        .bind(effective.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn regchange_list(&self, org: &str) -> Result<Vec<RegChangeRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, title, source, effective_date, status FROM regulatory_changes WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| RegChangeRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                title: x.get::<String, _>("title"),
                source: x.get::<String, _>("source"),
                effective_date: x.get::<String, _>("effective_date"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn regchange_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "acknowledged" | "implemented") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM regulatory_changes WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("open"), "acknowledged") | (Some("acknowledged"), "implemented")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE regulatory_changes SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn regchange_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM regulatory_changes WHERE id=? AND org_id=?")
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
        let who: String = assignee.trim().chars().take(120).collect();
        let r = sqlx::query(
            "INSERT INTO training_assignments (org_id, title, assignee, due_date) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(title)
        .bind(who)
        .bind(due.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn training_list(&self, org: &str) -> Result<Vec<TrainingRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, title, assignee, due_date, status FROM training_assignments WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| TrainingRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                title: x.get::<String, _>("title"),
                assignee: x.get::<String, _>("assignee"),
                due_date: x.get::<String, _>("due_date"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn training_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "in_progress" | "completed") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM training_assignments WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("assigned"), "in_progress") | (Some("in_progress"), "completed")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE training_assignments SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn training_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM training_assignments WHERE id=? AND org_id=?")
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
        let cat: String = category.trim().chars().take(60).collect();
        let who: String = owner.trim().chars().take(120).collect();
        let r = sqlx::query(
            "INSERT INTO risk_register (org_id, title, category, likelihood, impact, owner) VALUES (?,?,?,?,?,?)",
        )
        .bind(org)
        .bind(title)
        .bind(cat)
        .bind(likelihood)
        .bind(impact)
        .bind(who)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn risk_list(&self, org: &str) -> Result<Vec<RiskRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, title, category, likelihood, impact, owner, status FROM risk_register WHERE org_id=? ORDER BY (likelihood*impact) DESC, id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| RiskRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                title: x.get::<String, _>("title"),
                category: x.get::<String, _>("category"),
                likelihood: x.get::<i8, _>("likelihood") as i64,
                impact: x.get::<i8, _>("impact") as i64,
                owner: x.get::<String, _>("owner"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn risk_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "mitigating" | "mitigated" | "closed") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM risk_register WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("open"), "mitigating")
                | (Some("mitigating"), "mitigated")
                | (Some("mitigated"), "closed")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE risk_register SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn risk_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM risk_register WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO dsar_requests (org_id, subject, kind, detail, due_date) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(subject)
        .bind(kind)
        .bind(detail.trim())
        .bind(due.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn dsar_list(&self, org: &str) -> Result<Vec<DsarRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, subject, kind, detail, due_date, status FROM dsar_requests WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| DsarRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                subject: x.get::<String, _>("subject"),
                kind: x.get::<String, _>("kind"),
                detail: x.get::<String, _>("detail"),
                due_date: x.get::<String, _>("due_date"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn dsar_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "in_review" | "fulfilled" | "rejected") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM dsar_requests WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("open"), "in_review")
                | (Some("in_review"), "fulfilled")
                | (Some("in_review"), "rejected")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE dsar_requests SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn dsar_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM dsar_requests WHERE id=? AND org_id=?")
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
        let y: String = year.trim().chars().take(4).collect();
        let r = sqlx::query(
            "INSERT INTO ethics_declarations (org_id, declarant, year, statement) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(declarant)
        .bind(y)
        .bind(statement.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn ethics_list(&self, org: &str) -> Result<Vec<EthicsRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, declarant, year, statement, status FROM ethics_declarations WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| EthicsRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                declarant: x.get::<String, _>("declarant"),
                year: x.get::<String, _>("year"),
                statement: x.get::<String, _>("statement"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn ethics_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if to != "acknowledged" {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM ethics_declarations WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        if cur.as_deref() != Some("submitted") {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE ethics_declarations SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn ethics_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM ethics_declarations WHERE id=? AND org_id=?")
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
        let who: String = counterpart.trim().chars().take(120).collect();
        let r = sqlx::query(
            "INSERT INTO gift_register (org_id, direction, description, counterpart, given_at) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(direction)
        .bind(description)
        .bind(who)
        .bind(given_at.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn gift_list(&self, org: &str) -> Result<Vec<GiftRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, direction, description, counterpart, given_at, status FROM gift_register WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| GiftRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                direction: x.get::<String, _>("direction"),
                description: x.get::<String, _>("description"),
                counterpart: x.get::<String, _>("counterpart"),
                given_at: x.get::<String, _>("given_at"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn gift_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "reviewed" | "cleared" | "returned") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM gift_register WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("logged"), "reviewed")
                | (Some("reviewed"), "cleared")
                | (Some("reviewed"), "returned")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE gift_register SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn gift_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM gift_register WHERE id=? AND org_id=?")
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
        let own: Option<String> = sqlx::query("SELECT org_id FROM training_assignments WHERE id=?")
            .bind(training_id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        let r =
            sqlx::query("INSERT INTO training_evidence (org_id, training_id, note) VALUES (?,?,?)")
                .bind(org)
                .bind(training_id as u64)
                .bind(note)
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn tevidence_list(
        &self,
        org: &str,
        training_id: i64,
    ) -> Result<Vec<TrainingEvidenceRow>, StoreError> {
        let own: Option<String> = sqlx::query("SELECT org_id FROM training_assignments WHERE id=?")
            .bind(training_id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        let rows = sqlx::query("SELECT id, org_id, training_id, note FROM training_evidence WHERE org_id=? AND training_id=? ORDER BY id DESC LIMIT 100")
            .bind(org).bind(training_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| TrainingEvidenceRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                training_id: x.get::<u64, _>("training_id") as i64,
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn tevidence_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM training_evidence WHERE id=? AND org_id=?")
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
        let ar: String = area.trim().chars().take(60).collect();
        let r = sqlx::query(
            "INSERT INTO audit_findings (org_id, title, area, severity) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(title)
        .bind(ar)
        .bind(severity)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn afinding_list(&self, org: &str) -> Result<Vec<AuditFindingRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, title, area, severity, status FROM audit_findings WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| AuditFindingRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                title: x.get::<String, _>("title"),
                area: x.get::<String, _>("area"),
                severity: x.get::<String, _>("severity"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn afinding_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "in_remediation" | "closed") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM audit_findings WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("open"), "in_remediation") | (Some("in_remediation"), "closed")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE audit_findings SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn afinding_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM audit_findings WHERE id=? AND org_id=?")
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
        let cat: String = category.trim().chars().take(60).collect();
        let who: String = contact.trim().chars().take(160).collect();
        let r = sqlx::query(
            "INSERT INTO whistle_reports (org_id, channel, category, summary, contact) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(channel)
        .bind(cat)
        .bind(summary)
        .bind(who)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn whistle_list(&self, org: &str) -> Result<Vec<WhistleRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, channel, category, summary, contact, status FROM whistle_reports WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| WhistleRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                channel: x.get::<String, _>("channel"),
                category: x.get::<String, _>("category"),
                summary: x.get::<String, _>("summary"),
                contact: x.get::<String, _>("contact"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn whistle_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "under_review" | "resolved" | "dismissed") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM whistle_reports WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("received"), "under_review")
                | (Some("under_review"), "resolved")
                | (Some("under_review"), "dismissed")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE whistle_reports SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn whistle_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM whistle_reports WHERE id=? AND org_id=?")
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
        let who: String = requested_by.trim().chars().take(120).collect();
        let r = sqlx::query(
            "INSERT INTO exception_waivers (org_id, title, justification, requested_by, expires_at) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(title)
        .bind(justification.trim())
        .bind(who)
        .bind(expires_at.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn waiver_list(&self, org: &str) -> Result<Vec<WaiverRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, title, justification, requested_by, expires_at, status FROM exception_waivers WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| WaiverRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                title: x.get::<String, _>("title"),
                justification: x.get::<String, _>("justification"),
                requested_by: x.get::<String, _>("requested_by"),
                expires_at: x.get::<String, _>("expires_at"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn waiver_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "approved" | "rejected" | "revoked") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM exception_waivers WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("requested"), "approved")
                | (Some("requested"), "rejected")
                | (Some("approved"), "revoked")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE exception_waivers SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn waiver_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM exception_waivers WHERE id=? AND org_id=?")
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
        let ver: String = version.trim().chars().take(20).collect();
        let r = sqlx::query("INSERT INTO policies (org_id, title, version, body) VALUES (?,?,?,?)")
            .bind(org)
            .bind(title)
            .bind(ver)
            .bind(body.trim())
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn policy_list(&self, org: &str) -> Result<Vec<PolicyRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, title, version, body, status FROM policies WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| PolicyRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                title: x.get::<String, _>("title"),
                version: x.get::<String, _>("version"),
                body: x.get::<String, _>("body"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn policy_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "published" | "archived") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM policies WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("draft"), "published") | (Some("published"), "archived")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE policies SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn policy_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM policies WHERE id=? AND org_id=?")
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
    async fn policy_ack(&self, org: &str, policy_id: i64, user: &str) -> Result<i64, StoreError> {
        let cur: Option<String> =
            sqlx::query("SELECT status FROM policies WHERE id=? AND org_id=?")
                .bind(policy_id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        match cur.as_deref() {
            Some("published") => {}
            Some(_) => return Err(StoreError::Conflict("not published".into())),
            None => return Err(StoreError::NotFound),
        }
        let r = sqlx::query("INSERT INTO policy_acks (org_id, policy_id, user_id) VALUES (?,?,?)")
            .bind(org)
            .bind(policy_id as u64)
            .bind(user)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("Duplicate") {
                    StoreError::Conflict("already acked".into())
                } else {
                    db_err(e)
                }
            })?;
        Ok(r.last_insert_id() as i64)
    }
    async fn policy_ack_list(
        &self,
        org: &str,
        policy_id: i64,
    ) -> Result<Vec<PolicyAckRow>, StoreError> {
        let own: Option<String> = sqlx::query("SELECT org_id FROM policies WHERE id=?")
            .bind(policy_id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        let rows = sqlx::query("SELECT id, org_id, policy_id, user_id FROM policy_acks WHERE org_id=? AND policy_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).bind(policy_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| PolicyAckRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                policy_id: x.get::<u64, _>("policy_id") as i64,
                user_id: x.get::<String, _>("user_id"),
            })
            .collect())
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
        let ar: String = area.trim().chars().take(60).collect();
        let r = sqlx::query(
            "INSERT INTO controls (org_id, code, title, area, frequency) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(code)
        .bind(title)
        .bind(ar)
        .bind(frequency)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Duplicate") {
                StoreError::Conflict("duplicate code".into())
            } else {
                db_err(e)
            }
        })?;
        Ok(r.last_insert_id() as i64)
    }
    async fn control_list(&self, org: &str) -> Result<Vec<ControlRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, code, title, area, frequency, active FROM controls WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| ControlRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                code: x.get::<String, _>("code"),
                title: x.get::<String, _>("title"),
                area: x.get::<String, _>("area"),
                frequency: x.get::<String, _>("frequency"),
                active: x.get::<i8, _>("active") != 0,
            })
            .collect())
    }
    async fn control_set_active(&self, org: &str, id: i64, active: bool) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE controls SET active=? WHERE id=? AND org_id=?")
            .bind(if active { 1i8 } else { 0i8 })
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
    async fn control_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM controls WHERE id=? AND org_id=?")
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
        let own: Option<String> = sqlx::query("SELECT org_id FROM controls WHERE id=?")
            .bind(control_id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        let r = sqlx::query(
            "INSERT INTO control_tests (org_id, control_id, tested_at, result, note) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(control_id as u64)
        .bind(tested_at.trim())
        .bind(result)
        .bind(note.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn ctest_list(
        &self,
        org: &str,
        control_id: i64,
    ) -> Result<Vec<ControlTestRow>, StoreError> {
        let own: Option<String> = sqlx::query("SELECT org_id FROM controls WHERE id=?")
            .bind(control_id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        let rows = sqlx::query("SELECT id, org_id, control_id, tested_at, result, note FROM control_tests WHERE org_id=? AND control_id=? ORDER BY id DESC LIMIT 100")
            .bind(org).bind(control_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| ControlTestRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                control_id: x.get::<u64, _>("control_id") as i64,
                tested_at: x.get::<String, _>("tested_at"),
                result: x.get::<String, _>("result"),
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn ctest_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM control_tests WHERE id=? AND org_id=?")
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
        let per: String = period.trim().chars().take(20).collect();
        let sc: String = scope.trim().chars().take(255).collect();
        let r =
            sqlx::query("INSERT INTO audit_plans (org_id, title, period, scope) VALUES (?,?,?,?)")
                .bind(org)
                .bind(title)
                .bind(per)
                .bind(sc)
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn aplan_list(&self, org: &str) -> Result<Vec<AuditPlanRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, title, period, scope, status FROM audit_plans WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| AuditPlanRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                title: x.get::<String, _>("title"),
                period: x.get::<String, _>("period"),
                scope: x.get::<String, _>("scope"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn aplan_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "in_progress" | "completed") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM audit_plans WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("planned"), "in_progress") | (Some("in_progress"), "completed")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE audit_plans SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn aplan_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM audit_plans WHERE id=? AND org_id=?")
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
    async fn clist_add(&self, org: &str, matter_id: i64, item: &str) -> Result<i64, StoreError> {
        let item = item.trim();
        if item.is_empty() || item.len() > 200 {
            return Err(StoreError::Conflict("item".into()));
        }
        let r = sqlx::query(
            "INSERT INTO matter_checklist_items (org_id, matter_id, item) VALUES (?,?,?)",
        )
        .bind(org)
        .bind(matter_id as u64)
        .bind(item)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn clist_list(&self, org: &str, matter_id: i64) -> Result<Vec<ChecklistRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, matter_id, item, done FROM matter_checklist_items WHERE org_id=? AND matter_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).bind(matter_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| ChecklistRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                matter_id: x.get::<u64, _>("matter_id") as i64,
                item: x.get::<String, _>("item"),
                done: x.get::<i8, _>("done") != 0,
            })
            .collect())
    }
    async fn clist_toggle(&self, org: &str, id: i64, done: bool) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE matter_checklist_items SET done=? WHERE id=? AND org_id=?")
            .bind(if done { 1i8 } else { 0i8 })
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
    async fn clist_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM matter_checklist_items WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO compliance_evidence (org_id, title, register_name, register_id, note) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(title)
        .bind(register_name)
        .bind(register_id as u64)
        .bind(note.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn evidence_list(
        &self,
        org: &str,
        register_name: Option<&str>,
        register_id: Option<i64>,
    ) -> Result<Vec<EvidenceRow>, StoreError> {
        let (sql, has_name, has_id) = match (register_name, register_id) {
            (Some(_), Some(_)) => (
                "SELECT id, org_id, title, register_name, register_id, note FROM compliance_evidence WHERE org_id=? AND register_name=? AND register_id=? ORDER BY id DESC LIMIT 200",
                true, true,
            ),
            (Some(_), None) => (
                "SELECT id, org_id, title, register_name, register_id, note FROM compliance_evidence WHERE org_id=? AND register_name=? ORDER BY id DESC LIMIT 200",
                true, false,
            ),
            _ => (
                "SELECT id, org_id, title, register_name, register_id, note FROM compliance_evidence WHERE org_id=? ORDER BY id DESC LIMIT 200",
                false, false,
            ),
        };
        let mut q = sqlx::query(sql).bind(org);
        if has_name {
            q = q.bind(register_name.unwrap_or(""));
        }
        if has_id {
            q = q.bind(register_id.unwrap_or(0) as u64);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| EvidenceRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                title: x.get::<String, _>("title"),
                register_name: x.get::<String, _>("register_name"),
                register_id: x.get::<u64, _>("register_id") as i64,
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn evidence_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM compliance_evidence WHERE id=? AND org_id=?")
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
    async fn pbook_create(&self, org: &str, book_no: &str, year: &str) -> Result<i64, StoreError> {
        let book_no = book_no.trim();
        if book_no.is_empty() || book_no.len() > 40 {
            return Err(StoreError::Conflict("book_no".into()));
        }
        let y: String = year.trim().chars().take(4).collect();
        let r =
            sqlx::query("INSERT INTO protocol_books (org_id, book_no, book_year) VALUES (?,?,?)")
                .bind(org)
                .bind(book_no)
                .bind(y)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    let msg = e.to_string();
                    if msg.contains("Duplicate") {
                        StoreError::Conflict("duplicate book_no".into())
                    } else {
                        db_err(e)
                    }
                })?;
        Ok(r.last_insert_id() as i64)
    }
    async fn pbook_list(&self, org: &str) -> Result<Vec<PbookRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, book_no, book_year, status FROM protocol_books WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| PbookRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                book_no: x.get::<String, _>("book_no"),
                book_year: x.get::<String, _>("book_year"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn pbook_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "closed" | "sealed") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM protocol_books WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("open"), "closed") | (Some("closed"), "sealed")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE protocol_books SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn pbook_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM protocol_books WHERE id=? AND org_id=?")
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
        let own: Option<String> = sqlx::query("SELECT org_id FROM protocol_books WHERE id=?")
            .bind(book_id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        let fh: String = from_holder.trim().chars().take(120).collect();
        let th: String = to_holder.trim().chars().take(120).collect();
        let r = sqlx::query(
            "INSERT INTO protocol_handovers (org_id, book_id, from_holder, to_holder, handed_at, note) VALUES (?,?,?,?,?,?)",
        )
        .bind(org)
        .bind(book_id as u64)
        .bind(fh)
        .bind(th)
        .bind(handed_at.trim())
        .bind(note.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn phandover_list(
        &self,
        org: &str,
        book_id: i64,
    ) -> Result<Vec<PhandoverRow>, StoreError> {
        let own: Option<String> = sqlx::query("SELECT org_id FROM protocol_books WHERE id=?")
            .bind(book_id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        let rows = sqlx::query("SELECT id, org_id, book_id, from_holder, to_holder, handed_at, note FROM protocol_handovers WHERE org_id=? AND book_id=? ORDER BY id DESC LIMIT 100")
            .bind(org).bind(book_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| PhandoverRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                book_id: x.get::<u64, _>("book_id") as i64,
                from_holder: x.get::<String, _>("from_holder"),
                to_holder: x.get::<String, _>("to_holder"),
                handed_at: x.get::<String, _>("handed_at"),
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn phandover_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM protocol_handovers WHERE id=? AND org_id=?")
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
        if chrono::NaiveDateTime::parse_from_str(scheduled_at.trim(), "%Y-%m-%d %H:%M").is_err() {
            return Err(StoreError::Conflict("scheduled_at YYYY-MM-DD HH:MM".into()));
        }
        let pur: String = purpose.trim().chars().take(255).collect();
        let r = sqlx::query(
            "INSERT INTO appointments (org_id, client_name, purpose, scheduled_at) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(client_name)
        .bind(pur)
        .bind(scheduled_at.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn appt_list(&self, org: &str) -> Result<Vec<AppointmentRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, client_name, purpose, scheduled_at, status FROM appointments WHERE org_id=? ORDER BY scheduled_at ASC, id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| AppointmentRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                client_name: x.get::<String, _>("client_name"),
                purpose: x.get::<String, _>("purpose"),
                scheduled_at: x.get::<String, _>("scheduled_at"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn appt_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "checked_in" | "done" | "cancelled") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM appointments WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("scheduled"), "checked_in")
                | (Some("checked_in"), "done")
                | (Some("scheduled"), "cancelled")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE appointments SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn appt_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM appointments WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO price_items (org_id, code, name, amount_minor) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(code)
        .bind(name)
        .bind(amount_minor)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Duplicate") {
                StoreError::Conflict("duplicate code".into())
            } else {
                db_err(e)
            }
        })?;
        Ok(r.last_insert_id() as i64)
    }
    async fn price_list(&self, org: &str) -> Result<Vec<PriceRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, code, name, amount_minor, active FROM price_items WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| PriceRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                code: x.get::<String, _>("code"),
                name: x.get::<String, _>("name"),
                amount_minor: x.get::<i64, _>("amount_minor"),
                active: x.get::<i8, _>("active") != 0,
            })
            .collect())
    }
    async fn price_set_active(&self, org: &str, id: i64, active: bool) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE price_items SET active=? WHERE id=? AND org_id=?")
            .bind(if active { 1i8 } else { 0i8 })
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
    async fn price_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM price_items WHERE id=? AND org_id=?")
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
        // Snapshot amounts from ACTIVE price codes; whole quote is one transaction.
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let mut total: i64 = 0;
        let mut lines: Vec<(String, i64, i64)> = Vec::with_capacity(items.len());
        for (code, qty) in items {
            if *qty <= 0 || *qty > 1000 {
                return Err(StoreError::Conflict("qty".into()));
            }
            let row: Option<(i64, i8)> = sqlx::query_as(
                "SELECT amount_minor, active FROM price_items WHERE org_id=? AND code=?",
            )
            .bind(org)
            .bind(code)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            match row {
                Some((amt, 1)) => {
                    let line = amt.saturating_mul(*qty);
                    total = total.saturating_add(line);
                    lines.push((code.clone(), *qty, line));
                }
                _ => {
                    return Err(StoreError::Conflict(format!(
                        "unknown/inactive price {code}"
                    )))
                }
            }
        }
        let kind: String = deed_kind.trim().chars().take(60).collect();
        let r = sqlx::query(
            "INSERT INTO quotations (org_id, client_name, deed_kind, total_minor) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(client_name)
        .bind(kind)
        .bind(total)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        let qid = r.last_insert_id() as i64;
        for (code, qty, line) in lines {
            sqlx::query(
                "INSERT INTO quotation_items (org_id, quotation_id, price_code, qty, amount_minor) VALUES (?,?,?,?,?)",
            )
            .bind(org)
            .bind(qid as u64)
            .bind(code)
            .bind(qty)
            .bind(line)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }
        tx.commit().await.map_err(db_err)?;
        Ok(qid)
    }
    async fn quote_list(&self, org: &str) -> Result<Vec<QuotationRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, client_name, deed_kind, total_minor, status FROM quotations WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| QuotationRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                client_name: x.get::<String, _>("client_name"),
                deed_kind: x.get::<String, _>("deed_kind"),
                total_minor: x.get::<i64, _>("total_minor"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn quote_items(
        &self,
        org: &str,
        quotation_id: i64,
    ) -> Result<Vec<QuotationItemRow>, StoreError> {
        let own: Option<String> = sqlx::query("SELECT org_id FROM quotations WHERE id=?")
            .bind(quotation_id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        let rows = sqlx::query("SELECT id, org_id, quotation_id, price_code, qty, amount_minor FROM quotation_items WHERE org_id=? AND quotation_id=? ORDER BY id ASC LIMIT 100")
            .bind(org).bind(quotation_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| QuotationItemRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                quotation_id: x.get::<u64, _>("quotation_id") as i64,
                price_code: x.get::<String, _>("price_code"),
                qty: x.get::<i32, _>("qty") as i64,
                amount_minor: x.get::<i64, _>("amount_minor"),
            })
            .collect())
    }
    async fn quote_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "issued" | "accepted" | "rejected") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM quotations WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("draft"), "issued") | (Some("issued"), "accepted") | (Some("issued"), "rejected")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE quotations SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn quote_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM quotations WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO cash_entries (org_id, kind, amount_minor, note, occurred_on) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(kind)
        .bind(amount_minor)
        .bind(note.trim())
        .bind(occurred_on.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn cash_list(&self, org: &str) -> Result<Vec<CashEntryRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, kind, amount_minor, note, occurred_on FROM cash_entries WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| CashEntryRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                kind: x.get::<String, _>("kind"),
                amount_minor: x.get::<i64, _>("amount_minor"),
                note: x.get::<String, _>("note"),
                occurred_on: x.get::<String, _>("occurred_on"),
            })
            .collect())
    }
    async fn cash_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM cash_entries WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO advances (org_id, holder, amount_minor, purpose) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(holder)
        .bind(amount_minor)
        .bind(purpose.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn advance_list(&self, org: &str) -> Result<Vec<AdvanceRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, holder, amount_minor, purpose, status FROM advances WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| AdvanceRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                holder: x.get::<String, _>("holder"),
                amount_minor: x.get::<i64, _>("amount_minor"),
                purpose: x.get::<String, _>("purpose"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn advance_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "settled" | "cancelled") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM advances WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        if cur.as_deref() != Some("open") {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE advances SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn advance_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM advances WHERE id=? AND org_id=?")
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
        let un: String = unit.trim().chars().take(20).collect();
        let r = sqlx::query(
            "INSERT INTO inventory_items (org_id, code, name, qty, unit, min_stock, kind) VALUES (?,?,?,?,?,?,?)",
        )
        .bind(org)
        .bind(code)
        .bind(name)
        .bind(qty)
        .bind(un)
        .bind(min_stock)
        .bind(kind)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Duplicate") {
                StoreError::Conflict("duplicate code".into())
            } else {
                db_err(e)
            }
        })?;
        Ok(r.last_insert_id() as i64)
    }
    async fn stock_list(
        &self,
        org: &str,
        kind: Option<&str>,
    ) -> Result<Vec<InventoryRow>, StoreError> {
        let (sql, has_kind) = match kind {
            Some(_) => (
                "SELECT id, org_id, kind, code, name, qty, unit, min_stock FROM inventory_items WHERE org_id=? AND kind=? ORDER BY id DESC LIMIT 200",
                true,
            ),
            None => (
                "SELECT id, org_id, kind, code, name, qty, unit, min_stock FROM inventory_items WHERE org_id=? ORDER BY id DESC LIMIT 200",
                false,
            ),
        };
        let mut q = sqlx::query(sql).bind(org);
        if has_kind {
            q = q.bind(kind.unwrap_or(""));
        }
        let rows = q.fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| InventoryRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                kind: x.get::<String, _>("kind"),
                code: x.get::<String, _>("code"),
                name: x.get::<String, _>("name"),
                qty: x.get::<i64, _>("qty"),
                unit: x.get::<String, _>("unit"),
                min_stock: x.get::<i64, _>("min_stock"),
            })
            .collect())
    }
    async fn stock_adjust(&self, org: &str, id: i64, delta: i64) -> Result<i64, StoreError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let cur: Option<i64> =
            sqlx::query("SELECT qty FROM inventory_items WHERE id=? AND org_id=? FOR UPDATE")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<i64, _>("qty"));
        let cur = cur.ok_or(StoreError::NotFound)?;
        let next = cur.saturating_add(delta);
        if next < 0 {
            return Err(StoreError::Conflict("insufficient stock".into()));
        }
        sqlx::query("UPDATE inventory_items SET qty=? WHERE id=? AND org_id=?")
            .bind(next)
            .bind(id as u64)
            .bind(org)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(next)
    }
    async fn stock_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM inventory_items WHERE id=? AND org_id=?")
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
        // Atomic decrement + log in one transaction: no oversell, no orphan log.
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let cur: Option<i64> =
            sqlx::query("SELECT qty FROM inventory_items WHERE id=? AND org_id=? FOR UPDATE")
                .bind(item_id as u64)
                .bind(org)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<i64, _>("qty"));
        let cur = cur.ok_or(StoreError::NotFound)?;
        if cur < qty {
            return Err(StoreError::Conflict("insufficient stock".into()));
        }
        sqlx::query("UPDATE inventory_items SET qty=qty-? WHERE id=? AND org_id=?")
            .bind(qty)
            .bind(item_id as u64)
            .bind(org)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        let r = sqlx::query(
            "INSERT INTO meterai_usage (org_id, item_id, qty, need, used_on) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(item_id as u64)
        .bind(qty)
        .bind(need.trim())
        .bind(used_on.trim())
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        let id = r.last_insert_id() as i64;
        tx.commit().await.map_err(db_err)?;
        Ok(id)
    }
    async fn stock_use_list(&self, org: &str, item_id: i64) -> Result<Vec<UsageRow>, StoreError> {
        let own: Option<String> = sqlx::query("SELECT org_id FROM inventory_items WHERE id=?")
            .bind(item_id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        let rows = sqlx::query("SELECT id, org_id, item_id, qty, need, used_on FROM meterai_usage WHERE org_id=? AND item_id=? ORDER BY id DESC LIMIT 100")
            .bind(org).bind(item_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| UsageRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                item_id: x.get::<u64, _>("item_id") as i64,
                qty: x.get::<i64, _>("qty"),
                need: x.get::<String, _>("need"),
                used_on: x.get::<String, _>("used_on"),
            })
            .collect())
    }
    async fn stock_use_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM meterai_usage WHERE id=? AND org_id=?")
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
        let desc: String = description.trim().chars().take(255).collect();
        let r = sqlx::query(
            "INSERT INTO archive_boxes (org_id, box_code, location, description) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(box_code)
        .bind(location.trim())
        .bind(desc)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Duplicate") {
                StoreError::Conflict("duplicate box_code".into())
            } else {
                db_err(e)
            }
        })?;
        Ok(r.last_insert_id() as i64)
    }
    async fn abox_list(&self, org: &str) -> Result<Vec<ArchiveBoxRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, box_code, location, description FROM archive_boxes WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| ArchiveBoxRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                box_code: x.get::<String, _>("box_code"),
                location: x.get::<String, _>("location"),
                description: x.get::<String, _>("description"),
            })
            .collect())
    }
    async fn abox_move(&self, org: &str, id: i64, location: &str) -> Result<(), StoreError> {
        if location.len() > 120 {
            return Err(StoreError::Conflict("location".into()));
        }
        let n = sqlx::query("UPDATE archive_boxes SET location=? WHERE id=? AND org_id=?")
            .bind(location.trim())
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
    async fn abox_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM archive_boxes WHERE id=? AND org_id=?")
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
        let nd: String = need.trim().chars().take(255).collect();
        let r = sqlx::query("INSERT INTO visitors (org_id, name, need, check_in) VALUES (?,?,?,?)")
            .bind(org)
            .bind(name)
            .bind(nd)
            .bind(check_in.trim())
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn visitor_list(&self, org: &str) -> Result<Vec<VisitorRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, name, need, check_in, check_out, status FROM visitors WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| VisitorRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                name: x.get::<String, _>("name"),
                need: x.get::<String, _>("need"),
                check_in: x.get::<String, _>("check_in"),
                check_out: x.get::<String, _>("check_out"),
                status: x.get::<String, _>("status"),
            })
            .collect())
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
        let cur: Option<String> =
            sqlx::query("SELECT status FROM visitors WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        if cur.as_deref() != Some("in") {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("already out".into())
            });
        }
        sqlx::query("UPDATE visitors SET status='out', check_out=? WHERE id=? AND org_id=?")
            .bind(check_out.trim())
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn visitor_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM visitors WHERE id=? AND org_id=?")
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
        let r =
            sqlx::query("INSERT INTO announcements (org_id, title, body, pinned) VALUES (?,?,?,?)")
                .bind(org)
                .bind(title)
                .bind(body.trim())
                .bind(if pinned { 1i8 } else { 0i8 })
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn announce_list(&self, org: &str) -> Result<Vec<AnnouncementRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, title, body, pinned, active FROM announcements WHERE org_id=? ORDER BY pinned DESC, id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| AnnouncementRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                title: x.get::<String, _>("title"),
                body: x.get::<String, _>("body"),
                pinned: x.get::<i8, _>("pinned") != 0,
                active: x.get::<i8, _>("active") != 0,
            })
            .collect())
    }
    async fn announce_set(
        &self,
        org: &str,
        id: i64,
        pinned: Option<bool>,
        active: Option<bool>,
    ) -> Result<(), StoreError> {
        let own: Option<String> = sqlx::query("SELECT org_id FROM announcements WHERE id=?")
            .bind(id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        if let Some(p) = pinned {
            sqlx::query("UPDATE announcements SET pinned=? WHERE id=? AND org_id=?")
                .bind(if p { 1i8 } else { 0i8 })
                .bind(id as u64)
                .bind(org)
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        }
        if let Some(a) = active {
            sqlx::query("UPDATE announcements SET active=? WHERE id=? AND org_id=?")
                .bind(if a { 1i8 } else { 0i8 })
                .bind(id as u64)
                .bind(org)
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        }
        Ok(())
    }
    async fn announce_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM announcements WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO down_payments (org_id, receipt_no, client_name, matter_id, amount_minor, issued_on) VALUES (?,?,?,?,?,?)",
        )
        .bind(org)
        .bind(receipt_no)
        .bind(client_name)
        .bind(matter_id.map(|v| v as u64))
        .bind(amount_minor)
        .bind(issued_on.trim())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Duplicate") {
                StoreError::Conflict("duplicate receipt_no".into())
            } else {
                db_err(e)
            }
        })?;
        Ok(r.last_insert_id() as i64)
    }
    async fn dp_list(&self, org: &str) -> Result<Vec<DownPaymentRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, receipt_no, client_name, matter_id, amount_minor, issued_on, status FROM down_payments WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| DownPaymentRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                receipt_no: x.get::<String, _>("receipt_no"),
                client_name: x.get::<String, _>("client_name"),
                matter_id: x.get::<Option<u64>, _>("matter_id").map(|v| v as i64),
                amount_minor: x.get::<i64, _>("amount_minor"),
                issued_on: x.get::<String, _>("issued_on"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn dp_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "applied" | "refunded") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM down_payments WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        if cur.as_deref() != Some("issued") {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE down_payments SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn dp_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM down_payments WHERE id=? AND org_id=?")
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
        let r =
            sqlx::query("INSERT INTO witnesses (org_id, name, id_number, phone) VALUES (?,?,?,?)")
                .bind(org)
                .bind(name)
                .bind(id_number.trim())
                .bind(phone.trim())
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn witness_list(&self, org: &str) -> Result<Vec<WitnessRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, name, id_number, phone, active FROM witnesses WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| WitnessRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                name: x.get::<String, _>("name"),
                id_number: x.get::<String, _>("id_number"),
                phone: x.get::<String, _>("phone"),
                active: x.get::<i8, _>("active") != 0,
            })
            .collect())
    }
    async fn witness_set_active(&self, org: &str, id: i64, active: bool) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE witnesses SET active=? WHERE id=? AND org_id=?")
            .bind(if active { 1i8 } else { 0i8 })
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
    async fn witness_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM witnesses WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO rechecks (org_id, subject, method, result, verified_on, note) VALUES (?,?,?,?,?,?)",
        )
        .bind(org)
        .bind(subject)
        .bind(method)
        .bind(result)
        .bind(verified_on.trim())
        .bind(note.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn recheck_list(&self, org: &str) -> Result<Vec<RecheckRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, subject, method, result, verified_on, note FROM rechecks WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| RecheckRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                subject: x.get::<String, _>("subject"),
                method: x.get::<String, _>("method"),
                result: x.get::<String, _>("result"),
                verified_on: x.get::<String, _>("verified_on"),
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn recheck_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM rechecks WHERE id=? AND org_id=?")
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
        // Upsert on (org, weekday): one row per day.
        let cur: Option<u64> =
            sqlx::query("SELECT id FROM office_hours WHERE org_id=? AND weekday=?")
                .bind(org)
                .bind(weekday as u8)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<u64, _>("id"));
        match cur {
            Some(id) => {
                sqlx::query("UPDATE office_hours SET open_time=?, close_time=?, active=? WHERE id=? AND org_id=?")
                    .bind(open_time.trim())
                    .bind(close_time.trim())
                    .bind(if active { 1i8 } else { 0i8 })
                    .bind(id)
                    .bind(org)
                    .execute(&self.pool)
                    .await
                    .map_err(db_err)?;
                Ok(id as i64)
            }
            None => {
                let r = sqlx::query(
                    "INSERT INTO office_hours (org_id, weekday, open_time, close_time, active) VALUES (?,?,?,?,?)",
                )
                .bind(org)
                .bind(weekday as u8)
                .bind(open_time.trim())
                .bind(close_time.trim())
                .bind(if active { 1i8 } else { 0i8 })
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
                Ok(r.last_insert_id() as i64)
            }
        }
    }
    async fn hours_list(&self, org: &str) -> Result<Vec<OfficeHoursRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, weekday, open_time, close_time, active FROM office_hours WHERE org_id=? ORDER BY weekday ASC")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| OfficeHoursRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                weekday: x.get::<u8, _>("weekday") as i64,
                open_time: x.get::<String, _>("open_time"),
                close_time: x.get::<String, _>("close_time"),
                active: x.get::<i8, _>("active") != 0,
            })
            .collect())
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
        let rl: String = role.trim().chars().take(60).collect();
        let r =
            sqlx::query("INSERT INTO roster_members (org_id, name, role, shift) VALUES (?,?,?,?)")
                .bind(org)
                .bind(name)
                .bind(rl)
                .bind(shift)
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn roster_list(&self, org: &str) -> Result<Vec<RosterRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, name, role, shift, active FROM roster_members WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| RosterRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                name: x.get::<String, _>("name"),
                role: x.get::<String, _>("role"),
                shift: x.get::<String, _>("shift"),
                active: x.get::<i8, _>("active") != 0,
            })
            .collect())
    }
    async fn roster_set_active(&self, org: &str, id: i64, active: bool) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE roster_members SET active=? WHERE id=? AND org_id=?")
            .bind(if active { 1i8 } else { 0i8 })
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
    async fn roster_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM roster_members WHERE id=? AND org_id=?")
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
        let sub: String = substitute.trim().chars().take(120).collect();
        let r = sqlx::query(
            "INSERT INTO leaves (org_id, holder, kind, from_date, to_date, substitute) VALUES (?,?,?,?,?,?)",
        )
        .bind(org)
        .bind(holder)
        .bind(kind)
        .bind(from_date.trim())
        .bind(to_date.trim())
        .bind(sub)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn leave_list(&self, org: &str) -> Result<Vec<LeaveRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, holder, kind, from_date, to_date, substitute, status FROM leaves WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| LeaveRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                holder: x.get::<String, _>("holder"),
                kind: x.get::<String, _>("kind"),
                from_date: x.get::<String, _>("from_date"),
                to_date: x.get::<String, _>("to_date"),
                substitute: x.get::<String, _>("substitute"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn leave_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "approved" | "rejected") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> = sqlx::query("SELECT status FROM leaves WHERE id=? AND org_id=?")
            .bind(id as u64)
            .bind(org)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("status"));
        if cur.as_deref() != Some("requested") {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE leaves SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn leave_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM leaves WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO storage_boxes (org_id, box_code, location, capacity) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(box_code)
        .bind(location.trim())
        .bind(capacity)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Duplicate") {
                StoreError::Conflict("duplicate box_code".into())
            } else {
                db_err(e)
            }
        })?;
        Ok(r.last_insert_id() as i64)
    }
    async fn sbox_list(&self, org: &str) -> Result<Vec<StorageBoxRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, box_code, location, capacity, status FROM storage_boxes WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| StorageBoxRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                box_code: x.get::<String, _>("box_code"),
                location: x.get::<String, _>("location"),
                capacity: x.get::<i64, _>("capacity"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn sbox_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "full" | "sealed") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM storage_boxes WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("available"), "full") | (Some("full"), "sealed") | (Some("available"), "sealed")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE storage_boxes SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn sbox_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM storage_boxes WHERE id=? AND org_id=?")
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
        if let Some(bid) = book_id {
            let own: Option<String> = sqlx::query("SELECT org_id FROM protocol_books WHERE id=?")
                .bind(bid as u64)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("org_id"));
            if own.as_deref() != Some(org) {
                return Err(StoreError::NotFound);
            }
        }
        let r = sqlx::query(
            "INSERT INTO mpd_transfers (org_id, book_id, recipient, transfer_date, receipt_no) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(book_id.map(|v| v as u64))
        .bind(recipient)
        .bind(transfer_date.trim())
        .bind(receipt_no.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn mtransfer_list(&self, org: &str) -> Result<Vec<MpdTransferRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, book_id, recipient, transfer_date, receipt_no, status FROM mpd_transfers WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| MpdTransferRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                book_id: x.get::<Option<u64>, _>("book_id").map(|v| v as i64),
                recipient: x.get::<String, _>("recipient"),
                transfer_date: x.get::<String, _>("transfer_date"),
                receipt_no: x.get::<String, _>("receipt_no"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn mtransfer_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if to != "transferred" {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM mpd_transfers WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        if cur.as_deref() != Some("prepared") {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE mpd_transfers SET status='transferred' WHERE id=? AND org_id=?")
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn mtransfer_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM mpd_transfers WHERE id=? AND org_id=?")
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
    async fn dclist_add(&self, org: &str, doc_id: i64, item: &str) -> Result<i64, StoreError> {
        let item = item.trim();
        if item.is_empty() || item.len() > 200 {
            return Err(StoreError::Conflict("item".into()));
        }
        let r =
            sqlx::query("INSERT INTO doc_checklist_items (org_id, doc_id, item) VALUES (?,?,?)")
                .bind(org)
                .bind(doc_id as u64)
                .bind(item)
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn dclist_list(&self, org: &str, doc_id: i64) -> Result<Vec<DocCheckRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, doc_id, item, done FROM doc_checklist_items WHERE org_id=? AND doc_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).bind(doc_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| DocCheckRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                doc_id: x.get::<u64, _>("doc_id") as i64,
                item: x.get::<String, _>("item"),
                done: x.get::<i8, _>("done") != 0,
            })
            .collect())
    }
    async fn dclist_toggle(&self, org: &str, id: i64, done: bool) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE doc_checklist_items SET done=? WHERE id=? AND org_id=?")
            .bind(if done { 1i8 } else { 0i8 })
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
    async fn dclist_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM doc_checklist_items WHERE id=? AND org_id=?")
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
        let r =
            sqlx::query("INSERT INTO office_incidents (org_id, title, severity) VALUES (?,?,?)")
                .bind(org)
                .bind(title)
                .bind(severity)
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn oincident_list(&self, org: &str) -> Result<Vec<OfficeIncidentRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, title, severity, status FROM office_incidents WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| OfficeIncidentRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                title: x.get::<String, _>("title"),
                severity: x.get::<String, _>("severity"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn oincident_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "investigating" | "resolved" | "closed") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM office_incidents WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("open"), "investigating")
                | (Some("investigating"), "resolved")
                | (Some("resolved"), "closed")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE office_incidents SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn oincident_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM office_incidents WHERE id=? AND org_id=?")
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
        let tn: String = tenor.trim().chars().take(40).collect();
        let r = sqlx::query(
            "INSERT INTO bank_facilities (org_id, bank_name, facility, rate_text, tenor) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(bank_name)
        .bind(facility.trim())
        .bind(rate_text.trim())
        .bind(tn)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn bankf_list(&self, org: &str) -> Result<Vec<BankFacilityRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, bank_name, facility, rate_text, tenor, active FROM bank_facilities WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| BankFacilityRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                bank_name: x.get::<String, _>("bank_name"),
                facility: x.get::<String, _>("facility"),
                rate_text: x.get::<String, _>("rate_text"),
                tenor: x.get::<String, _>("tenor"),
                active: x.get::<i8, _>("active") != 0,
            })
            .collect())
    }
    async fn bankf_set_active(&self, org: &str, id: i64, active: bool) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE bank_facilities SET active=? WHERE id=? AND org_id=?")
            .bind(if active { 1i8 } else { 0i8 })
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
    async fn bankf_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM bank_facilities WHERE id=? AND org_id=?")
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
        let nm: String = name.trim().chars().take(120).collect();
        let r =
            sqlx::query("INSERT INTO feedbacks (org_id, name, message, rating) VALUES (?,?,?,?)")
                .bind(org)
                .bind(nm)
                .bind(message)
                .bind(rating as i8)
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn feedback_list(&self, org: &str) -> Result<Vec<FeedbackRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, name, message, rating, status FROM feedbacks WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| FeedbackRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                name: x.get::<String, _>("name"),
                message: x.get::<String, _>("message"),
                rating: x.get::<i8, _>("rating") as i64,
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn feedback_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "reviewed" | "archived") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM feedbacks WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("new"), "reviewed") | (Some("reviewed"), "archived")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE feedbacks SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn feedback_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM feedbacks WHERE id=? AND org_id=?")
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
        let ver: String = version.trim().chars().take(20).collect();
        let r = sqlx::query(
            "INSERT INTO sop_docs (org_id, code, title, version, body) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(code)
        .bind(title)
        .bind(ver)
        .bind(body.trim())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Duplicate") {
                StoreError::Conflict("duplicate code".into())
            } else {
                db_err(e)
            }
        })?;
        Ok(r.last_insert_id() as i64)
    }
    async fn sop_list(&self, org: &str) -> Result<Vec<SopRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, code, title, version, body, status FROM sop_docs WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| SopRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                code: x.get::<String, _>("code"),
                title: x.get::<String, _>("title"),
                version: x.get::<String, _>("version"),
                body: x.get::<String, _>("body"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn sop_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "published" | "archived") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM sop_docs WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("draft"), "published") | (Some("published"), "archived")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE sop_docs SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn sop_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM sop_docs WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO backup_checks (org_id, checked_at, method, result, note) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(checked_at.trim())
        .bind(method)
        .bind(result)
        .bind(note.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn bcheck_list(&self, org: &str) -> Result<Vec<BackupCheckRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, checked_at, method, result, note FROM backup_checks WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| BackupCheckRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                checked_at: x.get::<String, _>("checked_at"),
                method: x.get::<String, _>("method"),
                result: x.get::<String, _>("result"),
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn bcheck_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM backup_checks WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO selfcheck_runs (org_id, period, passed_items, total_items, note) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(period)
        .bind(passed)
        .bind(total)
        .bind(note.trim())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Duplicate") {
                StoreError::Conflict("duplicate period".into())
            } else {
                db_err(e)
            }
        })?;
        Ok(r.last_insert_id() as i64)
    }
    async fn selfcheck_list(&self, org: &str) -> Result<Vec<SelfcheckRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, period, passed_items, total_items, note FROM selfcheck_runs WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| SelfcheckRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                period: x.get::<String, _>("period"),
                passed_items: x.get::<i64, _>("passed_items"),
                total_items: x.get::<i64, _>("total_items"),
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn selfcheck_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM selfcheck_runs WHERE id=? AND org_id=?")
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
    async fn sealy_prepare(&self, org: &str, year: &str, note: &str) -> Result<i64, StoreError> {
        let year = year.trim();
        if year.len() != 4 || year.parse::<u32>().is_err() {
            return Err(StoreError::Conflict("year YYYY".into()));
        }
        if note.len() > 500 {
            return Err(StoreError::Conflict("note".into()));
        }
        let count: i64 =
            sqlx::query("SELECT COUNT(*) AS c FROM protocol_books WHERE org_id=? AND book_year=?")
                .bind(org)
                .bind(year)
                .fetch_one(&self.pool)
                .await
                .map_err(db_err)?
                .get::<i64, _>("c");
        let r = sqlx::query(
            "INSERT INTO year_end_seals (org_id, year, book_count, note) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(year)
        .bind(count)
        .bind(note.trim())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Duplicate") {
                StoreError::Conflict("duplicate year".into())
            } else {
                db_err(e)
            }
        })?;
        Ok(r.last_insert_id() as i64)
    }
    async fn sealy_list(&self, org: &str) -> Result<Vec<YearSealRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, year, book_count, note, status FROM year_end_seals WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| YearSealRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                year: x.get::<String, _>("year"),
                book_count: x.get::<i64, _>("book_count"),
                note: x.get::<String, _>("note"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn sealy_seal(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let cur: Option<String> =
            sqlx::query("SELECT status FROM year_end_seals WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        if cur.as_deref() != Some("prepared") {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE year_end_seals SET status='sealed' WHERE id=? AND org_id=?")
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn sealy_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM year_end_seals WHERE id=? AND org_id=?")
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
        let own: Option<String> = sqlx::query("SELECT org_id FROM ppat_entries WHERE id=?")
            .bind(ppat_id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        // Upsert on (ppat_id, kind): one charge row per kind per deed.
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let cur: Option<u64> =
            sqlx::query("SELECT id FROM deed_charges WHERE org_id=? AND ppat_id=? AND kind=?")
                .bind(org)
                .bind(ppat_id as u64)
                .bind(kind)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<u64, _>("id"));
        let id = match cur {
            Some(id) => {
                sqlx::query("UPDATE deed_charges SET creditor=?, ceiling_minor=?, valid_until=?, note=? WHERE id=?")
                    .bind(creditor)
                    .bind(ceiling_minor)
                    .bind(valid_until.trim())
                    .bind(note.trim())
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_err)?;
                id as i64
            }
            None => {
                let r = sqlx::query(
                    "INSERT INTO deed_charges (org_id, ppat_id, kind, creditor, ceiling_minor, valid_until, note) VALUES (?,?,?,?,?,?,?)",
                )
                .bind(org)
                .bind(ppat_id as u64)
                .bind(kind)
                .bind(creditor)
                .bind(ceiling_minor)
                .bind(valid_until.trim())
                .bind(note.trim())
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
                r.last_insert_id() as i64
            }
        };
        tx.commit().await.map_err(db_err)?;
        Ok(id)
    }
    async fn charge_list(&self, org: &str, ppat_id: i64) -> Result<Vec<DeedChargeRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, ppat_id, kind, creditor, ceiling_minor, valid_until, note FROM deed_charges WHERE org_id=? AND ppat_id=? ORDER BY id DESC LIMIT 20")
            .bind(org).bind(ppat_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| DeedChargeRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                ppat_id: x.get::<u64, _>("ppat_id") as i64,
                kind: x.get::<String, _>("kind"),
                creditor: x.get::<String, _>("creditor"),
                ceiling_minor: x.get::<i64, _>("ceiling_minor"),
                valid_until: x.get::<String, _>("valid_until"),
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn charge_due(&self, org: &str, days: i64) -> Result<Vec<DeedChargeRow>, StoreError> {
        let days = days.clamp(0, 365);
        let horizon = (chrono::Utc::now().naive_utc().date() + chrono::Duration::days(days))
            .format("%Y-%m-%d")
            .to_string();
        let rows = sqlx::query("SELECT id, org_id, ppat_id, kind, creditor, ceiling_minor, valid_until, note FROM deed_charges WHERE org_id=? AND valid_until<>'' AND valid_until<=? ORDER BY valid_until ASC LIMIT 200")
            .bind(org).bind(horizon).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| DeedChargeRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                ppat_id: x.get::<u64, _>("ppat_id") as i64,
                kind: x.get::<String, _>("kind"),
                creditor: x.get::<String, _>("creditor"),
                ceiling_minor: x.get::<i64, _>("ceiling_minor"),
                valid_until: x.get::<String, _>("valid_until"),
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn charge_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM deed_charges WHERE id=? AND org_id=?")
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
        let own: Option<String> = sqlx::query("SELECT org_id FROM ppat_entries WHERE id=?")
            .bind(ppat_id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        let r = sqlx::query(
            "INSERT INTO ppat_parties (org_id, ppat_id, role, name, id_number) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(ppat_id as u64)
        .bind(role)
        .bind(name)
        .bind(id_number.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn pparty_list(&self, org: &str, ppat_id: i64) -> Result<Vec<PpatPartyRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, ppat_id, role, name, id_number, verified FROM ppat_parties WHERE org_id=? AND ppat_id=? ORDER BY id DESC LIMIT 100")
            .bind(org).bind(ppat_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| PpatPartyRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                ppat_id: x.get::<u64, _>("ppat_id") as i64,
                role: x.get::<String, _>("role"),
                name: x.get::<String, _>("name"),
                id_number: x.get::<String, _>("id_number"),
                verified: x.get::<i8, _>("verified") != 0,
            })
            .collect())
    }
    async fn pparty_set_verified(
        &self,
        org: &str,
        id: i64,
        verified: bool,
    ) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE ppat_parties SET verified=? WHERE id=? AND org_id=?")
            .bind(if verified { 1i8 } else { 0i8 })
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
    async fn pparty_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM ppat_parties WHERE id=? AND org_id=?")
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
        let own: Option<String> = sqlx::query("SELECT org_id FROM ppat_entries WHERE id=?")
            .bind(ppat_id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        let rec: String = receipt_no.trim().chars().take(60).collect();
        let r = sqlx::query(
            "INSERT INTO cert_handovers (org_id, ppat_id, handed_to, handed_at, receipt_no) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(ppat_id as u64)
        .bind(handed_to)
        .bind(handed_at.trim())
        .bind(rec)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn chandover_list(
        &self,
        org: &str,
        ppat_id: i64,
    ) -> Result<Vec<CertHandoverRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, ppat_id, handed_to, handed_at, receipt_no, status FROM cert_handovers WHERE org_id=? AND ppat_id=? ORDER BY id DESC LIMIT 100")
            .bind(org).bind(ppat_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| CertHandoverRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                ppat_id: x.get::<u64, _>("ppat_id") as i64,
                handed_to: x.get::<String, _>("handed_to"),
                handed_at: x.get::<String, _>("handed_at"),
                receipt_no: x.get::<String, _>("receipt_no"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn chandover_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if to != "handed" {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM cert_handovers WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        if cur.as_deref() != Some("prepared") {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE cert_handovers SET status='handed' WHERE id=? AND org_id=?")
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn chandover_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM cert_handovers WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO zones (org_id, area_code, area_name, njop_minor) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(area_code)
        .bind(area_name)
        .bind(njop_minor)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Duplicate") {
                StoreError::Conflict("duplicate area_code".into())
            } else {
                db_err(e)
            }
        })?;
        Ok(r.last_insert_id() as i64)
    }
    async fn zone_list(&self, org: &str) -> Result<Vec<ZoneRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, area_code, area_name, njop_minor FROM zones WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| ZoneRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                area_code: x.get::<String, _>("area_code"),
                area_name: x.get::<String, _>("area_name"),
                njop_minor: x.get::<i64, _>("njop_minor"),
            })
            .collect())
    }
    async fn zone_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM zones WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO pbb_checks (org_id, nop, tax_year, result, checked_on, note) VALUES (?,?,?,?,?,?)",
        )
        .bind(org)
        .bind(nop)
        .bind(tax_year)
        .bind(result)
        .bind(checked_on.trim())
        .bind(note.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn pbb_list(&self, org: &str) -> Result<Vec<PbbRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, nop, tax_year, result, checked_on, note FROM pbb_checks WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| PbbRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                nop: x.get::<String, _>("nop"),
                tax_year: x.get::<String, _>("tax_year"),
                result: x.get::<String, _>("result"),
                checked_on: x.get::<String, _>("checked_on"),
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn pbb_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM pbb_checks WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO land_splits (org_id, source_parcel, new_parcels, split_date) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(sp)
        .bind(np)
        .bind(split_date.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn lsplit_list(&self, org: &str) -> Result<Vec<LandSplitRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, source_parcel, new_parcels, split_date, status FROM land_splits WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| LandSplitRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                source_parcel: x.get::<String, _>("source_parcel"),
                new_parcels: x.get::<String, _>("new_parcels"),
                split_date: x.get::<String, _>("split_date"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn lsplit_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "surveyed" | "registered") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM land_splits WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("planned"), "surveyed") | (Some("surveyed"), "registered")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE land_splits SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn lsplit_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM land_splits WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO land_merges (org_id, source_parcels, result_parcel, merge_date) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(sp)
        .bind(rp)
        .bind(merge_date.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn lmerge_list(&self, org: &str) -> Result<Vec<LandMergeRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, source_parcels, result_parcel, merge_date, status FROM land_merges WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| LandMergeRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                source_parcels: x.get::<String, _>("source_parcels"),
                result_parcel: x.get::<String, _>("result_parcel"),
                merge_date: x.get::<String, _>("merge_date"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn lmerge_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "surveyed" | "registered") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM land_merges WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("planned"), "surveyed") | (Some("surveyed"), "registered")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE land_merges SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn lmerge_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM land_merges WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO land_disputes (org_id, parcel_ref, kind, description) VALUES (?,?,?,?)",
        )
        .bind(org)
        .bind(parcel_ref)
        .bind(kind)
        .bind(description)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn ldispute_list(&self, org: &str) -> Result<Vec<LandDisputeRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, parcel_ref, kind, description, status FROM land_disputes WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| LandDisputeRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                parcel_ref: x.get::<String, _>("parcel_ref"),
                kind: x.get::<String, _>("kind"),
                description: x.get::<String, _>("description"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn ldispute_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "in_review" | "cleared") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM land_disputes WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("flagged"), "in_review") | (Some("in_review"), "cleared")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE land_disputes SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn ldispute_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM land_disputes WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO boundary_checks (org_id, parcel_ref, declared_area, measured_area, result, checked_on) VALUES (?,?,?,?,?,?)",
        )
        .bind(org)
        .bind(parcel_ref)
        .bind(declared_area)
        .bind(measured_area)
        .bind(result)
        .bind(checked_on.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn bndry_list(&self, org: &str) -> Result<Vec<BoundaryRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, parcel_ref, declared_area, measured_area, result, checked_on FROM boundary_checks WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| BoundaryRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                parcel_ref: x.get::<String, _>("parcel_ref"),
                declared_area: x.get::<i64, _>("declared_area"),
                measured_area: x.get::<i64, _>("measured_area"),
                result: x.get::<String, _>("result"),
                checked_on: x.get::<String, _>("checked_on"),
            })
            .collect())
    }
    async fn bndry_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM boundary_checks WHERE id=? AND org_id=?")
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
        let own: Option<String> = sqlx::query("SELECT org_id FROM ppat_entries WHERE id=?")
            .bind(ppat_id as u64)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .map(|x| x.get::<String, _>("org_id"));
        if own.as_deref() != Some(org) {
            return Err(StoreError::NotFound);
        }
        let r = sqlx::query(
            "INSERT INTO deed_consents (org_id, ppat_id, kind, party, note) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(ppat_id as u64)
        .bind(kind)
        .bind(party)
        .bind(note.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn dconsent_list(
        &self,
        org: &str,
        ppat_id: i64,
    ) -> Result<Vec<DeedConsentRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, ppat_id, kind, party, verified, note FROM deed_consents WHERE org_id=? AND ppat_id=? ORDER BY id DESC LIMIT 50")
            .bind(org).bind(ppat_id as u64).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| DeedConsentRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                ppat_id: x.get::<u64, _>("ppat_id") as i64,
                kind: x.get::<String, _>("kind"),
                party: x.get::<String, _>("party"),
                verified: x.get::<u8, _>("verified") != 0,
                note: x.get::<String, _>("note"),
            })
            .collect())
    }
    async fn dconsent_verify(&self, org: &str, id: i64, verified: bool) -> Result<(), StoreError> {
        let n = sqlx::query("UPDATE deed_consents SET verified=? WHERE id=? AND org_id=?")
            .bind(if verified { 1u8 } else { 0u8 })
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
    async fn dconsent_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM deed_consents WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO land_history (org_id, parcel_ref, event, detail, event_date) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(parcel_ref)
        .bind(event)
        .bind(detail)
        .bind(event_date.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn lhist_list(
        &self,
        org: &str,
        parcel_ref: &str,
    ) -> Result<Vec<LandHistoryRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, parcel_ref, event, detail, event_date FROM land_history WHERE org_id=? AND parcel_ref=? ORDER BY id DESC LIMIT 200")
            .bind(org).bind(parcel_ref).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| LandHistoryRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                parcel_ref: x.get::<String, _>("parcel_ref"),
                event: x.get::<String, _>("event"),
                detail: x.get::<String, _>("detail"),
                event_date: x.get::<String, _>("event_date"),
            })
            .collect())
    }
    async fn lhist_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM land_history WHERE id=? AND org_id=?")
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
        let r = sqlx::query(
            "INSERT INTO unit_handovers (org_id, unit_ref, handed_from, handed_to, bast_no, handed_on) VALUES (?,?,?,?,?,?)",
        )
        .bind(org)
        .bind(unit_ref)
        .bind(handed_from.trim())
        .bind(handed_to.trim())
        .bind(bast_no.trim())
        .bind(handed_on.trim())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn uho_list(&self, org: &str) -> Result<Vec<UnitHandoverRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, unit_ref, handed_from, handed_to, bast_no, handed_on, status FROM unit_handovers WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| UnitHandoverRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                unit_ref: x.get::<String, _>("unit_ref"),
                handed_from: x.get::<String, _>("handed_from"),
                handed_to: x.get::<String, _>("handed_to"),
                bast_no: x.get::<String, _>("bast_no"),
                handed_on: x.get::<String, _>("handed_on"),
                status: x.get::<String, _>("status"),
            })
            .collect())
    }
    async fn uho_set_status(&self, org: &str, id: i64, to: &str) -> Result<(), StoreError> {
        if !matches!(to, "signed" | "delivered") {
            return Err(StoreError::Conflict("bad transition".into()));
        }
        let cur: Option<String> =
            sqlx::query("SELECT status FROM unit_handovers WHERE id=? AND org_id=?")
                .bind(id as u64)
                .bind(org)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?
                .map(|x| x.get::<String, _>("status"));
        let ok = matches!(
            (cur.as_deref(), to),
            (Some("draft"), "signed") | (Some("signed"), "delivered")
        );
        if !ok {
            return Err(if cur.is_none() {
                StoreError::NotFound
            } else {
                StoreError::Conflict("bad transition".into())
            });
        }
        sqlx::query("UPDATE unit_handovers SET status=? WHERE id=? AND org_id=?")
            .bind(to)
            .bind(id as u64)
            .bind(org)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    async fn uho_delete(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM unit_handovers WHERE id=? AND org_id=?")
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
    async fn notif_pref_list(&self, user: &str) -> Result<Vec<(String, bool)>, StoreError> {
        let rows = sqlx::query(
            "SELECT kind, enabled FROM notification_prefs WHERE user_id=? ORDER BY kind",
        )
        .bind(user)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| {
                let en: i8 = x.get("enabled");
                (x.get::<String, _>("kind"), en != 0)
            })
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
        let r = sqlx::query(
            "INSERT INTO org_invites (org_id, email, role, token_hash, expires_at) VALUES (?,?,?,?,?)",
        )
        .bind(org)
        .bind(email)
        .bind(role.trim())
        .bind(hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_id() as i64)
    }
    async fn invite_list(&self, org: &str) -> Result<Vec<InviteRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, email, role, accepted_at IS NOT NULL AS done FROM org_invites WHERE org_id=? ORDER BY id DESC LIMIT 200")
            .bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| InviteRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                email: x.get::<String, _>("email"),
                role: x.get::<String, _>("role"),
                accepted: x.get::<bool, _>("done"),
            })
            .collect())
    }
    async fn invite_peek(&self, hash: &str) -> Result<Option<InviteRow>, StoreError> {
        let r = sqlx::query(
            "SELECT id, org_id, email, role, accepted_at IS NOT NULL AS done FROM org_invites WHERE token_hash=? AND expires_at > NOW() AND accepted_at IS NULL",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.map(|x| InviteRow {
            id: x.get::<u64, _>("id") as i64,
            org: x.get::<String, _>("org_id"),
            email: x.get::<String, _>("email"),
            role: x.get::<String, _>("role"),
            accepted: x.get::<bool, _>("done"),
        }))
    }
    async fn invite_redeem(&self, hash: &str) -> Result<InviteRow, StoreError> {
        let r = sqlx::query(
            "SELECT id, org_id, email, role, accepted_at IS NOT NULL AS done FROM org_invites WHERE token_hash=? AND expires_at > NOW()",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .ok_or(StoreError::NotFound)?;
        let id: u64 = r.get("id");
        let n = sqlx::query(
            "UPDATE org_invites SET accepted_at=NOW() WHERE id=? AND accepted_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?
        .rows_affected();
        if n == 0 {
            return Err(StoreError::Conflict("already accepted".into()));
        }
        Ok(InviteRow {
            id: id as i64,
            org: r.get::<String, _>("org_id"),
            email: r.get::<String, _>("email"),
            role: r.get::<String, _>("role"),
            accepted: true,
        })
    }
    async fn invite_revoke(&self, org: &str, id: i64) -> Result<(), StoreError> {
        let n = sqlx::query("DELETE FROM org_invites WHERE id=? AND org_id=?")
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
        unread_only: bool,
        limit: i64,
    ) -> Result<Vec<NotifRow>, StoreError> {
        let lim = limit.clamp(1, 100);
        let rows = match (kind, unread_only) {
            (Some(k), true) => sqlx::query("SELECT id, kind, title, UNIX_TIMESTAMP(read_at) AS read_unix FROM notifications WHERE user_id=? AND kind=? AND read_at IS NULL ORDER BY id DESC LIMIT ?").bind(user).bind(k).bind(lim).fetch_all(&self.pool).await.map_err(db_err)?,
            (Some(k), false) => sqlx::query("SELECT id, kind, title, UNIX_TIMESTAMP(read_at) AS read_unix FROM notifications WHERE user_id=? AND kind=? ORDER BY id DESC LIMIT ?").bind(user).bind(k).bind(lim).fetch_all(&self.pool).await.map_err(db_err)?,
            (None, true) => sqlx::query("SELECT id, kind, title, UNIX_TIMESTAMP(read_at) AS read_unix FROM notifications WHERE user_id=? AND read_at IS NULL ORDER BY id DESC LIMIT ?").bind(user).bind(lim).fetch_all(&self.pool).await.map_err(db_err)?,
            (None, false) => sqlx::query("SELECT id, kind, title, UNIX_TIMESTAMP(read_at) AS read_unix FROM notifications WHERE user_id=? ORDER BY id DESC LIMIT ?").bind(user).bind(lim).fetch_all(&self.pool).await.map_err(db_err)?,
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
        actor: Option<&str>,
        limit: i64,
    ) -> Result<Vec<AuditRow>, StoreError> {
        let like = actor.map(|q| format!("%{q}%"));
        let rows = match (entity, like) {
            (Some(e), Some(l)) => sqlx::query("SELECT id, actor_id, action, entity, entity_id, DATE_FORMAT(created_at,'%Y-%m-%d %H:%i:%s') AS created_at FROM audit_events WHERE tenant_id=? AND entity=? AND actor_id LIKE ? ORDER BY id DESC LIMIT ?").bind(org).bind(e).bind(l).bind(limit).fetch_all(&self.pool).await.map_err(db_err)?,
            (Some(e), None) => sqlx::query("SELECT id, actor_id, action, entity, entity_id, DATE_FORMAT(created_at,'%Y-%m-%d %H:%i:%s') AS created_at FROM audit_events WHERE tenant_id=? AND entity=? ORDER BY id DESC LIMIT ?").bind(org).bind(e).bind(limit).fetch_all(&self.pool).await.map_err(db_err)?,
            (None, Some(l)) => sqlx::query("SELECT id, actor_id, action, entity, entity_id, DATE_FORMAT(created_at,'%Y-%m-%d %H:%i:%s') AS created_at FROM audit_events WHERE tenant_id=? AND actor_id LIKE ? ORDER BY id DESC LIMIT ?").bind(org).bind(l).bind(limit).fetch_all(&self.pool).await.map_err(db_err)?,
            (None, None) => sqlx::query("SELECT id, actor_id, action, entity, entity_id, DATE_FORMAT(created_at,'%Y-%m-%d %H:%i:%s') AS created_at FROM audit_events WHERE tenant_id=? ORDER BY id DESC LIMIT ?").bind(org).bind(limit).fetch_all(&self.pool).await.map_err(db_err)?,
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
