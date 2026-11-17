# 07 — Panduan AI & RAG (AI/RAG Guide)

Fitur AI terbagi dua: **RAG lokal (berfungsi)** untuk pencarian dokumen
`text/*`, dan **chat AI (terblokir `501`)** karena belum ada provider yang
dikonfigurasi. Penggunaan token tetap dicatat secara jujur meski chat
terblokir.

## 1. RAG index & search (berfungsi, lokal)

```powershell
# Index dokumen teks (hanya mime text/*; binary → 422 "binary parsing not supported";
# dokumen tanpa konten → 422 "no content"; non-UTF8 → 422 "not utf8")
curl -X POST http://localhost:3000/api/v1/rag/index `
  -H "Authorization: Bearer TOKEN" -H 'Content-Type: application/json' `
  -d '{"document_id":1}'
# → {"chunks": N}  (chunk 800 karakter, overlap 100 — legalrust_rag::chunk_text)

# Cari (q wajib non-kosong; kosong → 422 "q")
curl "http://localhost:3000/api/v1/rag/search?q=hak+milik&limit=5" -H "Authorization: Bearer TOKEN"
```

- `POST /api/v1/rag/index` — pecah dokumen menjadi chunk dan simpan (izin Create).
- `GET /api/v1/rag/search` — pencarian kemiripan atas chunk terindeks (izin View).
- RAG berjalan lokal tanpa provider eksternal; tidak ada klaim embedding
  cloud di sini.

## 2. Chat AI — BLOCKED (`501`), tetapi usage TETAP dicatat

```powershell
curl -X POST http://localhost:3000/api/v1/ai/chat `
  -H "Authorization: Bearer TOKEN" -H 'Content-Type: application/json' `
  -d '{"messages":[{"role":"user","content":"Ringkas kontrak ini"}]}'
# → 501 EXTERNAL_BLOCKED: "no AI provider configured; estimated input tokens: N
#    (logged, zero output)"
```

Perilaku jujur yang perlu dipahami pengguna:

1. Request tetap **membuat conversation** (`conv_create`) dan **menyimpan
   pesan** (`msg_add`) — riwayat tersimpan meski tidak ada jawaban.
2. Estimasi token input (`karakter / 4`, min. 1) **dicatat ke usage**
   (`usage_add` dengan output 0) sebelum 501 dikembalikan.
3. Baru kemudian server mengembalikan 501 — tidak ada konten AI yang
   difabrikasi ("zero output").

- `POST /api/v1/ai/chat` — selalu 501 sampai provider AI dikonfigurasi dan
  diverifikasi live. Validasi: `messages` wajib ≥ 1 dan tidak boleh ada konten
  kosong (→ 422 `"messages"`); `system` opsional (maks 200 karakter).
- `GET /api/v1/ai/usage` — **berfungsi**: total pemakaian org
  (`{"input_tokens": I, "output_tokens": O}`), termasuk token yang tercatat
  dari percobaan chat yang terblokir. Gunakan endpoint ini untuk memantau/
  menagih pemakaian secara transparan.

```powershell
curl http://localhost:3000/api/v1/ai/usage -H "Authorization: Bearer TOKEN"
# → {"input_tokens": 1234, "output_tokens": 0}
```

## 3. Ringkasan kejujuran

| Fitur | Status | Bukti |
|---|---|---|
| RAG index/search dokumen teks | Berfungsi | `rag_index`, `rag_search` nyata di router |
| Chat AI generatif | BLOCKED 501 | `ai_chat` selalu `BlockedExternal` tanpa provider |
| Pencatatan usage | Berfungsi | `ai_usage` + `usage_add` tercatat walau chat 501 |
