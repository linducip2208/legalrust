pub fn to_csv(rows: &[Vec<String>]) -> String {
    rows.iter()
        .map(|r| {
            r.iter()
                .map(|c| {
                    if c.contains([',', '"', '\n']) {
                        format!("\"{}\"", c.replace('"', "\"\""))
                    } else {
                        c.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn escape(cell: &str) -> String {
    if cell.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterSummaryRow {
    pub id: String,
    pub title: String,
    pub status: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustLedgerEntry {
    pub id: String,
    pub matter_id: String,
    pub amount_cents: i64,
    pub memo: String,
}

pub fn matter_summary_csv(rows: &[MatterSummaryRow]) -> String {
    let mut out = String::from("id,title,status,updated_at");
    for r in rows {
        out.push('\n');
        out.push_str(&escape(&r.id));
        out.push(',');
        out.push_str(&escape(&r.title));
        out.push(',');
        out.push_str(&escape(&r.status));
        out.push(',');
        out.push_str(&escape(&r.updated_at));
    }
    out
}

pub fn trust_ledger_csv(entries: &[TrustLedgerEntry]) -> String {
    let mut out = String::from("id,matter_id,amount_cents,memo");
    for e in entries {
        out.push('\n');
        out.push_str(&escape(&e.id));
        out.push(',');
        out.push_str(&escape(&e.matter_id));
        out.push(',');
        out.push_str(&e.amount_cents.to_string());
        out.push(',');
        out.push_str(&escape(&e.memo));
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArAgingEntry {
    pub id: String,
    pub number: String,
    pub matter_id: String,
    pub total_minor: i64,
    pub paid_minor: i64,
    pub balance_minor: i64,
}

pub fn ar_aging_csv(entries: &[ArAgingEntry]) -> String {
    let mut out = String::from("id,number,matter_id,total_minor,paid_minor,balance_minor");
    for e in entries {
        out.push('\n');
        out.push_str(&escape(&e.id));
        out.push(',');
        out.push_str(&escape(&e.number));
        out.push(',');
        out.push_str(&escape(&e.matter_id));
        out.push(',');
        out.push_str(&e.total_minor.to_string());
        out.push(',');
        out.push_str(&e.paid_minor.to_string());
        out.push(',');
        out.push_str(&e.balance_minor.to_string());
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct OcrAccuracyRow {
    pub doc_type: String,
    pub jobs: i64,
    pub fields: i64,
    pub avg_confidence: f32,
}

/// Group per-job (doc_type, field_count, mean_confidence) samples into
/// per-type accuracy rows (mean of job means, unweighted).
pub fn ocr_accuracy_aggregate(samples: &[(String, i64, f32)]) -> Vec<OcrAccuracyRow> {
    use std::collections::BTreeMap;
    let mut acc: BTreeMap<&str, (i64, i64, f64)> = BTreeMap::new();
    for (t, nf, mc) in samples {
        let e = acc.entry(t.as_str()).or_insert((0, 0, 0.0));
        e.0 += 1;
        e.1 += *nf;
        e.2 += *mc as f64;
    }
    acc.into_iter()
        .map(|(t, (jobs, fields, sum))| OcrAccuracyRow {
            doc_type: t.to_string(),
            jobs,
            fields,
            avg_confidence: if jobs > 0 {
                (sum / jobs as f64) as f32
            } else {
                0.0
            },
        })
        .collect()
}
pub fn ocr_accuracy_csv(rows: &[OcrAccuracyRow]) -> String {
    let mut out = String::from("doc_type,jobs,fields,avg_confidence");
    for r in rows {
        out.push('\n');
        out.push_str(&escape(&r.doc_type));
        out.push(',');
        out.push_str(&r.jobs.to_string());
        out.push(',');
        out.push_str(&r.fields.to_string());
        out.push(',');
        out.push_str(&format!("{:.3}", r.avg_confidence));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quotes_commas() {
        assert_eq!(to_csv(&[vec!["a".into(), "b,c".into()]]), "a,\"b,c\"");
    }
    #[test]
    fn matter_csv_header_and_escaping() {
        let rows = vec![MatterSummaryRow {
            id: "m-1".into(),
            title: "Sale \"Villa\", Bali".into(),
            status: "open".into(),
            updated_at: "2026-01-01".into(),
        }];
        assert_eq!(
            matter_summary_csv(&rows),
            "id,title,status,updated_at\nm-1,\"Sale \"\"Villa\"\", Bali\",open,2026-01-01"
        );
        assert_eq!(matter_summary_csv(&[]), "id,title,status,updated_at");
    }
    #[test]
    fn trust_ledger_csv_header_and_escaping() {
        let entries = vec![TrustLedgerEntry {
            id: "t-1".into(),
            matter_id: "m-1".into(),
            amount_cents: 1500,
            memo: "deposit, \"escrow\"".into(),
        }];
        assert_eq!(
            trust_ledger_csv(&entries),
            "id,matter_id,amount_cents,memo\nt-1,m-1,1500,\"deposit, \"\"escrow\"\"\""
        );
    }
    #[test]
    fn ar_aging_csv_header_and_math() {
        let entries = vec![ArAgingEntry {
            id: "1".into(),
            number: "INV-0001".into(),
            matter_id: "7".into(),
            total_minor: 1000000,
            paid_minor: 400000,
            balance_minor: 600000,
        }];
        assert_eq!(
            ar_aging_csv(&entries),
            "id,number,matter_id,total_minor,paid_minor,balance_minor\n1,INV-0001,7,1000000,400000,600000"
        );
        assert_eq!(
            ar_aging_csv(&[]),
            "id,number,matter_id,total_minor,paid_minor,balance_minor"
        );
    }
    #[test]
    fn ocr_accuracy_csv_header_and_rounding() {
        let rows = vec![
            OcrAccuracyRow {
                doc_type: "ktp".into(),
                jobs: 2,
                fields: 6,
                avg_confidence: 0.91234,
            },
            OcrAccuracyRow {
                doc_type: "general".into(),
                jobs: 1,
                fields: 0,
                avg_confidence: 0.0,
            },
        ];
        assert_eq!(
            ocr_accuracy_csv(&rows),
            "doc_type,jobs,fields,avg_confidence\nktp,2,6,0.912\ngeneral,1,0,0.000"
        );
        assert_eq!(ocr_accuracy_csv(&[]), "doc_type,jobs,fields,avg_confidence");
    }
    #[test]
    fn ocr_accuracy_aggregate_groups_by_type() {
        let out = ocr_accuracy_aggregate(&[
            ("ktp".to_string(), 3, 0.9),
            ("ktp".to_string(), 1, 0.7),
            ("kk".to_string(), 2, 0.5),
        ]);
        assert_eq!(out.len(), 2);
        let kk = out.iter().find(|r| r.doc_type == "kk").unwrap();
        assert_eq!((kk.jobs, kk.fields), (1, 2));
        assert!((kk.avg_confidence - 0.5).abs() < 1e-6);
        let ktp = out.iter().find(|r| r.doc_type == "ktp").unwrap();
        assert_eq!((ktp.jobs, ktp.fields), (2, 4));
        assert!((ktp.avg_confidence - 0.8).abs() < 1e-6);
        assert!(ocr_accuracy_aggregate(&[]).is_empty());
    }
}
