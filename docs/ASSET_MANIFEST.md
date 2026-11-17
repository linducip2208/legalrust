# Asset Manifest — ComfyUI SDXL batch 2026-11-17

Model for all: `RealVisXL_V5.0_fp16.safetensors` (SDXL photoreal).
Sampler for all: euler, 28 steps, cfg 5.0, denoise 1.0. Resolution: 1216x768.
Source files: `D:/ComfyUI/output/legalrust_<name>_00001_.png`.
No Wan model exists on host — SDXL stills only. Zero rejections (4/4 approved first try).

| Filename (apps/server/static/) | Prompt summary | Seed | Model | Resolution | Usage | Status |
|-------------------------------|----------------|------|-------|------------|-------|--------|
| office-interior.png | Modern Indonesian law office interior, daylight through large windows, wooden bookshelves, desk + leather chairs, tropical plants, no people | 42101 | RealVisXL_V5.0_fp16 | 1216x768 | Hero / office section | APPROVED |
| ocr-scan.png | Close-up hands scanning legal document beside laptop, paper stack on desk, shallow depth | 42102 | RealVisXL_V5.0_fp16 | 1216x768 | OCR feature section | APPROVED (note: paper/laptop show illegible pseudo-text lines only, no readable words; hands anatomically clean) |
| ai-assist.png | Abstract glowing blue neural network over open document pages, navy background, premium tech | 42103 | RealVisXL_V5.0_fp16 | 1216x768 | AI-assist feature section | APPROVED (note: book shows illegible pseudo-text lines only) |
| security.png | Server-room corridor in blue light, glowing racks; shield motif requested but rendered only as ambient light (no explicit shield, no text) | 42104 | RealVisXL_V5.0_fp16 | 1216x768 | Security section | APPROVED with deviation noted |

## Verification (2026-11-17, Get-ChildItem)

- office-interior.png — 1566229 bytes, opens OK
- ocr-scan.png — 1049915 bytes, opens OK
- ai-assist.png — 1350286 bytes, opens OK
- security.png — 1294591 bytes, opens OK

All non-zero, all visually inspected at full resolution. No Rust code,
migrations, or other docs touched. Pre-existing files
(cta-building.png, hero-office.png) untouched.
