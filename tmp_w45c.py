import io

def patch(path, old, new, count=1):
    s = io.open(path, encoding='utf-8', newline='').read()
    assert old in s, 'NOT FOUND in ' + path + ' :: ' + old[:70]
    io.open(path, 'w', encoding='utf-8', newline='').write(s.replace(old, new, count))
    print('patched', path)

patch('docs/13-development-progress/DAILY-LOG.md',
      '## 2026-11-18 AUDIT-1000 wave 44: tenggat + postur (main)',
      '## 2026-11-18 AUDIT-1000 wave 44b+45: hapus /skor + undangan anggota (main)\n'
      '- /skor dihapus dari UI; invite flow + delegasi e2e; workspace **337 passed, 0 failed**.\n\n'
      '## 2026-11-18 AUDIT-1000 wave 44: tenggat + postur (main)')
patch('docs/13-development-progress/TEST-RESULTS.md',
      'AUDIT-1000 wave 44: tenggat + postur (main)',
      'AUDIT-1000 wave 44b+45: hapus /skor + undangan (main)\n'
      '- Invite e2e + decide + guards + unit; workspace **337 passed, 0 failed**.\n\n'
      '## 2026-11-18 AUDIT-1000 wave 44: tenggat + postur (main)')
