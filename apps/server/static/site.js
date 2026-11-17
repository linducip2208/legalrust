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
  window.apptab = function (x) {
    ['ringkas', 'dok', 'akta', 'keu'].forEach(function (k) {
      document.getElementById('p-' + k).classList.toggle('hidden', k !== x);
    });
    var lbl = { ringkas: 'ringkasan', dok: 'dokumen', akta: 'akta', keu: 'keuangan' }[x];
    qsa('#tabs .nav-link', function (a) { a.classList.toggle('active', a.textContent.toLowerCase().includes(lbl)); });
  };
  qsa('#tabs .nav-link', function (a, i) {
    a.addEventListener('click', function (ev) { ev.preventDefault(); window.apptab(['ringkas', 'dok', 'akta', 'keu'][i]); });
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
  (async function () {
    try {
      var me = await j('/api/v1/orgs/me');
      document.getElementById('hi').textContent = 'Dasbor — ' + me.org_id.slice(0, 8) + '…';
      var c = await j('/api/v1/clients'), m = await j('/api/v1/matters'),
        d = await j('/api/v1/documents'), a = await j('/api/v1/deeds'),
        tr = await j('/api/v1/trust/accounts');
      var cards = [['Klien', c.length], ['Perkara', m.length], ['Dokumen', d.length], ['Akta', a.length], ['Trust', tr.length], ['Saldo trust', 'Rp ' + tr.reduce(function (s, x) { return s + (x.balance || 0); }, 0).toLocaleString('id-ID')]];
      document.getElementById('g').innerHTML = cards.map(function (x) {
        return '<div class="col-6 col-md-4"><div class="card"><div class="card-body"><div class="text-muted">' + x[0] + '</div><div class="h1 m-0">' + x[1] + '</div></div></div></div>';
      }).join('');
      document.getElementById('m').innerHTML = m.map(function (x) {
        return '<tr><td><strong>' + x.number + '</strong></td><td>' + x.title + '</td><td><span class="badge bg-green-lt">' + x.status + '</span></td></tr>';
      }).join('');
      document.getElementById('dl').innerHTML = d.map(function (x) {
        return '<tr><td>' + x.id + '</td><td>' + x.name + '</td></tr>';
      }).join('');
      document.getElementById('al').innerHTML = a.map(function (x) {
        return '<tr><td>' + x.id + '</td><td>' + x.number + '</td><td><span class="badge bg-blue-lt">' + x.state + '</span></td><td><button class="btn btn-sm btn-outline-primary adv" data-id="' + x.id + '" data-st="' + x.state + '">Maju →</button></td></tr>';
      }).join('');
      qsa('.adv', function (btn) {
        btn.addEventListener('click', async function () {
          var NX = { draft: 'review', review: 'approved', approved: 'signed' };
          var to = NX[btn.dataset.st];
          if (!to) { document.getElementById('ae').textContent = 'Status akhir.'; return; }
          var r = await fetch('/api/v1/deeds/' + btn.dataset.id + '/transition', { method: 'POST', headers: Object.assign({}, H, { 'Content-Type': 'application/json' }), body: JSON.stringify({ to: to }) });
          if (!r.ok) { document.getElementById('ae').textContent = 'Gagal (' + r.status + '): transisi ilegal.'; return; }
          location.reload();
        });
      });
      document.getElementById('tl').innerHTML = tr.map(function (x) {
        return '<tr><td>' + x.id + '</td><td>' + x.client_id + '</td><td>Rp ' + (x.balance || 0).toLocaleString('id-ID') + '</td></tr>';
      }).join('');
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
})();
