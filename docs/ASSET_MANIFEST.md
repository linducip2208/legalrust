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

## Batch 2 — section backgrounds 2026-11-17 (1216x640)

Model for all: `RealVisXL_V5.0_fp16.safetensors` (SDXL photoreal).
Sampler for all: euler, 28 steps, cfg 5.0, denoise 1.0. Resolution: 1216x640.
Source files: `D:/ComfyUI/output/legalrust_<name>_00001_.png`.
WAN 2.1 T2V (`wan2.1_t2v_1.3B_fp16`) verified present in diffusion_models but
NOT used — user chose PHOTOS ONLY, SDXL stills only. Zero rejections (6/6 approved first try, no retries).

| Filename (apps/server/static/) | Prompt summary | Seed | Model | Resolution | Usage | Status |
|-------------------------------|----------------|------|-------|------------|-------|--------|
| meeting-room.png | Modern Indonesian law office meeting room, wooden conference table + leather chairs, floor-to-ceiling daylight windows, bookshelf wall, tropical plants, no people | 53101 | RealVisXL_V5.0_fp16 | 1216x640 | Meeting-room section background | APPROVED |
| doc-scan.png | Close-up hands scanning blank legal papers beside laptop, document stack, shallow depth | 53102 | RealVisXL_V5.0_fp16 | 1216x640 | Document-scan section background | APPROVED (note: papers/laptop show illegible pseudo-text lines only, no readable words; hands anatomically clean) |
| library.png | Law firm library wall of books, wooden shelves + ladder, warm lamps, leather armchair, no people | 53103 | RealVisXL_V5.0_fp16 | 1216x640 | Library section background | APPROVED (note: book spines illegible, no readable text) |
| courthouse.png | Modern Indonesian courthouse exterior daylight, concrete + glass facade with columns and steps, palms, blue sky, no people close-up | 53104 | RealVisXL_V5.0_fp16 | 1216x640 | Courthouse section background | APPROVED (no text signs) |
| server-room.png | Premium data center corridor, blue glowing server racks both sides, reflective floor, cinematic blue light, no people | 53105 | RealVisXL_V5.0_fp16 | 1216x640 | Server-room section background | APPROVED |
| signature.png | Close-up luxury fountain pen signing blank paper contract on wooden desk, shallow depth | 53106 | RealVisXL_V5.0_fp16 | 1216x640 | Signature section background | APPROVED (note: illegible pseudo-script lines only, no readable words) |

### Verification batch 2 (PIL + PNG magic)

- meeting-room.png — 965369 bytes, PNG magic OK, 1216x640 RGB, opens OK
- doc-scan.png — 860120 bytes, PNG magic OK, 1216x640 RGB, opens OK
- library.png — 1064823 bytes, PNG magic OK, 1216x640 RGB, opens OK
- courthouse.png — 1156361 bytes, PNG magic OK, 1216x640 RGB, opens OK
- server-room.png — 1067783 bytes, PNG magic OK, 1216x640 RGB, opens OK
- signature.png — 829532 bytes, PNG magic OK, 1216x640 RGB, opens OK

## Hero video loop 2026-11-18 (WAN 2.1 T2V, progressive enhancement)

Model: `wan2.1_t2v_1.3B_fp16.safetensors` (fp16, offload_device) + text encoder
`umt5-xxl-enc-fp8_e4m3fn.safetensors`.
Prompts: positive "slow cinematic drift over modern blue-glass office towers, daylight, photorealistic, no people, no text";
negative "blurry, low quality, distorted, watermark, text, logo, people, deformed, ugly".
Params: 640x384, 25 frames @8fps (~3.1s), 12 steps, cfg 6.0, shift 5.0, scheduler unipc, seed 7,
VAE `wan_2.1_vae.safetensors` fp16 with tiling (272px tile / 144x128 stride).
Source: `D:/ComfyUI/output/hero-loop_00001.mp4`. ComfyUI prompt_ids:
`aa701e15-b35c-462d-88ac-f9da6bc4d64d` (FAILED at LoadWanVideoT5TextEncoder:
"Invalid T5 text encoder model, fp8 scaled is not supported by this node" —
`umt5_xxl_fp8_e4m3fn_scaled.safetensors` is listed in node inputs but rejected at
runtime; fixed by switching to `umt5-xxl-enc-fp8_e4m3fn.safetensors`) +
`5e237709-8a94-4458-a0cb-868419335720` (success).
Also fixed once: VHS_VideoCombine requires `save_output` + `pingpong` inputs.

| Filename (apps/server/static/) | Prompt summary | Seed | Model | Resolution | Usage | Status |
|-------------------------------|----------------|------|-------|------------|-------|--------|
| hero-loop.mp4 (296502 bytes, <8MB) | Slow cinematic drift over modern blue-glass office towers, daylight, photorealistic, no people, no text; 25 frames, 12 steps, cfg 6.0, seed 7 | 7 | wan2.1_t2v_1.3B_fp16 | 640x384, 25f @8fps (~3.1s) | Hero background loop, progressive enhancement only — photos remain primary; gated behind prefers-reduced-motion (do not autoplay when reduced motion is requested), muted/loop/playsinline | DELIVERED, unreviewed visually at full motion |

Poster: no ffmpeg on host, so no first-frame extraction — reuse existing
`hero-office.png` as poster (no new poster file written). No Rust code touched.
