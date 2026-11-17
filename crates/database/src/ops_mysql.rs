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
        })
    }
    async fn deed_list(&self, org: &str) -> Result<Vec<DeedRow>, StoreError> {
        let rows = sqlx::query("SELECT id, org_id, matter_id, number, deed_type, state FROM deeds WHERE org_id=? ORDER BY id DESC LIMIT 200").bind(org).fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|x| DeedRow {
                id: x.get::<u64, _>("id") as i64,
                org: x.get::<String, _>("org_id"),
                matter_id: x.get::<u64, _>("matter_id") as i64,
                number: x.get::<String, _>("number"),
                deed_type: x.get::<String, _>("deed_type"),
                state: x.get::<String, _>("state"),
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
            "SELECT id, org_id, matter_id, number, deed_type, state FROM deeds WHERE id=?",
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
}
