/* LegalRust public pages — no business logic, only fetch + render glue. */
(function () {
  'use strict';
  var io = new IntersectionObserver(function (es) {
    es.forEach(function (x) { if (x.isIntersecting) x.target.classList.add('in'); });
  }, { threshold: 0.1 });
  document.querySelectorAll('.rv').forEach(function (el) { io.observe(el); });

  function on(id, ev, fn) { var el = document.getElementById(id); if (el) el.addEventListener(ev, fn); }
  function qsa(sel, fn) { document.querySelectorAll(sel).forEach(fn); }

  /* ---- login page ---- */
  var mode = 1;
  window.tab = function (m) {
    mode = m;
    var t1 = document.getElementById('t1'), t2 = document.getElementById('t2');
    if (!t1) return;
    t1.className = m === 1 ? 'btn btn-primary' : 'btn';
    t2.className = m === 2 ? 'btn btn-primary' : 'btn';
    document.getElementById('f_reg').classList.toggle('hidden', m !== 2);
    document.getElementById('go').textContent = m === 1 ? 'Masuk aplikasi' : 'Daftar & masuk';
  };
  on('go', 'click', async function () {
    var e = document.getElementById('err'); e.textContent = '';
    var btn = document.getElementById('go'); btn.disabled = true;
    var url = mode === 1 ? '/api/v1/auth/login' : '/api/v1/auth/register';
    var email = document.getElementById('email').value, pwd = document.getElementById('pwd').value;
    var body = mode === 1 ? { email: email, password: pwd } :
      { org_name: document.getElementById('org').value, email: email, name: document.getElementById('nm').value, password: pwd };
    try {
      var r = await fetch(url, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) });
      var jj = await r.json();
      if (!r.ok) { e.textContent = mode === 1 ? 'Gagal masuk (' + r.status + '). Periksa email & kata sandi.' : 'Gagal daftar (' + r.status + ').'; btn.disabled = false; return; }
      localStorage.setItem('lr_token', jj.token); location.href = '/app';
    } catch (x) { e.textContent = 'Server tidak terjangkau.'; btn.disabled = false; }
  });
  qsa('#t1', function (el) { el.addEventListener('click', function () { window.tab(1); }); });
  qsa('#t2', function (el) { el.addEventListener('click', function () { window.tab(2); }); });
  on('demo', 'click', function () {
    document.getElementById('email').value = 'admin@legalrust.test';
    document.getElementById('pwd').value = 'LegalRust-Admin-2026';
    document.getElementById('err').textContent = 'Akun demo terisi — klik Masuk aplikasi.';
  });

  /* ---- app dashboard ---- */
  if (!document.getElementById('hi')) return;
  var t = localStorage.getItem('lr_token');
  if (!t) { location.href = '/login'; return; }
  var H = { 'Authorization': 'Bearer ' + t };
  async function j(u) {
    var r = await fetch(u, { headers: H });
    if (r.status === 401) { location.href = '/login'; return null; }
    return r.json();
  }
  var TABS = ['ringkas', 'klien', 'dok', 'ocr', 'akta', 'keu', 'mkt', 'pengaturan'];
  var TABLBL = { ringkas: 'ringkasan', klien: 'klien', dok: 'dokumen', ocr: 'ocr', akta: 'akta', keu: 'keuangan', mkt: 'marketing', pengaturan: 'pengaturan' };
  window.apptab = function (x) {
    TABS.forEach(function (k) {
      var el = document.getElementById('p-' + k);
      if (el) el.classList.toggle('hidden', k !== x);
    });
    var lbl = TABLBL[x];
    qsa('#tabs .nav-link', function (a) { a.classList.toggle('active', a.textContent.toLowerCase().indexOf(lbl) !== -1); });
    qsa('.snav', function (a) { a.classList.toggle('active', a.dataset.tab === x); });
  };
  qsa('.snav', function (a) {
    a.addEventListener('click', function (ev) {
      ev.preventDefault();
      window.apptab(a.dataset.tab);
      var f = a.dataset.focus;
      if (f) { var t = document.getElementById(f); if (t) t.focus(); }
      var sb = document.getElementById('sidebar');
      if (sb && window.innerWidth < 960) sb.classList.remove('open');
    });
  });
  document.querySelectorAll('details.nav-dd').forEach(function (d) {
    d.addEventListener('toggle', function () {
      if (!d.open) return;
      document.querySelectorAll('details.nav-dd').forEach(function (o) { if (o !== d) o.open = false; });
    });
  });
qsa('#sbToggle, #sbToggle2', function (b) {
    b.addEventListener('click', function () {
      var sb = document.getElementById('sidebar');
      if (sb) sb.classList.toggle('open');
    });
  });
  qsa('#tabs .nav-link', function (a, i) {
    a.addEventListener('click', function (ev) { ev.preventDefault(); window.apptab(TABS[i]); });
  });
  qsa('.logout', function (a) {
    a.addEventListener('click', async function (ev) {
      ev.preventDefault();
      try { await fetch('/api/v1/auth/logout', { method: 'POST', headers: H }); } catch (x) {}
      localStorage.removeItem('lr_token'); location.href = '/login';
    });
  });
  /* ---- global search: Enter to search, grouped results ---- */
  function hesc(s) { return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;'); }
  on('q', 'keydown', async function (ev) {
    if (ev.key !== 'Enter') return;
    ev.preventDefault();
    var box = document.getElementById('qres');
    var query = document.getElementById('q').value.trim();
    if (query.length < 2) { box.textContent = 'Ketik minimal 2 huruf lalu tekan Enter.'; return; }
    box.textContent = 'Mencari…';
    try {
      var r = await fetch('/api/v1/search?q=' + encodeURIComponent(query), { headers: H });
      if (!r.ok) { box.textContent = 'Pencarian gagal (' + r.status + ').'; return; }
      var d = await r.json();
      var cs = d.clients || [], ms = d.matters || [], ds = d.documents || [];
      if (!cs.length && !ms.length && !ds.length) {
        box.textContent = 'Tidak ada hasil untuk "' + query + '".';
        return;
      }
      var html = '';
      function group(title, items, fmt) {
        html += '<div class="q-group"><strong>' + title + ' (' + items.length + ')</strong>';
        html += items.length
          ? '<ul>' + items.map(function (x) { return '<li><a href="/app">' + fmt(x) + '</a></li>'; }).join('') + '</ul>'
          : '<div class="text-muted">Tidak ada hasil.</div>';
        html += '</div>';
      }
      group('Klien', cs, function (x) { return hesc(x.name); });
      group('Perkara', ms, function (x) { return hesc(x.number + ' — ' + x.title); });
      group('Dokumen', ds, function (x) { return hesc(x.name); });
      box.innerHTML = html;
    } catch (x) { box.textContent = 'Server tidak terjangkau.'; }
  });
  /* ---- dashboard helpers (live data only, no fake series) ---- */
  var DEED_NEXT = { draft: 'collection', collection: 'verification', verification: 'legal_review', legal_review: 'approval', revision: 'legal_review', approval: 'signing', signing: 'registration', registration: 'completion', completion: 'archive' };
  var TASK_NEXT = { open: ['in_progress', 'blocked'], in_progress: ['done', 'blocked'], blocked: ['open'] };
  function esc(s) { return String(s == null ? '' : s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;'); }
  function ppost(u, body) { return fetch(u, { method: 'POST', headers: Object.assign({}, H, { 'Content-Type': 'application/json' }), body: JSON.stringify(body) }); }
  function pmsg(id, t) { var e = document.getElementById(id); if (e) e.textContent = t; }
  function pval(id) { var e = document.getElementById(id); return e ? e.value.trim() : ''; }
  function fmtRp(n) { return 'Rp ' + Number(n || 0).toLocaleString('id-ID'); }
  function setK(id, v) { var e = document.getElementById(id); if (e) e.textContent = v; }
  function badge(st) { return '<span class="badge bg-blue-lt">' + esc(st) + '</span>'; }
  function optSel(list, vf, fn) { return list.map(function (x) { return '<option value="' + x[vf] + '">' + esc(fn(x)) + '</option>'; }).join(''); }
  async function deedAdvance(id, st, errId) {
    var to = DEED_NEXT[st];
    if (!to) { pmsg(errId, 'Status akhir.'); return; }
    var r = await ppost('/api/v1/deeds/' + id + '/transition', { to: to });
    if (!r.ok) { pmsg(errId, 'Gagal (' + r.status + '): transisi ilegal.'); return; }
    location.reload();
  }
  function wireAdv(tbodyId, errId) {
    qsa('#' + tbodyId + ' .adv', function (btn) {
      btn.addEventListener('click', function () { deedAdvance(btn.dataset.id, btn.dataset.st, errId); });
    });
  }
  function renderFunnel(deeds) {
    var box = document.getElementById('funnel');
    if (!box) return;
    var order = ['draft', 'collection', 'verification', 'legal_review', 'revision', 'approval', 'signing', 'registration', 'completion', 'archive'];
    var cnt = {};
    deeds.forEach(function (x) { cnt[x.state] = (cnt[x.state] || 0) + 1; });
    var mx = 1;
    order.forEach(function (s) { if ((cnt[s] || 0) > mx) mx = cnt[s]; });
    var rows = order.map(function (s) {
      var n = cnt[s] || 0;
      var w = n === 0 ? 0 : Math.max(10, Math.round((n / mx) * 100 / 10) * 10);
      return '<div class="fnl-row"><span class="fnl-lab">' + esc(s) + '</span><span class="fnl-track"><span class="fnl-fill fw-' + w + '"></span></span><span class="fnl-n">' + n + '</span></div>';
    }).join('');
    box.innerHTML = rows + '<div class="text-muted small mt-1">Hitungan live dari ' + deeds.length + ' akta.</div>';
  }
  function renderChart(matters, docs, deeds) {
    var box = document.getElementById('chart');
    if (!box) return;
    var cats = {};
    matters.forEach(function (x) { cats[x.status] = (cats[x.status] || 0) + 1; });
    var items = Object.keys(cats).map(function (s) { return { label: 'Perkara ' + s, n: cats[s] }; });
    items.push({ label: 'Dokumen', n: docs.length });
    items.push({ label: 'Akta', n: deeds.length });
    var mx = 1;
    items.forEach(function (it) { if (it.n > mx) mx = it.n; });
    var W = 220, y = 0, Hh = items.length * 34 + 8;
    var svg = '<svg viewBox="0 0 340 ' + Hh + '" role="img" aria-label="Aktivitas perkara, dokumen, akta">';
    items.forEach(function (it) {
      var bw = Math.round((it.n / mx) * W);
      var lab = it.label.length > 20 ? it.label.slice(0, 20) : it.label;
      svg += '<text x="0" y="' + (y + 12) + '" font-size="11" fill="#5A6B8C">' + esc(lab) + '</text>';
      svg += '<rect x="0" y="' + (y + 16) + '" width="' + bw + '" height="10" rx="5" fill="#1D5BD8"></rect>';
      svg += '<text x="' + (bw + 6) + '" y="' + (y + 25) + '" font-size="11" fill="#101C3D">' + it.n + '</text>';
      y += 34;
    });
    box.innerHTML = svg + '</svg><div class="text-muted small">Kategori nyata — tanpa deret waktu fiktif.</div>';
  }
  function renderCal() {
    var box = document.getElementById('cal');
    if (!box) return;
    var now = new Date(new Date().toLocaleString('en-US', { timeZone: 'Asia/Jakarta' }));
    var y = now.getFullYear(), mo = now.getMonth(), today = now.getDate();
    var months = ['Januari', 'Februari', 'Maret', 'April', 'Mei', 'Juni', 'Juli', 'Agustus', 'September', 'Oktober', 'November', 'Desember'];
    var first = new Date(y, mo, 1).getDay();
    var dim = new Date(y, mo + 1, 0).getDate();
    var html = '<div class="cal-head">' + months[mo] + ' ' + y + ' (Asia/Jakarta)</div><div class="cal-grid">';
    ['Min', 'Sen', 'Sel', 'Rab', 'Kam', 'Jum', 'Sab'].forEach(function (d) { html += '<span class="cal-dow">' + d + '</span>'; });
    for (var i = 0; i < first; i++) html += '<span class="cal-d empty"></span>';
    for (var dd = 1; dd <= dim; dd++) html += '<span class="cal-d' + (dd === today ? ' cal-today' : '') + '">' + dd + '</span>';
    box.innerHTML = html + '</div><div class="text-muted small mt-1">Tidak ada item berjadwal — API tugas tidak menyediakan tenggat.</div>';
  }
  (async function () {
    try {
      var me = await j('/api/v1/orgs/me');
      if (!me) return;
      var org = me.org_id || '';
      document.getElementById('hi').textContent = 'Dasbor — ' + String(org).slice(0, 8) + '…';
      var os = document.getElementById('orgsel');
      if (os) os.innerHTML = '<option value="' + esc(org) + '">' + esc(String(org).slice(0, 13)) + '…</option>';
      var who = 'Pengguna ' + String(me.user_id || '?').slice(0, 8);
      var role = (me.roles && me.roles[0]) || 'anggota';
      var un = document.getElementById('uname'); if (un) un.textContent = who;
      var ur = document.getElementById('urole'); if (ur) ur.textContent = role;
      var ini = String(me.user_id || '?').slice(0, 1).toUpperCase();
      var av = document.getElementById('avatar'); if (av) av.textContent = ini;
      var av2 = document.getElementById('avatar2'); if (av2) av2.textContent = ini;
      var onm = document.getElementById('orgname'); if (onm) onm.textContent = 'Org ' + String(org).slice(0, 8) + '…';
      var c = (await j('/api/v1/clients')) || [], m = (await j('/api/v1/matters')) || [],
        d = (await j('/api/v1/documents')) || [], a = (await j('/api/v1/deeds')) || [],
        tr = (await j('/api/v1/trust/accounts')) || [], oj = (await j('/api/v1/ocr/jobs')) || [],
        nt = (await j('/api/v1/notifications')) || [];
      setK('k1', c.length);
      setK('k2', m.length);
      var openTasks = 0, i;
      for (i = 0; i < Math.min(m.length, 20); i++) {
        var ts = await j('/api/v1/matters/' + m[i].id + '/tasks');
        if (ts) openTasks += ts.filter(function (x) { return x.status !== 'done'; }).length;
      }
      setK('k3', openTasks);
      setK('k4', d.length);
      setK('k5', oj.filter(function (x) { return x.status === 'queued'; }).length);
      setK('k6', a.filter(function (x) { return x.state !== 'completion' && x.state !== 'archive'; }).length);
      var invBal = 0, invN = 0, invCap = false, k;
      var mcap = m.slice(0, 50);
      if (m.length > 50) invCap = true;
      var invIds = [];
      for (k = 0; k < mcap.length; k++) {
        var ivs = await j('/api/v1/matters/' + mcap[k].id + '/invoices');
        if (ivs) for (var v = 0; v < ivs.length; v++) invIds.push(ivs[v].id);
      }
      var invShown = invIds.slice(0, 20);
      if (invIds.length > 20) invCap = true;
      for (var qi = 0; qi < invShown.length; qi++) {
        var gg = await j('/api/v1/invoices/' + invShown[qi]);
        if (gg) { invBal += (gg.balance || 0); invN++; }
      }
      var trustSum = tr.reduce(function (s, x) { return s + (x.balance || 0); }, 0);
      setK('k7', fmtRp(invBal) + (invCap ? ' *' : ''));
      setK('k8', fmtRp(trustSum));
      document.getElementById('m').innerHTML = m.map(function (x) {
        return '<tr><td><strong>' + esc(x.number) + '</strong></td><td>' + esc(x.title) + '</td><td>' + badge(x.status) + '</td></tr>';
      }).join('') || '<tr><td colspan="3" class="text-muted">Belum ada perkara.</td></tr>';
      document.getElementById('m-all').innerHTML = m.map(function (x) {
        return '<tr><td>' + x.id + '</td><td>' + esc(x.number) + '</td><td>' + esc(x.title) + '</td><td>' + badge(x.status) + '</td></tr>';
      }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada perkara.</td></tr>';
      document.getElementById('cl').innerHTML = c.map(function (x) {
        return '<tr><td>' + x.id + '</td><td>' + esc(x.display_name) + '</td><td>' + badge(x.kind) + '</td></tr>';
      }).join('') || '<tr><td colspan="3" class="text-muted">Belum ada klien.</td></tr>';
      document.getElementById('dl').innerHTML = d.map(function (x) {
        return '<tr><td>' + x.id + '</td><td>' + esc(x.name) + '</td></tr>';
      }).join('') || '<tr><td colspan="2" class="text-muted">Belum ada dokumen.</td></tr>';
      document.getElementById('dl5').innerHTML = d.slice(-5).reverse().map(function (x) {
        return '<tr><td>' + x.id + '</td><td>' + esc(x.name) + '</td></tr>';
      }).join('') || '<tr><td colspan="2" class="text-muted">Belum ada dokumen.</td></tr>';
      document.getElementById('ocr-recent').innerHTML = oj.slice(-5).reverse().map(function (x) {
        return '<tr><td>' + x.id + '</td><td>' + esc(x.doc_type) + '</td><td>' + badge(x.status) + '</td></tr>';
      }).join('') || '<tr><td colspan="3" class="text-muted">Belum ada job OCR.</td></tr>';
      document.getElementById('ocr-jobs').innerHTML = oj.map(function (x) {
        return '<tr><td>' + x.id + '</td><td>' + esc(x.doc_type) + '</td><td>' + badge(x.status) + '</td><td><button class="btn btn-sm btn-outline-primary ocrv" data-id="' + x.id + '">Lihat</button></td></tr>';
      }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada job OCR.</td></tr>';
      wireOcrView();
      var queue = a.filter(function (x) { return x.state === 'legal_review' || x.state === 'approval'; });
      document.getElementById('al-q').innerHTML = queue.map(function (x) {
        return '<tr><td>' + x.id + '</td><td>' + esc(x.number) + '</td><td>' + badge(x.state) + '</td><td><button class="btn btn-sm btn-outline-primary adv" data-id="' + x.id + '" data-st="' + esc(x.state) + '">Maju</button></td></tr>';
      }).join('') || '<tr><td colspan="4" class="text-muted">Antrean kosong.</td></tr>';
      wireAdv('al-q', 'ae-q');
      document.getElementById('al').innerHTML = a.map(function (x) {
        var nx = DEED_NEXT[x.state];
        return '<tr><td>' + x.id + '</td><td>' + esc(x.number) + '</td><td>' + badge(x.state) + '</td><td>' + (nx ? '<button class="btn btn-sm btn-outline-primary adv" data-id="' + x.id + '" data-st="' + esc(x.state) + '">Maju</button>' : '<span class="text-muted">akhir</span>') + '</td></tr>';
      }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada akta.</td></tr>';
      wireAdv('al', 'ae');
      document.getElementById('tl').innerHTML = tr.map(function (x) {
        return '<tr><td>' + x.id + '</td><td>' + x.client_id + '</td><td>' + fmtRp(x.balance) + '</td></tr>';
      }).join('') || '<tr><td colspan="3" class="text-muted">Belum ada rekening.</td></tr>';
      renderFunnel(a);
      renderChart(m, d, a);
      renderCal();
      var mOpts = '<option value="">Pilih perkara…</option>' + optSel(m, 'id', function (x) { return x.number + ' — ' + x.title; });
      var tm = document.getElementById('task-matter');
      if (tm) { tm.innerHTML = mOpts; if (m[0]) tm.value = m[0].id; }
      var mc = document.getElementById('mat-client');
      if (mc) mc.innerHTML = '<option value="">Tanpa klien</option>' + optSel(c, 'id', function (x) { return x.display_name; });
      var tpls = (await j('/api/v1/deed-templates')) || [];
      var dt = document.getElementById('deed-tpl');
      if (dt) dt.innerHTML = '<option value="">Pilih template…</option>' + optSel(tpls, 'id', function (x) { return x.name + ' v' + x.version; });
      var fs = document.getElementById('fin-sum');
      if (fs) fs.innerHTML = '<div>Trust total: <strong>' + fmtRp(trustSum) + '</strong> (' + tr.length + ' rekening)</div><div>Invoice outstanding: <strong>' + fmtRp(invBal) + '</strong> (' + invN + ' invoice' + (invCap ? ', 20 terakhir' : '') + ')</div>';
      refreshNotifs();
      loadTasks();
    } catch (e) { document.getElementById('err').textContent = 'Gagal memuat: ' + e; }
  })();
  on('showinv', 'click', async function () {
    var v = await j('/api/v1/matters/' + document.getElementById('km').value + '/invoices');
    document.getElementById('il').innerHTML = v.map(function (x) {
      return '<div>#' + x.number + ' — Rp ' + x.total.toLocaleString('id-ID') + '</div>';
    }).join('') || 'Belum ada invoice.';
  });
  on('dotrf', 'click', async function () {
    var r = await fetch('/api/v1/trust/transfer', { method: 'POST', headers: Object.assign({}, H, { 'Content-Type': 'application/json' }), body: JSON.stringify({ from: +document.getElementById('tf').value, to: +document.getElementById('tt').value, amount_minor: +document.getElementById('ta').value }) });
    if (!r.ok) { document.getElementById('te').textContent = 'Gagal (' + r.status + '): saldo tidak cukup?'; return; }
    location.reload();
  });
  on('doup', 'click', async function () {
    var f = document.getElementById('df').files[0];
    if (!f) { document.getElementById('ue').textContent = 'Pilih berkas dulu.'; return; }
    var buf = await f.arrayBuffer();
    var h = await crypto.subtle.digest('SHA-256', buf);
    var sha = [...new Uint8Array(h)].map(function (b) { return b.toString(16).padStart(2, '0'); }).join('');
    var meta = await (await fetch('/api/v1/documents', { method: 'POST', headers: Object.assign({}, H, { 'Content-Type': 'application/json' }), body: JSON.stringify({ name: document.getElementById('dn').value || f.name, mime: f.type || 'application/octet-stream', size: f.size, sha: sha }) })).json();
    var r = await fetch('/api/v1/documents/' + meta.id + '/content', { method: 'PUT', headers: H, body: buf });
    if (!r.ok) { document.getElementById('ue').textContent = 'Gagal unggah (' + r.status + ')'; return; }
    location.reload();
  });

  /* ---- command-center actions (all endpoints live; 501 noted in UI) ---- */
  on('bell', 'click', function () { var l = document.getElementById('bell-list'); if (l) l.classList.toggle('show'); });
  /* ---- notifications dropdown: 5 latest, unread badge, click marks read ---- */
  async function refreshNotifs() {
    var bn = document.getElementById('bell-n');
    var bl = document.getElementById('bell-list');
    if (!bl) return;
    var list = await j('/api/v1/notifications');
    if (!list) { bl.innerHTML = '<div class="text-muted">Gagal memuat.</div>'; return; }
    var unread = list.filter(function (x) { return !x.read; });
    if (bn) bn.textContent = unread.length ? (unread.length > 5 ? '5+' : String(unread.length)) : '';
    bl.innerHTML = list.slice(-5).reverse().map(function (x) {
      var act = x.read ? '<span class="text-muted small">dibaca</span>'
        : '<button class="btn btn-sm btn-outline-primary nread" data-id="' + x.id + '">Tandai dibaca</button>';
      return '<div class="q-group"><strong>' + esc(x.kind) + '</strong><div>' + esc(x.title) + '</div><div class="mt-1">' + act + '</div></div>';
    }).join('') || '<div class="text-muted">Tidak ada notifikasi.</div>';
    qsa('.nread', function (b) {
      b.addEventListener('click', async function (ev) {
        ev.stopPropagation();
        var r = await ppost('/api/v1/notifications/' + b.dataset.id + '/read', {});
        if (!r.ok) return;
        refreshNotifs();
      });
    });
  }
  /* ---- pengaturan: token API, kata sandi, peran ---- */
  async function loadTokens() {
    var box = document.getElementById('tok-list');
    if (!box) return;
    var v = await j('/api/v1/auth/tokens');
    if (!v) { box.innerHTML = '<tr><td colspan="3" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + esc(x.id) + '</td><td>' + esc(x.name) + '</td><td><button class="btn btn-sm btn-outline-danger trev" data-id="' + esc(x.id) + '">Cabut</button></td></tr>';
    }).join('') || '<tr><td colspan="3" class="text-muted">Belum ada token.</td></tr>';
    qsa('.trev', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/auth/tokens/' + encodeURIComponent(b.dataset.id), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('tok-msg', 'Gagal mencabut (' + r.status + ').'); return; }
        pmsg('tok-msg', '');
        loadTokens();
      });
    });
  }
  on('tok-btn', 'click', async function () {
    var nm = pval('tok-name');
    if (nm.length < 2) { pmsg('tok-msg', 'Nama minimal 2 huruf.'); return; }
    var r = await ppost('/api/v1/auth/tokens', { name: nm });
    if (!r.ok) { pmsg('tok-msg', 'Gagal (' + r.status + ').'); return; }
    var v = await r.json();
    pmsg('tok-msg', '');
    var pl = document.getElementById('tok-plain');
    if (pl) { pl.textContent = 'Token (sekali tampil): ' + v.token; pl.classList.remove('hidden'); }
    var ni = document.getElementById('tok-name'); if (ni) ni.value = '';
    loadTokens();
  });
  on('pw-btn', 'click', async function () {
    var r = await ppost('/api/v1/auth/password', { current_password: pval('pw-cur'), new_password: pval('pw-new') });
    if (!r.ok) { pmsg('pw-msg', 'Gagal (' + r.status + '). Periksa sandi lama (min 12).'); return; }
    var a = document.getElementById('pw-cur'); if (a) a.value = '';
    var b = document.getElementById('pw-new'); if (b) b.value = '';
    pmsg('pw-msg', 'Kata sandi diubah.');
  });
  on('role-btn', 'click', async function () {
    var uid = pval('role-uid');
    var sel = document.getElementById('role-sel');
    if (!uid) { pmsg('role-msg', 'Isi UID pengguna manual.'); return; }
    var r = await ppost('/api/v1/users/' + encodeURIComponent(uid) + '/roles', { role: sel ? sel.value : '' });
    if (!r.ok) { pmsg('role-msg', 'Gagal (' + r.status + '). Perlu hak Assign / peran valid.'); return; }
    pmsg('role-msg', 'Peran ditetapkan.');
  });
  on('cli-csv', 'click', async function () {
    var r = await fetch('/api/v1/clients.csv', { headers: H });
    if (!r.ok) { pmsg('cli-msg', 'Gagal ekspor (' + r.status + ').'); return; }
    var blob = await r.blob();
    var a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = 'clients.csv';
    document.body.appendChild(a);
    a.click();
    a.remove();
  });
  loadTokens();
  [['qa-client', 'klien', 'cli-name'], ['qa-matter', 'klien', 'mat-title'], ['qa-upload', 'dok', 'dn'], ['qa-ocr', 'ocr', 'ocr-doc'], ['qa-deed', 'akta', 'deed-number'], ['qa-invoice', 'keu', 'invm-total']].forEach(function (x) {
    on(x[0], 'click', function () { window.apptab(x[1]); var t = document.getElementById(x[2]); if (t) t.focus(); });
  });
  async function loadTasks() {
    var sel = document.getElementById('task-matter');
    var box = document.getElementById('task-list');
    if (!sel || !box) return;
    if (!sel.value) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Pilih perkara dulu.</td></tr>'; return; }
    var ts = await j('/api/v1/matters/' + sel.value + '/tasks');
    if (!ts) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = ts.map(function (t) {
      var nx = TASK_NEXT[t.status] || [];
      return '<tr><td>' + t.id + '</td><td>' + esc(t.title) + '</td><td>' + badge(t.status) + '</td><td>' + nx.map(function (n) { return '<button class="btn btn-sm btn-outline-primary tadv" data-id="' + t.id + '" data-to="' + n + '">' + esc(n) + '</button>'; }).join(' ') + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada tugas.</td></tr>';
    qsa('.tadv', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/tasks/' + b.dataset.id, { status: b.dataset.to });
        if (!r.ok) { pmsg('task-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('task-msg', '');
        loadTasks();
      });
    });
  }
  on('task-matter', 'change', loadTasks);
  on('task-btn', 'click', async function () {
    var mid = pval('task-matter'), title = pval('task-title');
    if (!mid) { pmsg('task-msg', 'Pilih perkara dulu.'); return; }
    if (title.length < 2) { pmsg('task-msg', 'Judul minimal 2 huruf.'); return; }
    var r = await ppost('/api/v1/matters/' + mid + '/tasks', { title: title });
    if (!r.ok) { pmsg('task-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('task-msg', '');
    document.getElementById('task-title').value = '';
    loadTasks();
  });
  on('cli-btn', 'click', async function () {
    var r = await ppost('/api/v1/clients', { display_name: pval('cli-name'), kind: pval('cli-kind') || 'individual' });
    if (!r.ok) { pmsg('cli-msg', 'Gagal (' + r.status + ').'); return; }
    location.reload();
  });
  on('mat-btn', 'click', async function () {
    var cid = pval('mat-client');
    var body = { title: pval('mat-title') };
    if (cid) body.client_id = +cid;
    var r = await ppost('/api/v1/matters', body);
    if (!r.ok) { pmsg('mat-msg', 'Gagal (' + r.status + ').'); return; }
    location.reload();
  });
  on('ocr-btn', 'click', async function () {
    var r = await ppost('/api/v1/ocr/jobs', { document_id: +pval('ocr-doc'), doc_type: pval('ocr-type') });
    if (!r.ok) { pmsg('ocr-msg', 'Gagal (' + r.status + ').'); return; }
    location.reload();
  });
  function wireOcrView() {
    qsa('.ocrv', function (b) {
      b.addEventListener('click', async function () {
        var v = await j('/api/v1/ocr/jobs/' + b.dataset.id);
        var box = document.getElementById('ocr-detail');
        if (!box) return;
        if (!v) { box.textContent = 'Gagal memuat.'; return; }
        var rows = (v.fields || []).map(function (f) { return '<div>' + esc(f.name) + ': <strong>' + esc(f.value) + '</strong> <span class="text-muted">(' + f.confidence + ')</span></div>'; }).join('') || '<div class="text-muted">Belum ada field.</div>';
        box.innerHTML = '<div>Status: ' + badge(v.status) + '</div>' + rows + '<div class="mt-2"><button class="btn btn-sm btn-primary ocrd" data-id="' + v.id + '" data-d="approve">Setujui</button> <button class="btn btn-sm btn-outline-danger ocrd" data-id="' + v.id + '" data-d="reject">Tolak</button> <span class="text-danger" id="ocr-rev-msg"></span></div>';
        qsa('.ocrd', function (x) {
          x.addEventListener('click', async function () {
            var r2 = await ppost('/api/v1/ocr/jobs/' + x.dataset.id + '/review', { decision: x.dataset.d });
            if (!r2.ok) { pmsg('ocr-rev-msg', 'Gagal (' + r2.status + '). Perlu hak Verify.'); return; }
            location.reload();
          });
        });
      });
    });
  }
  on('deed-btn', 'click', async function () {
    var vars = {};
    try { vars = JSON.parse(pval('deed-vars') || '{}'); } catch (e) { pmsg('deed-msg', 'Variabel bukan JSON valid.'); return; }
    var r = await ppost('/api/v1/deeds/from-template', { template_id: +pval('deed-tpl'), matter_id: +pval('deed-matter'), number: pval('deed-number'), deed_type: pval('deed-type'), variables: vars });
    if (!r.ok) { pmsg('deed-msg', 'Gagal (' + r.status + ').'); return; }
    location.reload();
  });
  on('tpl-btn', 'click', async function () {
    var r = await ppost('/api/v1/deed-templates', { name: pval('tpl-name'), body: pval('tpl-body') });
    if (!r.ok) { pmsg('tpl-msg', 'Gagal (' + r.status + ').'); return; }
    location.reload();
  });
  on('invm-btn', 'click', async function () {
    var r = await ppost('/api/v1/matters/' + pval('invm-matter') + '/invoices', { total_minor: +pval('invm-total') });
    if (!r.ok) { pmsg('invm-msg', 'Gagal (' + r.status + ').'); return; }
    location.reload();
  });
  on('sh-btn', 'click', async function () {
    var hrs = parseInt(pval('sh-hours'), 10) || 72;
    var r = await ppost('/api/v1/documents/' + pval('sh-doc') + '/shares', { hours: hrs });
    var box = document.getElementById('sh-list');
    if (!r.ok) { pmsg('sh-msg', 'Gagal (' + r.status + ').'); return; }
    var v = await r.json();
    pmsg('sh-msg', '');
    if (box) box.innerHTML = '<div>Tautan: <a href="' + esc(v.url) + '">' + esc(v.url) + '</a></div>';
  });
  /* ---- marketing admin (RBAC-gated: Permission::Approve; 401 -> /login) ---- */
  var MKT_NEXT = { new: 'contacted', contacted: 'qualified', qualified: 'demo', demo: 'proposal', proposal: 'negotiation', negotiation: 'won' };
  var mktLeadCache = [], mktPlanCache = [], mktArtCache = [];
  function mktErr(m) { var e = document.getElementById('mkt-err'); if (e) e.textContent = m; }
  async function mktFetch(u, opts) {
    opts = opts || {};
    opts.headers = Object.assign({}, H, opts.headers || {});
    var r = await fetch(u, opts);
    if (r.status === 401) { location.href = '/login'; return null; }
    return r;
  }
  function mktPost(u, body) {
    return mktFetch(u, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) });
  }
  async function mktLeads() {
    if (!document.getElementById('mkt-leads')) return;
    var f = document.getElementById('mkt-status-filter');
    var q = f && f.value ? '?status=' + encodeURIComponent(f.value) : '';
    var r = await mktFetch('/api/v1/admin/leads' + q);
    if (!r) return;
    if (!r.ok) { mktErr('Gagal memuat leads (' + r.status + ').'); return; }
    mktErr('');
    mktLeadCache = await r.json();
    document.getElementById('mkt-leads').innerHTML = mktLeadCache.map(function (l) {
      var nx = MKT_NEXT[l.status];
      return '<tr><td>' + l.id + '</td><td>' + hesc(l.name) + '</td><td>' + hesc(l.email) + '</td>'
        + '<td><span class="badge bg-blue-lt">' + hesc(l.status) + '</span></td><td>'
        + (nx ? '<button class="btn btn-sm btn-outline-primary mkt-adv" data-id="' + l.id + '" data-to="' + nx + '">' + nx + '</button> ' : '')
        + '<input class="form-control mkt-assign" data-id="' + l.id + '" placeholder="assignee" value="' + hesc(l.assigned_to || '') + '">'
        + '<details class="mkt-notes"><summary>Catatan</summary><div class="mkt-note-form"><input class="form-control mkt-note" data-id="' + l.id + '" placeholder="tulis catatan"><button class="btn btn-sm btn-secondary mkt-note-btn" data-id="' + l.id + '">Kirim</button></div></details>'
        + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada leads.</td></tr>';
    qsa('.mkt-adv', function (b) {
      b.addEventListener('click', async function () {
        var r2 = await mktFetch('/api/v1/admin/leads/' + b.dataset.id, { method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ status: b.dataset.to }) });
        if (!r2) return;
        if (!r2.ok) { mktErr('Gagal ubah status (' + r2.status + ').'); return; }
        mktLeads();
      });
    });
    qsa('.mkt-assign', function (inp) {
      inp.addEventListener('change', async function () {
        var r2 = await mktPost('/api/v1/admin/leads/' + inp.dataset.id + '/assign', { assignee: inp.value || null });
        if (!r2) return;
        if (!r2.ok) { mktErr('Gagal assign (' + r2.status + ').'); return; }
        mktLeads();
      });
    });
    qsa('.mkt-note-btn', function (b) {
      b.addEventListener('click', async function () {
        var box = document.querySelector('.mkt-note[data-id="' + b.dataset.id + '"]');
        var r2 = await mktPost('/api/v1/admin/leads/' + b.dataset.id + '/notes', { note: box ? box.value : '' });
        if (!r2) return;
        if (!r2.ok) { mktErr('Gagal simpan catatan (' + r2.status + ').'); return; }
        if (box) box.value = '';
        mktErr('Catatan tersimpan.');
      });
    });
  }
  async function mktDemos() {
    if (!document.getElementById('mkt-demos')) return;
    var r = await mktFetch('/api/v1/admin/demo-requests');
    if (!r || !r.ok) return;
    var v = await r.json();
    document.getElementById('mkt-demos').innerHTML = v.map(function (d) {
      return '<tr><td>' + d.id + '</td><td>' + hesc(d.name) + '</td><td>' + hesc(d.email) + '</td><td>' + hesc(d.company || '-') + '</td><td>' + hesc(d.status) + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada permintaan demo.</td></tr>';
  }
  async function mktContacts() {
    if (!document.getElementById('mkt-contacts')) return;
    var r = await mktFetch('/api/v1/admin/contacts');
    if (!r || !r.ok) return;
    var v = await r.json();
    document.getElementById('mkt-contacts').innerHTML = v.map(function (c) {
      return '<tr><td>' + c.id + '</td><td>' + hesc(c.name) + '</td><td>' + hesc(c.subject) + '</td><td>' + hesc(c.status) + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada pesan kontak.</td></tr>';
  }
  async function mktPlans() {
    if (!document.getElementById('mkt-plans')) return;
    var r = await mktFetch('/api/v1/public/pricing');
    if (!r || !r.ok) return;
    mktPlanCache = await r.json();
    document.getElementById('mkt-plans').innerHTML = mktPlanCache.map(function (p, i) {
      return '<tr><td>' + hesc(p.code) + '</td><td>' + hesc(p.name) + '</td><td>Rp ' + (p.monthly_minor || 0).toLocaleString('id-ID') + '</td>'
        + '<td>' + (p.active ? 'aktif' : 'nonaktif') + '</td>'
        + '<td><button class="btn btn-sm btn-outline-primary mkt-plan-toggle" data-i="' + i + '">' + (p.active ? 'Nonaktifkan' : 'Aktifkan') + '</button></td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada paket.</td></tr>';
    qsa('.mkt-plan-toggle', function (b) {
      b.addEventListener('click', async function () {
        var p = mktPlanCache[+b.dataset.i];
        var feats = Array.isArray(p.features) ? p.features : [];
        var r2 = await mktPost('/api/v1/admin/pricing', { code: p.code, name: p.name, monthly_minor: p.monthly_minor, annual_minor: p.annual_minor, features: feats, active: !p.active });
        if (!r2) return;
        if (!r2.ok) { mktErr('Gagal toggle paket (' + r2.status + ').'); return; }
        mktPlans();
      });
    });
  }
  async function mktFaqs() {
    if (!document.getElementById('mkt-faqs')) return;
    var r = await mktFetch('/api/v1/public/faqs');
    if (!r || !r.ok) return;
    var v = await r.json();
    document.getElementById('mkt-faqs').innerHTML = v.map(function (f) {
      return '<tr><td>' + hesc(f.slug) + '</td><td>' + hesc(f.question) + '</td></tr>';
    }).join('') || '<tr><td colspan="2" class="text-muted">Belum ada FAQ.</td></tr>';
  }
  async function mktArticles() {
    if (!document.getElementById('mkt-articles')) return;
    var r = await mktFetch('/api/v1/public/articles?published=0');
    if (!r || !r.ok) return;
    mktArtCache = await r.json();
    document.getElementById('mkt-articles').innerHTML = mktArtCache.map(function (a, i) {
      return '<tr><td>' + hesc(a.slug) + '</td><td>' + hesc(a.title) + '</td><td>' + (a.published ? 'ya' : 'tidak') + '</td>'
        + '<td><button class="btn btn-sm btn-outline-primary mkt-art-toggle" data-i="' + i + '">' + (a.published ? 'Unpublish' : 'Publish') + '</button></td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada artikel.</td></tr>';
    qsa('.mkt-art-toggle', function (b) {
      b.addEventListener('click', async function () {
        var a = mktArtCache[+b.dataset.i];
        var r2 = await mktPost('/api/v1/admin/articles', { slug: a.slug, title: a.title, body_md: a.body_md, published: !a.published });
        if (!r2) return;
        if (!r2.ok) { mktErr('Gagal toggle artikel (' + r2.status + ').'); return; }
        mktArticles();
      });
    });
  }
  function mktRefresh() { mktLeads(); mktDemos(); mktContacts(); mktPlans(); mktFaqs(); mktArticles(); }
  on('mkt-reload', 'click', mktRefresh);
  on('mkt-status-filter', 'change', mktLeads);
  on('mkt-csv', 'click', function () {
    var rows = [['id', 'name', 'email', 'status', 'assigned_to']].concat(mktLeadCache.map(function (l) {
      return [l.id, l.name, l.email, l.status, l.assigned_to || ''];
    }));
    var csv = rows.map(function (r) {
      return r.map(function (v) {
        v = String(v);
        return /[",\n]/.test(v) ? '"' + v.replace(/"/g, '""') + '"' : v;
      }).join(',');
    }).join('\n');
    var a = document.createElement('a');
    a.href = URL.createObjectURL(new Blob([csv], { type: 'text/csv' }));
    a.download = 'leads.csv';
    document.body.appendChild(a);
    a.click();
    a.remove();
  });
  on('mkt-sub-btn', 'click', async function () {
    var em = document.getElementById('mkt-sub-email').value;
    var msg = document.getElementById('mkt-sub-msg');
    var r = await mktPost('/api/v1/public/newsletter/subscribe', { email: em });
    if (!r) return;
    if (!r.ok) { msg.textContent = 'Subscribe gagal (' + r.status + ').'; return; }
    var c = document.getElementById('mkt-news-count');
    c.textContent = String(+c.textContent + 1);
    msg.textContent = 'Berhasil subscribe.';
  });
  on('mkt-faq-add', 'click', async function () {
    var r = await mktPost('/api/v1/admin/faqs', {
      slug: document.getElementById('faq-slug').value,
      question: document.getElementById('faq-q').value,
      answer: document.getElementById('faq-a').value
    });
    if (!r) return;
    if (!r.ok) { mktErr('Gagal tambah FAQ (' + r.status + ').'); return; }
    mktFaqs();
  });
  on('mkt-art-add', 'click', async function () {
    var r = await mktPost('/api/v1/admin/articles', {
      slug: document.getElementById('art-slug').value,
      title: document.getElementById('art-title').value,
      body_md: document.getElementById('art-body').value || '-',
      published: document.getElementById('art-pub').checked
    });
    if (!r) return;
    if (!r.ok) { mktErr('Gagal simpan artikel (' + r.status + ').'); return; }
    mktArticles();
  });
  mktRefresh();
})();
