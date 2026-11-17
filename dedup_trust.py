#!/usr/bin/env python3
from pathlib import Path
p = Path(r"D:\project laravel\legalrust\crates\database\src\ops_mysql.rs")
lines = p.read_text(encoding="utf-8").split("\n")
# delete old trust block: 1-based lines 460..743
assert lines[459].strip().startswith("async fn trust_create_account"), lines[459]
assert lines[742].strip() == "}", lines[742]
del lines[459:743]
t = "\n".join(lines)
# extract post_entry from impl -> free function
start = t.find("    async fn post_entry(")
assert start > 0
# find matching end: the "\n    }\n" after start that closes it (first occurrence at 4-space indent)
end = t.find("\n    }\n", start)
assert end > 0
block = t[start:end + len("\n    }\n")]
t = t[:start] + t[end + len("\n    }\n"):]
# dedent 4 spaces
ded = "\n".join(l[4:] if l.startswith("    ") else l for l in block.strip("\n").split("\n"))
t = t.replace("impl OpsStore for MysqlOps {", ded + "\n\nimpl OpsStore for MysqlOps {", 1)
t = t.replace("Self::post_entry(", "post_entry(")
p.write_text(t, encoding="utf-8", newline="\n")
print("DEDUP OK")
