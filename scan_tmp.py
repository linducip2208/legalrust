import json
d = json.load(open('docs/03-FEATURE-CATALOG/feature-catalog.json', encoding='utf-8'))
from collections import defaultdict
g = defaultdict(list)
for x in d:
    g[x['domain']].append(x)
out = []
for dom in sorted(g):
    out.append("== %s %s" % (dom, g[dom][0].get('domain_name')))
    for x in g[dom]:
        out.append(" %s | %s | %s" % (x['id'], x['priority'], x['title']))
open('scan_titles.txt', 'w', encoding='utf-8').write("\n".join(out))
print("wrote", len(out))
