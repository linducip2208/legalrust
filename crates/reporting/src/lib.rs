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
}
