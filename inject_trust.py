#!/usr/bin/env python3
from pathlib import Path
p = Path(r"D:\project laravel\legalrust\crates\database\src\ops_mysql.rs")
t = p.read_text(encoding="utf-8")
assert t.rstrip().endswith("}")
body = open(r"D:\project laravel\legalrust\trust_body.rs.txt", encoding="utf-8").read()
t = t.rstrip()
t = t[:t.rfind("\n}")] + "\n" + body + "}\n"
p.write_text(t, encoding="utf-8", newline="\n")
print("MYSQL TRUST OK")
