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
  var TABS = ['ringkas', 'klien', 'dok', 'ocr', 'akta', 'keu', 'appr', 'mkt', 'pengaturan', 'audit', 'jobs', 'kepatuhan'];
  var TABLBL = { ringkas: 'ringkasan', klien: 'klien', dok: 'dokumen', ocr: 'ocr', akta: 'akta', keu: 'keuangan', appr: 'persetujuan', mkt: 'marketing', pengaturan: 'pengaturan', audit: 'audit', jobs: 'jobs', kepatuhan: 'kepatuhan' };
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
      if (!a.dataset.tab) return; // plain link: follow href normally
      ev.preventDefault();
      window.apptab(a.dataset.tab);
      var f = a.dataset.focus;
      if (f) { var t = document.getElementById(f); if (t) t.focus(); }
      var sb = document.getElementById('sidebar');
      if (sb && window.innerWidth < 960) sb.classList.remove('open');
    });
  });
  /* ---- sidebar menu search: filter links by keyword ---- */
  on('menu-q', 'input', function () {
    var q = (pval('menu-q') || '').toLowerCase();
    qsa('.side-group', function (g) {
      var links = Array.prototype.slice.call(g.querySelectorAll('.snav'));
      if (!links.length) return;
      var hit = links.filter(function (a) { return a.textContent.toLowerCase().indexOf(q) !== -1; });
      links.forEach(function (a) { a.style.display = (!q || hit.indexOf(a) !== -1) ? '' : 'none'; });
      g.style.display = (!q || hit.length) ? '' : 'none';
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
  /* ---- global search: Enter to search, grouped results + pager ---- */
  function hesc(s) { return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;'); }
  var qpage = 1;
  async function runSearch() {
    var box = document.getElementById('qres');
    var query = document.getElementById('q').value.trim();
    if (query.length < 2) { box.textContent = 'Ketik minimal 2 huruf lalu tekan Enter.'; return; }
    box.textContent = 'Mencari…';
    try {
      var r = await fetch('/api/v1/search?q=' + encodeURIComponent(query) + '&page=' + qpage, { headers: H });
      if (!r.ok) { box.textContent = 'Pencarian gagal (' + r.status + ').'; return; }
      var d = await r.json();
      var cs = d.clients || [], ms = d.matters || [], ds = d.documents || [];
      var totals = d.totals || {};
      if (!cs.length && !ms.length && !ds.length) {
        box.textContent = 'Tidak ada hasil untuk "' + query + '".';
        return;
      }
      var html = '';
      function group(title, items, total, fmt) {
        var shown = total && total > items.length ? items.length + ' dari ' + total : String(items.length);
        html += '<div class="q-group"><strong>' + title + ' (' + shown + ')</strong>';
        html += items.length
          ? '<ul>' + items.map(function (x) { return '<li><a href="/app">' + fmt(x) + '</a></li>'; }).join('') + '</ul>'
          : '<div class="text-muted">Tidak ada hasil.</div>';
        html += '</div>';
      }
      group('Klien', cs, totals.clients, function (x) { return hesc(x.name); });
      group('Perkara', ms, totals.matters, function (x) { return hesc(x.number + ' — ' + x.title); });
      group('Dokumen', ds, totals.documents, function (x) { return hesc(x.name); });
      var anyTotal = (totals.clients || 0) + (totals.matters || 0) + (totals.documents || 0);
      var anyShown = cs.length + ms.length + ds.length;
      if (anyTotal > anyShown) {
        html += '<div class="q-pager"><button class="btn btn-sm" id="qprev" type="button">Sebelumnya</button> <span>Halaman ' + (d.page || qpage) + '</span> <button class="btn btn-sm" id="qnext" type="button">Berikutnya</button></div>';
      }
      box.innerHTML = html;
      on('qprev', 'click', function () { if (qpage > 1) { qpage -= 1; runSearch(); } });
      on('qnext', 'click', function () { qpage += 1; runSearch(); });
    } catch (x) { box.textContent = 'Server tidak terjangkau.'; }
  }
  on('q', 'keydown', async function (ev) {
    if (ev.key !== 'Enter') return;
    ev.preventDefault();
    qpage = 1;
    runSearch();
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
  async function renderCal() {
    var box = document.getElementById('cal');
    if (!box) return;
    var now = new Date(new Date().toLocaleString('en-US', { timeZone: 'Asia/Jakarta' }));
    var y = now.getFullYear(), mo = now.getMonth(), today = now.getDate();
    var months = ['Januari', 'Februari', 'Maret', 'April', 'Mei', 'Juni', 'Juli', 'Agustus', 'September', 'Oktober', 'November', 'Desember'];
    var first = new Date(y, mo, 1).getDay();
    var dim = new Date(y, mo + 1, 0).getDate();
    var dues = await j('/api/v1/tasks/due?limit=100');
    var byDay = {};
    (dues || []).forEach(function (t) {
      if (!t.due) return;
      var m = /^(\d{4})-(\d{2})-(\d{2})/.exec(t.due);
      if (!m || +m[1] !== y || +m[2] !== mo + 1) return;
      var d = +m[3];
      (byDay[d] = byDay[d] || []).push(t);
    });
    var html = '<div class="cal-head">' + months[mo] + ' ' + y + ' (Asia/Jakarta)</div><div class="cal-grid">';
    ['Min', 'Sen', 'Sel', 'Rab', 'Kam', 'Jum', 'Sab'].forEach(function (d) { html += '<span class="cal-dow">' + d + '</span>'; });
    for (var i = 0; i < first; i++) html += '<span class="cal-d empty"></span>';
    for (var dd = 1; dd <= dim; dd++) {
      var mark = byDay[dd] ? ' ●' : '';
      var cls = 'cal-d' + (dd === today ? ' cal-today' : '') + (byDay[dd] ? ' cal-due' : '');
      var tip = byDay[dd] ? ' title="' + esc(byDay[dd].map(function (t) { return t.title; }).join('; ')) + '"' : '';
      html += '<span class="' + cls + '"' + tip + '>' + dd + mark + '</span>';
    }
    var upcoming = (dues || []).filter(function (t) { return t.due && t.status !== 'done'; }).slice(0, 5);
    box.innerHTML = html + '</div><div class="text-muted small mt-1">' + (upcoming.length
      ? 'Tenggat terdekat: ' + upcoming.map(function (t) { return esc(t.title) + ' (' + esc(t.due.slice(0, 16)) + ')'; }).join('; ')
      : 'Tidak ada tenggat terbuka.') + '</div>';
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
        return '<tr><td>' + x.id + '</td><td>' + x.client_id + '</td><td>' + fmtRp(x.balance) + '</td><td><button class="btn btn-sm" data-trust-view="' + x.id + '" type="button">Mutasi</button></td></tr>';
      }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada rekening.</td></tr>';
      qsa('[data-trust-view]', function (b) {
        b.addEventListener('click', function () {
          document.getElementById('trust-view-id').value = b.getAttribute('data-trust-view');
          trustView();
        });
      });
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
    var mid = document.getElementById('km').value;
    if (!mid) { pmsg('il-msg', 'Isi ID perkara dulu.'); return; }
    pmsg('il-msg', '');
    invRefresh(mid);
  });
  async function invRefresh(mid) {
    var box = document.getElementById('il');
    var v = await j('/api/v1/matters/' + mid + '/invoices');
    if (!v) { box.innerHTML = '<p class="text-muted">Gagal memuat.</p>'; return; }
    box.innerHTML = '<div class="table-responsive"><table class="table card-table"><thead><tr><th>Nomor</th><th>Total</th><th>Terbayar</th><th>Sisa</th><th>Bayar</th></tr></thead><tbody>' + v.map(function (x) {
      var paid = x.paid || 0, bal = (x.balance != null ? x.balance : x.total - paid);
      var payForm = bal > 0
        ? '<input class="form-control form-control-sm" style="width:110px;display:inline-block" id="pay-amt-' + x.id + '" placeholder="nominal" inputmode="numeric"> '
        + '<select class="form-control form-control-sm" style="width:auto;display:inline-block" id="pay-method-' + x.id + '"><option value="transfer">transfer</option><option value="cash">cash</option><option value="qris">qris</option></select> '
        + '<button class="btn btn-sm btn-primary" data-pay="' + x.id + '" type="button">Catat bayar</button>'
        : '<span class="text-muted">lunas</span>';
      payForm += ' <button class="btn btn-sm btn-outline-secondary" data-receipt="' + x.id + '" type="button">Kuitansi</button>';
      return '<tr><td>#' + esc(x.number) + '</td><td>' + fmtRp(x.total) + '</td><td>' + fmtRp(paid) + '</td><td>' + fmtRp(bal) + '</td><td>' + payForm + '</td></tr>';
    }).join('') + '</tbody></table></div>' || '<p class="text-muted">Belum ada invoice.</p>';
    qsa('[data-pay]', function (b) {
      b.addEventListener('click', async function () {
        var id = b.getAttribute('data-pay');
        var amt = +(document.getElementById('pay-amt-' + id) || {}).value;
        var msel = document.getElementById('pay-method-' + id);
        if (!amt || amt <= 0) { pmsg('il-msg', 'Isi nominal bayar dulu.'); return; }
        var r = await ppost('/api/v1/invoices/' + id + '/payments', { amount_minor: amt, method: msel ? msel.value : 'transfer' });
        if (!r.ok) { pmsg('il-msg', r.status === 409 ? 'Gagal: nominal melebihi sisa.' : 'Gagal catat bayar (' + r.status + ').'); return; }
        pmsg('il-msg', '');
        invRefresh(document.getElementById('km').value);
      });
    });
    qsa('[data-receipt]', function (b) {
      b.addEventListener('click', async function () {
        var id = b.getAttribute('data-receipt');
        var v = await j('/api/v1/invoices/' + id + '/receipt');
        var box = document.getElementById('il');
        if (!v) { pmsg('il-msg', 'Gagal memuat kuitansi.'); return; }
        pmsg('il-msg', '');
        box.innerHTML = '<div class="card"><div class="card-body" id="receipt-print">'
          + '<h2 class="card-title">Kuitansi #' + esc(v.number) + '</h2>'
          + '<p class="text-muted">Diterbitkan: ' + esc(v.issued_at) + ' · Status: ' + badge(v.status) + '</p>'
          + '<p>Total: <strong>' + fmtRp(v.total) + '</strong> · Terbayar: <strong>' + fmtRp(v.paid) + '</strong> · Sisa: <strong>' + fmtRp(v.balance) + '</strong></p>'
          + '<div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nominal</th><th>Metode</th></tr></thead><tbody>'
          + ((v.payments || []).map(function (p) {
            return '<tr><td>' + p.id + '</td><td>' + fmtRp(p.amount) + '</td><td>' + esc(p.method) + '</td></tr>';
          }).join('') || '<tr><td colspan="3" class="text-muted">Belum ada pembayaran.</td></tr>')
          + '</tbody></table></div>'
          + '<button class="btn btn-sm" id="receipt-back" type="button">Kembali</button> '
          + '<button class="btn btn-sm btn-outline-secondary" id="receipt-print-btn" type="button">Cetak</button></div></div>';
        on('receipt-back', 'click', function () { invRefresh(document.getElementById('km').value); });
        on('receipt-print-btn', 'click', function () { window.print(); });
      });
    });
  }
  on('dotrf', 'click', async function () {
    var r = await fetch('/api/v1/trust/transfer', { method: 'POST', headers: Object.assign({}, H, { 'Content-Type': 'application/json' }), body: JSON.stringify({ from: +document.getElementById('tf').value, to: +document.getElementById('tt').value, amount_minor: +document.getElementById('ta').value }) });
    if (!r.ok) { document.getElementById('te').textContent = 'Gagal (' + r.status + '): saldo tidak cukup?'; return; }
    location.reload();
  });
  /* ---- trust ledger: account detail + manual entries ---- */
  async function trustView() {
    var box = document.getElementById('trust-detail');
    if (!box) return;
    var id = pval('trust-view-id');
    if (!id) { pmsg('trust-view-msg', 'Isi ID rekening dulu.'); return; }
    pmsg('trust-view-msg', '');
    var v = await j('/api/v1/trust/accounts/' + encodeURIComponent(id));
    if (!v) { box.innerHTML = '<p class="text-muted">Gagal memuat (cek ID / hak akses).</p>'; return; }
    box.innerHTML = '<p><strong>Rekening #' + v.id + '</strong> — Saldo: <strong>' + fmtRp(v.balance) + '</strong></p>'
      + '<div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Jenis</th><th>Nominal</th></tr></thead><tbody>'
      + ((v.entries || []).map(function (e) {
        return '<tr><td>' + e.id + '</td><td>' + badge(e.kind) + '</td><td>' + fmtRp(e.amount) + '</td></tr>';
      }).join('') || '<tr><td colspan="3" class="text-muted">Belum ada mutasi.</td></tr>')
      + '</tbody></table></div>';
  }
  on('trust-view', 'click', trustView);
  on('trust-entry-add', 'click', async function () {
    var id = pval('trust-view-id');
    var amt = +pval('trust-entry-amount');
    if (!id) { pmsg('trust-entry-msg', 'Isi ID rekening dulu (lihat mutasi).'); return; }
    if (!amt || amt <= 0) { pmsg('trust-entry-msg', 'Isi nominal minor dulu.'); return; }
    var mid = pval('trust-entry-matter');
    var r = await ppost('/api/v1/trust/accounts/' + encodeURIComponent(id) + '/entries', { kind: pval('trust-entry-kind') || 'credit', matter_id: mid ? +mid : null, amount_minor: amt });
    if (!r.ok) { pmsg('trust-entry-msg', 'Gagal (' + r.status + '). Cek saldo untuk debit.'); return; }
    pmsg('trust-entry-msg', '');
    trustView();
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
  /* ---- notifications dropdown: 5 latest server-side, unread badge ---- */
  async function refreshNotifs() {
    var bn = document.getElementById('bell-n');
    var bl = document.getElementById('bell-list');
    if (!bl) return;
    var list = await j('/api/v1/notifications?limit=5');
    if (!list) { bl.innerHTML = '<div class="text-muted">Gagal memuat.</div>'; return; }
    var unread = await j('/api/v1/notifications?unread_only=true&limit=100');
    var n = unread ? unread.length : list.filter(function (x) { return !x.read; }).length;
    if (bn) bn.textContent = n ? (n > 5 ? '5+' : String(n)) : '';
    bl.innerHTML = list.slice(0, 5).map(function (x) {
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
    /* preference center: per-kind opt-out (F-D18-006) */
    var kinds = ['approval', 'matter', 'invoice', 'ocr', 'ai'];
    var prefs = await j('/api/v1/notifications/prefs');
    var off = {};
    (prefs || []).forEach(function (p) { if (!p.enabled) off[p.kind] = true; });
    bl.innerHTML += '<div class="q-group"><strong>Preferensi</strong>'
      + kinds.map(function (k) {
        return '<label class="form-check"><input class="form-check-input npref" data-kind="' + k + '" type="checkbox"' + (off[k] ? '' : ' checked') + '><span class="form-check-label">' + k + '</span></label>';
      }).join('') + '</div>';
    qsa('.npref', function (c) {
      c.addEventListener('change', async function () {
        await ppost('/api/v1/notifications/prefs', { kind: c.dataset.kind, enabled: !!c.checked });
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
      return '<tr><td>' + esc(x.id) + '</td><td>' + esc(x.name) + '</td><td>' + esc(x.expires_at || '-') + '</td><td><button class="btn btn-sm btn-outline-danger trev" data-id="' + esc(x.id) + '">Cabut</button></td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada token.</td></tr>';
    qsa('.trev', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/auth/tokens/' + encodeURIComponent(b.dataset.id), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('tok-msg', 'Gagal mencabut (' + r.status + ').'); return; }
        pmsg('tok-msg', '');
        loadTokens();
      });
    });
  }
  /* ---- session console (F-D16-019): list + revoke others ---- */
  async function loadSessions() {
    var box = document.getElementById('sess-list');
    if (!box) return;
    var v = await j('/api/v1/auth/sessions');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var st = x.revoked ? 'dicabut' : (x.current ? 'sesi ini' : 'aktif');
      var act = (!x.revoked && !x.current) ? '<button class="btn btn-sm btn-outline-danger srev" data-id="' + esc(x.id) + '">Cabut</button>' : '<span class="text-muted">—</span>';
      return '<tr><td><code>' + esc(x.id.slice(0, 8)) + '…</code></td><td>' + esc(x.expires_at) + '</td><td>' + st + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Tidak ada sesi.</td></tr>';
    qsa('.srev', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/auth/sessions/' + encodeURIComponent(b.dataset.id) + '/revoke', {});
        if (!r.ok) { pmsg('sess-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('sess-msg', '');
        loadSessions();
      });
    });
  }
  on('sess-reload', 'click', loadSessions);
  on('tok-btn', 'click', async function () {
    var nm = pval('tok-name');
    if (nm.length < 2) { pmsg('tok-msg', 'Nama minimal 2 huruf.'); return; }
    var dd = parseInt(pval('tok-days'), 10);
    var body = { name: nm };
    if (!isNaN(dd)) body.expires_in_days = dd;
    var r = await ppost('/api/v1/auth/tokens', body);
    if (!r.ok) { pmsg('tok-msg', 'Gagal (' + r.status + '). Masa 1–365 hari.'); return; }
    var v = await r.json();
    pmsg('tok-msg', '');
    var pl = document.getElementById('tok-plain');
    if (pl) { pl.textContent = 'Token (sekali tampil): ' + v.token + ' — kedaluwarsa ' + (v.expires_at || '-'); pl.classList.remove('hidden'); }
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
  async function loadRoleUsers() {
    var sel = document.getElementById('role-uid');
    if (!sel) return;
    var v = await j('/api/v1/users');
    if (!v) { sel.innerHTML = '<option value="">Gagal memuat (perlu hak Assign).</option>'; return; }
    sel.innerHTML = v.map(function (x) {
      return '<option value="' + esc(x.id) + '">' + esc(x.email) + ' — ' + esc((x.roles || []).join(', ') || 'tanpa peran') + '</option>';
    }).join('') || '<option value="">Tidak ada pengguna.</option>';
  }
  /* ---- SoD conflict check (F-D14-038): advisory, never auto-revokes ---- */
  async function sodRefresh() {
    var box = document.getElementById('sod-list');
    if (!box) return;
    var uid = pval('role-uid');
    if (!uid) { box.innerHTML = '<p class="text-muted">Pilih pengguna dulu.</p>'; return; }
    var v = await j('/api/v1/roles/sod?user=' + encodeURIComponent(uid));
    if (!v) { box.innerHTML = '<p class="text-muted">Gagal memuat (perlu hak Assign).</p>'; return; }
    box.innerHTML = '<p>Hak: ' + v.permissions.map(function (p) { return '<span class="badge">' + esc(p) + '</span>'; }).join(' ') + '</p>'
      + ((v.conflicts || []).length
        ? '<div class="alert alert-warning">Konflik SoD: ' + v.conflicts.map(function (c) { return esc(c.pair.join(' + ')) + ' — ' + esc(c.reason); }).join('; ') + '</div>'
        : '<p class="text-muted">Tidak ada konflik SoD.</p>');
  }
  on('sod-btn', 'click', sodRefresh);
  /* ---- role catalog matrix (least-privilege reference) ---- */
  async function loadRoleMatrix() {
    var box = document.getElementById('role-matrix');
    if (!box) return;
    var v = await j('/api/v1/roles/matrix');
    if (!v) { box.innerHTML = '<tr><td colspan="2" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (r) {
      return '<tr><td><code>' + esc(r.role) + '</code></td><td>' + r.permissions.map(function (p) { return '<span class="badge">' + esc(p) + '</span>'; }).join(' ') + '</td></tr>';
    }).join('');
  }
  on('role-btn', 'click', async function () {
    var uid = pval('role-uid');
    var sel = document.getElementById('role-sel');
    if (!uid) { pmsg('role-msg', 'Pilih pengguna dulu.'); return; }
    var r = await ppost('/api/v1/users/' + encodeURIComponent(uid) + '/roles', { role: sel ? sel.value : '' });
    if (!r.ok) { pmsg('role-msg', 'Gagal (' + r.status + '). Perlu hak Assign / peran valid.'); return; }
    pmsg('role-msg', 'Peran ditetapkan.');
    loadRoleUsers();
    loadSessions();
  });
  /* ---- organization invites (join existing org via token) ---- */
  async function loadInvites() {
    var box = document.getElementById('inv-list');
    if (!box) return;
    var v = await j('/api/v1/orgs/invites');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat (perlu hak Assign).</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var act = x.accepted ? '<span class="text-muted">diterima</span>' : '<button class="btn btn-sm btn-outline-danger inv-rev" data-id="' + x.id + '" type="button">Cabut</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.email) + '</td><td>' + esc(x.role) + '</td><td>' + (x.accepted ? 'ya' : 'belum') + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada undangan.</td></tr>';
    qsa('.inv-rev', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/orgs/invites/' + b.dataset.id, { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('inv-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('inv-msg', '');
        loadInvites();
      });
    });
  }
  on('inv-add', 'click', async function () {
    var em = pval('inv-email'), rl = pval('inv-role') || 'legal_staff';
    if (!em || em.indexOf('@') < 0) { pmsg('inv-msg', 'Isi email valid.'); return; }
    var r = await ppost('/api/v1/orgs/invites', { email: em, role: rl });
    if (!r.ok) { pmsg('inv-msg', 'Gagal (' + r.status + ').'); return; }
    var v = await r.json();
    pmsg('inv-msg', '');
    var pl = document.getElementById('inv-plain');
    if (pl) { pl.textContent = 'Token undangan (sekali tampil, 72 jam): ' + v.token; pl.classList.remove('hidden'); }
    loadInvites();
  });
  on('inv-reload', 'click', loadInvites);
  qsa('.snav[data-tab="pengaturan"]', function (a) {
    a.addEventListener('click', function () { loadRoleUsers(); loadSessions(); loadRoleMatrix(); loadInvites(); });
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
  loadRoleUsers();
  loadRoleMatrix();
  loadSessions();
  loadInvites();
  [['qa-client', 'klien', 'cli-name'], ['qa-matter', 'klien', 'mat-title'], ['qa-upload', 'dok', 'dn'], ['qa-ocr', 'ocr', 'ocr-doc'], ['qa-deed', 'akta', 'deed-number'], ['qa-invoice', 'keu', 'invm-total']].forEach(function (x) {
    on(x[0], 'click', function () { window.apptab(x[1]); var t = document.getElementById(x[2]); if (t) t.focus(); });
  });
  async function loadTasks() {
    var sel = document.getElementById('task-matter');
    var box = document.getElementById('task-list');
    if (!sel || !box) return;
    if (!sel.value) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Pilih perkara dulu.</td></tr>'; return; }
    var ts = await j('/api/v1/matters/' + sel.value + '/tasks');
    if (!ts) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = ts.map(function (t) {
      var nx = TASK_NEXT[t.status] || [];
      var dueCell = t.due ? esc(t.due) : '<span class="text-muted">—</span>';
      var dueSet = '<input class="form-control form-control-sm" style="width:150px;display:inline-block" type="datetime-local" id="due-' + t.id + '" value="' + (t.due ? esc(t.due.replace(' ', 'T').slice(0, 16)) : '') + '"> <button class="btn btn-sm" data-due-set="' + t.id + '" type="button">Set</button>';
      return '<tr><td>' + t.id + '</td><td>' + esc(t.title) + '</td><td>' + badge(t.status) + '</td><td>' + dueCell + '<br>' + dueSet + '</td><td>' + nx.map(function (n) { return '<button class="btn btn-sm btn-outline-primary tadv" data-id="' + t.id + '" data-to="' + n + '">' + esc(n) + '</button>'; }).join(' ') + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada tugas.</td></tr>';
    qsa('[data-due-set]', function (b) {
      b.addEventListener('click', async function () {
        var inp = document.getElementById('due-' + b.getAttribute('data-due-set'));
        var val = inp && inp.value ? inp.value.replace('T', ' ') + ':00' : null;
        var r = await ppost('/api/v1/tasks/' + b.getAttribute('data-due-set') + '/due', { due: val });
        if (!r.ok) { pmsg('task-msg', 'Gagal (' + r.status + '). Format: YYYY-MM-DD HH:MM.'); return; }
        pmsg('task-msg', '');
        loadTasks();
      });
    });
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
    var created = await r.json();
    var dv = pval('task-due');
    if (dv) {
      var r2 = await ppost('/api/v1/tasks/' + created.id + '/due', { due: dv.replace('T', ' ') + ':00' });
      if (!r2.ok) { pmsg('task-msg', 'Tugas dibuat, tenggat gagal (' + r2.status + ').'); loadTasks(); return; }
    }
    pmsg('task-msg', '');
    document.getElementById('task-title').value = '';
    var di = document.getElementById('task-due'); if (di) di.value = '';
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
  on('ocr-acc-btn', 'click', function () { dlCsv('/api/v1/reports/ocr-accuracy.csv', 'ocr-accuracy.csv', 'ocr-msg'); });
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
  /* ---- land certificate intake (Akta tab; jobs processed in OCR tab) ---- */
  async function certRefresh() {
    var box = document.getElementById('cert-list');
    if (!box) return;
    var v = await j('/api/v1/ocr/jobs');
    if (!v) { box.innerHTML = '<tr><td colspan="2" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.filter(function (x) { return x.doc_type === 'land_certificate'; }).map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + badge(x.status) + '</td></tr>';
    }).join('') || '<tr><td colspan="2" class="text-muted">Belum ada job sertifikat.</td></tr>';
  }
  on('cert-btn', 'click', async function () {
    var id = pval('cert-doc');
    if (!id) { pmsg('cert-msg', 'Isi ID dokumen sertifikat dulu.'); return; }
    var r = await ppost('/api/v1/ocr/jobs', { document_id: +id, doc_type: 'land_certificate' });
    if (!r.ok) { pmsg('cert-msg', 'Gagal (' + r.status + '). Cek ID dokumen.'); return; }
    pmsg('cert-msg', '');
    certRefresh();
  });
  on('cert-reload', 'click', certRefresh);
  on('cert-reload', 'click', certRefresh);
  /* ---- deed clause library (versioned snippets) ---- */
  async function clauseRefresh() {
    var box = document.getElementById('clause-list');
    if (!box) return;
    var v = await j('/api/v1/deed-clauses');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.name) + '</td><td>' + esc(x.category) + '</td><td>v' + x.version + '</td>'
        + '<td><button class="btn btn-sm" data-clause-edit="' + x.id + '" type="button">Revisi</button> <button class="btn btn-sm btn-outline-danger" data-clause-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada klausa.</td></tr>';
    qsa('[data-clause-edit]', function (b) {
      b.addEventListener('click', async function () {
        var body = pval('clause-body');
        if (!body) { pmsg('clause-msg', 'Isi revisi klausa dulu.'); return; }
        var r = await fetch('/api/v1/deed-clauses/' + b.getAttribute('data-clause-edit'), { method: 'PUT', headers: Object.assign({}, H, { 'Content-Type': 'application/json' }), body: JSON.stringify({ body: body }) });
        if (!r.ok) { pmsg('clause-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('clause-msg', '');
        clauseRefresh();
      });
    });
    qsa('[data-clause-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/deed-clauses/' + b.getAttribute('data-clause-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('clause-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('clause-msg', '');
        clauseRefresh();
      });
    });
  }
  on('clause-add', 'click', async function () {
    if (!pval('clause-name') || !pval('clause-body')) { pmsg('clause-msg', 'Isi nama dan isi klausa.'); return; }
    var r = await ppost('/api/v1/deed-clauses', { name: pval('clause-name'), category: pval('clause-cat') || 'umum', body: pval('clause-body') });
    if (!r.ok) { pmsg('clause-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('clause-msg', '');
    clauseRefresh();
  });
  on('clause-reload', 'click', clauseRefresh);
  qsa('.snav[data-tab="akta"]', function (a) {
    a.addEventListener('click', function () { certRefresh(); clauseRefresh(); });
  });
  qsa('.snav[data-tab="akta"]', function (a) {
    a.addEventListener('click', function () { certRefresh(); });
  });
  /* ---- PPAT deed register (draft -> signed -> registered) ---- */
  var PPAT_NEXT = { draft: 'signed', signed: 'registered' };
  async function ppatRefresh() {
    var box = document.getElementById('ppat-list');
    if (!box) return;
    var v = await j('/api/v1/ppat');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = PPAT_NEXT[x.status];
      var act = nx ? '<button class="btn btn-sm" data-ppat-next="' + x.id + '" data-ppat-to="' + nx + '" type="button">→ ' + nx + '</button>' : '<span class="text-muted">selesai</span>';
      act += ' <button class="btn btn-sm btn-outline-secondary" data-ppat-check="' + x.id + '" type="button">Cek</button> <span class="text-muted small" id="ppat-chk-' + x.id + '"></span>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.number) + '</td><td>' + esc(x.deed_kind) + '</td><td>' + esc(x.seller) + ' → ' + esc(x.buyer) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada akta PPAT.</td></tr>';
    qsa('[data-ppat-check]', function (b) {
      b.addEventListener('click', async function () {
        var id = b.getAttribute('data-ppat-check');
        var v = await j('/api/v1/ppat/' + id + '/checklist');
        var box2 = document.getElementById('ppat-chk-' + id);
        if (!v) { if (box2) box2.textContent = 'gagal'; return; }
        if (box2) box2.textContent = v.complete ? 'lengkap, siap ' + (PPAT_NEXT[v.status] || 'arsip') : 'kurang: ' + v.missing.join(', ');
      });
    });
    qsa('[data-ppat-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/ppat/' + b.getAttribute('data-ppat-next') + '/status', { status: b.getAttribute('data-ppat-to') });
        if (!r.ok) { pmsg('ppat-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('ppat-msg', '');
        ppatRefresh();
      });
    });
  }
  on('ppat-add', 'click', async function () {
    if (!pval('ppat-number')) { pmsg('ppat-msg', 'Isi nomor akta.'); return; }
    var r = await ppost('/api/v1/ppat', { number: pval('ppat-number'), deed_kind: pval('ppat-kind') || 'ajb', seller: pval('ppat-seller') || '', buyer: pval('ppat-buyer') || '', object: pval('ppat-object') || '' });
    if (!r.ok) { pmsg('ppat-msg', 'Gagal (' + r.status + '). Nomor mungkin sudah dipakai.'); return; }
    pmsg('ppat-msg', '');
    ppatRefresh();
  });
  on('ppat-reload', 'click', ppatRefresh);
  qsa('.snav[data-tab="akta"]', function (a) {
    a.addEventListener('click', function () { certRefresh(); ppatRefresh(); });
  });
  /* ---- document detail + version history ---- */
  async function docVersionRefresh() {
    var box = document.getElementById('doc-version-detail');
    if (!box) return;
    var id = pval('doc-version-id');
    if (!id) { pmsg('doc-version-msg', 'Isi ID dokumen dulu.'); return; }
    pmsg('doc-version-msg', '');
    var v = await j('/api/v1/documents/' + encodeURIComponent(id));
    if (!v) { box.innerHTML = '<p class="text-muted">Gagal memuat (cek ID / hak akses).</p>'; return; }
    box.innerHTML = '<p><strong>#' + v.id + '</strong> ' + esc(v.name) + ' <span class="text-muted">' + esc(v.mime) + ' · ' + v.size + ' byte</span><br>SHA-256: <code>' + esc(v.sha) + '</code></p>'
      + '<div class="table-responsive"><table class="table card-table"><thead><tr><th>Versi</th><th>Kunci objek</th></tr></thead><tbody>'
      + ((v.versions || []).map(function (x) {
        return '<tr><td>v' + x.v + '</td><td>' + esc(x.key) + '</td></tr>';
      }).join('') || '<tr><td colspan="2" class="text-muted">Belum ada versi tersimpan.</td></tr>')
      + '</tbody></table></div>';
  }
  on('doc-version-btn', 'click', docVersionRefresh);
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
  on('mkt-unsub-btn', 'click', async function () {
    var em = document.getElementById('mkt-sub-email').value;
    var msg = document.getElementById('mkt-sub-msg');
    var r = await mktPost('/api/v1/public/newsletter/unsubscribe', { email: em });
    if (!r) return;
    if (!r.ok) { msg.textContent = 'Unsubscribe gagal (' + r.status + ').'; return; }
    var c = document.getElementById('mkt-news-count');
    c.textContent = String(Math.max(0, +c.textContent - 1));
    msg.textContent = 'Berhasil berhenti berlangganan.';
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
  /* ---- CRM: meetings + tags (additive, Bearer fetch, inline errors) ---- */
  function crmToStamp(v) {
    if (!v) return '';
    var t = String(v).replace('T', ' ');
    if (/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}$/.test(t)) return t + ':00';
    return t;
  }
  async function crmMeetings() {
    var box = document.getElementById('crm-meet-list');
    if (!box) return;
    var cid = pval('crm-meet-client');
    if (!cid) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Isi ID klien dulu.</td></tr>'; return; }
    var v = await j('/api/v1/clients/' + encodeURIComponent(cid) + '/meetings');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + esc(x.happened_at) + '</td><td><button class="btn btn-sm btn-outline-danger crm-mdel" data-id="' + x.id + '">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada pertemuan.</td></tr>';
    pmsg('crm-meet-msg', '');
    qsa('.crm-mdel', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/meetings/' + b.dataset.id, { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('crm-meet-msg', 'Gagal hapus (' + r.status + ').'); return; }
        crmMeetings();
        crmRecent();
      });
    });
  }
  async function crmTags() {
    var box = document.getElementById('crm-tag-list');
    if (!box) return;
    var v = await j('/api/v1/tags');
    if (!v) { box.innerHTML = '<tr><td colspan="3" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.name) + '</td><td><button class="btn btn-sm btn-outline-danger crm-tdel" data-id="' + x.id + '">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="3" class="text-muted">Belum ada tag.</td></tr>';
    pmsg('crm-tag-msg', '');
    qsa('.crm-tdel', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/tags/' + b.dataset.id, { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('crm-tag-msg', 'Gagal hapus (' + r.status + ').'); return; }
        crmTags();
      });
    });
  }
  async function crmClientTags() {
    var box = document.getElementById('crm-ct-list');
    if (!box) return;
    var cid = pval('crm-ct-client');
    if (!cid) { box.innerHTML = '<tr><td colspan="3" class="text-muted">Isi ID klien dulu.</td></tr>'; return; }
    var v = await j('/api/v1/clients/' + encodeURIComponent(cid) + '/tags');
    if (!v) { box.innerHTML = '<tr><td colspan="3" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.name) + '</td><td><button class="btn btn-sm btn-outline-danger crm-tdet" data-id="' + x.id + '">Lepas</button></td></tr>';
    }).join('') || '<tr><td colspan="3" class="text-muted">Belum ada tag untuk klien ini.</td></tr>';
    pmsg('crm-ct-msg', '');
    qsa('.crm-tdet', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/clients/' + encodeURIComponent(cid) + '/tags/' + b.dataset.id, { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('crm-ct-msg', 'Gagal lepas (' + r.status + ').'); return; }
        crmClientTags();
      });
    });
  }
  async function crmRecent() {
    var box = document.getElementById('crm-recent');
    if (!box) return;
    var v = await j('/api/v1/meetings?limit=50');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + x.client_id + '</td><td>' + esc(x.title) + '</td><td>' + esc(x.happened_at) + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada pertemuan.</td></tr>';
    pmsg('crm-recent-msg', '');
  }
  on('crm-meet-load', 'click', crmMeetings);
  on('crm-meet-btn', 'click', async function () {
    var cid = pval('crm-meet-client');
    if (!cid) { pmsg('crm-meet-msg', 'Isi ID klien dulu.'); return; }
    var title = pval('crm-meet-title');
    if (title.length < 3) { pmsg('crm-meet-msg', 'Judul minimal 3 huruf.'); return; }
    var at = crmToStamp(pval('crm-meet-at'));
    if (!/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/.test(at)) { pmsg('crm-meet-msg', 'Isi waktu (datetime-local), format YYYY-MM-DD HH:MM:SS.'); return; }
    var r = await ppost('/api/v1/clients/' + encodeURIComponent(cid) + '/meetings', { title: title, notes: pval('crm-meet-notes') || null, happened_at: at });
    if (!r.ok) { pmsg('crm-meet-msg', 'Gagal (' + r.status + '). Periksa ID klien / format waktu.'); return; }
    pmsg('crm-meet-msg', '');
    document.getElementById('crm-meet-title').value = '';
    crmMeetings();
    crmRecent();
  });
  on('crm-tag-btn', 'click', async function () {
    var nm = pval('crm-tag-name');
    if (!nm) { pmsg('crm-tag-msg', 'Isi nama tag dulu.'); return; }
    var r = await ppost('/api/v1/tags', { name: nm });
    if (!r.ok) { pmsg('crm-tag-msg', r.status === 409 ? 'Tag sudah ada (409).' : 'Gagal (' + r.status + ').'); return; }
    pmsg('crm-tag-msg', '');
    document.getElementById('crm-tag-name').value = '';
    crmTags();
  });
  on('crm-ct-load', 'click', crmClientTags);
  on('crm-ct-attach', 'click', async function () {
    var cid = pval('crm-ct-client'), tid = pval('crm-ct-tag');
    if (!cid || !tid) { pmsg('crm-ct-msg', 'Isi ID klien dan ID tag.'); return; }
    var r = await ppost('/api/v1/clients/' + encodeURIComponent(cid) + '/tags', { tag_id: +tid });
    if (!r.ok) { pmsg('crm-ct-msg', 'Gagal (' + r.status + '). Periksa ID klien / tag.'); return; }
    pmsg('crm-ct-msg', '');
    crmClientTags();
  });
  on('crm-recent-btn', 'click', crmRecent);
  crmTags();
  crmRecent();
  /* ---- audit viewer + jobs queue (additive) ---- */
  async function dlCsv(url, filename, msgId) {
    var r = await fetch(url, { headers: H });
    if (!r.ok) { pmsg(msgId, 'Gagal ekspor (' + r.status + ').'); return; }
    var blob = await r.blob();
    var a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    a.remove();
  }
  on('rep-matters', 'click', function () { dlCsv('/api/v1/reports/matters.csv', 'matters.csv', 'rep-msg'); });
  on('rep-deadlines', 'click', function () { dlCsv('/api/v1/reports/deadline-compliance.csv', 'deadline-compliance.csv', 'rep-msg'); });
  on('rep-posture', 'click', function () { dlCsv('/api/v1/reports/compliance-posture.csv', 'compliance-posture.csv', 'rep-msg'); });
  on('rep-exec', 'click', async function () {
    var box = document.getElementById('exec-sum');
    if (!box) return;
    var v = await j('/api/v1/reports/executive-summary');
    if (!v) { box.innerHTML = '<p class="text-muted">Gagal memuat.</p>'; return; }
    var rows = [['Klien', v.clients], ['Perkara', v.matters], ['Akta', v.deeds], ['Invoice terbuka', v.open_invoices], ['Sisa tagihan', fmtRp(v.open_balance)], ['Persetujuan pending', v.pending_approvals], ['Obligasi terbuka', v.open_obligations], ['Tugas bertenggat', v.tasks_due_tracked], ['Tugas lewat tenggat', v.tasks_overdue]];
    box.innerHTML = '<div class="table-responsive"><table class="table card-table"><tbody>' + rows.map(function (r) { return '<tr><td>' + r[0] + '</td><td><strong>' + r[1] + '</strong></td></tr>'; }).join('') + '</tbody></table></div>';
  });
  on('keu-csv', 'click', function () { dlCsv('/api/v1/reports/matters-csv', 'matters.csv', 'keu-msg'); });
  on('ar-csv', 'click', function () { dlCsv('/api/v1/reports/ar-aging.csv', 'ar-aging.csv', 'ar-msg'); });
  on('appr-csv', 'click', function () { dlCsv('/api/v1/reports/approvals-bottleneck.csv', 'approvals-bottleneck.csv', 'ar-msg'); });
  on('prof-csv', 'click', function () { dlCsv('/api/v1/reports/profitability.csv', 'profitability.csv', 'ar-msg'); });
  async function whRefresh() {
    var box = document.getElementById('wh-list');
    if (!box) return;
    var v = await j('/api/v1/webhooks/deliveries');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.provider) + '</td><td>' + esc(x.event) + '</td><td>' + badge(x.status) + '</td><td>' + esc(x.detail) + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada webhook masuk.</td></tr>';
  }
  on('wh-reload', 'click', whRefresh);
  async function wheRefresh() {
    var box = document.getElementById('whe-list');
    if (!box) return;
    var v = await j('/api/v1/webhooks/endpoints');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.url) + '</td><td>' + esc(x.events) + '</td><td>' + (x.active ? 'aktif' : 'mati') + '</td><td><button class="btn btn-sm" data-whe-test="' + x.id + '" type="button">Uji</button> <button class="btn btn-sm" data-whe-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada endpoint.</td></tr>';
    qsa('[data-whe-test]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/webhooks/endpoints/' + b.getAttribute('data-whe-test') + '/test', { event: 'ping' });
        if (!r.ok) { pmsg('whe-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('whe-msg', '');
        whRefresh();
      });
    });
    qsa('[data-whe-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/webhooks/endpoints/' + b.getAttribute('data-whe-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('whe-msg', 'Gagal (' + r.status + '). Butuh hak Approve.'); return; }
        pmsg('whe-msg', '');
        wheRefresh();
      });
    });
  }
  on('whe-add', 'click', async function () {
    if (!pval('whe-url')) { pmsg('whe-msg', 'Isi URL endpoint.'); return; }
    var r = await ppost('/api/v1/webhooks/endpoints', { url: pval('whe-url'), secret: pval('whe-secret') || null, events: '*' });
    if (!r.ok) { pmsg('whe-msg', 'Gagal (' + r.status + '). URL harus http(s).'); return; }
    pmsg('whe-msg', '');
    document.getElementById('whe-url').value = '';
    wheRefresh();
  });
  on('whe-reload', 'click', wheRefresh);
  qsa('.snav[data-tab="audit"]', function (a) {
    a.addEventListener('click', function () { whRefresh(); });
  });
  async function loadAudit() {
    var box = document.getElementById('audit-rows');
    if (!box) return;
    var sel = document.getElementById('audit-entity');
    var ent = sel && sel.value ? '?entity=' + encodeURIComponent(sel.value) : '';
    box.innerHTML = '<tr><td colspan="5" class="text-muted">Memuat…</td></tr>';
    try {
      var r = await fetch('/api/v1/audit' + ent, { headers: H });
      if (r.status === 401) { location.href = '/login'; return; }
      if (r.status === 403) {
        box.innerHTML = '<tr><td colspan="5" class="text-muted">Tidak ada akses.</td></tr>';
        pmsg('audit-msg', '');
        var note = document.getElementById('audit-note');
        if (note) note.textContent = 'Butuh peran Owner, Admin, atau Auditor untuk membaca jejak audit.';
        return;
      }
      if (!r.ok) { pmsg('audit-msg', 'Gagal memuat (' + r.status + ').'); return; }
      pmsg('audit-msg', '');
      var v = await r.json();
      box.innerHTML = v.map(function (x) {
        return '<tr><td>' + x.id + '</td><td>' + esc(x.actor) + '</td><td>' + esc(x.action) + '</td><td>' + esc(x.entity) + (x.entity_id ? ' #' + x.entity_id : '') + '</td><td>' + esc(x.created) + '</td></tr>';
      }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada entri audit.</td></tr>';
    } catch (e) { pmsg('audit-msg', 'Server tidak terjangkau.'); }
  }
  async function loadJobs() {
    var box = document.getElementById('job-rows');
    if (!box) return;
    var v = await j('/api/v1/jobs');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.kind) + '</td><td>' + badge(x.status) + '</td><td>' + x.attempts + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Antrean kosong.</td></tr>';
    var sum = document.getElementById('job-summary');
    if (sum) {
      var c = {};
      v.forEach(function (x) { c[x.status] = (c[x.status] || 0) + 1; });
      sum.textContent = Object.keys(c).map(function (k) { return k + ': ' + c[k]; }).join(' · ') || 'kosong';
    }
  }
  on('audit-reload', 'click', loadAudit);
  on('audit-entity', 'change', loadAudit);
  on('job-btn', 'click', async function () {
    var kind = pval('job-kind');
    if (!kind) { pmsg('job-msg', 'Isi jenis job dulu.'); return; }
    var r = await ppost('/api/v1/jobs', { kind: kind });
    if (!r.ok) { pmsg('job-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('job-msg', '');
    document.getElementById('job-kind').value = '';
    loadJobs();
  });
  qsa('.snav[data-tab="audit"], .snav[data-tab="jobs"]', function (a) {
    a.addEventListener('click', function () {
      if (a.dataset.tab === 'audit') loadAudit(); else loadJobs();
    });
  });
  setInterval(function () {
    var p = document.getElementById('p-jobs');
    if (p && !p.classList.contains('hidden')) loadJobs();
  }, 15000);
  /* ---- command palette: Ctrl+K / Cmd+K focuses global search ---- */
  document.addEventListener('keydown', function (ev) {
    if (!(ev.ctrlKey || ev.metaKey) || (ev.key !== 'k' && ev.key !== 'K')) {
      if (ev.key === 'Escape') {
        var qr = document.getElementById('qres');
        if (qr) qr.textContent = '';
        var bl = document.getElementById('bell-list');
        if (bl) bl.classList.remove('show');
      }
      return;
    }
    var q = document.getElementById('q');
    if (!q) return;
    ev.preventDefault();
    q.focus();
    if (q.select) q.select();
  });
  on('trust-csv-btn', 'click', function () {
    var id = pval('trust-csv-acct');
    if (!id) { pmsg('trust-csv-msg', 'Isi ID rekening trust.'); return; }
    pmsg('trust-csv-msg', '');
    dlCsv('/api/v1/reports/trust-ledger.csv?account_id=' + encodeURIComponent(id), 'trust-' + id + '.csv', 'trust-csv-msg');
  });
  on('deed-csv', 'click', function () {
    var f = pval('deed-rep-from'), t = pval('deed-rep-to');
    var ok = /^\d{4}-\d{2}-\d{2}$/;
    if ((f && !ok.test(f)) || (t && !ok.test(t))) { pmsg('deed-csv-msg', 'Format tanggal YYYY-MM-DD.'); return; }
    if (f && t && f > t) { pmsg('deed-csv-msg', 'Rentang terbalik: dari > sampai.'); return; }
    pmsg('deed-csv-msg', '');
    var qs = [];
    if (f) qs.push('from=' + encodeURIComponent(f));
    if (t) qs.push('to=' + encodeURIComponent(t));
    dlCsv('/api/v1/reports/deeds.csv' + (qs.length ? '?' + qs.join('&') : ''), 'repertorium' + (qs.length ? '-' + qs.join('_').replace(/[^0-9A-Za-z_-]/g, '') : '') + '.csv', 'deed-csv-msg');
  });
  on('deed-cmp', 'click', function () {
    var f = pval('deed-rep-from'), t = pval('deed-rep-to');
    var cf = pval('deed-cmp-from'), ct = pval('deed-cmp-to');
    var ok = /^\d{4}-\d{2}-\d{2}$/;
    if (!f || !t || !cf || !ct || ![f, t, cf, ct].every(function (d) { return ok.test(d); })) { pmsg('deed-csv-msg', 'Isi keempat tanggal (periode + pembanding) YYYY-MM-DD.'); return; }
    if (f > t || cf > ct) { pmsg('deed-csv-msg', 'Rentang terbalik.'); return; }
    pmsg('deed-csv-msg', '');
    dlCsv('/api/v1/reports/deeds.csv?compare=1&from=' + encodeURIComponent(f) + '&to=' + encodeURIComponent(t) + '&cmp_from=' + encodeURIComponent(cf) + '&cmp_to=' + encodeURIComponent(ct), 'repertorium-banding.csv', 'deed-csv-msg');
  });
  /* ---- approvals engine UI ---- */
  async function apprRefresh() {
    var box = document.getElementById('appr-list');
    if (!box) return;
    var f = pval('appr-filter');
    var v = await j('/api/v1/approvals' + (f ? '?status=' + encodeURIComponent(f) : ''));
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + esc(x.entity) + ' #' + x.entity_id + '</td><td>' + badge(x.status) + '</td><td><button class="btn btn-sm" data-appr="' + x.id + '" type="button">Detail</button></td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada permintaan.</td></tr>';
    qsa('[data-appr]', function (b) {
      b.addEventListener('click', function () {
        document.getElementById('appr-id').value = b.getAttribute('data-appr');
        apprShow();
      });
    });
  }
  async function apprShow() {
    var box = document.getElementById('appr-detail');
    if (!box) return;
    var id = pval('appr-id');
    if (!id) { pmsg('appr-dmsg', 'Isi ID permintaan.'); return; }
    pmsg('appr-dmsg', '');
    var v = await j('/api/v1/approvals/' + encodeURIComponent(id));
    if (!v) { box.innerHTML = '<p class="text-muted">Gagal memuat.</p>'; return; }
    box.innerHTML = '<p><strong>#' + v.id + '</strong> ' + esc(v.title) + ' — ' + esc(v.entity) + ' #' + v.entity_id + ' ' + badge(v.status) + '</p>'
      + '<ol>' + (v.steps || []).map(function (s) {
        return '<li>tahap ' + s.step_no + ' (' + esc(s.role) + '): ' + badge(s.status) + (s.comment ? ' — <em>' + esc(s.comment) + '</em>' : '') + '</li>';
      }).join('') + '</ol>';
  }
  async function apprDecide(decision) {
    var id = pval('appr-id'), step = pval('appr-step');
    if (!id || !step) { pmsg('appr-dmsg', 'Isi ID permintaan dan no. tahap.'); return; }
    var r = await ppost('/api/v1/approvals/' + encodeURIComponent(id) + '/decisions',
      { step_no: +step, decision: decision, comment: pval('appr-comment') || null });
    if (!r.ok) { pmsg('appr-dmsg', 'Gagal (' + r.status + '). Cek urutan tahap & peran.'); return; }
    pmsg('appr-dmsg', '');
    apprShow();
    apprRefresh();
  }
  on('appr-btn', 'click', async function () {
    var roles = [pval('appr-role1'), pval('appr-role2'), pval('appr-role3')].filter(function (x) { return x; });
    if (!pval('appr-entity-id') || !pval('appr-title') || !roles.length) { pmsg('appr-msg', 'Isi ID entitas, judul, dan min. 1 peran.'); return; }
    var r = await ppost('/api/v1/approvals', {
      entity: document.getElementById('appr-entity').value,
      entity_id: +pval('appr-entity-id'),
      title: pval('appr-title'),
      steps: roles.map(function (x, i) { return { step_no: i + 1, role: x }; })
    });
    if (!r.ok) { pmsg('appr-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('appr-msg', '');
    apprRefresh();
  });
  on('appr-reload', 'click', apprRefresh);
  on('appr-filter', 'change', apprRefresh);
  /* ---- approval notifications (kind-routed action node) ---- */
  async function apprNotifRefresh() {
    var box = document.getElementById('appr-notif-list');
    if (!box) return;
    var onlyUnread = document.getElementById('appr-notif-unread-only');
    var qs = '/api/v1/notifications?kind=approval' + ((onlyUnread && onlyUnread.checked) ? '&unread_only=true' : '');
    var v = await j(qs);
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    var unread = v.filter(function (x) { return !x.read; }).length;
    var badge = document.getElementById('appr-notif-unread');
    if (badge) badge.textContent = unread ? unread + ' belum dibaca' : 'semua dibaca';
    box.innerHTML = v.slice(0, 5).map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + (x.read ? 'ya' : 'belum') + '</td><td>'
        + (x.read ? '<span class="text-muted">—</span>' : '<button class="btn btn-sm" data-notif-read="' + x.id + '" type="button">Tandai dibaca</button>') + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada notifikasi approval.</td></tr>';
    qsa('[data-notif-read]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/notifications/' + b.getAttribute('data-notif-read') + '/read', {});
        if (r.ok) apprNotifRefresh();
      });
    });
  }
  on('appr-notif-reload', 'click', apprNotifRefresh);
  on('appr-notif-unread-only', 'change', apprNotifRefresh);
  /* ---- approval delegation (out-of-office substitutes) ---- */
  async function delegRefresh() {
    var box = document.getElementById('deleg-list');
    if (!box) return;
    var v = await j('/api/v1/delegations');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat (perlu hak Approve).</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var act = x.active ? '<button class="btn btn-sm btn-outline-danger" data-deleg-off="' + esc(x.delegate) + '" type="button">Cabut</button>' : '<span class="text-muted">nonaktif</span>';
      return '<tr><td>' + x.id + '</td><td><code>' + esc(String(x.delegator).slice(0, 8)) + '</code></td><td><code>' + esc(String(x.delegate).slice(0, 8)) + '</code></td><td>' + (x.active ? 'ya' : 'tidak') + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada delegasi.</td></tr>';
    qsa('[data-deleg-off]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/delegations', { delegate_uid: b.getAttribute('data-deleg-off'), active: false });
        if (!r.ok) { pmsg('deleg-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('deleg-msg', '');
        delegRefresh();
      });
    });
  }
  on('deleg-add', 'click', async function () {
    var uid = pval('deleg-uid');
    if (!uid) { pmsg('deleg-msg', 'Isi UID penerima kuasa.'); return; }
    var r = await ppost('/api/v1/delegations', { delegate_uid: uid, active: true });
    if (!r.ok) { pmsg('deleg-msg', 'Gagal (' + r.status + '). UID harus satu organisasi.'); return; }
    pmsg('deleg-msg', '');
    delegRefresh();
  });
  on('deleg-reload', 'click', delegRefresh);
  qsa('.snav[data-tab="appr"]', function (a) {
    a.addEventListener('click', function () { apprRefresh(); apprNotifRefresh(); delegRefresh(); });
  });
  qsa('.snav[data-tab="appr"]', function (a) {
    a.addEventListener('click', function () { apprRefresh(); apprNotifRefresh(); });
  });
  on('appr-show', 'click', apprShow);
  on('appr-ok', 'click', function () { apprDecide('approve'); });
  on('appr-no', 'click', function () { apprDecide('reject'); });
  on('appr-rw', 'click', function () { apprDecide('rework'); });
  on('appr-revoke', 'click', async function () {
    var id = pval('appr-id');
    if (!id) { pmsg('appr-dmsg', 'Isi ID permintaan.'); return; }
    var r = await ppost('/api/v1/approvals/' + encodeURIComponent(id) + '/revoke', {});
    if (!r.ok) { pmsg('appr-dmsg', 'Gagal (' + r.status + ').'); return; }
    pmsg('appr-dmsg', '');
    apprShow();
    apprRefresh();
  });
  qsa('.snav[data-tab="appr"]', function (a) {
    a.addEventListener('click', function () { apprRefresh(); });
  });
  /* ---- compliance: UU PDP obligations + consent ---- */
  async function kepRefresh() {
    var box = document.getElementById('kep-list');
    if (!box) return;
    var f = pval('kep-filter');
    var v = await j('/api/v1/compliance/obligations' + (f ? '?status=' + encodeURIComponent(f) : ''));
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var acts = x.status === 'done'
        ? '<span class="text-muted">selesai</span>'
        : '<button class="btn btn-sm" data-kep-next="' + x.id + '" data-kep-to="' + (x.status === 'open' ? 'in_progress' : 'done') + '" type="button">Maju</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + esc(x.category) + '</td><td>' + badge(x.status) + '</td><td>' + acts + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada obligasi.</td></tr>';
    qsa('[data-kep-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/obligations/' + b.getAttribute('data-kep-next') + '/status', { status: b.getAttribute('data-kep-to') });
        if (!r.ok) { pmsg('kep-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('kep-msg', '');
        kepRefresh();
      });
    });
  }
  async function conRefresh() {
    var box = document.getElementById('con-list');
    if (!box) return;
    var f = pval('con-filter');
    var v = await j('/api/v1/consents' + (f ? '?subject=' + encodeURIComponent(f) : ''));
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var acts = x.status === 'granted'
        ? '<button class="btn btn-sm" data-con-wd="' + x.id + '" type="button">Cabut</button>'
        : '<span class="text-muted">dicabut</span>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.subject) + '</td><td>' + esc(x.purpose) + '</td><td>' + badge(x.status) + '</td><td>' + acts + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada persetujuan.</td></tr>';
    qsa('[data-con-wd]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/consents/' + b.getAttribute('data-con-wd') + '/withdraw', {});
        if (!r.ok) { pmsg('con-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('con-msg', '');
        conRefresh();
      });
    });
  }
  on('kep-add', 'click', async function () {
    if (!pval('kep-title')) { pmsg('kep-msg', 'Isi judul obligasi.'); return; }
    var r = await ppost('/api/v1/compliance/obligations', { title: pval('kep-title'), category: pval('kep-cat') || 'general', owner: pval('kep-owner') || '' });
    if (!r.ok) { pmsg('kep-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('kep-msg', '');
    kepRefresh();
  });
  on('kep-reload', 'click', kepRefresh);
  on('kep-filter', 'change', kepRefresh);
  on('con-grant', 'click', async function () {
    if (!pval('con-subject') || !pval('con-purpose')) { pmsg('con-msg', 'Isi subjek dan tujuan.'); return; }
    var r = await ppost('/api/v1/consents', { subject: pval('con-subject'), purpose: pval('con-purpose') });
    if (!r.ok) { pmsg('con-msg', 'Gagal (' + r.status + '). Mungkin sudah ada yang aktif.'); return; }
    pmsg('con-msg', '');
    conRefresh();
  });
  on('con-reload', 'click', conRefresh);
  /* ---- compliance: ROPA register ---- */
  async function ropaRefresh() {
    var box = document.getElementById('ropa-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/ropa');
    if (!v) { box.innerHTML = '<tr><td colspan="7" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.activity) + '</td><td>' + esc(x.purpose) + '</td><td>' + esc(x.data_categories) + '</td><td>' + esc(x.legal_basis) + '</td><td>' + esc(x.retention) + '</td><td><button class="btn btn-sm" data-ropa-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="7" class="text-muted">Belum ada entri ROPA.</td></tr>';
    qsa('[data-ropa-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/ropa/' + b.getAttribute('data-ropa-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('ropa-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('ropa-msg', '');
        ropaRefresh();
      });
    });
  }
  on('ropa-add', 'click', async function () {
    if (!pval('ropa-activity') || !pval('ropa-purpose')) { pmsg('ropa-msg', 'Isi aktivitas dan tujuan.'); return; }
    var r = await ppost('/api/v1/compliance/ropa', { activity: pval('ropa-activity'), purpose: pval('ropa-purpose'), data_categories: pval('ropa-cats') || '', legal_basis: pval('ropa-basis') || 'consent', retention: pval('ropa-ret') || '' });
    if (!r.ok) { pmsg('ropa-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('ropa-msg', '');
    ropaRefresh();
  });
  on('ropa-reload', 'click', ropaRefresh);
  /* ---- compliance: DPIA assessments ---- */
  var DPIA_NEXT = { draft: 'review', review: 'approved' };
  async function dpiaRefresh() {
    var box = document.getElementById('dpia-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/dpia');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = DPIA_NEXT[x.status];
      var act = nx ? '<button class="btn btn-sm" data-dpia-next="' + x.id + '" data-dpia-to="' + nx + '" type="button">→ ' + nx + '</button>' : '<span class="text-muted">selesai</span>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + esc(x.data_types) + '</td><td>' + badge(x.risk_level) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada asesmen DPIA.</td></tr>';
    qsa('[data-dpia-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/dpia/' + b.getAttribute('data-dpia-next') + '/status', { status: b.getAttribute('data-dpia-to') });
        if (!r.ok) { pmsg('dpia-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('dpia-msg', '');
        dpiaRefresh();
      });
    });
  }
  on('dpia-add', 'click', async function () {
    if (!pval('dpia-title')) { pmsg('dpia-msg', 'Isi judul asesmen.'); return; }
    var r = await ppost('/api/v1/compliance/dpia', { title: pval('dpia-title'), data_types: pval('dpia-types') || '', risk_level: pval('dpia-risk') || 'medium' });
    if (!r.ok) { pmsg('dpia-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('dpia-msg', '');
    dpiaRefresh();
  });
  on('dpia-reload', 'click', dpiaRefresh);
  /* ---- compliance: breach log (72h) ---- */
  var BREACH_NEXT = { draft: 'assessing', assessing: 'notified', notified: 'closed' };
  async function breachRefresh() {
    var box = document.getElementById('breach-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/breach');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = BREACH_NEXT[x.status];
      var act = nx ? '<button class="btn btn-sm" data-breach-next="' + x.id + '" data-breach-to="' + nx + '" type="button">→ ' + nx + '</button>' : '<span class="text-muted">ditutup</span>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + esc(x.discovered) + '</td><td>' + x.affected + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada insiden.</td></tr>';
    qsa('[data-breach-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/breach/' + b.getAttribute('data-breach-next') + '/status', { status: b.getAttribute('data-breach-to') });
        if (!r.ok) { pmsg('breach-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('breach-msg', '');
        breachRefresh();
      });
    });
  }
  on('breach-add', 'click', async function () {
    if (!pval('breach-title')) { pmsg('breach-msg', 'Isi judul insiden.'); return; }
    var aff = parseInt(pval('breach-affected'), 10);
    var r = await ppost('/api/v1/compliance/breach', { title: pval('breach-title'), discovered: pval('breach-date') || '', affected: isNaN(aff) ? 0 : aff });
    if (!r.ok) { pmsg('breach-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('breach-msg', '');
    breachRefresh();
  });
  on('breach-reload', 'click', breachRefresh);
  /* ---- compliance: vendor register + DPA ---- */
  var DPA_NEXT = { draft: 'review', review: 'signed' };
  async function vendorRefresh() {
    var box = document.getElementById('vendor-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/vendors');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = DPA_NEXT[x.dpa];
      var act = nx ? '<button class="btn btn-sm" data-vendor-next="' + x.id + '" data-vendor-to="' + nx + '" type="button">→ ' + nx + '</button>' : '<span class="text-muted">signed</span>';
      act += ' <button class="btn btn-sm btn-outline-danger" data-vendor-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.name) + '</td><td>' + esc(x.service) + '</td><td>' + badge(x.dpa) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada vendor.</td></tr>';
    qsa('[data-vendor-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/vendors/' + b.getAttribute('data-vendor-next') + '/dpa', { dpa: b.getAttribute('data-vendor-to') });
        if (!r.ok) { pmsg('vendor-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('vendor-msg', '');
        vendorRefresh();
      });
    });
    qsa('[data-vendor-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/vendors/' + b.getAttribute('data-vendor-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('vendor-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('vendor-msg', '');
        vendorRefresh();
      });
    });
  }
  on('vendor-add', 'click', async function () {
    if (!pval('vendor-name')) { pmsg('vendor-msg', 'Isi nama vendor.'); return; }
    var r = await ppost('/api/v1/compliance/vendors', { name: pval('vendor-name'), service: pval('vendor-service') || '' });
    if (!r.ok) { pmsg('vendor-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('vendor-msg', '');
    vendorRefresh();
  });
  on('vendor-reload', 'click', vendorRefresh);
  qsa('.snav[data-tab="kepatuhan"]', function (a) {
    a.addEventListener('click', function () { kepRefresh(); conRefresh(); ropaRefresh(); dpiaRefresh(); breachRefresh(); vendorRefresh(); });
  });
  on('wh-dl-reload', 'click', whDlRefresh);
  async function whDlRefresh() {
    var box = document.getElementById('wh-dl-list');
    if (!box) return;
    var v = await j('/api/v1/webhooks/deliveries?status=rejected');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.provider) + '</td><td>' + esc(x.event) + '</td><td>' + esc(x.detail) + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Tidak ada dead letter. Semua terkirim.</td></tr>';
  }
  qsa('.snav[data-tab="jobs"]', function (a) {
    a.addEventListener('click', function () { whDlRefresh(); });
  });
  mktRefresh();
  loadAudit();
  loadJobs();
  apprRefresh();
  apprNotifRefresh();
  delegRefresh();
  certRefresh();
  ppatRefresh();
  clauseRefresh();
  whRefresh();
  kepRefresh();
  conRefresh();
  ropaRefresh();
  dpiaRefresh();
  breachRefresh();
  vendorRefresh();
  whDlRefresh();
})();
