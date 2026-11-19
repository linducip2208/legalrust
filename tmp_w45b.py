import io

def patch(path, old, new, count=1):
    s = io.open(path, encoding='utf-8', newline='').read()
    assert old in s, 'NOT FOUND in ' + path + ' :: ' + old[:70]
    io.open(path, 'w', encoding='utf-8', newline='').write(s.replace(old, new, count))
    print('patched', path)

patch('docs/AUTONOMOUS/AUDIT-1000/RESUME.md',
      '## Done wave 44: kepatuhan tenggat + postur kepatuhan (main)',
      '## Done wave 44b: hapus halaman /skor publik (main)\n'
      '- /skor + link sidebar + JS dihapus dari tampilan pengguna (khusus legal); API katalog + data audit tetap sebagai backbone bukti.\n\n'
      '## Done wave 44: kepatuhan tenggat + postur kepatuhan (main)',
      1)
patch('docs/AUTONOMOUS/AUDIT-1000/RESUME.md',
      '## Done wave 44b: hapus halaman /skor publik (main)',
      '## Done wave 45: undangan anggota + delegasi e2e (main)\n'
      '- Migration 0025; invite 72 jam; register-with-token; decide e2e via delegasi (member lawyer).\n'
      '- F-D09-003/004 PARTIAL->IMPLEMENTED (78.5). Workspace 337/0; registry 19.9; mean 20.7 (66 v).\n\n'
      '## Done wave 44b: hapus halaman /skor publik (main)',
      1)
