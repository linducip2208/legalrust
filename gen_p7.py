#!/usr/bin/env python3
"""Final push part 1: migrations 0005-0007 + OpsStore (trait/memory/mysql)."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust")
def w(rel, text):
    p = R/rel; p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text.strip()+"\n", encoding="utf-8", newline="\n")

w("migrations/0005_ocr.sql", """-- 0005 OCR pipeline
CREATE TABLE ocr_jobs (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, document_id BIGINT UNSIGNED NOT NULL,
  doc_type VARCHAR(40) NOT NULL, status VARCHAR(40) NOT NULL DEFAULT 'queued',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_ocrjob_doc FOREIGN KEY (document_id) REFERENCES documents (id) ON DELETE CASCADE,
  KEY ix_ocrjobs_org (org_id, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE ocr_fields (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  job_id BIGINT UNSIGNED NOT NULL, name VARCHAR(120) NOT NULL,
  value TEXT NOT NULL, confidence FLOAT NOT NULL DEFAULT 0, page INT NOT NULL DEFAULT 1,
  CONSTRAINT fk_ocrfield_job FOREIGN KEY (job_id) REFERENCES ocr_jobs (id) ON DELETE CASCADE,
  KEY ix_ocrfields_job (job_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE ocr_reviews (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  job_id BIGINT UNSIGNED NOT NULL, reviewer_id CHAR(36) NOT NULL,
  decision VARCHAR(20) NOT NULL, note TEXT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_ocrrev_job FOREIGN KEY (job_id) REFERENCES ocr_jobs (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
""")
w("migrations/0006_deeds.sql", """-- 0006 deed templates + rendered deed body
CREATE TABLE deed_templates (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, name VARCHAR(200) NOT NULL, body MEDIUMTEXT NOT NULL,
  version INT NOT NULL DEFAULT 1,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  KEY ix_tpl_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
ALTER TABLE deeds ADD COLUMN rendered MEDIUMTEXT NULL;
""")
w("migrations/0007_ai_finance.sql", """-- 0007 AI conversations/usage, RAG chunks
CREATE TABLE ai_conversations (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, title VARCHAR(200) NOT NULL DEFAULT 'chat',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  KEY ix_conv_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE ai_messages (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  conv_id BIGINT UNSIGNED NOT NULL, role VARCHAR(20) NOT NULL, content MEDIUMTEXT NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_msg_conv FOREIGN KEY (conv_id) REFERENCES ai_conversations (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE ai_usage (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, input_tokens BIGINT NOT NULL DEFAULT 0, output_tokens BIGINT NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  KEY ix_usage_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE rag_chunks (
  doc_id BIGINT UNSIGNED NOT NULL, idx INT NOT NULL, text MEDIUMTEXT NOT NULL,
  PRIMARY KEY (doc_id, idx),
  CONSTRAINT fk_rag_doc FOREIGN KEY (doc_id) REFERENCES documents (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
""")
w("crates/database/src/ops.rs", """use super::store::StoreError;
#[derive(Debug, Clone)]
pub struct OcrJobRow { pub id: i64, pub org: String, pub doc: i64, pub doc_type: String, pub status: String }
#[derive(Debug, Clone)]
pub struct OcrFieldRow { pub name: String, pub value: String, pub confidence: f32, pub page: i64 }
#[derive(Debug, Clone)]
pub struct TemplateRow { pub id: i64, pub org: String, pub name: String, pub body: String, pub version: i64 }
#[derive(Debug, Clone)]
pub struct DeedRow { pub id: i64, pub org: String, pub matter_id: i64, pub number: String, pub deed_type: String, pub state: String }
#[derive(Debug, Clone)]
pub struct InvoiceRow { pub id: i64, pub org: String, pub matter_id: i64, pub number: String, pub total: i64, pub currency: String, pub status: String }
#[derive(Debug, Clone)]
pub struct PaymentRow { pub id: i64, pub invoice_id: i64, pub amount: i64, pub method: String }
#[derive(Debug, Clone)]
pub struct RagHit { pub doc_id: i64, pub idx: i64, pub text: String }
#[async_trait::async_trait]
pub trait OpsStore: Send + Sync {
    async fn ocr_create(&self, org: &str, doc: i64, doc_type: &str) -> Result<i64, StoreError>;
    async fn ocr_list(&self, org: &str) -> Result<Vec<OcrJobRow>, StoreError>;
    async fn ocr_get(&self, org: &str, id: i64) -> Result<(OcrJobRow, Vec<OcrFieldRow>), StoreError>;
    async fn ocr_set_status(&self, org: &str, id: i64, status: &str) -> Result<(), StoreError>;
    async fn ocr_review(&self, org: &str, id: i64, reviewer: &str, decision: &str, note: Option<&str>) -> Result<(), StoreError>;
    async fn tpl_create(&self, org: &str, name: &str, body: &str) -> Result<TemplateRow, StoreError>;
    async fn tpl_list(&self, org: &str) -> Result<Vec<TemplateRow>, StoreError>;
    async fn tpl_get(&self, org: &str, id: i64) -> Result<TemplateRow, StoreError>;
    async fn tpl_update(&self, org: &str, id: i64, body: &str) -> Result<TemplateRow, StoreError>;
    async fn deed_create(&self, org: &str, matter: i64, number: &str, deed_type: &str, rendered: &str) -> Result<DeedRow, StoreError>;
    async fn deed_list(&self, org: &str) -> Result<Vec<DeedRow>, StoreError>;
    async fn deed_set_state(&self, org: &str, id: i64, state: &str) -> Result<DeedRow, StoreError>;
    async fn conv_create(&self, org: &str, title: &str) -> Result<i64, StoreError>;
    async fn msg_add(&self, conv: i64, role: &str, content: &str) -> Result<(), StoreError>;
    async fn usage_add(&self, org: &str, input: i64, output: i64) -> Result<(), StoreError>;
    async fn usage_sum(&self, org: &str) -> Result<(i64, i64), StoreError>;
    async fn rag_put(&self, doc: i64, chunks: &[String]) -> Result<(), StoreError>;
    async fn rag_search(&self, org: &str, q: &str, limit: i64) -> Result<Vec<RagHit>, StoreError>;
    async fn inv_create(&self, org: &str, matter: i64, number: &str, total: i64, currency: &str) -> Result<InvoiceRow, StoreError>;
    async fn inv_list(&self, org: &str, matter: i64) -> Result<Vec<InvoiceRow>, StoreError>;
    async fn inv_get(&self, org: &str, id: i64) -> Result<(InvoiceRow, Vec<PaymentRow>), StoreError>;
    async fn pay_add(&self, org: &str, inv: i64, amount: i64, method: &str) -> Result<PaymentRow, StoreError>;
}
""")
print("P1 OK")
