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
