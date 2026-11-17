#!/usr/bin/env python3
from pathlib import Path
p = Path(r"D:\project laravel\legalrust\crates\database\src\ops_mysql.rs")
t = p.read_text(encoding="utf-8")
t = t.replace('"transfer_out", amount', '"debit", amount')
t = t.replace('"transfer_in", amount', '"credit", amount')
t = t.replace('["receipt", "disbursement", "transfer_in", "transfer_out"]', '["debit", "credit"]')
t = t.replace('kind == "disbursement" || kind == "transfer_out"', 'kind == "debit"')
p.write_text(t, encoding="utf-8", newline="\n")
print("MYSQL KINDS UNIFIED")
