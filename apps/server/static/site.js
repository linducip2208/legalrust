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
  function ppostKey(u, body, key) {
    var h = Object.assign({}, H, { 'Content-Type': 'application/json' });
    if (key) h['Idempotency-Key'] = key;
    return fetch(u, { method: 'POST', headers: h, body: JSON.stringify(body) });
  }
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
  /* ---- office: price list ---- */
  async function priceRefresh() {
    var box = document.getElementById('price-list');
    if (!box) return;
    var v = await j('/api/v1/prices');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var act = '<button class="btn btn-sm" data-price-tg="' + x.id + '" data-price-on="' + (!x.active) + '" type="button">' + (x.active ? 'Nonaktifkan' : 'Aktifkan') + '</button> '
        + '<button class="btn btn-sm btn-outline-danger" data-price-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.code) + '</td><td>' + esc(x.name) + '</td><td>' + fmtRp(x.amount_minor) + '</td><td>' + (x.active ? '<span class="badge bg-green-lt">aktif</span>' : '<span class="badge bg-yellow-lt">nonaktif</span>') + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada tarif.</td></tr>';
    qsa('[data-price-tg]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/prices/' + b.getAttribute('data-price-tg') + '/active', { active: b.getAttribute('data-price-on') === 'true' });
        if (!r.ok) { pmsg('price-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('price-msg', '');
        priceRefresh();
      });
    });
    qsa('[data-price-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/prices/' + b.getAttribute('data-price-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('price-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('price-msg', '');
        priceRefresh();
      });
    });
  }
  on('price-add', 'click', async function () {
    if (!pval('price-code') || !pval('price-name') || !(+pval('price-amount') >= 0)) { pmsg('price-msg', 'Isi kode, nama, dan nominal.'); return; }
    var r = await ppost('/api/v1/prices', { code: pval('price-code'), name: pval('price-name'), amount_minor: +pval('price-amount') });
    if (!r.ok) { pmsg('price-msg', 'Gagal (' + r.status + '). Kode mungkin duplikat.'); return; }
    pmsg('price-msg', '');
    priceRefresh();
  });
  on('price-reload', 'click', priceRefresh);
  /* ---- office: quotations ---- */
  async function quotRefresh() {
    var box = document.getElementById('quot-list');
    if (!box) return;
    var v = await j('/api/v1/quotations');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var items = (x.items || []).map(function (i) { return esc(i.price_code) + '×' + i.qty; }).join(', ');
      var act = (x.status === 'draft' ? '<button class="btn btn-sm" data-quot-issue="' + x.id + '" type="button">Terbitkan</button> ' : '')
        + (x.status === 'issued' ? '<button class="btn btn-sm btn-success" data-quot-acc="' + x.id + '" type="button">Terima</button> <button class="btn btn-sm btn-danger" data-quot-rej="' + x.id + '" type="button">Tolak</button> ' : '')
        + '<button class="btn btn-sm btn-outline-danger" data-quot-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.client_name) + ' <span class="text-muted">(' + esc(x.deed_kind) + ': ' + items + ')</span></td><td><strong>' + fmtRp(x.total_minor) + '</strong></td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada penawaran.</td></tr>';
    qsa('[data-quot-issue]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/quotations/' + b.getAttribute('data-quot-issue') + '/status', { status: 'issued' });
        if (!r.ok) { pmsg('quot-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('quot-msg', '');
        quotRefresh();
      });
    });
    qsa('[data-quot-acc]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/quotations/' + b.getAttribute('data-quot-acc') + '/status', { status: 'accepted' });
        if (!r.ok) { pmsg('quot-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('quot-msg', '');
        quotRefresh();
      });
    });
    qsa('[data-quot-rej]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/quotations/' + b.getAttribute('data-quot-rej') + '/status', { status: 'rejected' });
        if (!r.ok) { pmsg('quot-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('quot-msg', '');
        quotRefresh();
      });
    });
    qsa('[data-quot-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/quotations/' + b.getAttribute('data-quot-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('quot-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('quot-msg', '');
        quotRefresh();
      });
    });
  }
  function parseQuotItems(s) {
    return s.split(',').map(function (p) {
      var kv = p.split(':');
      return { code: (kv[0] || '').trim(), qty: parseInt(kv[1], 10) || 0 };
    }).filter(function (i) { return i.code && i.qty > 0; });
  }
  on('quot-add', 'click', async function () {
    var items = parseQuotItems(pval('quot-items') || '');
    if (!pval('quot-client') || !items.length) { pmsg('quot-msg', 'Isi klien dan item (kode:qty, dipisah koma).'); return; }
    var r = await ppost('/api/v1/quotations', { client_name: pval('quot-client'), deed_kind: pval('quot-kind') || '', items: items });
    if (!r.ok) { pmsg('quot-msg', 'Gagal (' + r.status + '). Kode harus aktif di daftar harga.'); return; }
    pmsg('quot-msg', '');
    quotRefresh();
  });
  on('quot-reload', 'click', quotRefresh);
  /* ---- office: cash book ---- */
  async function cashRefresh() {
    var box = document.getElementById('cash-list');
    var sum = document.getElementById('cash-sum');
    if (!box) return;
    var v = await j('/api/v1/cashbook');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    if (sum) sum.textContent = 'Saldo: ' + fmtRp(v.balance_minor);
    box.innerHTML = (v.entries || []).map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + badge(x.kind) + ' <span class="text-muted">(' + esc(x.note) + ')</span></td><td>' + fmtRp(x.amount_minor) + '</td><td>' + esc(x.occurred_on) + '</td><td><button class="btn btn-sm btn-outline-danger" data-cash-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada mutasi kas.</td></tr>';
    qsa('[data-cash-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/cashbook/' + b.getAttribute('data-cash-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('cash-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('cash-msg', '');
        cashRefresh();
      });
    });
  }
  on('cash-add', 'click', async function () {
    if (!(+pval('cash-amount') > 0)) { pmsg('cash-msg', 'Isi nominal minor.'); return; }
    var r = await ppost('/api/v1/cashbook', { kind: pval('cash-kind') || 'in', amount_minor: +pval('cash-amount'), note: pval('cash-note') || '', occurred_on: pval('cash-date') || '' });
    if (!r.ok) { pmsg('cash-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('cash-msg', '');
    cashRefresh();
  });
  on('cash-reload', 'click', cashRefresh);
  /* ---- office: advances / IOU ---- */
  async function advRefresh() {
    var box = document.getElementById('adv-list');
    if (!box) return;
    var v = await j('/api/v1/advances');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var act = (x.status === 'open' ? '<button class="btn btn-sm btn-success" data-adv-ok="' + x.id + '" type="button">Lunas</button> <button class="btn btn-sm btn-warning" data-adv-no="' + x.id + '" type="button">Batal</button> ' : '<span class="text-muted">final</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-adv-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.holder) + ' <span class="text-muted">(' + esc(x.purpose) + ')</span></td><td>' + fmtRp(x.amount_minor) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada panjar.</td></tr>';
    qsa('[data-adv-ok]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/advances/' + b.getAttribute('data-adv-ok') + '/status', { status: 'settled' });
        if (!r.ok) { pmsg('adv-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('adv-msg', '');
        advRefresh();
      });
    });
    qsa('[data-adv-no]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/advances/' + b.getAttribute('data-adv-no') + '/status', { status: 'cancelled' });
        if (!r.ok) { pmsg('adv-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('adv-msg', '');
        advRefresh();
      });
    });
    qsa('[data-adv-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/advances/' + b.getAttribute('data-adv-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('adv-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('adv-msg', '');
        advRefresh();
      });
    });
  }
  on('adv-add', 'click', async function () {
    if (!pval('adv-holder') || !(+pval('adv-amount') > 0)) { pmsg('adv-msg', 'Isi pemegang dan nominal.'); return; }
    var r = await ppost('/api/v1/advances', { holder: pval('adv-holder'), amount_minor: +pval('adv-amount'), purpose: pval('adv-purpose') || '' });
    if (!r.ok) { pmsg('adv-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('adv-msg', '');
    advRefresh();
  });
  on('adv-reload', 'click', advRefresh);
  /* ---- office: inventory ---- */
  async function invRefresh() {
    var box = document.getElementById('inv-list');
    if (!box) return;
    var v = await j('/api/v1/inventory');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var st = x.low ? '<span class="badge bg-red-lt">menipis</span>' : '<span class="badge bg-green-lt">aman</span>';
      var act = '<button class="btn btn-sm" data-inv-plus="' + x.id + '" type="button">+10</button> <button class="btn btn-sm" data-inv-min="' + x.id + '" type="button">−10</button> '
        + '<button class="btn btn-sm btn-outline-danger" data-inv-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.code) + ' <span class="text-muted">(' + esc(x.name) + ')</span></td><td><strong>' + x.qty + '</strong> ' + esc(x.unit) + '</td><td>' + st + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada barang.</td></tr>';
    qsa('[data-inv-plus]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/inventory/' + b.getAttribute('data-inv-plus') + '/adjust', { delta: 10 });
        if (!r.ok) { pmsg('inv-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('inv-msg', '');
        invRefresh();
      });
    });
    qsa('[data-inv-min]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/inventory/' + b.getAttribute('data-inv-min') + '/adjust', { delta: -10 });
        if (!r.ok) { pmsg('inv-msg', 'Gagal (' + r.status + '). Stok tidak cukup.'); return; }
        pmsg('inv-msg', '');
        invRefresh();
      });
    });
    qsa('[data-inv-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/inventory/' + b.getAttribute('data-inv-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('inv-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('inv-msg', '');
        invRefresh();
      });
    });
  }
  on('inv-add', 'click', async function () {
    if (!pval('inv-code') || !pval('inv-name')) { pmsg('inv-msg', 'Isi kode dan nama.'); return; }
    var r = await ppost('/api/v1/inventory', { code: pval('inv-code'), name: pval('inv-name'), qty: +pval('inv-qty') || 0, unit: 'pcs', min_stock: +pval('inv-min') || 0 });
    if (!r.ok) { pmsg('inv-msg', 'Gagal (' + r.status + '). Kode mungkin duplikat.'); return; }
    pmsg('inv-msg', '');
    invRefresh();
  });
  on('inv-reload', 'click', invRefresh);
  /* ---- office: ATK (kind-filtered inventory) ---- */
  async function atkRefresh() {
    var box = document.getElementById('atk-list');
    if (!box) return;
    var v = await j('/api/v1/inventory?kind=atk');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var st = x.low ? '<span class="badge bg-red-lt">menipis</span>' : '<span class="badge bg-green-lt">aman</span>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.code) + ' <span class="text-muted">(' + esc(x.name) + ')</span></td><td><strong>' + x.qty + '</strong> ' + esc(x.unit) + '</td><td>' + st + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada ATK.</td></tr>';
  }
  on('atk-add', 'click', async function () {
    if (!pval('atk-code') || !pval('atk-name')) { pmsg('atk-msg', 'Isi kode dan nama.'); return; }
    var r = await ppost('/api/v1/inventory', { code: pval('atk-code'), name: pval('atk-name'), qty: +pval('atk-qty') || 0, unit: 'pcs', min_stock: 0, kind: 'atk' });
    if (!r.ok) { pmsg('atk-msg', 'Gagal (' + r.status + '). Kode mungkin duplikat.'); return; }
    pmsg('atk-msg', '');
    atkRefresh();
    invRefresh();
  });
  on('atk-reload', 'click', atkRefresh);
  /* ---- office: archive boxes ---- */
  async function aboxRefresh() {
    var box = document.getElementById('abox-list');
    if (!box) return;
    var v = await j('/api/v1/archive-boxes');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.box_code) + ' <span class="text-muted">(' + esc(x.description) + ')</span></td><td>' + esc(x.location) + '</td><td><button class="btn btn-sm" data-abox-mv="' + x.id + '" type="button">Pindah</button> <button class="btn btn-sm btn-outline-danger" data-abox-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada box arsip.</td></tr>';
    qsa('[data-abox-mv]', function (b) {
      b.addEventListener('click', async function () {
        var loc = (document.getElementById('abox-loc') || {}).value || '';
        if (!loc) { pmsg('abox-msg', 'Isi lokasi tujuan dulu.'); return; }
        var r = await ppost('/api/v1/archive-boxes/' + b.getAttribute('data-abox-mv') + '/move', { location: loc });
        if (!r.ok) { pmsg('abox-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('abox-msg', '');
        aboxRefresh();
      });
    });
    qsa('[data-abox-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/archive-boxes/' + b.getAttribute('data-abox-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('abox-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('abox-msg', '');
        aboxRefresh();
      });
    });
  }
  on('abox-add', 'click', async function () {
    if (!pval('abox-code')) { pmsg('abox-msg', 'Isi kode box.'); return; }
    var r = await ppost('/api/v1/archive-boxes', { box_code: pval('abox-code'), location: pval('abox-loc') || '', description: pval('abox-desc') || '' });
    if (!r.ok) { pmsg('abox-msg', 'Gagal (' + r.status + '). Kode mungkin duplikat.'); return; }
    pmsg('abox-msg', '');
    aboxRefresh();
  });
  on('abox-reload', 'click', aboxRefresh);
  /* ---- office: meterai usage ---- */
  async function useRefresh() {
    var box = document.getElementById('use-list');
    if (!box) return;
    var iid = pval('use-item');
    if (!iid) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Isi ID barang lalu Muat.</td></tr>'; return; }
    var v = await j('/api/v1/inventory/' + encodeURIComponent(iid) + '/usage');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + x.qty + '</td><td>' + esc(x.need) + '</td><td>' + esc(x.used_on) + '</td><td><button class="btn btn-sm btn-outline-danger" data-use-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada pemakaian.</td></tr>';
    qsa('[data-use-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/usage/' + b.getAttribute('data-use-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('use-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('use-msg', '');
        useRefresh();
        invRefresh();
      });
    });
  }
  on('use-add', 'click', async function () {
    if (!pval('use-item') || !(+pval('use-qty') > 0)) { pmsg('use-msg', 'Isi ID barang dan jumlah.'); return; }
    var r = await ppost('/api/v1/inventory/' + encodeURIComponent(pval('use-item')) + '/usage', { qty: +pval('use-qty'), need: pval('use-need') || '', used_on: pval('use-on') || '' });
    if (!r.ok) { pmsg('use-msg', 'Gagal (' + r.status + '). Stok tidak cukup?'); return; }
    pmsg('use-msg', '');
    useRefresh();
    invRefresh();
  });
  on('use-reload', 'click', useRefresh);
  /* ---- office: down-payments ---- */
  async function dpRefresh() {
    var box = document.getElementById('dp-list');
    if (!box) return;
    var v = await j('/api/v1/down-payments');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var act = (x.status === 'issued' ? '<button class="btn btn-sm btn-success" data-dp-ok="' + x.id + '" type="button">Terapkan</button> <button class="btn btn-sm btn-warning" data-dp-no="' + x.id + '" type="button">Refund</button> ' : '<span class="text-muted">final</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-dp-del="' + x.id + '" type="button">Hapus</button>';
      var ref = x.matter_id ? ' <span class="text-muted">(perkara #' + x.matter_id + ')</span>' : '';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.receipt_no) + ' <span class="text-muted">(' + esc(x.client_name) + ')' + ref + '</span></td><td>' + fmtRp(x.amount_minor) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada uang muka.</td></tr>';
    qsa('[data-dp-ok]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/down-payments/' + b.getAttribute('data-dp-ok') + '/status', { status: 'applied' });
        if (!r.ok) { pmsg('dp-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('dp-msg', '');
        dpRefresh();
      });
    });
    qsa('[data-dp-no]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/down-payments/' + b.getAttribute('data-dp-no') + '/status', { status: 'refunded' });
        if (!r.ok) { pmsg('dp-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('dp-msg', '');
        dpRefresh();
      });
    });
    qsa('[data-dp-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/down-payments/' + b.getAttribute('data-dp-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('dp-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('dp-msg', '');
        dpRefresh();
      });
    });
  }
  on('dp-add', 'click', async function () {
    if (!pval('dp-no') || !pval('dp-client') || !(+pval('dp-amount') > 0)) { pmsg('dp-msg', 'Isi kwitansi, klien, dan nominal.'); return; }
    var mid = pval('dp-matter');
    var r = await ppost('/api/v1/down-payments', { receipt_no: pval('dp-no'), client_name: pval('dp-client'), matter_id: mid ? +mid : null, amount_minor: +pval('dp-amount'), issued_on: pval('dp-date') || '' });
    if (!r.ok) { pmsg('dp-msg', 'Gagal (' + r.status + '). Nomor duplikat / perkara tidak ada?'); return; }
    pmsg('dp-msg', '');
    dpRefresh();
  });
  on('dp-reload', 'click', dpRefresh);
  /* ---- office: bank facilities ---- */
  async function bankRefresh() {
    var box = document.getElementById('bank-list');
    if (!box) return;
    var v = await j('/api/v1/bank-facilities');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var act = '<button class="btn btn-sm" data-bank-tg="' + x.id + '" data-bank-on="' + (!x.active) + '" type="button">' + (x.active ? 'Nonaktifkan' : 'Aktifkan') + '</button> '
        + '<button class="btn btn-sm btn-outline-danger" data-bank-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.bank_name) + '</td><td>' + esc(x.facility) + ' <span class="text-muted">(' + esc(x.rate_text) + ' / ' + esc(x.tenor) + ')</span></td><td>' + (x.active ? '<span class="badge bg-green-lt">aktif</span>' : '<span class="badge bg-yellow-lt">nonaktif</span>') + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada fasilitas.</td></tr>';
    qsa('[data-bank-tg]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/bank-facilities/' + b.getAttribute('data-bank-tg') + '/active', { active: b.getAttribute('data-bank-on') === 'true' });
        if (!r.ok) { pmsg('bank-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('bank-msg', '');
        bankRefresh();
      });
    });
    qsa('[data-bank-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/bank-facilities/' + b.getAttribute('data-bank-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('bank-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('bank-msg', '');
        bankRefresh();
      });
    });
  }
  on('bank-add', 'click', async function () {
    if (!pval('bank-name') || !pval('bank-fac')) { pmsg('bank-msg', 'Isi bank dan fasilitas.'); return; }
    var r = await ppost('/api/v1/bank-facilities', { bank_name: pval('bank-name'), facility: pval('bank-fac'), rate_text: pval('bank-rate') || '', tenor: pval('bank-tenor') || '' });
    if (!r.ok) { pmsg('bank-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('bank-msg', '');
    bankRefresh();
  });
  on('bank-reload', 'click', bankRefresh);
  /* ---- office: feedback ---- */
  var FB_NEXT = { new: 'reviewed', reviewed: 'archived' };
  async function fbRefresh() {
    var box = document.getElementById('fb-list');
    if (!box) return;
    var v = await j('/api/v1/feedbacks');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = FB_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-fb-next="' + x.id + '" data-fb-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">arsip</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-fb-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.name) + '</td><td>' + esc(x.message) + '</td><td>' + x.rating + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada umpan balik.</td></tr>';
    qsa('[data-fb-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/feedbacks/' + b.getAttribute('data-fb-next') + '/status', { status: b.getAttribute('data-fb-to') });
        if (!r.ok) { pmsg('fb-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('fb-msg', '');
        fbRefresh();
      });
    });
    qsa('[data-fb-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/feedbacks/' + b.getAttribute('data-fb-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('fb-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('fb-msg', '');
        fbRefresh();
      });
    });
  }
  on('fb-add', 'click', async function () {
    if (!pval('fb-msg-in')) { pmsg('fb-msg', 'Isi pesan umpan balik.'); return; }
    var r = await ppost('/api/v1/feedbacks', { name: pval('fb-name') || '', message: pval('fb-msg-in'), rating: +pval('fb-rating') || 0 });
    if (!r.ok) { pmsg('fb-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('fb-msg', '');
    fbRefresh();
  });
  on('fb-reload', 'click', fbRefresh);
  /* ---- office: klapper ---- */
  async function klapperRefresh() {
    var box = document.getElementById('klapper-list');
    if (!box) return;
    var v = await j('/api/v1/klapper');
    if (!v) { box.innerHTML = '<tr><td colspan="3" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td><strong>' + esc(x.name) + '</strong></td><td>' + badge(x.kind) + '</td><td class="text-muted">' + esc(x.reference) + '</td></tr>';
    }).join('') || '<tr><td colspan="3" class="text-muted">Indeks kosong.</td></tr>';
  }
  on('klapper-reload', 'click', klapperRefresh);
  /* ---- office: performance dashboard ---- */
  async function perfRefresh() {
    var box = document.getElementById('perf-sum');
    if (!box) return;
    var v = await j('/api/v1/reports/office-performance');
    if (!v) { box.innerHTML = '<p class="text-muted">Gagal memuat.</p>'; return; }
    var states = Object.keys(v.deeds_by_state || {}).map(function (k) { return k + ': ' + v.deeds_by_state[k]; }).join(' • ') || '—';
    box.innerHTML = '<div class="table-responsive"><table class="table card-table"><tbody>'
      + '<tr><td>Total akta</td><td><strong>' + v.deeds_total + '</strong></td></tr>'
      + '<tr><td>Akta per status</td><td>' + esc(states) + '</td></tr>'
      + '<tr><td>Janji temu (selesai / total)</td><td><strong>' + v.appointments_done + ' / ' + v.appointments_total + '</strong></td></tr>'
      + '<tr><td>Quotation diterima (Rp)</td><td><strong>' + fmtRp(v.quotations_accepted_sum_minor) + '</strong> (' + v.quotations_total + ' quotation)</td></tr>'
      + '<tr><td>Tamu (di dalam / total)</td><td><strong>' + v.visitors_inside + ' / ' + v.visitors_total + '</strong></td></tr>'
      + '</tbody></table></div>';
  }
  on('perf-reload', 'click', perfRefresh);
  /* ---- office: deed volume ---- */
  async function volRefresh() {
    var box = document.getElementById('vol-list');
    if (!box) return;
    var g = pval('vol-group') || 'month';
    var v = await j('/api/v1/reports/deed-volume?group=' + encodeURIComponent(g));
    if (!v) { box.innerHTML = '<tr><td colspan="2" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = (v.buckets || []).map(function (x) {
      return '<tr><td>' + esc(x.key) + '</td><td><strong>' + x.count + '</strong></td></tr>';
    }).join('') || '<tr><td colspan="2" class="text-muted">Belum ada akta.</td></tr>';
  }
  on('vol-group', 'change', volRefresh);
  on('vol-reload', 'click', volRefresh);
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
  /* ---- office: multi-desk dispatch board ---- */
  async function dspRefresh() {
    var box = document.getElementById('dsp-board');
    if (!box) return;
    var v = await j('/api/v1/tasks/dispatch');
    if (!v) { box.innerHTML = '<p class="text-muted">Gagal memuat.</p>'; return; }
    box.innerHTML = v.map(function (g) {
      return '<h4 class="mt-2">' + esc(g.desk) + ' (' + g.tasks.length + ')</h4><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Status</th><th>Petugas</th></tr></thead><tbody>'
        + g.tasks.map(function (x) {
          return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + badge(x.status) + '</td><td>' + esc(x.assignee || '—') + '</td></tr>';
        }).join('') + '</tbody></table></div>';
    }).join('') || '<p class="text-muted">Tidak ada tugas terbuka.</p>';
  }
  on('dsp-add', 'click', async function () {
    if (!pval('dsp-task')) { pmsg('dsp-msg', 'Isi ID tugas.'); return; }
    var r = await ppost('/api/v1/tasks/' + encodeURIComponent(pval('dsp-task')) + '/assign', { assignee: pval('dsp-who') || '', desk: pval('dsp-desk') || '' });
    if (!r.ok) { pmsg('dsp-msg', 'Gagal (' + r.status + '). Tugas tidak ada?'); return; }
    pmsg('dsp-msg', '');
    dspRefresh();
    loadTasks();
  });
  on('dsp-reload', 'click', dspRefresh);
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
      act += ' <button class="btn btn-sm btn-outline-primary" data-ppat-deed="' + x.id + '" type="button">Buat akta</button>';
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
    qsa('[data-ppat-deed]', function (b) {
      b.addEventListener('click', async function () {
        var id = b.getAttribute('data-ppat-deed');
        var v = await j('/api/v1/ppat');
        if (!v) return;
        var r = (v || []).filter(function (x) { return String(x.id) === String(id); })[0];
        if (!r) return;
        window.apptab('akta');
        var nn = document.getElementById('deed-number'); if (nn) nn.value = r.number;
        var dt = document.getElementById('deed-type'); if (dt) dt.value = r.deed_kind;
        pmsg('deed-msg', 'Nomor & jenis terisi dari register PPAT #' + r.id + '. Pilih template lalu buat akta.');
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
  /* ---- property: protocol books ---- */
  var PB_NEXT = { open: 'closed', closed: 'sealed' };
  async function pbRefresh() {
    var box = document.getElementById('pb-list');
    if (!box) return;
    var v = await j('/api/v1/protocol-books');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = PB_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-pb-next="' + x.id + '" data-pb-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">disegel</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-pb-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.book_no) + '</td><td>' + esc(x.year) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada buku protokol.</td></tr>';
    qsa('[data-pb-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/protocol-books/' + b.getAttribute('data-pb-next') + '/status', { status: b.getAttribute('data-pb-to') });
        if (!r.ok) { pmsg('pb-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('pb-msg', '');
        pbRefresh();
      });
    });
    qsa('[data-pb-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/protocol-books/' + b.getAttribute('data-pb-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('pb-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('pb-msg', '');
        pbRefresh();
      });
    });
  }
  on('pb-add', 'click', async function () {
    if (!pval('pb-no')) { pmsg('pb-msg', 'Isi nomor buku.'); return; }
    var r = await ppost('/api/v1/protocol-books', { book_no: pval('pb-no'), year: pval('pb-year') || '' });
    if (!r.ok) { pmsg('pb-msg', 'Gagal (' + r.status + '). Nomor mungkin duplikat.'); return; }
    pmsg('pb-msg', '');
    pbRefresh();
  });
  on('pb-reload', 'click', pbRefresh);
  /* ---- property: custody handover log ---- */
  async function phRefresh() {
    var box = document.getElementById('ph-list');
    if (!box) return;
    var bid = pval('ph-book');
    if (!bid) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Isi ID buku lalu Muat.</td></tr>'; return; }
    var v = await j('/api/v1/protocol-books/' + encodeURIComponent(bid) + '/handovers');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.from_holder) + ' → ' + esc(x.to_holder) + '</td><td>' + esc(x.handed_at) + '</td><td><button class="btn btn-sm btn-outline-danger" data-ph-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada serah terima.</td></tr>';
    qsa('[data-ph-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/protocol-handovers/' + b.getAttribute('data-ph-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('ph-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('ph-msg', '');
        phRefresh();
      });
    });
  }
  on('ph-add', 'click', async function () {
    if (!pval('ph-book') || !pval('ph-from') || !pval('ph-to')) { pmsg('ph-msg', 'Isi ID buku, dari, dan kepada.'); return; }
    var r = await ppost('/api/v1/protocol-books/' + encodeURIComponent(pval('ph-book')) + '/handovers', { from_holder: pval('ph-from'), to_holder: pval('ph-to'), handed_at: pval('ph-at') || '', note: pval('ph-note') || '' });
    if (!r.ok) { pmsg('ph-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('ph-msg', '');
    phRefresh();
  });
  on('ph-reload', 'click', phRefresh);
  /* ---- property: storage boxes ---- */
  var SBOX_NEXT = { available: 'full', full: 'sealed' };
  async function sboxRefresh() {
    var box = document.getElementById('sbox-list');
    if (!box) return;
    var v = await j('/api/v1/storage-boxes');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = SBOX_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-sbox-next="' + x.id + '" data-sbox-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">disegel</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-sbox-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.box_code) + ' <span class="text-muted">(kap. ' + x.capacity + ')</span></td><td>' + esc(x.location) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada box.</td></tr>';
    qsa('[data-sbox-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/storage-boxes/' + b.getAttribute('data-sbox-next') + '/status', { status: b.getAttribute('data-sbox-to') });
        if (!r.ok) { pmsg('sbox-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('sbox-msg', '');
        sboxRefresh();
      });
    });
    qsa('[data-sbox-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/storage-boxes/' + b.getAttribute('data-sbox-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('sbox-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('sbox-msg', '');
        sboxRefresh();
      });
    });
  }
  on('sbox-add', 'click', async function () {
    if (!pval('sbox-code')) { pmsg('sbox-msg', 'Isi kode box.'); return; }
    var r = await ppost('/api/v1/storage-boxes', { box_code: pval('sbox-code'), location: pval('sbox-loc') || '', capacity: +pval('sbox-cap') || 0 });
    if (!r.ok) { pmsg('sbox-msg', 'Gagal (' + r.status + '). Kode mungkin duplikat.'); return; }
    pmsg('sbox-msg', '');
    sboxRefresh();
  });
  on('sbox-reload', 'click', sboxRefresh);
  /* ---- office: year-end seals ---- */
  async function sealRefresh() {
    var box = document.getElementById('seal-list');
    if (!box) return;
    var v = await j('/api/v1/year-seals');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var act = (x.status === 'prepared' ? '<button class="btn btn-sm btn-success" data-seal-go="' + x.id + '" type="button">Segel</button> ' : '<span class="text-muted">disegel</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-seal-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.year) + '</td><td>' + x.book_count + ' buku</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada penyegelan.</td></tr>';
    qsa('[data-seal-go]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/year-seals/' + b.getAttribute('data-seal-go') + '/seal', {});
        if (!r.ok) { pmsg('seal-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('seal-msg', '');
        sealRefresh();
      });
    });
    qsa('[data-seal-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/year-seals/' + b.getAttribute('data-seal-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('seal-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('seal-msg', '');
        sealRefresh();
      });
    });
  }
  on('seal-add', 'click', async function () {
    if (!pval('seal-year')) { pmsg('seal-msg', 'Isi tahun (YYYY).'); return; }
    var r = await ppost('/api/v1/year-seals', { year: pval('seal-year'), note: pval('seal-note') || '' });
    if (!r.ok) { pmsg('seal-msg', 'Gagal (' + r.status + '). Tahun duplikat?'); return; }
    pmsg('seal-msg', '');
    sealRefresh();
  });
  on('seal-reload', 'click', sealRefresh);
  /* ---- property: zones ---- */
  async function zoneRefresh() {
    var box = document.getElementById('zone-list');
    if (!box) return;
    var v = await j('/api/v1/zones');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.area_code) + ' <span class="text-muted">(' + esc(x.area_name) + ')</span></td><td>' + fmtRp(x.njop_minor) + '</td><td><button class="btn btn-sm btn-outline-danger" data-zone-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada zona.</td></tr>';
    qsa('[data-zone-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/zones/' + b.getAttribute('data-zone-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('zone-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('zone-msg', '');
        zoneRefresh();
      });
    });
  }
  on('zone-add', 'click', async function () {
    if (!pval('zone-code') || !pval('zone-name')) { pmsg('zone-msg', 'Isi kode dan nama area.'); return; }
    var r = await ppost('/api/v1/zones', { area_code: pval('zone-code'), area_name: pval('zone-name'), njop_minor: +pval('zone-njop') || 0 });
    if (!r.ok) { pmsg('zone-msg', 'Gagal (' + r.status + '). Kode mungkin duplikat.'); return; }
    pmsg('zone-msg', '');
    zoneRefresh();
  });
  on('zone-reload', 'click', zoneRefresh);
  /* ---- property: tax worksheets ---- */
  on('tax-calc', 'click', async function () {
    var box = document.getElementById('tax-out');
    var v = +pval('tax-value') || 0;
    if (!(v > 0)) { pmsg('tax-msg', 'Isi nilai transaksi minor.'); return; }
    var npoptkp = pval('tax-npoptkp');
    var p = await j('/api/v1/worksheets/pph?value_minor=' + v);
    var b = await j('/api/v1/worksheets/bphtb?npop=' + v + (npoptkp ? '&npoptkp=' + encodeURIComponent(npoptkp) : ''));
    var f = await j('/api/v1/worksheets/ppat-fee?value_minor=' + v);
    if (!p || !b || !f) { pmsg('tax-msg', 'Gagal menghitung.'); if (box) box.innerHTML = ''; return; }
    pmsg('tax-msg', '');
    if (box) box.innerHTML = '<div class="table-responsive"><table class="table card-table"><tbody>'
      + '<tr><td>PPh (' + p.rate_pct + '%)</td><td><strong>' + fmtRp(p.tax_minor) + '</strong></td></tr>'
      + '<tr><td>BPHTB (' + b.rate_pct + '% × (' + fmtRp(b.npop) + ' − ' + fmtRp(b.npoptkp) + '))</td><td><strong>' + fmtRp(b.tax_minor) + '</strong></td></tr>'
      + '<tr><td>Fee PPAT maks (' + f.rate_pct + '%)</td><td><strong>' + fmtRp(f.fee_minor) + '</strong></td></tr>'
      + '</tbody></table></div>';
  });
  /* ---- property: PBB checks ---- */
  async function pbbRefresh() {
    var box = document.getElementById('pbb-list');
    if (!box) return;
    var v = await j('/api/v1/pbb-checks');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.nop) + '</td><td>' + esc(x.tax_year) + '</td><td>' + badge(x.result) + '</td><td><button class="btn btn-sm btn-outline-danger" data-pbb-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada verifikasi.</td></tr>';
    qsa('[data-pbb-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/pbb-checks/' + b.getAttribute('data-pbb-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('pbb-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('pbb-msg', '');
        pbbRefresh();
      });
    });
  }
  on('pbb-add', 'click', async function () {
    if (!pval('pbb-nop') || !pval('pbb-year')) { pmsg('pbb-msg', 'Isi NOP dan tahun (YYYY).'); return; }
    var r = await ppost('/api/v1/pbb-checks', { nop: pval('pbb-nop'), tax_year: pval('pbb-year'), result: pval('pbb-res') || 'unknown', checked_on: pval('pbb-on') || '', note: '' });
    if (!r.ok) { pmsg('pbb-msg', 'Gagal (' + r.status + '). Tahun harus YYYY.'); return; }
    pmsg('pbb-msg', '');
    pbbRefresh();
  });
  on('pbb-reload', 'click', pbbRefresh);
  /* ---- property: land split + merge ---- */
  var LAND_NEXT = { planned: 'surveyed', surveyed: 'registered' };
  function landRow(prefix, x, cols) {
    var nx = LAND_NEXT[x.status];
    var act = (nx ? '<button class="btn btn-sm" data-' + prefix + '-next="' + x.id + '" data-' + prefix + '-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">terdaftar</span> ')
      + '<button class="btn btn-sm btn-outline-danger" data-' + prefix + '-del="' + x.id + '" type="button">Hapus</button>';
    return '<tr><td>' + x.id + '</td>' + cols + '<td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
  }
  async function spRefresh() {
    var box = document.getElementById('sp-list');
    if (!box) return;
    var v = await j('/api/v1/land-splits');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return landRow('sp', x, '<td>' + esc(x.source_parcel) + '</td><td>' + esc(x.new_parcels) + '</td>');
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada pemisahan.</td></tr>';
    qsa('[data-sp-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/land-splits/' + b.getAttribute('data-sp-next') + '/status', { status: b.getAttribute('data-sp-to') });
        if (!r.ok) { pmsg('sp-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('sp-msg', '');
        spRefresh();
      });
    });
    qsa('[data-sp-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/land-splits/' + b.getAttribute('data-sp-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('sp-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('sp-msg', '');
        spRefresh();
      });
    });
  }
  on('sp-add', 'click', async function () {
    if (!pval('sp-src') || !pval('sp-new')) { pmsg('sp-msg', 'Isi sumber dan hasil.'); return; }
    var r = await ppost('/api/v1/land-splits', { source_parcel: pval('sp-src'), new_parcels: pval('sp-new'), split_date: pval('sp-date') || '' });
    if (!r.ok) { pmsg('sp-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('sp-msg', '');
    spRefresh();
  });
  on('sp-reload', 'click', spRefresh);
  async function mgRefresh() {
    var box = document.getElementById('mg-list');
    if (!box) return;
    var v = await j('/api/v1/land-merges');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return landRow('mg', x, '<td>' + esc(x.source_parcels) + '</td><td>' + esc(x.result_parcel) + '</td>');
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada penggabungan.</td></tr>';
    qsa('[data-mg-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/land-merges/' + b.getAttribute('data-mg-next') + '/status', { status: b.getAttribute('data-mg-to') });
        if (!r.ok) { pmsg('mg-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('mg-msg', '');
        mgRefresh();
      });
    });
    qsa('[data-mg-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/land-merges/' + b.getAttribute('data-mg-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('mg-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('mg-msg', '');
        mgRefresh();
      });
    });
  }
  on('mg-add', 'click', async function () {
    if (!pval('mg-src') || !pval('mg-res')) { pmsg('mg-msg', 'Isi sumber dan hasil.'); return; }
    var r = await ppost('/api/v1/land-merges', { source_parcels: pval('mg-src'), result_parcel: pval('mg-res'), merge_date: pval('mg-date') || '' });
    if (!r.ok) { pmsg('mg-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('mg-msg', '');
    mgRefresh();
  });
  on('mg-reload', 'click', mgRefresh);
  /* ---- property: dispute flags ---- */
  var DSP2_NEXT = { flagged: 'in_review', in_review: 'cleared' };
  async function dsp2Refresh() {
    var box = document.getElementById('dsp2-list');
    if (!box) return;
    var v = await j('/api/v1/land-disputes');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = DSP2_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-dsp2-next="' + x.id + '" data-dsp2-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">selesai</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-dsp2-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.parcel_ref) + '</td><td>' + badge(x.kind) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada flag.</td></tr>';
    qsa('[data-dsp2-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/land-disputes/' + b.getAttribute('data-dsp2-next') + '/status', { status: b.getAttribute('data-dsp2-to') });
        if (!r.ok) { pmsg('dsp2-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('dsp2-msg', '');
        dsp2Refresh();
      });
    });
    qsa('[data-dsp2-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/land-disputes/' + b.getAttribute('data-dsp2-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('dsp2-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('dsp2-msg', '');
        dsp2Refresh();
      });
    });
  }
  on('dsp2-add', 'click', async function () {
    if (!pval('dsp2-parcel') || !pval('dsp2-desc')) { pmsg('dsp2-msg', 'Isi bidang dan uraian.'); return; }
    var r = await ppost('/api/v1/land-disputes', { parcel_ref: pval('dsp2-parcel'), kind: pval('dsp2-kind') || 'sengketa', description: pval('dsp2-desc') });
    if (!r.ok) { pmsg('dsp2-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('dsp2-msg', '');
    dsp2Refresh();
  });
  on('dsp2-reload', 'click', dsp2Refresh);
  /* ---- property: boundary checks ---- */
  async function bndRefresh() {
    var box = document.getElementById('bnd-list');
    if (!box) return;
    var v = await j('/api/v1/boundary-checks');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var rc = x.result === 'match' ? '<span class="badge bg-green-lt">cocok</span>' : (x.result === 'mismatch' ? '<span class="badge bg-red-lt">selisih</span>' : '<span class="badge bg-yellow-lt">tunda</span>');
      return '<tr><td>' + x.id + '</td><td>' + esc(x.parcel_ref) + '</td><td>' + x.declared_area + ' vs ' + x.measured_area + ' m²</td><td>' + rc + '</td><td><button class="btn btn-sm btn-outline-danger" data-bnd-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada verifikasi.</td></tr>';
    qsa('[data-bnd-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/boundary-checks/' + b.getAttribute('data-bnd-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('bnd-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('bnd-msg', '');
        bndRefresh();
      });
    });
  }
  on('bnd-add', 'click', async function () {
    if (!pval('bnd-parcel')) { pmsg('bnd-msg', 'Isi bidang.'); return; }
    var r = await ppost('/api/v1/boundary-checks', { parcel_ref: pval('bnd-parcel'), declared_area: +pval('bnd-decl') || 0, measured_area: +pval('bnd-meas') || 0, checked_on: pval('bnd-on') || '' });
    if (!r.ok) { pmsg('bnd-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('bnd-msg', '');
    bndRefresh();
  });
  on('bnd-reload', 'click', bndRefresh);
  /* ---- property: deed consents (marital / heir) ---- */
  function consentRow(prefix, x) {
    var st = x.verified ? '<span class="badge bg-green-lt">terverifikasi</span>' : '<span class="badge bg-yellow-lt">menunggu</span>';
    var act = (!x.verified ? '<button class="btn btn-sm" data-' + prefix + '-ok="' + x.id + '" type="button">Verifikasi</button> ' : '')
      + '<button class="btn btn-sm btn-outline-danger" data-' + prefix + '-del="' + x.id + '" type="button">Hapus</button>';
    return '<tr><td>' + x.id + '</td><td>' + esc(x.party) + '</td><td>' + st + '</td><td>' + act + '</td></tr>';
  }
  function consentRefresh(prefix, pidId, boxId, msgId, kind) {
    var box = document.getElementById(boxId);
    if (!box) return;
    var pid = pval(pidId);
    if (!pid) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Isi ID akta PPAT lalu Muat.</td></tr>'; return; }
    j('/api/v1/ppat/' + encodeURIComponent(pid) + '/consents').then(function (v) {
      if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
      var rows = v.filter(function (x) { return x.kind === kind; });
      box.innerHTML = rows.map(function (x) { return consentRow(prefix, x); }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada catatan.</td></tr>';
      qsa('[data-' + prefix + '-ok]', function (b) {
        b.addEventListener('click', async function () {
          var r = await ppost('/api/v1/consents/' + b.getAttribute('data-' + prefix + '-ok') + '/verify', { verified: true });
          if (!r.ok) { pmsg(msgId, 'Gagal (' + r.status + ').'); return; }
          pmsg(msgId, '');
          consentRefresh(prefix, pidId, boxId, msgId, kind);
        });
      });
      qsa('[data-' + prefix + '-del]', function (b) {
        b.addEventListener('click', async function () {
          var r = await fetch('/api/v1/consents/' + b.getAttribute('data-' + prefix + '-del'), { method: 'DELETE', headers: H });
          if (!r.ok) { pmsg(msgId, 'Gagal (' + r.status + ').'); return; }
          pmsg(msgId, '');
          consentRefresh(prefix, pidId, boxId, msgId, kind);
        });
      });
    });
  }
  on('mrt-add', 'click', async function () {
    if (!pval('mrt-ppat') || !pval('mrt-party')) { pmsg('mrt-msg', 'Isi ID akta dan nama pihak.'); return; }
    var r = await ppost('/api/v1/ppat/' + encodeURIComponent(pval('mrt-ppat')) + '/consents', { kind: 'marital', party: pval('mrt-party') });
    if (!r.ok) { pmsg('mrt-msg', 'Gagal (' + r.status + '). Akta tidak ada?'); return; }
    pmsg('mrt-msg', '');
    consentRefresh('mrt', 'mrt-ppat', 'mrt-list', 'mrt-msg', 'marital');
  });
  on('mrt-reload', 'click', function () { consentRefresh('mrt', 'mrt-ppat', 'mrt-list', 'mrt-msg', 'marital'); });
  on('hir-add', 'click', async function () {
    if (!pval('hir-ppat') || !pval('hir-party')) { pmsg('hir-msg', 'Isi ID akta dan nama pihak.'); return; }
    var r = await ppost('/api/v1/ppat/' + encodeURIComponent(pval('hir-ppat')) + '/consents', { kind: 'heir', party: pval('hir-party') });
    if (!r.ok) { pmsg('hir-msg', 'Gagal (' + r.status + '). Akta tidak ada?'); return; }
    pmsg('hir-msg', '');
    consentRefresh('hir', 'hir-ppat', 'hir-list', 'hir-msg', 'heir');
  });
  on('hir-reload', 'click', function () { consentRefresh('hir', 'hir-ppat', 'hir-list', 'hir-msg', 'heir'); });
  on('bod-add', 'click', async function () {
    if (!pval('bod-ppat') || !pval('bod-party')) { pmsg('bod-msg', 'Isi ID akta dan nama pihak.'); return; }
    var r = await ppost('/api/v1/ppat/' + encodeURIComponent(pval('bod-ppat')) + '/consents', { kind: 'board', party: pval('bod-party') });
    if (!r.ok) { pmsg('bod-msg', 'Gagal (' + r.status + '). Akta tidak ada?'); return; }
    pmsg('bod-msg', '');
    consentRefresh('bod', 'bod-ppat', 'bod-list', 'bod-msg', 'board');
  });
  on('bod-reload', 'click', function () { consentRefresh('bod', 'bod-ppat', 'bod-list', 'bod-msg', 'board'); });
  on('pow-add', 'click', async function () {
    if (!pval('pow-ppat') || !pval('pow-party')) { pmsg('pow-msg', 'Isi ID akta dan nama pihak.'); return; }
    var r = await ppost('/api/v1/ppat/' + encodeURIComponent(pval('pow-ppat')) + '/consents', { kind: 'power', party: pval('pow-party') });
    if (!r.ok) { pmsg('pow-msg', 'Gagal (' + r.status + '). Akta tidak ada?'); return; }
    pmsg('pow-msg', '');
    consentRefresh('pow', 'pow-ppat', 'pow-list', 'pow-msg', 'power');
  });
  on('pow-reload', 'click', function () { consentRefresh('pow', 'pow-ppat', 'pow-list', 'pow-msg', 'power'); });
  /* ---- property: land history ---- */
  async function lhiRefresh() {
    var box = document.getElementById('lhi-list');
    if (!box) return;
    var parcel = pval('lhi-parcel');
    if (!parcel) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Isi referensi bidang lalu Muat.</td></tr>'; return; }
    var v = await j('/api/v1/land-history?parcel_ref=' + encodeURIComponent(parcel));
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + badge(x.event) + '</td><td>' + esc(x.event_date) + '</td><td>' + esc(x.detail) + '</td><td><button class="btn btn-sm btn-outline-danger" data-lhi-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada riwayat.</td></tr>';
    qsa('[data-lhi-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/land-history/' + b.getAttribute('data-lhi-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('lhi-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('lhi-msg', '');
        lhiRefresh();
      });
    });
  }
  on('lhi-add', 'click', async function () {
    if (!pval('lhi-parcel') || !pval('lhi-detail')) { pmsg('lhi-msg', 'Isi bidang dan uraian.'); return; }
    var r = await ppost('/api/v1/land-history', { parcel_ref: pval('lhi-parcel'), event: pval('lhi-event') || 'alas_hak', detail: pval('lhi-detail'), event_date: pval('lhi-date') || '' });
    if (!r.ok) { pmsg('lhi-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('lhi-msg', '');
    lhiRefresh();
  });
  on('lhi-reload', 'click', lhiRefresh);
  /* ---- property: unit handovers (BAST) ---- */
  var UHO_NEXT = { draft: 'signed', signed: 'delivered' };
  async function uhoRefresh() {
    var box = document.getElementById('uho-list');
    if (!box) return;
    var v = await j('/api/v1/unit-handovers');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = UHO_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-uho-next="' + x.id + '" data-uho-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">diserahkan</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-uho-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.unit_ref) + '</td><td>' + esc(x.bast_no) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada serah terima.</td></tr>';
    qsa('[data-uho-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/unit-handovers/' + b.getAttribute('data-uho-next') + '/status', { status: b.getAttribute('data-uho-to') });
        if (!r.ok) { pmsg('uho-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('uho-msg', '');
        uhoRefresh();
      });
    });
    qsa('[data-uho-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/unit-handovers/' + b.getAttribute('data-uho-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('uho-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('uho-msg', '');
        uhoRefresh();
      });
    });
  }
  on('uho-add', 'click', async function () {
    if (!pval('uho-unit') || !pval('uho-from') || !pval('uho-to') || !pval('uho-bast')) { pmsg('uho-msg', 'Lengkapi unit, pihak, dan no. BAST.'); return; }
    var r = await ppost('/api/v1/unit-handovers', { unit_ref: pval('uho-unit'), handed_from: pval('uho-from'), handed_to: pval('uho-to'), bast_no: pval('uho-bast'), handed_on: pval('uho-on') || '' });
    if (!r.ok) { pmsg('uho-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('uho-msg', '');
    uhoRefresh();
  });
  on('uho-reload', 'click', uhoRefresh);
  /* ---- property: PPAT monthly report + dashboard ---- */
  async function rpmRefresh() {
    var box = document.getElementById('rpm-list');
    if (!box) return;
    var mo = pval('rpm-month');
    var v = await j('/api/v1/reports/ppat-monthly' + (mo ? '?month=' + encodeURIComponent(mo) : ''));
    if (!v) { box.innerHTML = '<tr><td colspan="3" class="text-muted">Gagal memuat.</td></tr>'; return; }
    var tot = document.getElementById('rpm-total');
    if (tot) tot.textContent = 'Bulan ' + v.month + ' — total ' + v.total + ' akta';
    box.innerHTML = (v.rows || []).map(function (x) {
      return '<tr><td>' + badge(x.kind) + '</td><td>' + badge(x.status) + '</td><td>' + x.n + '</td></tr>';
    }).join('') || '<tr><td colspan="3" class="text-muted">Belum ada akta bulan ini.</td></tr>';
  }
  on('rpm-reload', 'click', rpmRefresh);
  async function pdbRefresh() {
    var box = document.getElementById('pdb-list');
    if (!box) return;
    var v = await j('/api/v1/ppat-dashboard');
    if (!v) { box.innerHTML = '<tr><td colspan="2" class="text-muted">Gagal memuat.</td></tr>'; return; }
    var kinds = (v.by_kind || []).map(function (x) { return x.kind + ': ' + x.n; }).join(', ') || '—';
    var rows = [
      ['Total akta', v.total_deeds],
      ['Per jenis', kinds],
      ['Persetujuan belum verifikasi', v.unverified_consents],
      ['Pembebanan jatuh tempo ≤30 hari', v.charges_due_30],
      ['Sengketa terbuka', v.open_disputes],
      ['Serah terima tertunda', v.pending_handovers],
    ];
    box.innerHTML = rows.map(function (r) {
      return '<tr><td>' + r[0] + '</td><td><strong>' + r[1] + '</strong></td></tr>';
    }).join('');
  }
  on('pdb-reload', 'click', pdbRefresh);
  /* ---- property: notary monthly pack + MPN pack ---- */
  async function nmpRefresh() {
    var box = document.getElementById('nmp-list');
    if (!box) return;
    var mo = pval('nmp-month');
    var v = await j('/api/v1/reports/notary-monthly-pack' + (mo ? '?month=' + encodeURIComponent(mo) : ''));
    if (!v) { box.innerHTML = '<tr><td colspan="3" class="text-muted">Gagal memuat.</td></tr>'; return; }
    var tot = document.getElementById('nmp-total');
    if (tot) tot.textContent = 'Bulan ' + v.month + ' — ' + v.ppat_total + ' akta, ' + v.dashboard.total_deeds + ' total berjalan';
    box.innerHTML = (v.ppat_rows || []).map(function (x) {
      return '<tr><td>' + badge(x.kind) + '</td><td>' + badge(x.status) + '</td><td>' + x.n + '</td></tr>';
    }).join('') || '<tr><td colspan="3" class="text-muted">Belum ada akta bulan ini.</td></tr>';
  }
  on('nmp-reload', 'click', nmpRefresh);
  async function mpnRefresh() {
    var box = document.getElementById('mpn-list');
    if (!box) return;
    var v = await j('/api/v1/reports/mpn-pack');
    if (!v) { box.innerHTML = '<tr><td colspan="2" class="text-muted">Gagal memuat.</td></tr>'; return; }
    var gen = document.getElementById('mpn-gen');
    if (gen) gen.textContent = 'Dibuat ' + v.generated_at;
    var rows = [
      ['Entri register protokol', v.protocol_entries],
      ['Purge menunggu/tersetujui', v.pending_purges],
      ['Aturan retensi aktif', v.active_retention_rules],
      ['Peristiwa properti terkini', (v.recent_property_events || []).length],
    ];
    box.innerHTML = rows.map(function (r) {
      return '<tr><td>' + r[0] + '</td><td><strong>' + r[1] + '</strong></td></tr>';
    }).join('');
    var ev = document.getElementById('mpn-events');
    if (ev) ev.innerHTML = (v.recent_property_events || []).map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.action) + ' <span class="text-muted">(' + esc(x.entity) + ')</span></td><td>' + esc(x.created) + '</td></tr>';
    }).join('') || '<tr><td colspan="3" class="text-muted">Belum ada peristiwa.</td></tr>';
  }
  on('mpn-reload', 'click', mpnRefresh);
  /* ---- property: foreign guardrail ---- */
  on('fgd-check', 'click', async function () {
    var out = document.getElementById('fgd-out');
    var v = await j('/api/v1/foreign-guard?subject=' + encodeURIComponent(pval('fgd-sub')) + '&right=' + encodeURIComponent(pval('fgd-right')));
    if (!v) { pmsg('fgd-msg', 'Gagal memeriksa.'); return; }
    pmsg('fgd-msg', '');
    if (out) out.innerHTML = (v.allowed ? '<span class="badge bg-green-lt">diizinkan</span>' : '<span class="badge bg-red-lt">dilarang</span>') + ' <span class="text-muted">' + esc(v.rule) + '</span>';
  });
  /* ---- property: notary handoff checklist ---- */
  async function nhoRefresh() {
    var box = document.getElementById('nho-list');
    if (!box) return;
    var pid = pval('nho-ppat');
    if (!pid) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Isi ID akta PPAT lalu Muat.</td></tr>'; return; }
    var v = await j('/api/v1/ppat/' + encodeURIComponent(pid) + '/handoffs');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var st = x.received ? '<span class="badge bg-green-lt">diterima</span>' : '<span class="badge bg-yellow-lt">menunggu</span>';
      var act = (!x.received ? '<button class="btn btn-sm" data-nho-ok="' + x.id + '" type="button">Tandai diterima</button> ' : '')
        + '<button class="btn btn-sm btn-outline-danger" data-nho-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.doc_name) + '</td><td>' + st + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada dokumen.</td></tr>';
    qsa('[data-nho-ok]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/handoffs/' + b.getAttribute('data-nho-ok') + '/received', { received: true });
        if (!r.ok) { pmsg('nho-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('nho-msg', '');
        nhoRefresh();
      });
    });
    qsa('[data-nho-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/handoffs/' + b.getAttribute('data-nho-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('nho-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('nho-msg', '');
        nhoRefresh();
      });
    });
  }
  on('nho-add', 'click', async function () {
    if (!pval('nho-ppat') || !pval('nho-doc')) { pmsg('nho-msg', 'Isi ID akta dan nama dokumen.'); return; }
    var r = await ppost('/api/v1/ppat/' + encodeURIComponent(pval('nho-ppat')) + '/handoffs', { doc_name: pval('nho-doc') });
    if (!r.ok) { pmsg('nho-msg', 'Gagal (' + r.status + '). Akta tidak ada?'); return; }
    pmsg('nho-msg', '');
    nhoRefresh();
  });
  on('nho-reload', 'click', nhoRefresh);
  /* ---- property: girik intake ---- */
  var GRK_NEXT = { draft: 'verified', verified: 'converted' };
  async function grkRefresh() {
    var box = document.getElementById('grk-list');
    if (!box) return;
    var v = await j('/api/v1/girik-intakes');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = GRK_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-grk-next="' + x.id + '" data-grk-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">terkonversi</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-grk-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.letter_c_ref) + '</td><td>' + esc(x.holder) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada intake girik.</td></tr>';
    qsa('[data-grk-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/girik-intakes/' + b.getAttribute('data-grk-next') + '/status', { status: b.getAttribute('data-grk-to') });
        if (!r.ok) { pmsg('grk-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('grk-msg', '');
        grkRefresh();
      });
    });
    qsa('[data-grk-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/girik-intakes/' + b.getAttribute('data-grk-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('grk-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('grk-msg', '');
        grkRefresh();
      });
    });
  }
  on('grk-add', 'click', async function () {
    if (!pval('grk-ref') || !pval('grk-holder')) { pmsg('grk-msg', 'Isi referensi dan pemegang.'); return; }
    var r = await ppost('/api/v1/girik-intakes', { letter_c_ref: pval('grk-ref'), holder: pval('grk-holder'), area_minor: +pval('grk-area') || 0, village: pval('grk-vil') || '' });
    if (!r.ok) { pmsg('grk-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('grk-msg', '');
    grkRefresh();
  });
  on('grk-reload', 'click', grkRefresh);
  /* ---- property: lease terms ---- */
  function lseShow(x) {
    var out = document.getElementById('lse-out');
    if (!out) return;
    if (!x) { out.textContent = 'Belum ada ketentuan sewa untuk akta ini.'; return; }
    out.innerHTML = 'Sewa ' + fmtRp(x.monthly_minor) + '/bln × ' + x.duration_months + ' bln = <strong>' + fmtRp(x.total_minor) + '</strong> · deposit ' + fmtRp(x.deposit_minor) + ' · mulai ' + esc(x.start_date);
  }
  on('lse-save', 'click', async function () {
    if (!pval('lse-ppat')) { pmsg('lse-msg', 'Isi ID akta.'); return; }
    var r = await ppost('/api/v1/ppat/' + encodeURIComponent(pval('lse-ppat')) + '/lease', { monthly_minor: +pval('lse-month') || 0, duration_months: +pval('lse-dur') || 0, start_date: pval('lse-start') || '', deposit_minor: +pval('lse-dep') || 0 });
    if (!r.ok) { pmsg('lse-msg', 'Gagal (' + r.status + '). Akta tidak ada / durasi 1–1200?'); return; }
    pmsg('lse-msg', '');
    var v = await j('/api/v1/ppat/' + encodeURIComponent(pval('lse-ppat')) + '/lease');
    lseShow(v);
  });
  on('lse-load', 'click', async function () {
    if (!pval('lse-ppat')) { pmsg('lse-msg', 'Isi ID akta.'); return; }
    pmsg('lse-msg', '');
    var v = await j('/api/v1/ppat/' + encodeURIComponent(pval('lse-ppat')) + '/lease');
    lseShow(v);
  });
  on('lse-del', 'click', async function () {
    if (!pval('lse-ppat')) { pmsg('lse-msg', 'Isi ID akta.'); return; }
    var r = await fetch('/api/v1/ppat/' + encodeURIComponent(pval('lse-ppat')) + '/lease', { method: 'DELETE', headers: H });
    if (!r.ok) { pmsg('lse-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('lse-msg', '');
    lseShow(null);
  });
  /* ---- property: developer batches ---- */
  var DVB_NEXT = { planned: 'processing', processing: 'done' };
  async function dvbRefresh() {
    var box = document.getElementById('dvb-list');
    if (!box) return;
    var v = await j('/api/v1/dev-batches');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = DVB_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-dvb-next="' + x.id + '" data-dvb-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">selesai</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-dvb-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.developer) + '</td><td>' + esc(x.batch_no) + ' (' + x.total_units + ' unit)</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada batch.</td></tr>';
    qsa('[data-dvb-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/dev-batches/' + b.getAttribute('data-dvb-next') + '/status', { status: b.getAttribute('data-dvb-to') });
        if (!r.ok) { pmsg('dvb-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('dvb-msg', '');
        dvbRefresh();
      });
    });
    qsa('[data-dvb-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/dev-batches/' + b.getAttribute('data-dvb-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('dvb-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('dvb-msg', '');
        dvbRefresh();
      });
    });
  }
  on('dvb-add', 'click', async function () {
    if (!pval('dvb-dev') || !pval('dvb-no')) { pmsg('dvb-msg', 'Isi developer dan no. batch.'); return; }
    var r = await ppost('/api/v1/dev-batches', { developer: pval('dvb-dev'), batch_no: pval('dvb-no'), total_units: +pval('dvb-total') || 0 });
    if (!r.ok) { pmsg('dvb-msg', 'Gagal (' + r.status + '). No. batch ganda?'); return; }
    pmsg('dvb-msg', '');
    dvbRefresh();
  });
  on('dvb-reload', 'click', dvbRefresh);
  /* ---- property: developer units ---- */
  var DVU_NEXT = { available: 'booked', booked: 'sold' };
  async function dvuRefresh() {
    var box = document.getElementById('dvu-list');
    if (!box) return;
    var v = await j('/api/v1/dev-units');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = DVU_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-dvu-next="' + x.id + '" data-dvu-to="' + nx + '" type="button">→ ' + nx + '</button> ' : (x.status === 'sold' ? '<span class="text-muted">terjual</span>' : '<button class="btn btn-sm" data-dvu-next="' + x.id + '" data-dvu-to="available" type="button">batalkan booking</button> '))
        + ' <button class="btn btn-sm btn-outline-danger" data-dvu-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.unit_code) + '</td><td>' + badge(x.unit_type) + '</td><td>' + fmtRp(x.price_minor) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada unit.</td></tr>';
    qsa('[data-dvu-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/dev-units/' + b.getAttribute('data-dvu-next') + '/status', { status: b.getAttribute('data-dvu-to') });
        if (!r.ok) { pmsg('dvu-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('dvu-msg', '');
        dvuRefresh();
      });
    });
    qsa('[data-dvu-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/dev-units/' + b.getAttribute('data-dvu-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('dvu-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('dvu-msg', '');
        dvuRefresh();
      });
    });
  }
  on('dvu-add', 'click', async function () {
    if (!pval('dvu-code')) { pmsg('dvu-msg', 'Isi kode unit.'); return; }
    var bid = parseInt(pval('dvu-batch'), 10);
    var r = await ppost('/api/v1/dev-units', { batch_id: isNaN(bid) ? null : bid, unit_code: pval('dvu-code'), unit_type: pval('dvu-type') || 'rumah', price_minor: +pval('dvu-price') || 0 });
    if (!r.ok) { pmsg('dvu-msg', 'Gagal (' + r.status + '). Kode ganda / batch tidak ada?'); return; }
    pmsg('dvu-msg', '');
    dvuRefresh();
  });
  on('dvu-reload', 'click', dvuRefresh);
  /* ---- property: KPR packs ---- */
  var KPR_NEXT = { applied: 'approved', approved: 'disbursed' };
  async function kprRefresh() {
    var box = document.getElementById('kpr-list');
    if (!box) return;
    var pid = pval('kpr-ppat');
    if (!pid) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Isi ID akta PPAT lalu Muat.</td></tr>'; return; }
    var v = await j('/api/v1/ppat/' + encodeURIComponent(pid) + '/kpr');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = KPR_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-kpr-next="' + x.id + '" data-kpr-to="' + nx + '" type="button">→ ' + nx + '</button> <button class="btn btn-sm btn-outline-danger" data-kpr-rej="' + x.id + '" type="button">Tolak</button> ' : (x.status === 'disbursed' ? '<span class="text-muted">cair</span>' : '<span class="text-muted">ditolak</span> '))
        + '<button class="btn btn-sm btn-outline-danger" data-kpr-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.bank) + '</td><td>' + fmtRp(x.amount_minor) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada paket KPR.</td></tr>';
    qsa('[data-kpr-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/kpr/' + b.getAttribute('data-kpr-next') + '/status', { status: b.getAttribute('data-kpr-to') });
        if (!r.ok) { pmsg('kpr-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('kpr-msg', '');
        kprRefresh();
      });
    });
    qsa('[data-kpr-rej]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/kpr/' + b.getAttribute('data-kpr-rej') + '/status', { status: 'rejected' });
        if (!r.ok) { pmsg('kpr-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('kpr-msg', '');
        kprRefresh();
      });
    });
    qsa('[data-kpr-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/kpr/' + b.getAttribute('data-kpr-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('kpr-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('kpr-msg', '');
        kprRefresh();
      });
    });
  }
  on('kpr-add', 'click', async function () {
    if (!pval('kpr-ppat') || !pval('kpr-bank')) { pmsg('kpr-msg', 'Isi ID akta dan bank.'); return; }
    var r = await ppost('/api/v1/ppat/' + encodeURIComponent(pval('kpr-ppat')) + '/kpr', { bank: pval('kpr-bank'), amount_minor: +pval('kpr-amt') || 0 });
    if (!r.ok) { pmsg('kpr-msg', 'Gagal (' + r.status + '). Akta tidak ada?'); return; }
    pmsg('kpr-msg', '');
    kprRefresh();
  });
  on('kpr-reload', 'click', kprRefresh);
  /* ---- property: protocol register ---- */
  async function prgRefresh() {
    var box = document.getElementById('prg-list');
    if (!box) return;
    var v = await j('/api/v1/protocol-register');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.entry_date) + '</td><td>' + esc(x.book_ref) + '</td><td>' + esc(x.deed_no) + '</td><td>' + badge(x.direction === 'in' ? 'masuk' : 'keluar') + '</td><td><button class="btn btn-sm btn-outline-danger" data-prg-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada entri register.</td></tr>';
    qsa('[data-prg-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/protocol-register/' + b.getAttribute('data-prg-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('prg-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('prg-msg', '');
        prgRefresh();
      });
    });
  }
  on('prg-add', 'click', async function () {
    if (!pval('prg-book') || !pval('prg-party')) { pmsg('prg-msg', 'Isi buku dan pihak.'); return; }
    var r = await ppost('/api/v1/protocol-register', { entry_date: pval('prg-date') || '', book_ref: pval('prg-book'), deed_no: pval('prg-deed') || '', party: pval('prg-party'), direction: pval('prg-dir') || 'in' });
    if (!r.ok) { pmsg('prg-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('prg-msg', '');
    prgRefresh();
  });
  on('prg-reload', 'click', prgRefresh);
  /* ---- property: AJB drafts ---- */
  async function ajbRefresh() {
    var box = document.getElementById('ajb-list');
    if (!box) return;
    var pid = pval('ajb-ppat');
    if (!pid) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Isi ID akta PPAT lalu Muat.</td></tr>'; return; }
    var v = await j('/api/v1/ppat/' + encodeURIComponent(pid) + '/ajb-drafts');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.template_name || '—') + '</td><td>' + fmtRp(x.price_minor) + '</td><td>' + esc(x.preview) + '</td><td><button class="btn btn-sm btn-outline-danger" data-ajb-del="' + x.id + '" type="button">Hapus</button></td>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada draf.</td></tr>';
    qsa('[data-ajb-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/ajb-drafts/' + b.getAttribute('data-ajb-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('ajb-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('ajb-msg', '');
        ajbRefresh();
      });
    });
  }
  on('ajb-save', 'click', async function () {
    if (!pval('ajb-ppat') || (pval('ajb-body') || '').length < 20) { pmsg('ajb-msg', 'Isi ID akta dan isi draf (min. 20 karakter).'); return; }
    var r = await ppost('/api/v1/ppat/' + encodeURIComponent(pval('ajb-ppat')) + '/ajb-drafts', { template_name: pval('ajb-tpl') || '', body: pval('ajb-body'), price_minor: +pval('ajb-price') || 0 });
    if (!r.ok) { pmsg('ajb-msg', 'Gagal (' + r.status + '). Akta tidak ada?'); return; }
    pmsg('ajb-msg', '');
    ajbRefresh();
  });
  on('ajb-reload', 'click', ajbRefresh);
  /* ---- property: PPAT audit trail ---- */
  async function patRefresh() {
    var box = document.getElementById('pat-list');
    if (!box) return;
    var lim = parseInt(pval('pat-limit'), 10);
    if (!(lim >= 1)) lim = 50;
    var v = await j('/api/v1/reports/ppat-audit?limit=' + lim);
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat (butuh peran admin/auditor?).</td></tr>'; return; }
    box.innerHTML = (v.rows || []).map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.action) + '</td><td>' + badge(x.entity) + ' #' + x.entity_id + '</td><td>' + esc(x.created) + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada jejak.</td></tr>';
  }
  on('pat-reload', 'click', patRefresh);
  /* ---- property: certificate vault ---- */
  async function vltRefresh() {
    var box = document.getElementById('vlt-list');
    if (!box) return;
    var pid = pval('vlt-ppat');
    if (!pid) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Isi ID akta PPAT lalu Muat.</td></tr>'; return; }
    var v = await j('/api/v1/ppat/' + encodeURIComponent(pid) + '/vault');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td><a href="/api/v1/documents/' + x.doc_id + '" target="_blank" rel="noopener">' + esc(x.doc_name) + '</a></td><td>' + esc(x.label || '—') + '</td><td><button class="btn btn-sm btn-outline-danger" data-vlt-del="' + x.id + '" type="button">Lepas</button></td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada pindaian tertaut.</td></tr>';
    qsa('[data-vlt-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/vault/' + b.getAttribute('data-vlt-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('vlt-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('vlt-msg', '');
        vltRefresh();
      });
    });
  }
  on('vlt-add', 'click', async function () {
    if (!pval('vlt-ppat') || !pval('vlt-doc')) { pmsg('vlt-msg', 'Isi ID akta dan ID dokumen.'); return; }
    var r = await ppost('/api/v1/ppat/' + encodeURIComponent(pval('vlt-ppat')) + '/vault', { doc_id: +pval('vlt-doc'), label: pval('vlt-label') || '' });
    if (!r.ok) { pmsg('vlt-msg', 'Gagal (' + r.status + '). Akta/dokumen tidak ada, atau sudah tertaut?'); return; }
    pmsg('vlt-msg', '');
    vltRefresh();
  });
  on('vlt-reload', 'click', vltRefresh);
  /* ---- property: BPN tracker (one renderer x three kinds) ---- */
  var BPN_NEXT = {
    verification: { draft: 'submitted', submitted: 'answered' },
    queue: { waiting: 'called', called: 'done' },
    response: { draft: 'filed' },
  };
  async function bpnRefresh(kind, refId, boxId, msgId) {
    var box = document.getElementById(boxId);
    if (!box) return;
    var v = await j('/api/v1/bpn-tracks?kind=' + kind);
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    var nx = BPN_NEXT[kind] || {};
    box.innerHTML = v.map(function (x) {
      var to = nx[x.status];
      var act = (to ? '<button class="btn btn-sm" data-bpn-next="' + x.id + '" data-bpn-to="' + to + '" data-bpn-kind="' + kind + '" type="button">→ ' + to + '</button> ' : '<span class="text-muted">selesai</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-bpn-del="' + x.id + '" data-bpn-kind="' + kind + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.ref_no) + ' <span class="text-muted">(' + esc(x.office || '—') + ')</span></td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada catatan.</td></tr>';
    qsa('[data-bpn-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/bpn-tracks/' + b.getAttribute('data-bpn-next') + '/status', { status: b.getAttribute('data-bpn-to') });
        if (!r.ok) { pmsg(msgId, 'Gagal (' + r.status + ').'); return; }
        pmsg(msgId, '');
        bpnRefresh(b.getAttribute('data-bpn-kind'), refId, boxId, msgId);
      });
    });
    qsa('[data-bpn-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/bpn-tracks/' + b.getAttribute('data-bpn-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg(msgId, 'Gagal (' + r.status + ').'); return; }
        pmsg(msgId, '');
        bpnRefresh(b.getAttribute('data-bpn-kind'), refId, boxId, msgId);
      });
    });
  }
  function bpnWire(kind, refId, offId, addId, reloadId, boxId, msgId) {
    on(addId, 'click', async function () {
      if (!pval(refId)) { pmsg(msgId, 'Isi nomor referensi.'); return; }
      var r = await ppost('/api/v1/bpn-tracks', { kind: kind, ref_no: pval(refId), office: pval(offId) || '' });
      if (!r.ok) { pmsg(msgId, 'Gagal (' + r.status + ').'); return; }
      pmsg(msgId, '');
      bpnRefresh(kind, refId, boxId, msgId);
    });
    on(reloadId, 'click', function () { bpnRefresh(kind, refId, boxId, msgId); });
  }
  bpnWire('verification', 'bpnv-ref', 'bpnv-office', 'bpnv-add', 'bpnv-reload', 'bpnv-list', 'bpnv-msg');
  bpnWire('queue', 'bpnq-ref', 'bpnq-office', 'bpnq-add', 'bpnq-reload', 'bpnq-list', 'bpnq-msg');
  bpnWire('response', 'bpnr-ref', 'bpnr-office', 'bpnr-add', 'bpnr-reload', 'bpnr-list', 'bpnr-msg');
  /* ---- property: authenticity verdict ---- */
  on('aut-check', 'click', async function () {
    var box = document.getElementById('aut-list');
    var vd = document.getElementById('aut-verdict');
    if (!pval('aut-ppat')) { pmsg('aut-msg', 'Isi ID akta.'); return; }
    var v = await j('/api/v1/ppat/' + encodeURIComponent(pval('aut-ppat')) + '/authenticity');
    if (!v) { pmsg('aut-msg', 'Gagal memuat (akta tidak ada?).'); return; }
    pmsg('aut-msg', '');
    if (vd) vd.innerHTML = v.verdict ? '<span class="badge bg-green-lt">LENGKAP</span>' : '<span class="badge bg-red-lt">BELUM LENGKAP</span>';
    if (!box) return;
    box.innerHTML = (v.checks || []).map(function (x) {
      return '<tr><td>' + esc(x.label) + '</td><td>' + (x.pass ? '<span class="badge bg-green-lt">lolos</span>' : '<span class="badge bg-red-lt">gagal</span>') + '</td><td>' + esc(x.detail) + '</td></tr>';
    }).join('');
  });
  /* ---- property: MPD transfers ---- */
  async function mpdRefresh() {
    var box = document.getElementById('mpd-list');
    if (!box) return;
    var v = await j('/api/v1/mpd-transfers');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var act = (x.status === 'prepared' ? '<button class="btn btn-sm btn-success" data-mpd-go="' + x.id + '" type="button">Serahkan</button> ' : '<span class="text-muted">diserahkan</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-mpd-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + (x.book_id ? 'buku #' + x.book_id : '<span class="text-muted">—</span>') + '</td><td>' + esc(x.recipient) + ' <span class="text-muted">(' + esc(x.transfer_date) + ' / ' + esc(x.receipt_no) + ')</span></td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada transfer.</td></tr>';
    qsa('[data-mpd-go]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/mpd-transfers/' + b.getAttribute('data-mpd-go') + '/status', { status: 'transferred' });
        if (!r.ok) { pmsg('mpd-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('mpd-msg', '');
        mpdRefresh();
      });
    });
    qsa('[data-mpd-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/mpd-transfers/' + b.getAttribute('data-mpd-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('mpd-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('mpd-msg', '');
        mpdRefresh();
      });
    });
  }
  on('mpd-add', 'click', async function () {
    if (!pval('mpd-to')) { pmsg('mpd-msg', 'Isi MPD penerima.'); return; }
    var bid = pval('mpd-book');
    var r = await ppost('/api/v1/mpd-transfers', { book_id: bid ? +bid : null, recipient: pval('mpd-to'), transfer_date: pval('mpd-date') || '', receipt_no: pval('mpd-rcpt') || '' });
    if (!r.ok) { pmsg('mpd-msg', 'Gagal (' + r.status + '). Buku tidak ada?'); return; }
    pmsg('mpd-msg', '');
    mpdRefresh();
  });
  on('mpd-reload', 'click', mpdRefresh);
  /* ---- property: PPAT KYC bundle ---- */
  async function kycRefresh() {
    var box = document.getElementById('kyc-list');
    var sum = document.getElementById('kyc-sum');
    if (!box) return;
    var pid = pval('kyc-ppat');
    if (!pid) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Isi ID akta PPAT lalu Muat.</td></tr>'; if (sum) sum.textContent = ''; return; }
    var v = await j('/api/v1/ppat/' + encodeURIComponent(pid) + '/parties');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    var rows = (v && v.parties) || [];
    if (sum) sum.textContent = 'Terverifikasi: ' + (v.verified_count || 0) + '/' + (v.total_count || 0);
    box.innerHTML = rows.map(function (x) {
      var act = '<button class="btn btn-sm" data-kyc-tg="' + x.id + '" data-kyc-on="' + (!x.verified) + '" type="button">' + (x.verified ? 'Batalkan' : 'Verifikasi') + '</button> '
        + '<button class="btn btn-sm btn-outline-danger" data-kyc-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + badge(x.role) + '</td><td>' + esc(x.name) + ' <span class="text-muted">(' + esc(x.id_number) + ')</span></td><td>' + (x.verified ? '<span class="badge bg-green-lt">ya</span>' : '<span class="badge bg-yellow-lt">belum</span>') + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada pihak.</td></tr>';
    qsa('[data-kyc-tg]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/ppat-parties/' + b.getAttribute('data-kyc-tg') + '/verified', { verified: b.getAttribute('data-kyc-on') === 'true' });
        if (!r.ok) { pmsg('kyc-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('kyc-msg', '');
        kycRefresh();
      });
    });
    qsa('[data-kyc-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/ppat-parties/' + b.getAttribute('data-kyc-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('kyc-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('kyc-msg', '');
        kycRefresh();
      });
    });
  }
  on('kyc-add', 'click', async function () {
    if (!pval('kyc-ppat') || !pval('kyc-name')) { pmsg('kyc-msg', 'Isi ID akta dan nama.'); return; }
    var r = await ppost('/api/v1/ppat/' + encodeURIComponent(pval('kyc-ppat')) + '/parties', { role: pval('kyc-role') || 'seller', name: pval('kyc-name'), id_number: pval('kyc-id') || '' });
    if (!r.ok) { pmsg('kyc-msg', 'Gagal (' + r.status + '). Akta tidak ada?'); return; }
    pmsg('kyc-msg', '');
    kycRefresh();
  });
  on('kyc-reload', 'click', kycRefresh);
  /* ---- property: certificate handover receipts ---- */
  async function choRefresh() {
    var box = document.getElementById('cho-list');
    if (!box) return;
    var pid = pval('cho-ppat');
    if (!pid) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Isi ID akta PPAT lalu Muat.</td></tr>'; return; }
    var v = await j('/api/v1/ppat/' + encodeURIComponent(pid) + '/handovers');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var act = (x.status === 'prepared' ? '<button class="btn btn-sm btn-success" data-cho-go="' + x.id + '" type="button">Serahkan</button> ' : '<span class="text-muted">diserahkan</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-cho-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.handed_to) + ' <span class="text-muted">(' + esc(x.receipt_no) + ')</span></td><td>' + esc(x.handed_at) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada serah terima.</td></tr>';
    qsa('[data-cho-go]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/cert-handovers/' + b.getAttribute('data-cho-go') + '/status', { status: 'handed' });
        if (!r.ok) { pmsg('cho-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('cho-msg', '');
        choRefresh();
      });
    });
    qsa('[data-cho-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/cert-handovers/' + b.getAttribute('data-cho-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('cho-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('cho-msg', '');
        choRefresh();
      });
    });
  }
  on('cho-add', 'click', async function () {
    if (!pval('cho-ppat') || !pval('cho-to')) { pmsg('cho-msg', 'Isi ID akta dan penerima.'); return; }
    var r = await ppost('/api/v1/ppat/' + encodeURIComponent(pval('cho-ppat')) + '/handovers', { handed_to: pval('cho-to'), handed_at: pval('cho-at') || '', receipt_no: pval('cho-rcpt') || '' });
    if (!r.ok) { pmsg('cho-msg', 'Gagal (' + r.status + '). Akta tidak ada?'); return; }
    pmsg('cho-msg', '');
    choRefresh();
  });
  on('cho-reload', 'click', choRefresh);
  /* ---- property: APHT/SKMHT charges ---- */
  async function chgRefresh() {
    var box = document.getElementById('chg-list');
    if (!box) return;
    var pid = pval('chg-ppat');
    if (!pid) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Isi ID akta PPAT lalu Muat.</td></tr>'; return; }
    var v = await j('/api/v1/ppat/' + encodeURIComponent(pid) + '/charges');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + badge(x.kind) + '</td><td>' + esc(x.creditor) + '</td><td>' + fmtRp(x.ceiling_minor) + '</td><td>' + esc(x.valid_until) + '</td><td><button class="btn btn-sm btn-outline-danger" data-chg-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada pembebanan.</td></tr>';
    qsa('[data-chg-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/charges/' + b.getAttribute('data-chg-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('chg-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('chg-msg', '');
        chgRefresh();
      });
    });
  }
  on('chg-add', 'click', async function () {
    if (!pval('chg-ppat') || !pval('chg-cred')) { pmsg('chg-msg', 'Isi ID akta dan kreditor.'); return; }
    var r = await ppost('/api/v1/ppat/' + encodeURIComponent(pval('chg-ppat')) + '/charges', { kind: pval('chg-kind') || 'apht', creditor: pval('chg-cred'), ceiling_minor: +pval('chg-ceil') || 0, valid_until: pval('chg-until') || '', note: '' });
    if (!r.ok) { pmsg('chg-msg', 'Gagal (' + r.status + '). Akta tidak ada?'); return; }
    pmsg('chg-msg', '');
    chgRefresh();
  });
  on('chg-reload', 'click', chgRefresh);
  /* ---- property: charge expiry monitor ---- */
  async function chgdRefresh() {
    var box = document.getElementById('chgd-list');
    if (!box) return;
    var d = parseInt(pval('chgd-days'), 10);
    if (!(d >= 0)) d = 30;
    var v = await j('/api/v1/charges/due?days=' + d);
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = (v.due || []).map(function (x) {
      return '<tr><td>' + x.id + '</td><td>akta #' + x.ppat_id + '</td><td>' + badge(x.kind) + ' <span class="text-muted">(' + esc(x.creditor) + ')</span></td><td><strong>' + esc(x.valid_until) + '</strong></td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Tidak ada yang mendekati kedaluwarsa.</td></tr>';
  }
  on('chgd-reload', 'click', chgdRefresh);
  /* ---- property: kind-aware readiness ---- */
  async function rdyRefresh() {
    var box = document.getElementById('rdy-list');
    var sum = document.getElementById('rdy-sum');
    if (!box) return;
    var pid = pval('rdy-ppat');
    if (!pid) { box.innerHTML = '<tr><td colspan="3" class="text-muted">Isi ID akta PPAT lalu Periksa.</td></tr>'; if (sum) sum.textContent = ''; return; }
    var v = await j('/api/v1/ppat/' + encodeURIComponent(pid) + '/readiness');
    if (!v) { box.innerHTML = '<tr><td colspan="3" class="text-muted">Gagal memuat.</td></tr>'; return; }
    if (sum) sum.textContent = 'Jenis: ' + v.deed_kind + ' • ' + (v.ready ? 'SIAP' : 'belum siap') + ' (' + v.verified_parties + '/' + v.total_parties + ' KYC)';
    box.innerHTML = (v.checks || []).map(function (x) {
      return '<tr><td>' + esc(x.label) + '</td><td>' + (x.pass ? '<span class="badge bg-green-lt">lolos</span>' : '<span class="badge bg-yellow-lt">kurang</span>') + '</td><td class="text-muted">' + esc(x.detail) + '</td></tr>';
    }).join('');
  }
  on('rdy-reload', 'click', rdyRefresh);
  qsa('.snav[data-tab="akta"]', function (a) {
    a.addEventListener('click', function () { certRefresh(); ppatRefresh(); pbRefresh(); phRefresh(); mpdRefresh(); kycRefresh(); choRefresh(); chgRefresh(); chgdRefresh(); rdyRefresh(); spRefresh(); mgRefresh(); dsp2Refresh(); bndRefresh(); consentRefresh('mrt', 'mrt-ppat', 'mrt-list', 'mrt-msg', 'marital'); consentRefresh('hir', 'hir-ppat', 'hir-list', 'hir-msg', 'heir'); consentRefresh('bod', 'bod-ppat', 'bod-list', 'bod-msg', 'board'); consentRefresh('pow', 'pow-ppat', 'pow-list', 'pow-msg', 'power'); lhiRefresh(); uhoRefresh(); rpmRefresh(); pdbRefresh(); nmpRefresh(); mpnRefresh(); nhoRefresh(); grkRefresh(); dvbRefresh(); dvuRefresh(); kprRefresh(); prgRefresh(); ajbRefresh(); patRefresh(); vltRefresh(); bpnRefresh('verification', 'bpnv-ref', 'bpnv-list', 'bpnv-msg'); bpnRefresh('queue', 'bpnq-ref', 'bpnq-list', 'bpnq-msg'); bpnRefresh('response', 'bpnr-ref', 'bpnr-list', 'bpnr-msg'); klapperRefresh(); perfRefresh(); volRefresh(); sboxRefresh(); sealRefresh(); zoneRefresh(); pbbRefresh(); });
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
  /* ---- office: appointments + queue board ---- */
  var APPT_NEXT = { scheduled: 'checked_in', checked_in: 'done' };
  async function apptRefresh() {
    var box = document.getElementById('appt-list');
    if (!box) return;
    var v = await j('/api/v1/appointments');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = APPT_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-appt-next="' + x.id + '" data-appt-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '')
        + (x.status === 'scheduled' ? '<button class="btn btn-sm btn-warning" data-appt-cancel="' + x.id + '" type="button">Batal</button> ' : '<span class="text-muted">final</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-appt-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.client_name) + ' <span class="text-muted">(' + esc(x.purpose) + ')</span></td><td>' + esc(x.scheduled_at) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada janji temu.</td></tr>';
    qsa('[data-appt-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/appointments/' + b.getAttribute('data-appt-next') + '/status', { status: b.getAttribute('data-appt-to') });
        if (!r.ok) { pmsg('appt-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('appt-msg', '');
        apptRefresh();
        queueRefresh();
      });
    });
    qsa('[data-appt-cancel]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/appointments/' + b.getAttribute('data-appt-cancel') + '/status', { status: 'cancelled' });
        if (!r.ok) { pmsg('appt-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('appt-msg', '');
        apptRefresh();
        queueRefresh();
      });
    });
    qsa('[data-appt-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/appointments/' + b.getAttribute('data-appt-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('appt-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('appt-msg', '');
        apptRefresh();
        queueRefresh();
      });
    });
  }
  async function queueRefresh() {
    var box = document.getElementById('queue-list');
    if (!box) return;
    var d = (pval('queue-date') || '').trim();
    var v = await j('/api/v1/appointments/queue' + (d ? '?date=' + encodeURIComponent(d) : ''));
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    var q = (v && v.queue) || [];
    box.innerHTML = q.map(function (x) {
      return '<tr><td><strong>' + esc(x.scheduled_at.slice(11)) + '</strong></td><td>' + esc(x.client_name) + '</td><td>' + esc(x.purpose) + '</td><td>' + badge(x.status) + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Antrean kosong pada ' + esc((v && v.date) || '') + '.</td></tr>';
  }
  on('appt-add', 'click', async function () {
    if (!pval('appt-name') || !pval('appt-at')) { pmsg('appt-msg', 'Isi nama dan jadwal (YYYY-MM-DD HH:MM).'); return; }
    var r = await ppost('/api/v1/appointments', { client_name: pval('appt-name'), purpose: pval('appt-purpose') || '', scheduled_at: pval('appt-at') });
    if (!r.ok) { pmsg('appt-msg', 'Gagal (' + r.status + '). Format: YYYY-MM-DD HH:MM.'); return; }
    pmsg('appt-msg', '');
    apptRefresh();
    queueRefresh();
  });
  on('appt-reload', 'click', function () { apptRefresh(); queueRefresh(); });
  on('queue-reload', 'click', queueRefresh);
  /* ---- office: visitor log ---- */
  async function visRefresh() {
    var box = document.getElementById('vis-list');
    if (!box) return;
    var v = await j('/api/v1/visitors');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var act = (x.status === 'in' ? '<button class="btn btn-sm" data-vis-out="' + x.id + '" type="button">Check-out</button> ' : '<span class="text-muted">selesai</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-vis-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.name) + ' <span class="text-muted">(' + esc(x.need) + ')</span></td><td>' + esc(x.check_in) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada tamu.</td></tr>';
    qsa('[data-vis-out]', function (b) {
      b.addEventListener('click', async function () {
        var now = new Date();
        var pad = function (n) { return (n < 10 ? '0' : '') + n; };
        var stamp = now.getFullYear() + '-' + pad(now.getMonth() + 1) + '-' + pad(now.getDate()) + ' ' + pad(now.getHours()) + ':' + pad(now.getMinutes());
        var r = await ppost('/api/v1/visitors/' + b.getAttribute('data-vis-out') + '/checkout', { check_out: stamp });
        if (!r.ok) { pmsg('vis-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('vis-msg', '');
        visRefresh();
      });
    });
    qsa('[data-vis-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/visitors/' + b.getAttribute('data-vis-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('vis-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('vis-msg', '');
        visRefresh();
      });
    });
  }
  on('vis-add', 'click', async function () {
    if (!pval('vis-name') || !pval('vis-in')) { pmsg('vis-msg', 'Isi nama dan waktu masuk (YYYY-MM-DD HH:MM).'); return; }
    var r = await ppost('/api/v1/visitors', { name: pval('vis-name'), need: pval('vis-need') || '', check_in: pval('vis-in') });
    if (!r.ok) { pmsg('vis-msg', 'Gagal (' + r.status + '). Format: YYYY-MM-DD HH:MM.'); return; }
    pmsg('vis-msg', '');
    visRefresh();
  });
  on('vis-reload', 'click', visRefresh);
  /* ---- office: witness pool ---- */
  async function witRefresh() {
    var box = document.getElementById('wit-list');
    if (!box) return;
    var v = await j('/api/v1/witnesses');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var act = '<button class="btn btn-sm" data-wit-tg="' + x.id + '" data-wit-on="' + (!x.active) + '" type="button">' + (x.active ? 'Nonaktifkan' : 'Aktifkan') + '</button> '
        + '<button class="btn btn-sm btn-outline-danger" data-wit-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.name) + ' <span class="text-muted">(' + esc(x.phone) + ')</span></td><td>' + esc(x.id_number) + '</td><td>' + (x.active ? '<span class="badge bg-green-lt">aktif</span>' : '<span class="badge bg-yellow-lt">nonaktif</span>') + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada saksi.</td></tr>';
    qsa('[data-wit-tg]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/witnesses/' + b.getAttribute('data-wit-tg') + '/active', { active: b.getAttribute('data-wit-on') === 'true' });
        if (!r.ok) { pmsg('wit-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('wit-msg', '');
        witRefresh();
      });
    });
    qsa('[data-wit-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/witnesses/' + b.getAttribute('data-wit-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('wit-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('wit-msg', '');
        witRefresh();
      });
    });
  }
  on('wit-add', 'click', async function () {
    if (!pval('wit-name')) { pmsg('wit-msg', 'Isi nama saksi.'); return; }
    var r = await ppost('/api/v1/witnesses', { name: pval('wit-name'), id_number: pval('wit-id') || '', phone: pval('wit-phone') || '' });
    if (!r.ok) { pmsg('wit-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('wit-msg', '');
    witRefresh();
  });
  on('wit-reload', 'click', witRefresh);
  /* ---- office: re-verification log ---- */
  async function rchRefresh() {
    var box = document.getElementById('rch-list');
    if (!box) return;
    var v = await j('/api/v1/rechecks');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.subject) + '</td><td>' + badge(x.method) + '</td><td>' + badge(x.result) + '</td><td><button class="btn btn-sm btn-outline-danger" data-rch-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada verifikasi ulang.</td></tr>';
    qsa('[data-rch-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/rechecks/' + b.getAttribute('data-rch-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('rch-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('rch-msg', '');
        rchRefresh();
      });
    });
  }
  on('rch-add', 'click', async function () {
    if (!pval('rch-subject')) { pmsg('rch-msg', 'Isi subjek.'); return; }
    var r = await ppost('/api/v1/rechecks', { subject: pval('rch-subject'), method: pval('rch-method') || 'document', result: pval('rch-result') || 'pass', verified_on: pval('rch-on') || '', note: pval('rch-note') || '' });
    if (!r.ok) { pmsg('rch-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('rch-msg', '');
    rchRefresh();
  });
  on('rch-reload', 'click', rchRefresh);
  /* ---- office: working hours ---- */
  async function hrsRefresh() {
    var box = document.getElementById('hrs-list');
    if (!box) return;
    var v = await j('/api/v1/office-hours');
    if (!v) { box.innerHTML = '<tr><td colspan="3" class="text-muted">Gagal memuat.</td></tr>'; return; }
    var byDay = {};
    v.forEach(function (x) { byDay[x.weekday] = x; });
    var names = ['Senin', 'Selasa', 'Rabu', 'Kamis', 'Jumat', 'Sabtu', 'Ahad'];
    box.innerHTML = names.map(function (n, i) {
      var x = byDay[i];
      return '<tr><td>' + n + '</td><td>' + (x ? esc(x.open_time) + '–' + esc(x.close_time) : '<span class="text-muted">belum diatur</span>') + '</td><td>' + (x ? (x.active ? '<span class="badge bg-green-lt">buka</span>' : '<span class="badge bg-yellow-lt">tutup</span>') : '<span class="text-muted">—</span>') + '</td></tr>';
    }).join('');
  }
  on('hrs-set', 'click', async function () {
    if (!pval('hrs-open') || !pval('hrs-close')) { pmsg('hrs-msg', 'Isi jam buka dan tutup (HH:MM).'); return; }
    var r = await ppost('/api/v1/office-hours', { weekday: +pval('hrs-day'), open_time: pval('hrs-open'), close_time: pval('hrs-close'), active: true });
    if (!r.ok) { pmsg('hrs-msg', 'Gagal (' + r.status + '). Format HH:MM.'); return; }
    pmsg('hrs-msg', '');
    hrsRefresh();
  });
  on('hrs-reload', 'click', hrsRefresh);
  /* ---- office: roster ---- */
  async function rosRefresh() {
    var box = document.getElementById('ros-list');
    if (!box) return;
    var v = await j('/api/v1/roster');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var act = '<button class="btn btn-sm" data-ros-tg="' + x.id + '" data-ros-on="' + (!x.active) + '" type="button">' + (x.active ? 'Nonaktifkan' : 'Aktifkan') + '</button> '
        + '<button class="btn btn-sm btn-outline-danger" data-ros-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.name) + ' <span class="text-muted">(' + esc(x.role) + ')</span></td><td>' + badge(x.shift) + '</td><td>' + (x.active ? '<span class="badge bg-green-lt">aktif</span>' : '<span class="badge bg-yellow-lt">nonaktif</span>') + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Roster kosong.</td></tr>';
    qsa('[data-ros-tg]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/roster/' + b.getAttribute('data-ros-tg') + '/active', { active: b.getAttribute('data-ros-on') === 'true' });
        if (!r.ok) { pmsg('ros-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('ros-msg', '');
        rosRefresh();
      });
    });
    qsa('[data-ros-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/roster/' + b.getAttribute('data-ros-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('ros-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('ros-msg', '');
        rosRefresh();
      });
    });
  }
  on('ros-add', 'click', async function () {
    if (!pval('ros-name')) { pmsg('ros-msg', 'Isi nama staf.'); return; }
    var r = await ppost('/api/v1/roster', { name: pval('ros-name'), role: pval('ros-role') || 'staff', shift: pval('ros-shift') || 'pagi' });
    if (!r.ok) { pmsg('ros-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('ros-msg', '');
    rosRefresh();
  });
  on('ros-reload', 'click', rosRefresh);
  /* ---- office: leaves & substitutes ---- */
  async function levRefresh() {
    var box = document.getElementById('lev-list');
    if (!box) return;
    var v = await j('/api/v1/leaves');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var act = (x.status === 'requested' ? '<button class="btn btn-sm btn-success" data-lev-ok="' + x.id + '" type="button">Setujui</button> <button class="btn btn-sm btn-danger" data-lev-no="' + x.id + '" type="button">Tolak</button> ' : '<span class="text-muted">final</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-lev-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.holder) + ' <span class="text-muted">(' + esc(x.kind) + ')</span></td><td>' + esc(x.from_date) + ' → ' + esc(x.to_date) + '</td><td>' + esc(x.substitute) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada pengajuan.</td></tr>';
    qsa('[data-lev-ok]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/leaves/' + b.getAttribute('data-lev-ok') + '/status', { status: 'approved' });
        if (!r.ok) { pmsg('lev-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('lev-msg', '');
        levRefresh();
      });
    });
    qsa('[data-lev-no]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/leaves/' + b.getAttribute('data-lev-no') + '/status', { status: 'rejected' });
        if (!r.ok) { pmsg('lev-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('lev-msg', '');
        levRefresh();
      });
    });
    qsa('[data-lev-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/leaves/' + b.getAttribute('data-lev-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('lev-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('lev-msg', '');
        levRefresh();
      });
    });
  }
  on('lev-add', 'click', async function () {
    if (!pval('lev-who')) { pmsg('lev-msg', 'Isi pemegang cuti.'); return; }
    var r = await ppost('/api/v1/leaves', { holder: pval('lev-who'), kind: pval('lev-kind') || 'leave', from_date: pval('lev-from') || '', to_date: pval('lev-to') || '', substitute: pval('lev-sub') || '' });
    if (!r.ok) { pmsg('lev-msg', 'Gagal (' + r.status + '). Rentang tanggal terbalik?'); return; }
    pmsg('lev-msg', '');
    levRefresh();
  });
  on('lev-reload', 'click', levRefresh);
  /* ---- office: announcements ---- */
  async function annRefresh() {
    var box = document.getElementById('ann-list');
    if (!box) return;
    var v = await j('/api/v1/announcements');
    if (!v) { box.innerHTML = '<p class="text-muted">Gagal memuat.</p>'; return; }
    var act = v.filter(function (x) { return x.active; });
    box.innerHTML = act.map(function (x) {
      return '<div class="card mt-2"><div class="card-body"><h4>' + (x.pinned ? '📌 ' : '') + esc(x.title) + '</h4><p>' + esc(x.body) + '</p>'
        + '<div class="mkt-row"><button class="btn btn-sm" data-ann-pin="' + x.id + '" data-ann-on="' + (!x.pinned) + '" type="button">' + (x.pinned ? 'Lepas pin' : 'Pin') + '</button> '
        + '<button class="btn btn-sm" data-ann-hide="' + x.id + '" type="button">Arsipkan</button> '
        + '<button class="btn btn-sm btn-outline-danger" data-ann-del="' + x.id + '" type="button">Hapus</button></div></div></div>';
    }).join('') || '<p class="text-muted">Belum ada pengumuman aktif.</p>';
    qsa('[data-ann-pin]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/announcements/' + b.getAttribute('data-ann-pin'), { pinned: b.getAttribute('data-ann-on') === 'true' });
        if (!r.ok) { pmsg('ann-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('ann-msg', '');
        annRefresh();
      });
    });
    qsa('[data-ann-hide]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/announcements/' + b.getAttribute('data-ann-hide'), { active: false });
        if (!r.ok) { pmsg('ann-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('ann-msg', '');
        annRefresh();
      });
    });
    qsa('[data-ann-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/announcements/' + b.getAttribute('data-ann-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('ann-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('ann-msg', '');
        annRefresh();
      });
    });
  }
  on('ann-add', 'click', async function () {
    if (!pval('ann-title')) { pmsg('ann-msg', 'Isi judul pengumuman.'); return; }
    var r = await ppost('/api/v1/announcements', { title: pval('ann-title'), body: pval('ann-body') || '', pinned: false });
    if (!r.ok) { pmsg('ann-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('ann-msg', '');
    annRefresh();
  });
  on('ann-reload', 'click', annRefresh);
  /* ---- office: document checklist ---- */
  async function dclRefresh() {
    var box = document.getElementById('dcl-list');
    if (!box) return;
    var did = pval('dcl-doc');
    if (!did) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Isi ID dokumen lalu Muat.</td></tr>'; return; }
    var v = await j('/api/v1/documents/' + encodeURIComponent(did) + '/checklist');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    var done = v.filter(function (x) { return x.done; }).length;
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.item) + '</td><td>' + (x.done ? '<span class="badge bg-green-lt">ya</span>' : '<span class="badge bg-yellow-lt">belum</span>') + '</td>'
        + '<td><button class="btn btn-sm" data-dcl-tg="' + x.id + '" data-dcl-done="' + (!x.done) + '" type="button">Ubah</button> '
        + '<button class="btn btn-sm btn-outline-danger" data-dcl-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada butir.</td></tr>';
    if (v.length) box.innerHTML += '<tr><td colspan="4" class="text-muted">Progres: ' + done + '/' + v.length + ' selesai.</td></tr>';
    qsa('[data-dcl-tg]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/doc-checklist/' + b.getAttribute('data-dcl-tg') + '/toggle', { done: b.getAttribute('data-dcl-done') === 'true' });
        if (!r.ok) { pmsg('dcl-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('dcl-msg', '');
        dclRefresh();
      });
    });
    qsa('[data-dcl-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/doc-checklist/' + b.getAttribute('data-dcl-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('dcl-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('dcl-msg', '');
        dclRefresh();
      });
    });
  }
  on('dcl-add', 'click', async function () {
    if (!pval('dcl-doc') || !pval('dcl-item')) { pmsg('dcl-msg', 'Isi ID dokumen dan butir.'); return; }
    var r = await ppost('/api/v1/documents/' + encodeURIComponent(pval('dcl-doc')) + '/checklist', { item: pval('dcl-item') });
    if (!r.ok) { pmsg('dcl-msg', 'Gagal (' + r.status + '). Dokumen tidak ada?'); return; }
    pmsg('dcl-msg', '');
    dclRefresh();
  });
  on('dcl-reload', 'click', dclRefresh);
  /* ---- office: SOP library ---- */
  var SOP_NEXT = { draft: 'published', published: 'archived' };
  async function sopRefresh() {
    var box = document.getElementById('sop-list');
    if (!box) return;
    var v = await j('/api/v1/sops');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = SOP_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-sop-next="' + x.id + '" data-sop-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">arsip</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-sop-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.code) + '</td><td>' + esc(x.title) + ' <span class="text-muted">(v' + esc(x.version) + ')</span></td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada SOP.</td></tr>';
    qsa('[data-sop-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/sops/' + b.getAttribute('data-sop-next') + '/status', { status: b.getAttribute('data-sop-to') });
        if (!r.ok) { pmsg('sop-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('sop-msg', '');
        sopRefresh();
      });
    });
    qsa('[data-sop-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/sops/' + b.getAttribute('data-sop-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('sop-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('sop-msg', '');
        sopRefresh();
      });
    });
  }
  on('sop-add', 'click', async function () {
    if (!pval('sop-code') || !pval('sop-title') || !pval('sop-body')) { pmsg('sop-msg', 'Isi kode, judul, dan isi.'); return; }
    var r = await ppost('/api/v1/sops', { code: pval('sop-code'), title: pval('sop-title'), version: '1.0', body: pval('sop-body') });
    if (!r.ok) { pmsg('sop-msg', 'Gagal (' + r.status + '). Kode mungkin duplikat.'); return; }
    pmsg('sop-msg', '');
    sopRefresh();
  });
  on('sop-reload', 'click', sopRefresh);
  /* ---- office: backup checks ---- */
  async function bcRefresh() {
    var box = document.getElementById('bc-list');
    var last = document.getElementById('bc-last');
    if (!box) return;
    var v = await j('/api/v1/backup-checks');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    if (last && v.length) {
      var l = v[0];
      last.textContent = 'Terakhir: ' + l.checked_at + ' (' + l.method + ') = ' + l.result;
    } else if (last) {
      last.textContent = 'Belum pernah diverifikasi.';
    }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.checked_at) + '</td><td>' + badge(x.method) + '</td><td>' + badge(x.result) + '</td><td><button class="btn btn-sm btn-outline-danger" data-bc-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada verifikasi.</td></tr>';
    qsa('[data-bc-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/backup-checks/' + b.getAttribute('data-bc-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('bc-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('bc-msg', '');
        bcRefresh();
      });
    });
  }
  on('bc-add', 'click', async function () {
    if (!pval('bc-at')) { pmsg('bc-msg', 'Isi waktu (YYYY-MM-DD HH:MM).'); return; }
    var r = await ppost('/api/v1/backup-checks', { checked_at: pval('bc-at'), method: pval('bc-method') || 'dump', result: pval('bc-result') || 'pass', note: pval('bc-note') || '' });
    if (!r.ok) { pmsg('bc-msg', 'Gagal (' + r.status + '). Format: YYYY-MM-DD HH:MM.'); return; }
    pmsg('bc-msg', '');
    bcRefresh();
  });
  on('bc-reload', 'click', bcRefresh);
  /* ---- office: office incidents ---- */
  var OIN_NEXT = { open: 'investigating', investigating: 'resolved', resolved: 'closed' };
  async function oinRefresh() {
    var box = document.getElementById('oin-list');
    if (!box) return;
    var v = await j('/api/v1/office-incidents');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = OIN_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-oin-next="' + x.id + '" data-oin-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">ditutup</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-oin-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + ' <span class="text-muted">(' + esc(x.severity) + ')</span></td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada insiden.</td></tr>';
    qsa('[data-oin-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/office-incidents/' + b.getAttribute('data-oin-next') + '/status', { status: b.getAttribute('data-oin-to') });
        if (!r.ok) { pmsg('oin-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('oin-msg', '');
        oinRefresh();
      });
    });
    qsa('[data-oin-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/office-incidents/' + b.getAttribute('data-oin-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('oin-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('oin-msg', '');
        oinRefresh();
      });
    });
  }
  on('oin-add', 'click', async function () {
    if (!pval('oin-title')) { pmsg('oin-msg', 'Isi judul insiden.'); return; }
    var r = await ppost('/api/v1/office-incidents', { title: pval('oin-title'), severity: pval('oin-sev') || 'medium' });
    if (!r.ok) { pmsg('oin-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('oin-msg', '');
    oinRefresh();
  });
  on('oin-reload', 'click', oinRefresh);
  /* ---- office: staff productivity ---- */
  async function staffRefresh() {
    var box = document.getElementById('staff-list');
    var sum = document.getElementById('staff-sum');
    if (!box) return;
    var v = await j('/api/v1/reports/staff-productivity');
    if (!v) { box.innerHTML = '<tr><td colspan="2" class="text-muted">Gagal memuat.</td></tr>'; return; }
    if (sum) sum.textContent = 'Sampel: ' + v.sampled + ' aksi audit terakhir.';
    box.innerHTML = (v.by_actor || []).map(function (x) {
      return '<tr><td>' + esc(x.actor) + '</td><td><strong>' + x.actions + '</strong></td></tr>';
    }).join('') || '<tr><td colspan="2" class="text-muted">Belum ada aktivitas.</td></tr>';
  }
  on('staff-reload', 'click', staffRefresh);
  /* ---- office: waiting analytics ---- */
  async function waitRefresh() {
    var box = document.getElementById('wait-list');
    var sum = document.getElementById('wait-sum');
    if (!box) return;
    var v = await j('/api/v1/reports/waiting-analytics');
    if (!v) { box.innerHTML = '<tr><td colspan="2" class="text-muted">Gagal memuat.</td></tr>'; return; }
    if (sum) sum.textContent = 'Kunjungan selesai: ' + v.completed_visits + ' • Rata-rata: ' + (v.avg_minutes === null || v.avg_minutes === undefined ? '—' : v.avg_minutes + ' mnt') + (v.max_minutes === null || v.max_minutes === undefined ? '' : ' • Maks: ' + v.max_minutes + ' mnt');
    box.innerHTML = (v.by_day || []).map(function (x) {
      return '<tr><td>' + esc(x.day) + '</td><td><strong>' + x.count + '</strong></td></tr>';
    }).join('') || '<tr><td colspan="2" class="text-muted">Belum ada kunjungan selesai.</td></tr>';
  }
  on('wait-reload', 'click', waitRefresh);
  crmTags();
  crmRecent();
  apptRefresh();
  queueRefresh();
  visRefresh();
  witRefresh();
  rchRefresh();
  hrsRefresh();
  rosRefresh();
  levRefresh();
  dspRefresh();
  staffRefresh();
  waitRefresh();
  annRefresh();
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
  on('rep-sla', 'click', function () { dlCsv('/api/v1/reports/sla-breaches.csv', 'sla-breaches.csv', 'rep-msg'); });
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
    var act = document.getElementById('audit-actor');
    var qs = [];
    if (sel && sel.value) qs.push('entity=' + encodeURIComponent(sel.value));
    if (act && act.value.trim()) qs.push('actor=' + encodeURIComponent(act.value.trim()));
    box.innerHTML = '<tr><td colspan="5" class="text-muted">Memuat…</td></tr>';
    try {
      var r = await fetch('/api/v1/audit' + (qs.length ? '?' + qs.join('&') : ''), { headers: H });
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
  on('audit-actor', 'change', loadAudit);
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
      }).join('') + '</ol><div id="appr-docs"><p class="text-muted">Memuat lampiran…</p></div>';
    var at = await j('/api/v1/approvals/' + encodeURIComponent(id) + '/attachments');
    var docs = document.getElementById('appr-docs');
    if (!docs) return;
    if (!at) { docs.innerHTML = '<p class="text-muted">Lampiran tidak dapat dimuat.</p>'; return; }
    docs.innerHTML = '<p><strong>Lampiran (' + (at.docs || []).length + ')</strong> — perkara #' + at.matter_id + ':</p>'
      + ((at.docs || []).map(function (d) {
        return '<p><a href="/api/v1/documents/' + d.id + '" target="_blank" rel="noopener">' + esc(d.name) + '</a></p>';
      }).join('') || '<p class="text-muted">Belum ada dokumen pada perkara ini.</p>');
  }
  async function apprDecide(decision) {
    var id = pval('appr-id'), step = pval('appr-step');
    if (!id || !step) { pmsg('appr-dmsg', 'Isi ID permintaan dan no. tahap.'); return; }
    var r = await ppostKey('/api/v1/approvals/' + encodeURIComponent(id) + '/decisions',
      { step_no: +step, decision: decision, comment: pval('appr-comment') || null }, pval('appr-key'));
    if (!r.ok) {
      var e = null;
      try { e = await r.clone().json(); } catch (x) { /* plain */ }
      pmsg('appr-dmsg', 'Gagal (' + r.status + ').' + (e && e.error && e.error.message === 'replayed' ? ' Permintaan ganda ditolak.' : ' Cek urutan tahap & peran.'));
      return;
    }
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
  /* ---- approval simulation (F-D09-034) ---- */
  on('appr-sim', 'click', async function () {
    var box = document.getElementById('appr-sim-out');
    var id = pval('appr-id'), step = pval('appr-step');
    if (!id || !step) { pmsg('appr-dmsg', 'Isi ID permintaan dan no. tahap.'); return; }
    var r = await ppost('/api/v1/approvals/' + encodeURIComponent(id) + '/simulate', { step_no: +step, decision: 'approve' });
    if (!r.ok) { if (box) box.textContent = 'Simulasi gagal (' + r.status + ').'; return; }
    var v = await r.json();
    if (box) box.innerHTML = (v.ok ? '<span class="badge bg-green-lt">akan lolos</span>' : '<span class="badge bg-red-lt">tertahan</span>')
      + ' <span class="text-muted">hasil: ' + esc(v.outcome) + ' — ' + (v.checks || []).map(function (c) { return esc(c.label) + ': ' + (c.pass ? 'ya' : 'tidak'); }).join('; ') + '</span>';
  });
  /* ---- overdue approval alerts (F-D09-018) ---- */
  async function apprOverdueRefresh() {
    var box = document.getElementById('appr-overdue-list');
    if (!box) return;
    var d = parseInt(pval('appr-overdue-days'), 10);
    if (!(d >= 0)) d = 3;
    var v = await j('/api/v1/approvals/overdue?days=' + d);
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + x.age_days + '</td><td>' + esc(x.waiting_on) + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Tidak ada yang terlambat. Semua tepat waktu.</td></tr>';
  }
  on('appr-overdue-reload', 'click', apprOverdueRefresh);
  /* ---- approval calendar view (F-D09-019) ---- */
  async function apprCalRefresh() {
    var box = document.getElementById('appr-cal-list');
    if (!box) return;
    var m = (pval('appr-cal-month') || '').trim();
    var v = await j('/api/v1/approvals/calendar' + (m ? '?month=' + encodeURIComponent(m) : ''));
    if (!v) { box.innerHTML = '<p class="text-muted">Gagal memuat.</p>'; return; }
    var days = (v && v.days) || {};
    var keys = Object.keys(days).sort();
    if (!keys.length) { box.innerHTML = '<p class="text-muted">Tidak ada persetujuan pending pada ' + esc(v.month || '') + '.</p>'; return; }
    box.innerHTML = '<p class="text-muted">Bulan ' + esc(v.month || '') + ': ' + keys.length + ' hari ada antrean.</p>'
      + keys.map(function (d) {
        return '<h4>' + esc(d) + '</h4><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Umur (hari)</th><th>Menunggu</th></tr></thead><tbody>'
          + days[d].map(function (x) {
            return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + x.age_days + '</td><td>' + esc(x.waiting_on) + '</td></tr>';
          }).join('') + '</tbody></table></div>';
      }).join('');
  }
  on('appr-cal-reload', 'click', apprCalRefresh);
  /* ---- approval SLA (F-D09-005) ---- */
  async function slaRefresh() {
    var box = document.getElementById('sla-list');
    if (!box) return;
    var w = parseInt(pval('sla-warn'), 10);
    var b = parseInt(pval('sla-breach'), 10);
    if (!(w >= 1)) w = 24;
    if (!(b >= 1)) b = 72;
    var v = await j('/api/v1/approvals/sla?warn_hours=' + w + '&breach_hours=' + b);
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = (v.rows || []).map(function (x) {
      var s = x.sla === 'breach' ? '<span class="badge bg-red-lt">langgar</span>' : (x.sla === 'warn' ? '<span class="badge bg-yellow-lt">peringatan</span>' : '<span class="badge bg-green-lt">aman</span>');
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + x.age_hours + '</td><td>' + s + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Tidak ada antrean terbuka.</td></tr>';
  }
  on('sla-reload', 'click', slaRefresh);
  /* ---- approval conflicts (F-D09-033) ---- */
  async function cfxRefresh() {
    var box = document.getElementById('cfx-list');
    if (!box) return;
    var v = await j('/api/v1/approvals/conflicts');
    if (!v) { box.innerHTML = '<tr><td colspan="3" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = (v.conflicts || []).map(function (x) {
      return '<tr><td>' + badge(x.entity) + ' #' + x.entity_id + '</td><td>' + x.requests.map(function (r) { return '#' + r.id + ' ' + esc(r.title); }).join('; ') + '</td><td><strong>' + x.count + '</strong></td></tr>';
    }).join('') || '<tr><td colspan="3" class="text-muted">Tidak ada konflik. Satu entitas, satu antrean.</td></tr>';
  }
  on('cfx-reload', 'click', cfxRefresh);
  /* ---- approval bulk decide (F-D09-007) ---- */
  async function blkRefresh() {
    var box = document.getElementById('blk-list');
    if (!box) return;
    var v = await j('/api/v1/approvals/decidable');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = (v.rows || []).map(function (x, i) {
      return '<tr><td><input type="checkbox" data-blk="' + i + '"></td><td>#' + x.id + ' ' + esc(x.title) + '</td><td>' + x.step_no + '</td><td>' + badge(x.role) + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Tidak ada langkah untuk Anda.</td></tr>';
    box._rows = v.rows || [];
  }
  function blkSelected() {
    var box = document.getElementById('blk-list');
    var rows = (box && box._rows) || [];
    return Array.prototype.map.call(document.querySelectorAll('[data-blk]:checked'), function (c) { return rows[+c.getAttribute('data-blk')]; }).filter(Boolean);
  }
  async function blkDecide(decision) {
    var sel = blkSelected();
    if (!sel.length) { pmsg('blk-msg', 'Pilih minimal satu langkah.'); return; }
    var r = await ppostKey('/api/v1/approvals/decide-bulk', { items: sel.map(function (x) { return { id: x.id, step_no: x.step_no, decision: decision }; }) }, pval('blk-key'));
    if (!r.ok) { pmsg('blk-msg', 'Gagal (' + r.status + '). Dibatalkan semua — periksa antrean.'); return; }
    pmsg('blk-msg', '');
    blkRefresh();
    apprRefresh();
  }
  on('blk-reload', 'click', blkRefresh);
  on('blk-ok', 'click', function () { blkDecide('approve'); });
  on('blk-no', 'click', function () { blkDecide('reject'); });
  /* ---- approval archive (F-D09-031) ---- */
  async function arcRefresh() {
    var box = document.getElementById('arc-list');
    if (!box) return;
    var f = pval('arc-status');
    var v = await j('/api/v1/approvals/archive' + (f ? '?status=' + encodeURIComponent(f) : ''));
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = (v.rows || []).map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + esc(x.entity) + ' #' + x.entity_id + '</td><td>' + badge(x.status) + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Arsip kosong.</td></tr>';
  }
  on('arc-reload', 'click', arcRefresh);
  /* ---- approval digest prefs (F-D09-028) ---- */
  async function dgstRefresh() {
    var box = document.getElementById('dgst-out');
    var v = await j('/api/v1/approvals/prefs');
    if (!v) { if (box) box.textContent = 'Gagal memuat.'; return; }
    var sel = document.getElementById('dgst-mode');
    if (sel) sel.value = v.digest_mode || 'instant';
    var to = document.getElementById('dgst-to');
    if (to && !to.value) to.value = v.escalate_to || '';
    if (box) box.textContent = 'Mode: ' + (v.digest_mode || 'instant') + (v.escalate_to ? ' · eskalasi ke ' + v.escalate_to : '');
  }
  on('dgst-save', 'click', async function () {
    var r = await ppost('/api/v1/approvals/prefs', { digest_mode: pval('dgst-mode') || 'instant', escalate_to: pval('dgst-to') || '' });
    if (!r.ok) { pmsg('dgst-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('dgst-msg', '');
    dgstRefresh();
  });
  on('dgst-reload', 'click', dgstRefresh);
  /* ---- approval freeze (F-D09-023) ---- */
  async function frzRefresh() {
    var box = document.getElementById('frz-out');
    var v = await j('/api/v1/approvals/freeze');
    if (!v) { if (box) box.innerHTML = '<span class="text-muted">Gagal memuat.</span>'; return; }
    if (box) box.innerHTML = v.frozen ? '<span class="badge bg-red-lt">BEKU</span> <span class="text-muted">' + esc(v.reason || '') + '</span>' : '<span class="badge bg-green-lt">terbuka</span>';
  }
  async function frzSet(frozen) {
    var r = await ppost('/api/v1/approvals/freeze', { frozen: frozen, reason: pval('frz-reason') || '' });
    if (!r.ok) { pmsg('frz-msg', 'Gagal (' + r.status + '). Perlu admin?'); return; }
    pmsg('frz-msg', '');
    frzRefresh();
  }
  on('frz-on', 'click', function () { frzSet(true); });
  on('frz-off', 'click', function () { frzSet(false); });
  on('frz-reload', 'click', frzRefresh);
  on('appr-csv', 'click', function () { dlCsv('/api/v1/reports/approvals.csv', 'approvals.csv', 'appr-msg'); });
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
    a.addEventListener('click', function () { apprRefresh(); apprNotifRefresh(); delegRefresh(); apprOverdueRefresh(); apprCalRefresh(); slaRefresh(); cfxRefresh(); blkRefresh(); arcRefresh(); dgstRefresh(); frzRefresh(); });
  });
  qsa('.snav[data-tab="appr"]', function (a) {
    a.addEventListener('click', function () { apprRefresh(); apprNotifRefresh(); apprOverdueRefresh(); apprCalRefresh(); slaRefresh(); cfxRefresh(); blkRefresh(); arcRefresh(); dgstRefresh(); frzRefresh(); });
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
  /* ---- compliance: retention rules ---- */
  async function rtrRefresh() {
    var box = document.getElementById('rtr-list');
    if (!box) return;
    var v = await j('/api/v1/retention-rules');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var tgl = x.active ? 'nonaktifkan' : 'aktifkan';
      return '<tr><td>' + badge(x.entity) + '</td><td>' + x.retain_years + ' th</td><td>' + badge(x.action) + '</td><td>' + (x.active ? 'ya' : 'tidak') + '</td><td><button class="btn btn-sm" data-rtr-tgl="' + x.id + '" data-rtr-to="' + (x.active ? '0' : '1') + '" type="button">' + tgl + '</button> <button class="btn btn-sm btn-outline-danger" data-rtr-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada aturan.</td></tr>';
    qsa('[data-rtr-tgl]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/retention-rules/' + b.getAttribute('data-rtr-tgl') + '/active', { active: b.getAttribute('data-rtr-to') === '1' });
        if (!r.ok) { pmsg('rtr-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('rtr-msg', '');
        rtrRefresh();
      });
    });
    qsa('[data-rtr-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/retention-rules/' + b.getAttribute('data-rtr-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('rtr-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('rtr-msg', '');
        rtrRefresh();
      });
    });
  }
  on('rtr-save', 'click', async function () {
    var r = await ppost('/api/v1/retention-rules', { entity: pval('rtr-entity') || 'bpn', retain_years: +pval('rtr-years') || 5, action: pval('rtr-action') || 'archive' });
    if (!r.ok) { pmsg('rtr-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('rtr-msg', '');
    rtrRefresh();
  });
  on('rtr-reload', 'click', rtrRefresh);
  /* ---- compliance: purge requests ---- */
  async function prg2Refresh() {
    var box = document.getElementById('prg2-list');
    if (!box) return;
    var v = await j('/api/v1/purge-requests');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var act = '';
      if (x.status === 'requested') act = '<button class="btn btn-sm" data-prg2-yes="' + x.id + '" type="button">Setujui</button> <button class="btn btn-sm btn-outline-danger" data-prg2-no="' + x.id + '" type="button">Tolak</button>';
      else if (x.status === 'approved') act = '<button class="btn btn-sm btn-outline-danger" data-prg2-exe="' + x.id + '" type="button">Eksekusi</button>';
      else act = '<span class="text-muted">' + x.status + '</span>';
      return '<tr><td>' + x.id + '</td><td>' + badge(x.entity) + ' #' + x.entity_id + ' <span class="text-muted">(' + esc(x.reason) + ')</span></td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada permintaan.</td></tr>';
    qsa('[data-prg2-yes]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/purge-requests/' + b.getAttribute('data-prg2-yes') + '/decide', { approve: true });
        if (!r.ok) { pmsg('prg2-msg', 'Gagal (' + r.status + '). Bukan penyetuju / menyetujui sendiri?'); return; }
        pmsg('prg2-msg', '');
        prg2Refresh();
      });
    });
    qsa('[data-prg2-no]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/purge-requests/' + b.getAttribute('data-prg2-no') + '/decide', { approve: false });
        if (!r.ok) { pmsg('prg2-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('prg2-msg', '');
        prg2Refresh();
      });
    });
    qsa('[data-prg2-exe]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/purge-requests/' + b.getAttribute('data-prg2-exe') + '/execute', {});
        if (!r.ok) { pmsg('prg2-msg', 'Gagal (' + r.status + '). Perlu aturan purge aktif?'); return; }
        pmsg('prg2-msg', '');
        prg2Refresh();
      });
    });
  }
  on('prg2-add', 'click', async function () {
    if (!pval('prg2-id') || !pval('prg2-reason')) { pmsg('prg2-msg', 'Isi ID baris dan alasan.'); return; }
    var r = await ppost('/api/v1/purge-requests', { entity: pval('prg2-entity') || 'bpn', entity_id: +pval('prg2-id'), reason: pval('prg2-reason') });
    if (!r.ok) { pmsg('prg2-msg', 'Gagal (' + r.status + '). Entitas tidak dapat di-purge?'); return; }
    pmsg('prg2-msg', '');
    prg2Refresh();
  });
  on('prg2-reload', 'click', prg2Refresh);
  /* ---- compliance: protocol retention schedule ---- */
  async function pretRefresh() {
    var box = document.getElementById('pret-list');
    if (!box) return;
    var yrs = parseInt(pval('pret-years'), 10);
    if (!(yrs >= 1)) yrs = 25;
    var v = await j('/api/v1/reports/protocol-retention?years=' + yrs);
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    var cut = document.getElementById('pret-cut');
    if (cut) cut.textContent = 'Batas ' + v.cutoff + ' — ' + v.due_count + ' entri jatuh tempo';
    box.innerHTML = (v.due || []).map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.entry_date) + '</td><td>' + esc(x.book_ref) + '</td><td>' + esc(x.deed_no) + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Tidak ada yang jatuh tempo.</td></tr>';
  }
  on('pret-reload', 'click', pretRefresh);
  /* ---- compliance: AML scores ---- */
  async function amlRefresh() {
    var box = document.getElementById('aml-list');
    if (!box) return;
    var v = await j('/api/v1/aml-scores');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = (v.rows || []).map(function (x) {
      var bc = x.band === 'high' ? '<span class="badge bg-red-lt">tinggi</span>' : (x.band === 'medium' ? '<span class="badge bg-yellow-lt">sedang</span>' : '<span class="badge bg-green-lt">rendah</span>');
      return '<tr><td>' + esc(x.name) + ' <span class="text-muted">(' + esc(x.kind) + ', ' + x.matters + ' perkara)</span></td><td><strong>' + x.score + '</strong></td><td>' + bc + '</td><td>' + esc((x.factors || []).join('; ')) + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada klien.</td></tr>';
  }
  on('aml-reload', 'click', amlRefresh);
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
  /* ---- compliance: incidents ---- */
  var INC_NEXT = { open: 'investigating', investigating: 'resolved', resolved: 'closed' };
  async function incRefresh() {
    var box = document.getElementById('inc-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/incidents');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = INC_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-inc-next="' + x.id + '" data-inc-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">ditutup</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-inc-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + badge(x.severity) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada insiden.</td></tr>';
    qsa('[data-inc-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/incidents/' + b.getAttribute('data-inc-next') + '/status', { status: b.getAttribute('data-inc-to') });
        if (!r.ok) { pmsg('inc-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('inc-msg', '');
        incRefresh();
      });
    });
    qsa('[data-inc-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/incidents/' + b.getAttribute('data-inc-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('inc-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('inc-msg', '');
        incRefresh();
      });
    });
  }
  on('inc-add', 'click', async function () {
    if (!pval('inc-title')) { pmsg('inc-msg', 'Isi judul insiden.'); return; }
    var r = await ppost('/api/v1/compliance/incidents', { title: pval('inc-title'), severity: pval('inc-sev') || 'medium' });
    if (!r.ok) { pmsg('inc-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('inc-msg', '');
    incRefresh();
  });
  on('inc-reload', 'click', incRefresh);
  /* ---- compliance: CAPA tracker ---- */
  var CAPA_NEXT = { open: 'in_progress', in_progress: 'done', done: 'verified' };
  async function capaRefresh() {
    var box = document.getElementById('capa-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/capa');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = CAPA_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-capa-next="' + x.id + '" data-capa-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">terverifikasi</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-capa-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + esc(x.cause) + '</td><td>' + esc(x.action) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada CAPA.</td></tr>';
    qsa('[data-capa-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/capa/' + b.getAttribute('data-capa-next') + '/status', { status: b.getAttribute('data-capa-to') });
        if (!r.ok) { pmsg('capa-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('capa-msg', '');
        capaRefresh();
      });
    });
    qsa('[data-capa-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/capa/' + b.getAttribute('data-capa-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('capa-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('capa-msg', '');
        capaRefresh();
      });
    });
  }
  on('capa-add', 'click', async function () {
    if (!pval('capa-title')) { pmsg('capa-msg', 'Isi judul tindakan.'); return; }
    var r = await ppost('/api/v1/compliance/capa', { title: pval('capa-title'), cause: pval('capa-cause') || '', action: pval('capa-action') || '' });
    if (!r.ok) { pmsg('capa-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('capa-msg', '');
    capaRefresh();
  });
  on('capa-reload', 'click', capaRefresh);
  /* ---- compliance: regulatory change tracker ---- */
  var REG_NEXT = { open: 'acknowledged', acknowledged: 'implemented' };
  async function regRefresh() {
    var box = document.getElementById('reg-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/regchanges');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = REG_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-reg-next="' + x.id + '" data-reg-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">diterapkan</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-reg-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + esc(x.source) + '</td><td>' + esc(x.effective_date) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada perubahan regulasi.</td></tr>';
    qsa('[data-reg-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/regchanges/' + b.getAttribute('data-reg-next') + '/status', { status: b.getAttribute('data-reg-to') });
        if (!r.ok) { pmsg('reg-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('reg-msg', '');
        regRefresh();
      });
    });
    qsa('[data-reg-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/regchanges/' + b.getAttribute('data-reg-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('reg-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('reg-msg', '');
        regRefresh();
      });
    });
  }
  on('reg-add', 'click', async function () {
    if (!pval('reg-title')) { pmsg('reg-msg', 'Isi judul peraturan.'); return; }
    var r = await ppost('/api/v1/compliance/regchanges', { title: pval('reg-title'), source: pval('reg-source') || '', effective_date: pval('reg-effective') || '' });
    if (!r.ok) { pmsg('reg-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('reg-msg', '');
    regRefresh();
  });
  on('reg-reload', 'click', regRefresh);
  /* ---- compliance: training assignments ---- */
  var TRN_NEXT = { assigned: 'in_progress', in_progress: 'completed' };
  async function trnRefresh() {
    var box = document.getElementById('trn-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/trainings');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = TRN_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-trn-next="' + x.id + '" data-trn-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">selesai</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-trn-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + esc(x.assignee) + '</td><td>' + esc(x.due_date) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada penugasan pelatihan.</td></tr>';
    qsa('[data-trn-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/trainings/' + b.getAttribute('data-trn-next') + '/status', { status: b.getAttribute('data-trn-to') });
        if (!r.ok) { pmsg('trn-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('trn-msg', '');
        trnRefresh();
      });
    });
    qsa('[data-trn-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/trainings/' + b.getAttribute('data-trn-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('trn-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('trn-msg', '');
        trnRefresh();
      });
    });
  }
  on('trn-add', 'click', async function () {
    if (!pval('trn-title')) { pmsg('trn-msg', 'Isi judul pelatihan.'); return; }
    var r = await ppost('/api/v1/compliance/trainings', { title: pval('trn-title'), assignee: pval('trn-who') || '', due_date: pval('trn-due') || '' });
    if (!r.ok) { pmsg('trn-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('trn-msg', '');
    trnRefresh();
  });
  on('trn-reload', 'click', trnRefresh);
  /* ---- compliance: risk register + heatmap ---- */
  var RISK_NEXT = { open: 'mitigating', mitigating: 'mitigated', mitigated: 'closed' };
  async function riskRefresh() {
    var box = document.getElementById('risk-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/risks');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = RISK_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-risk-next="' + x.id + '" data-risk-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">ditutup</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-risk-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + ' <span class="text-muted">(' + esc(x.category) + ' K' + x.likelihood + '×D' + x.impact + ')</span></td><td><strong>' + x.score + '</strong></td><td>' + esc(x.owner) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada risiko.</td></tr>';
    qsa('[data-risk-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/risks/' + b.getAttribute('data-risk-next') + '/status', { status: b.getAttribute('data-risk-to') });
        if (!r.ok) { pmsg('risk-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('risk-msg', '');
        riskRefresh();
        riskHmRefresh();
      });
    });
    qsa('[data-risk-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/risks/' + b.getAttribute('data-risk-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('risk-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('risk-msg', '');
        riskRefresh();
        riskHmRefresh();
      });
    });
  }
  async function riskHmRefresh() {
    var box = document.getElementById('risk-hm');
    var sum = document.getElementById('risk-hm-sum');
    if (!box) return;
    var v = await j('/api/v1/compliance/risks/heatmap');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    if (sum) sum.textContent = 'Terbuka: ' + v.total_open + ' • Skor ≥12: ' + v.high_count;
    box.innerHTML = (v.matrix || []).map(function (row, li) {
      return '<tr><td><strong>K' + (li + 1) + '</strong></td>' + row.map(function (c, im) {
        var hot = (li + 1) * (im + 1) >= 12;
        return '<td' + (hot && c > 0 ? ' class="text-danger"' : '') + '>' + (c || '·') + '</td>';
      }).join('') + '</tr>';
    }).join('');
  }
  on('risk-add', 'click', async function () {
    if (!pval('risk-title')) { pmsg('risk-msg', 'Isi judul risiko.'); return; }
    var r = await ppost('/api/v1/compliance/risks', { title: pval('risk-title'), category: pval('risk-cat') || 'operational', likelihood: +pval('risk-like') || 3, impact: +pval('risk-imp') || 3, owner: pval('risk-owner') || '' });
    if (!r.ok) { pmsg('risk-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('risk-msg', '');
    riskRefresh();
    riskHmRefresh();
  });
  on('risk-reload', 'click', function () { riskRefresh(); riskHmRefresh(); });
  on('risk-hm-reload', 'click', riskHmRefresh);
  /* ---- compliance: data-subject rights (DSAR) ---- */
  var DSAR_NEXT = { open: 'in_review', in_review: 'fulfilled' };
  async function dsarRefresh() {
    var box = document.getElementById('dsar-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/dsar');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = DSAR_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-dsar-next="' + x.id + '" data-dsar-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">selesai</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-dsar-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.subject) + '</td><td>' + badge(x.kind) + '</td><td>' + esc(x.due_date) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada permintaan hak subjek.</td></tr>';
    qsa('[data-dsar-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/dsar/' + b.getAttribute('data-dsar-next') + '/status', { status: b.getAttribute('data-dsar-to') });
        if (!r.ok) { pmsg('dsar-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('dsar-msg', '');
        dsarRefresh();
      });
    });
    qsa('[data-dsar-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/dsar/' + b.getAttribute('data-dsar-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('dsar-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('dsar-msg', '');
        dsarRefresh();
      });
    });
  }
  on('dsar-add', 'click', async function () {
    if (!pval('dsar-subject') || !pval('dsar-detail')) { pmsg('dsar-msg', 'Isi subjek dan rincian.'); return; }
    var r = await ppost('/api/v1/compliance/dsar', { subject: pval('dsar-subject'), kind: pval('dsar-kind') || 'access', detail: pval('dsar-detail'), due_date: pval('dsar-due') || '' });
    if (!r.ok) { pmsg('dsar-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('dsar-msg', '');
    dsarRefresh();
  });
  on('dsar-reload', 'click', dsarRefresh);
  /* ---- compliance: ethics declarations ---- */
  async function ethicsRefresh() {
    var box = document.getElementById('ethics-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/ethics');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var act = (x.status === 'submitted' ? '<button class="btn btn-sm" data-ethics-next="' + x.id + '" type="button">→ acknowledged</button> ' : '<span class="text-muted">diketahui</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-ethics-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.declarant) + '</td><td>' + esc(x.year) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada deklarasi.</td></tr>';
    qsa('[data-ethics-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/ethics/' + b.getAttribute('data-ethics-next') + '/status', { status: 'acknowledged' });
        if (!r.ok) { pmsg('ethics-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('ethics-msg', '');
        ethicsRefresh();
      });
    });
    qsa('[data-ethics-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/ethics/' + b.getAttribute('data-ethics-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('ethics-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('ethics-msg', '');
        ethicsRefresh();
      });
    });
  }
  on('ethics-add', 'click', async function () {
    if (!pval('ethics-who') || !pval('ethics-stmt')) { pmsg('ethics-msg', 'Isi pendeklarasi dan pernyataan.'); return; }
    var r = await ppost('/api/v1/compliance/ethics', { declarant: pval('ethics-who'), year: pval('ethics-year') || '', statement: pval('ethics-stmt') });
    if (!r.ok) { pmsg('ethics-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('ethics-msg', '');
    ethicsRefresh();
  });
  on('ethics-reload', 'click', ethicsRefresh);
  /* ---- compliance: gift register ---- */
  var GIFT_NEXT = { logged: 'reviewed', reviewed: 'cleared' };
  async function giftRefresh() {
    var box = document.getElementById('gift-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/gifts');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = GIFT_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-gift-next="' + x.id + '" data-gift-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">selesai</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-gift-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + badge(x.direction) + '</td><td>' + esc(x.description) + ' <span class="text-muted">(' + esc(x.counterpart) + ')</span></td><td>' + esc(x.given_at) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada catatan gratifikasi.</td></tr>';
    qsa('[data-gift-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/gifts/' + b.getAttribute('data-gift-next') + '/status', { status: b.getAttribute('data-gift-to') });
        if (!r.ok) { pmsg('gift-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('gift-msg', '');
        giftRefresh();
      });
    });
    qsa('[data-gift-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/gifts/' + b.getAttribute('data-gift-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('gift-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('gift-msg', '');
        giftRefresh();
      });
    });
  }
  on('gift-add', 'click', async function () {
    if (!pval('gift-desc')) { pmsg('gift-msg', 'Isi uraian hadiah.'); return; }
    var r = await ppost('/api/v1/compliance/gifts', { direction: pval('gift-dir') || 'received', description: pval('gift-desc'), counterpart: pval('gift-who') || '', given_at: pval('gift-at') || '' });
    if (!r.ok) { pmsg('gift-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('gift-msg', '');
    giftRefresh();
  });
  on('gift-reload', 'click', giftRefresh);
  /* ---- compliance: training evidence ---- */
  async function tevRefresh() {
    var box = document.getElementById('tev-list');
    if (!box) return;
    var tid = pval('tev-trn');
    if (!tid) { box.innerHTML = '<tr><td colspan="3" class="text-muted">Isi ID pelatihan lalu Muat.</td></tr>'; return; }
    var v = await j('/api/v1/compliance/trainings/' + encodeURIComponent(tid) + '/evidence');
    if (!v) { box.innerHTML = '<tr><td colspan="3" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.note) + '</td><td><button class="btn btn-sm btn-outline-danger" data-tev-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="3" class="text-muted">Belum ada bukti.</td></tr>';
    qsa('[data-tev-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/evidence/' + b.getAttribute('data-tev-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('tev-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('tev-msg', '');
        tevRefresh();
      });
    });
  }
  on('tev-add', 'click', async function () {
    if (!pval('tev-trn') || !pval('tev-note')) { pmsg('tev-msg', 'Isi ID pelatihan dan catatan.'); return; }
    var r = await ppost('/api/v1/compliance/trainings/' + encodeURIComponent(pval('tev-trn')) + '/evidence', { note: pval('tev-note') });
    if (!r.ok) { pmsg('tev-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('tev-msg', '');
    tevRefresh();
  });
  on('tev-reload', 'click', tevRefresh);
  /* ---- compliance: audit findings ---- */
  var AF_NEXT = { open: 'in_remediation', in_remediation: 'closed' };
  async function afRefresh() {
    var box = document.getElementById('af-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/findings');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = AF_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-af-next="' + x.id + '" data-af-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">ditutup</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-af-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + esc(x.area) + '</td><td>' + badge(x.severity) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada temuan.</td></tr>';
    qsa('[data-af-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/findings/' + b.getAttribute('data-af-next') + '/status', { status: b.getAttribute('data-af-to') });
        if (!r.ok) { pmsg('af-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('af-msg', '');
        afRefresh();
      });
    });
    qsa('[data-af-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/findings/' + b.getAttribute('data-af-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('af-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('af-msg', '');
        afRefresh();
      });
    });
  }
  on('af-add', 'click', async function () {
    if (!pval('af-title')) { pmsg('af-msg', 'Isi judul temuan.'); return; }
    var r = await ppost('/api/v1/compliance/findings', { title: pval('af-title'), area: pval('af-area') || 'general', severity: pval('af-sev') || 'medium' });
    if (!r.ok) { pmsg('af-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('af-msg', '');
    afRefresh();
  });
  on('af-reload', 'click', afRefresh);
  /* ---- compliance: whistleblowing ---- */
  var WB_NEXT = { received: 'under_review', under_review: 'resolved' };
  async function wbRefresh() {
    var box = document.getElementById('wb-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/whistle');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = WB_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-wb-next="' + x.id + '" data-wb-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">selesai</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-wb-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + badge(x.channel) + ' <span class="text-muted">(' + esc(x.category) + ')</span></td><td>' + esc(x.summary) + '</td><td>' + esc(x.contact) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada laporan.</td></tr>';
    qsa('[data-wb-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/whistle/' + b.getAttribute('data-wb-next') + '/status', { status: b.getAttribute('data-wb-to') });
        if (!r.ok) { pmsg('wb-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('wb-msg', '');
        wbRefresh();
      });
    });
    qsa('[data-wb-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/whistle/' + b.getAttribute('data-wb-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('wb-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('wb-msg', '');
        wbRefresh();
      });
    });
  }
  on('wb-add', 'click', async function () {
    if (!pval('wb-sum')) { pmsg('wb-msg', 'Isi ringkasan laporan.'); return; }
    var r = await ppost('/api/v1/compliance/whistle', { channel: pval('wb-channel') || 'web', category: pval('wb-cat') || 'general', summary: pval('wb-sum'), contact: pval('wb-contact') || '' });
    if (!r.ok) { pmsg('wb-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('wb-msg', '');
    wbRefresh();
  });
  on('wb-reload', 'click', wbRefresh);
  /* ---- compliance: waivers ---- */
  async function wvRefresh() {
    var box = document.getElementById('wv-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/waivers');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var act;
      if (x.status === 'requested') {
        act = '<button class="btn btn-sm btn-success" data-wv-ok="' + x.id + '" type="button">Setujui</button> <button class="btn btn-sm btn-danger" data-wv-no="' + x.id + '" type="button">Tolak</button> ';
      } else if (x.status === 'approved') {
        act = '<button class="btn btn-sm" data-wv-rev="' + x.id + '" type="button">Cabut</button> ';
      } else {
        act = '<span class="text-muted">final</span> ';
      }
      act += '<button class="btn btn-sm btn-outline-danger" data-wv-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + esc(x.requested_by) + '</td><td>' + esc(x.expires_at) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada pengajuan.</td></tr>';
    qsa('[data-wv-ok]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/waivers/' + b.getAttribute('data-wv-ok') + '/status', { status: 'approved' });
        if (!r.ok) { pmsg('wv-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('wv-msg', '');
        wvRefresh();
      });
    });
    qsa('[data-wv-no]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/waivers/' + b.getAttribute('data-wv-no') + '/status', { status: 'rejected' });
        if (!r.ok) { pmsg('wv-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('wv-msg', '');
        wvRefresh();
      });
    });
    qsa('[data-wv-rev]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/waivers/' + b.getAttribute('data-wv-rev') + '/status', { status: 'revoked' });
        if (!r.ok) { pmsg('wv-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('wv-msg', '');
        wvRefresh();
      });
    });
    qsa('[data-wv-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/waivers/' + b.getAttribute('data-wv-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('wv-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('wv-msg', '');
        wvRefresh();
      });
    });
  }
  on('wv-add', 'click', async function () {
    if (!pval('wv-title') || !pval('wv-just')) { pmsg('wv-msg', 'Isi judul dan justifikasi.'); return; }
    var r = await ppost('/api/v1/compliance/waivers', { title: pval('wv-title'), justification: pval('wv-just'), requested_by: pval('wv-by') || '', expires_at: pval('wv-exp') || '' });
    if (!r.ok) { pmsg('wv-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('wv-msg', '');
    wvRefresh();
  });
  on('wv-reload', 'click', wvRefresh);
  /* ---- compliance: policy library + acks ---- */
  var POL_NEXT = { draft: 'published', published: 'archived' };
  async function polRefresh() {
    var box = document.getElementById('pol-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/policies');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = POL_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-pol-next="' + x.id + '" data-pol-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '')
        + (x.status === 'published' ? '<button class="btn btn-sm btn-success" data-pol-ack="' + x.id + '" type="button">Saya setuju</button> ' : '')
        + '<button class="btn btn-sm btn-outline-danger" data-pol-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + esc(x.version) + '</td><td>' + badge(x.status) + '</td><td>' + x.ack_count + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada kebijakan.</td></tr>';
    qsa('[data-pol-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/policies/' + b.getAttribute('data-pol-next') + '/status', { status: b.getAttribute('data-pol-to') });
        if (!r.ok) { pmsg('pol-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('pol-msg', '');
        polRefresh();
      });
    });
    qsa('[data-pol-ack]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/policies/' + b.getAttribute('data-pol-ack') + '/ack', {});
        if (!r.ok) { pmsg('pol-msg', 'Gagal (' + r.status + '). Mungkin sudah pernah.'); return; }
        pmsg('pol-msg', '');
        polRefresh();
      });
    });
    qsa('[data-pol-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/policies/' + b.getAttribute('data-pol-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('pol-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('pol-msg', '');
        polRefresh();
      });
    });
  }
  on('pol-add', 'click', async function () {
    if (!pval('pol-title') || !pval('pol-body')) { pmsg('pol-msg', 'Isi judul dan isi kebijakan.'); return; }
    var r = await ppost('/api/v1/compliance/policies', { title: pval('pol-title'), version: pval('pol-ver') || '1.0', body: pval('pol-body') });
    if (!r.ok) { pmsg('pol-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('pol-msg', '');
    polRefresh();
  });
  on('pol-reload', 'click', polRefresh);
  /* ---- compliance: control library ---- */
  async function ctlRefresh() {
    var box = document.getElementById('ctl-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/controls');
    if (!v) { box.innerHTML = '<tr><td colspan="6" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var act = '<button class="btn btn-sm" data-ctl-tg="' + x.id + '" data-ctl-on="' + (!x.active) + '" type="button">' + (x.active ? 'Nonaktifkan' : 'Aktifkan') + '</button> '
        + '<button class="btn btn-sm btn-outline-danger" data-ctl-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.code) + '</td><td>' + esc(x.title) + ' <span class="text-muted">(' + esc(x.area) + ')</span></td><td>' + esc(x.frequency) + '</td><td>' + (x.active ? '<span class="badge bg-green-lt">aktif</span>' : '<span class="badge bg-yellow-lt">nonaktif</span>') + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="6" class="text-muted">Belum ada kontrol.</td></tr>';
    qsa('[data-ctl-tg]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/controls/' + b.getAttribute('data-ctl-tg') + '/active', { active: b.getAttribute('data-ctl-on') === 'true' });
        if (!r.ok) { pmsg('ctl-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('ctl-msg', '');
        ctlRefresh();
      });
    });
    qsa('[data-ctl-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/controls/' + b.getAttribute('data-ctl-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('ctl-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('ctl-msg', '');
        ctlRefresh();
      });
    });
  }
  on('ctl-add', 'click', async function () {
    if (!pval('ctl-code') || !pval('ctl-title')) { pmsg('ctl-msg', 'Isi kode dan judul kontrol.'); return; }
    var r = await ppost('/api/v1/compliance/controls', { code: pval('ctl-code'), title: pval('ctl-title'), area: pval('ctl-area') || 'general', frequency: pval('ctl-freq') || 'quarterly' });
    if (!r.ok) { pmsg('ctl-msg', 'Gagal (' + r.status + '). Kode mungkin duplikat.'); return; }
    pmsg('ctl-msg', '');
    ctlRefresh();
  });
  on('ctl-reload', 'click', ctlRefresh);
  /* ---- compliance: control test scheduler ---- */
  async function ctdueRefresh() {
    var box = document.getElementById('ctdue-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/controls/due');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var st = x.overdue ? '<span class="badge bg-red-lt">terlambat</span>' : '<span class="badge bg-green-lt">tepat waktu</span>';
      return '<tr><td>' + esc(x.code) + ' <span class="text-muted">(' + esc(x.frequency) + ')</span></td><td>' + esc(x.last_tested_at || 'belum pernah') + '</td><td>' + esc(x.next_due) + '</td><td>' + st + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Tidak ada kontrol aktif.</td></tr>';
  }
  on('ctdue-reload', 'click', ctdueRefresh);
  async function ctestRefresh() {
    var box = document.getElementById('ctest-list');
    if (!box) return;
    var cid = pval('ctest-ctl');
    if (!cid) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Isi ID kontrol lalu Muat.</td></tr>'; return; }
    var v = await j('/api/v1/compliance/controls/' + encodeURIComponent(cid) + '/tests');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.tested_at) + '</td><td>' + badge(x.result) + '</td><td>' + esc(x.note) + '</td><td><button class="btn btn-sm btn-outline-danger" data-ctest-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada log uji.</td></tr>';
    qsa('[data-ctest-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/ctests/' + b.getAttribute('data-ctest-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('ctest-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('ctest-msg', '');
        ctestRefresh();
        ctdueRefresh();
      });
    });
  }
  on('ctest-add', 'click', async function () {
    if (!pval('ctest-ctl')) { pmsg('ctest-msg', 'Isi ID kontrol.'); return; }
    var r = await ppost('/api/v1/compliance/controls/' + encodeURIComponent(pval('ctest-ctl')) + '/tests', { result: pval('ctest-res') || 'pass', note: pval('ctest-note') || '' });
    if (!r.ok) { pmsg('ctest-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('ctest-msg', '');
    ctestRefresh();
    ctdueRefresh();
  });
  on('ctest-reload', 'click', ctestRefresh);
  /* ---- compliance: audit plans ---- */
  var AP_NEXT = { planned: 'in_progress', in_progress: 'completed' };
  async function aplanRefresh() {
    var box = document.getElementById('aplan-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/aplans');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var nx = AP_NEXT[x.status];
      var act = (nx ? '<button class="btn btn-sm" data-aplan-next="' + x.id + '" data-aplan-to="' + nx + '" type="button">→ ' + nx + '</button> ' : '<span class="text-muted">selesai</span> ')
        + '<button class="btn btn-sm btn-outline-danger" data-aplan-del="' + x.id + '" type="button">Hapus</button>';
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + ' <span class="text-muted">(' + esc(x.scope) + ')</span></td><td>' + esc(x.period) + '</td><td>' + badge(x.status) + '</td><td>' + act + '</td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada rencana audit.</td></tr>';
    qsa('[data-aplan-next]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/compliance/aplans/' + b.getAttribute('data-aplan-next') + '/status', { status: b.getAttribute('data-aplan-to') });
        if (!r.ok) { pmsg('aplan-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('aplan-msg', '');
        aplanRefresh();
      });
    });
    qsa('[data-aplan-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/aplans/' + b.getAttribute('data-aplan-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('aplan-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('aplan-msg', '');
        aplanRefresh();
      });
    });
  }
  on('aplan-add', 'click', async function () {
    if (!pval('aplan-title')) { pmsg('aplan-msg', 'Isi judul rencana.'); return; }
    var r = await ppost('/api/v1/compliance/aplans', { title: pval('aplan-title'), period: pval('aplan-period') || '', scope: pval('aplan-scope') || '' });
    if (!r.ok) { pmsg('aplan-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('aplan-msg', '');
    aplanRefresh();
  });
  on('aplan-reload', 'click', aplanRefresh);
  /* ---- compliance: matter checklist ---- */
  async function mclRefresh() {
    var box = document.getElementById('mcl-list');
    if (!box) return;
    var mid = pval('mcl-matter');
    if (!mid) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Isi ID perkara lalu Muat.</td></tr>'; return; }
    var v = await j('/api/v1/matters/' + encodeURIComponent(mid) + '/checklist');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    var done = v.filter(function (x) { return x.done; }).length;
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.item) + '</td><td>' + (x.done ? '<span class="badge bg-green-lt">ya</span>' : '<span class="badge bg-yellow-lt">belum</span>') + '</td>'
        + '<td><button class="btn btn-sm" data-mcl-tg="' + x.id + '" data-mcl-done="' + (!x.done) + '" type="button">Ubah</button> '
        + '<button class="btn btn-sm btn-outline-danger" data-mcl-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada butir.</td></tr>';
    if (v.length) box.innerHTML += '<tr><td colspan="4" class="text-muted">Progres: ' + done + '/' + v.length + ' selesai.</td></tr>';
    qsa('[data-mcl-tg]', function (b) {
      b.addEventListener('click', async function () {
        var r = await ppost('/api/v1/checklist/' + b.getAttribute('data-mcl-tg') + '/toggle', { done: b.getAttribute('data-mcl-done') === 'true' });
        if (!r.ok) { pmsg('mcl-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('mcl-msg', '');
        mclRefresh();
      });
    });
    qsa('[data-mcl-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/checklist/' + b.getAttribute('data-mcl-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('mcl-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('mcl-msg', '');
        mclRefresh();
      });
    });
  }
  on('mcl-add', 'click', async function () {
    if (!pval('mcl-matter') || !pval('mcl-item')) { pmsg('mcl-msg', 'Isi ID perkara dan butir.'); return; }
    var r = await ppost('/api/v1/matters/' + encodeURIComponent(pval('mcl-matter')) + '/checklist', { item: pval('mcl-item') });
    if (!r.ok) { pmsg('mcl-msg', 'Gagal (' + r.status + ').'); return; }
    pmsg('mcl-msg', '');
    mclRefresh();
  });
  on('mcl-reload', 'click', mclRefresh);
  /* ---- compliance: scorecard ---- */
  async function scoreRefresh() {
    var box = document.getElementById('score-list');
    var sum = document.getElementById('score-sum');
    if (!box) return;
    var v = await j('/api/v1/compliance/scorecard');
    if (!v) { box.innerHTML = '<tr><td colspan="2" class="text-muted">Gagal memuat.</td></tr>'; return; }
    if (sum) sum.textContent = 'Skor keseluruhan: ' + (v.overall === null || v.overall === undefined ? '—' : v.overall + '%');
    box.innerHTML = (v.parts || []).map(function (x) {
      return '<tr><td>' + esc(x.area) + '</td><td>' + (x.value === null || x.value === undefined ? '<span class="text-muted">—</span>' : '<strong>' + x.value + '%</strong>') + '</td></tr>';
    }).join('');
  }
  on('score-reload', 'click', scoreRefresh);
  /* ---- office: self-check runs ---- */
  async function scRefresh() {
    var box = document.getElementById('sc-list');
    if (!box) return;
    var v = await j('/api/v1/selfchecks');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.period) + '</td><td><strong>' + x.pct + '%</strong> <span class="text-muted">(' + x.passed_items + '/' + x.total_items + ')</span></td><td><button class="btn btn-sm btn-outline-danger" data-sc-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Belum ada self-check.</td></tr>';
    qsa('[data-sc-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/selfchecks/' + b.getAttribute('data-sc-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('sc-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('sc-msg', '');
        scRefresh();
      });
    });
  }
  on('sc-add', 'click', async function () {
    if (!pval('sc-period') || !(+pval('sc-total') > 0)) { pmsg('sc-msg', 'Isi periode dan total butir.'); return; }
    var r = await ppost('/api/v1/selfchecks', { period: pval('sc-period'), passed_items: +pval('sc-passed') || 0, total_items: +pval('sc-total'), note: pval('sc-note') || '' });
    if (!r.ok) { pmsg('sc-msg', 'Gagal (' + r.status + '). Lolos ≤ total; periode unik.'); return; }
    pmsg('sc-msg', '');
    scRefresh();
  });
  on('sc-reload', 'click', scRefresh);
  on('regpack-csv', 'click', function () { dlCsv('/api/v1/reports/regulator-pack.csv', 'regulator-pack.csv', 'posture-msg'); });
  /* ---- compliance: evidence vault ---- */
  async function evRefresh() {
    var box = document.getElementById('ev-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/evidence-vault');
    if (!v) { box.innerHTML = '<tr><td colspan="5" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.title) + '</td><td>' + esc(x.register) + ' #' + x.register_id + '</td><td>' + esc(x.note) + '</td><td><button class="btn btn-sm btn-outline-danger" data-ev-del="' + x.id + '" type="button">Hapus</button></td></tr>';
    }).join('') || '<tr><td colspan="5" class="text-muted">Belum ada bukti.</td></tr>';
    qsa('[data-ev-del]', function (b) {
      b.addEventListener('click', async function () {
        var r = await fetch('/api/v1/compliance/evidence-vault/' + b.getAttribute('data-ev-del'), { method: 'DELETE', headers: H });
        if (!r.ok) { pmsg('ev-msg', 'Gagal (' + r.status + ').'); return; }
        pmsg('ev-msg', '');
        evRefresh();
      });
    });
  }
  on('ev-add', 'click', async function () {
    if (!pval('ev-title') || !pval('ev-reg') || !pval('ev-rid')) { pmsg('ev-msg', 'Isi judul, register, dan ID entri.'); return; }
    var r = await ppost('/api/v1/compliance/evidence-vault', { title: pval('ev-title'), register: pval('ev-reg'), register_id: +pval('ev-rid'), note: pval('ev-note') || '' });
    if (!r.ok) { pmsg('ev-msg', 'Gagal (' + r.status + '). Register/ID harus valid.'); return; }
    pmsg('ev-msg', '');
    evRefresh();
  });
  on('ev-reload', 'click', evRefresh);
  /* ---- compliance: posture dashboard + exporter ---- */
  async function postureRefresh() {
    var box = document.getElementById('posture-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/posture');
    if (!v) { box.innerHTML = '<tr><td colspan="3" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      var detail = Object.keys(x.by_status || {}).map(function (k) { return esc(k) + ': ' + x.by_status[k]; }).join(' • ') || '<span class="text-muted">—</span>';
      return '<tr><td>' + esc(x.register) + '</td><td><strong>' + x.total + '</strong></td><td>' + detail + '</td></tr>';
    }).join('') || '<tr><td colspan="3" class="text-muted">Belum ada data.</td></tr>';
  }
  on('posture-reload', 'click', postureRefresh);
  on('posture-csv', 'click', function () { dlCsv('/api/v1/reports/compliance.csv', 'compliance.csv', 'posture-msg'); });
  /* ---- compliance: alert feed ---- */
  async function alertsRefresh() {
    var box = document.getElementById('alerts-list');
    if (!box) return;
    var v = await j('/api/v1/compliance/alerts');
    if (!v) { box.innerHTML = '<tr><td colspan="2" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + badge(x.kind) + '</td><td>' + esc(x.title) + '</td></tr>';
    }).join('') || '<tr><td colspan="2" class="text-muted">Tidak ada peringatan. Semua tertangani.</td></tr>';
  }
  on('alerts-reload', 'click', alertsRefresh);
  qsa('.snav[data-tab="kepatuhan"]', function (a) {
    a.addEventListener('click', function () { kepRefresh(); conRefresh(); rtrRefresh(); prg2Refresh(); pretRefresh(); amlRefresh(); ropaRefresh(); dpiaRefresh(); breachRefresh(); vendorRefresh(); incRefresh(); capaRefresh(); regRefresh(); trnRefresh(); tevRefresh(); riskRefresh(); riskHmRefresh(); dsarRefresh(); ethicsRefresh(); giftRefresh(); afRefresh(); wbRefresh(); wvRefresh(); polRefresh(); ctlRefresh(); ctdueRefresh(); ctestRefresh(); aplanRefresh(); mclRefresh(); scoreRefresh(); scRefresh(); evRefresh(); postureRefresh(); alertsRefresh(); });
  });
  on('wh-dl-reload', 'click', whDlRefresh);
  /* ---- integration health (F-D21-031, no secrets exposed) ---- */
  async function intRefresh() {
    var box = document.getElementById('int-list');
    if (!box) return;
    var v = await j('/api/v1/integrations/health');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + esc(x.name) + '</td><td>' + esc(x.kind) + '</td><td>' + (x.configured ? '<span class="badge bg-green-lt">siap</span>' : '<span class="badge bg-yellow-lt">belum</span>') + '</td><td class="text-muted">' + esc(x.detail) + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Tidak ada integrasi.</td></tr>';
  }
  on('int-reload', 'click', intRefresh);
  qsa('.snav[data-tab="jobs"]', function (a) {
    a.addEventListener('click', function () { whDlRefresh(); intRefresh(); bcRefresh(); });
  });
  async function whDlRefresh() {
    var box = document.getElementById('wh-dl-list');
    if (!box) return;
    var v = await j('/api/v1/webhooks/deliveries?status=rejected');
    if (!v) { box.innerHTML = '<tr><td colspan="4" class="text-muted">Gagal memuat.</td></tr>'; return; }
    box.innerHTML = v.map(function (x) {
      return '<tr><td>' + x.id + '</td><td>' + esc(x.provider) + '</td><td>' + esc(x.event) + '</td><td>' + esc(x.detail) + '</td></tr>';
    }).join('') || '<tr><td colspan="4" class="text-muted">Tidak ada dead letter. Semua terkirim.</td></tr>';
  }
  mktRefresh();
  fbRefresh();
  loadAudit();
  loadJobs();
  apprRefresh();
  apprNotifRefresh();
  apprOverdueRefresh();
  apprCalRefresh();
  slaRefresh();
  cfxRefresh();
  blkRefresh();
  arcRefresh();
  dgstRefresh();
  frzRefresh();
  delegRefresh();
  certRefresh();
  ppatRefresh();
  pbRefresh();
  phRefresh();
  mpdRefresh();
  kycRefresh();
  choRefresh();
  chgRefresh();
  chgdRefresh();
  rdyRefresh();
  spRefresh();
  mgRefresh();
  dsp2Refresh();
  bndRefresh();
  consentRefresh('mrt', 'mrt-ppat', 'mrt-list', 'mrt-msg', 'marital');
  consentRefresh('hir', 'hir-ppat', 'hir-list', 'hir-msg', 'heir');
  consentRefresh('bod', 'bod-ppat', 'bod-list', 'bod-msg', 'board');
  consentRefresh('pow', 'pow-ppat', 'pow-list', 'pow-msg', 'power');
  lhiRefresh();
  uhoRefresh();
  rpmRefresh();
  pdbRefresh();
  nmpRefresh();
  mpnRefresh();
  nhoRefresh();
  grkRefresh();
  dvbRefresh();
  dvuRefresh();
  kprRefresh();
  prgRefresh();
  ajbRefresh();
  patRefresh();
  vltRefresh();
  bpnRefresh('verification', 'bpnv-ref', 'bpnv-list', 'bpnv-msg');
  bpnRefresh('queue', 'bpnq-ref', 'bpnq-list', 'bpnq-msg');
  bpnRefresh('response', 'bpnr-ref', 'bpnr-list', 'bpnr-msg');  klapperRefresh();
  perfRefresh();
  volRefresh();
  sboxRefresh();
  sealRefresh();
  zoneRefresh();
  pbbRefresh();
  clauseRefresh();
  whRefresh();
  kepRefresh();
  conRefresh();
  rtrRefresh();
  prg2Refresh();
  pretRefresh();
  amlRefresh();
  ropaRefresh();
  dpiaRefresh();
  breachRefresh();
  vendorRefresh();
  incRefresh();
  capaRefresh();
  regRefresh();
  trnRefresh();
  riskRefresh();
  riskHmRefresh();
  dsarRefresh();
  ethicsRefresh();
  giftRefresh();
  afRefresh();
  wbRefresh();
  wvRefresh();
  polRefresh();
  ctlRefresh();
  ctdueRefresh();
  ctestRefresh();
  aplanRefresh();
  mclRefresh();
  scoreRefresh();
  scRefresh();
  evRefresh();
  postureRefresh();
  alertsRefresh();
  priceRefresh();
  quotRefresh();
  cashRefresh();
  advRefresh();
  invRefresh();
  atkRefresh();
  aboxRefresh();
  useRefresh();
  dclRefresh();
  sopRefresh();
  oinRefresh();
  dpRefresh();
  bankRefresh();
  whDlRefresh();
  bcRefresh();
  intRefresh();
})();
