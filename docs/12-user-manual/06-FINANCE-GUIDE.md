# 06 — Panduan Keuangan (Finance Guide)

Covers: invoice per matter, pembayaran manual, gateway (blocked), dan trust
ledger (rekening titipan). Semua guard saldo dijelaskan apa adanya.
Route persis dari `apps/server/src/lib.rs`.

## 1. Invoice

```powershell
# Buat invoice untuk matter 1 (amount_minor = rupiah * 100? gunakan minor-unit
# konsisten; validasi gagal → 422)
curl -X POST http://localhost:3000/api/v1/matters/1/invoices `
  -H "Authorization: Bearer TOKEN" -H 'Content-Type: application/json' `
  -d '{"amount_minor":1500000,"currency":"IDR",...}'

# Daftar invoice matter & detail satu invoice
curl http://localhost:3000/api/v1/matters/1/invoices -H "Authorization: Bearer TOKEN"
curl http://localhost:3000/api/v1/invoices/4 -H "Authorization: Bearer TOKEN"
```

- `POST /api/v1/matters/{mid}/invoices` — buat (matter tidak ada → 404).
- `GET /api/v1/matters/{mid}/invoices` — daftar.
- `GET /api/v1/invoices/{id}` — detail + sisa tagihan.
- Di UI, `FinancePage` (`apps/web/src/lib.rs`) me-render `number — balance`
  dengan `format_minor` (cth. `IDR 1.500.000`).

## 2. Pembayaran manual + guard overpay

```powershell
curl -X POST http://localhost:3000/api/v1/invoices/4/payments `
  -H "Authorization: Bearer TOKEN" -H 'Content-Type: application/json' `
  -d '{"amount_minor":500000,"method":"transfer"}'
```

- `POST /api/v1/invoices/{id}/payments` — catat pembayaran (izin Create).
- **Guard overpay:** pembayaran yang melebihi sisa tagihan ditolak dengan
  `409 "overpayment"` (`crates/database`: `ops_memory.rs`, `ops_mysql.rs`).
  Tidak ada pembayaran yang tercatat saat guard menolak — aman untuk dicoba
  ulang dengan nominal yang benar.

## 3. Gateway pembayaran Tripay — LIVE bila kredensial dikonfigurasi

Tanpa hardcode: semua kredensial via env (`TRIPAY_API_KEY`,
`TRIPAY_PRIVATE_KEY`, `TRIPAY_MERCHANT_CODE`, `TRIPAY_MODE=sandbox|production`,
opsional `TRIPAY_BASE_URL`, `TRIPAY_CALLBACK_URL`, `TRIPAY_RETURN_URL`).
Tanpa env → 501 `tripay not configured`. Mulai dari **sandbox**.

```powershell
# 1. Buat checkout (channel: QRIS, BRIVA, MANDIRI, dsb; expired 1..168 jam)
curl -X POST http://localhost:3000/api/v1/invoices/4/pay-gateway `
  -H "Authorization: Bearer TOKEN" -H 'Content-Type: application/json' `
  -d '{"channel":"QRIS","customer_name":"Budi Santoso","expired_hours":24}'
# → {"reference":"Txxx","merchant_ref":"LRINV4-...","checkout_url":"https://...","amount":...}
# Invoice lunas → 409 "invoice already settled".

# 2. Daftarkan callback di panel Tripay:
#    https://<host-anda>/api/v1/webhooks/incoming/tripay
#    Payload PAID terverifikasi (HMAC) otomatis mencatat pembayaran
#    (method "tripay:{reference}", idempoten, guard overpay tetap berlaku).
```

- `POST /api/v1/webhooks/incoming/tripay` — tanpa auth header (verifikasi
  `X-Callback-Signature`); status selain PAID diterima tanpa pembukuan.

## 4. Trust ledger (rekening titipan klien)

Rekening titipan mendukung entri debit/kredit dan transfer antar-rekening
dengan guard anti-overdraft:

```powershell
# Buat + daftar rekening
curl -X POST http://localhost:3000/api/v1/trust/accounts `
  -H "Authorization: Bearer TOKEN" -H 'Content-Type: application/json' `
  -d '{"currency":"IDR",...}'

curl http://localhost:3000/api/v1/trust/accounts -H "Authorization: Bearer TOKEN"
curl http://localhost:3000/api/v1/trust/accounts/2 -H "Authorization: Bearer TOKEN"

# Tambah entri (kind: "debit" | "credit", amount_minor ≥ 1)
curl -X POST http://localhost:3000/api/v1/trust/accounts/2/entries `
  -H "Authorization: Bearer TOKEN" -H 'Content-Type: application/json' `
  -d '{"kind":"credit","amount_minor":1000000}'

# Transfer antar-rekening
curl -X POST http://localhost:3000/api/v1/trust/transfer `
  -H "Authorization: Bearer TOKEN" -H 'Content-Type: application/json' `
  -d '{"from":2,"to":3,"amount_minor":250000}'
# → {"out": <entry_id>, "in": <entry_id>}
```

- `POST /api/v1/trust/accounts` / `GET /api/v1/trust/accounts` — buat & daftar.
- `GET /api/v1/trust/accounts/{id}` — saldo (`currency`, `balance_minor`).
- `POST /api/v1/trust/accounts/{id}/entries` — jurnal entri.
- `POST /api/v1/trust/transfer` — transfer double-entry (dua entri tercatat atomik).
- **Guard overdraft:** debit/transfer yang membuat saldo negatif ditolak dengan
  `409 "insufficient trust balance"` dan **tidak ada entri yang dibukukan**
  (`crates/domain::TrustError::InsufficientFunds`, ditegakkan di
  `ops_memory.rs`/`ops_mysql.rs`). Guard lain: nominal harus positif dan
  (untuk transfer multi-line) debit/kredit harus seimbang
  (`NonPositiveAmount`, `UnbalancedLines`, `CurrencyMismatch` di
  `crates/domain`).

## 5. Ekspor laporan

- `GET /api/v1/reports/matters-csv` dan `GET /api/v1/reports/matters.csv` —
  CSV matter (`id,number,title,status`).
