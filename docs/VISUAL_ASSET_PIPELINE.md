# Visual Asset Pipeline (ComfyUI + SDXL)

> Scope: landing-page PNG assets only. No Rust code, no migrations.

## Endpoint

- Base: `http://127.0.0.1:8188` (local ComfyUI, verified live 2026-11-17, v0.43.0, RTX 4060)
- Submit: `POST /prompt` with JSON body `{ "prompt": { "<id>": { "class_type": ..., "inputs": ... } } }`
  → returns `{ "prompt_id": "<uuid>" }`
- Poll: `GET /history/{prompt_id}` until `history.outputs` contains a `SaveImage` entry with `filename`
- Outputs on disk: `D:/ComfyUI/output/` (ComfyUI output dir)
- Approved copies: `apps/server/static/` (lowercase-hyphen `.png`)

## Verified nodes (`GET /object_info`, 1763 node types)

| Logical step   | `class_type`           | Present |
|----------------|------------------------|---------|
| Checkpoint load| `CheckpointLoaderSimple`| yes     |
| Prompt encode ±| `CLIPTextEncode`       | yes     |
| Latent         | `EmptyLatentImage`     | yes     |
| Sample         | `KSampler`             | yes     |
| Decode         | `VAEDecode`            | yes     |
| Save           | `SaveImage`            | yes     |

Note: the brief named `CheckpointLoaderSpot`; the installed ComfyUI exposes
`CheckpointLoaderSimple`, which is used instead (same role).

## Model

- Checkpoint: `RealVisXL_V5.0_fp16.safetensors` (photoreal SDXL)
- Other checkpoints present but NOT used: DreamShaperXL_Turbo, Juggernaut-XL v9,
  sd_xl_base_1.0, sdxl_lightning_8step, v1-5-pruned-emaonly
- Wan video model: `wan2.1_t2v_1.3B_fp16.safetensors` (+ `wan2.1_fun_inp_1.3B_bf16`)
  verified present in `GET /models/diffusion_models` 2026-11-17 — but out of scope:
  user chose PHOTOS ONLY, SDXL stills only, no video rendering.

## Resolutions

- Landscape: `1216x768` (hero / wide banners)
- Portrait: `768x1216` (tall cards) — unused in current batch, all four are landscape
- Sampler: `euler`, 28 steps, `cfg 5`, `denoise 1.0`

## Workflow JSON template

```json
{
  "prompt": {
    "1": { "class_type": "CheckpointLoaderSimple",
           "inputs": { "ckpt_name": "RealVisXL_V5.0_fp16.safetensors" } },
    "2": { "class_type": "CLIPTextEncode",
           "inputs": { "clip": ["1", 1], "text": "<POSITIVE>" } },
    "3": { "class_type": "CLIPTextEncode",
           "inputs": { "clip": ["1", 1], "text": "<NEGATIVE>" } },
    "4": { "class_type": "EmptyLatentImage",
           "inputs": { "width": 1216, "height": 768, "batch_size": 1 } },
    "5": { "class_type": "KSampler",
           "inputs": { "model": ["1", 0], "positive": ["2", 0], "negative": ["3", 0],
                        "latent_image": ["4", 0], "seed": <SEED>,
                        "steps": 28, "cfg": 5.0, "sampler_name": "euler",
                        "scheduler": "normal", "denoise": 1.0 } },
    "6": { "class_type": "VAEDecode",
           "inputs": { "samples": ["5", 0], "vae": ["1", 2] } },
    "7": { "class_type": "SaveImage",
           "inputs": { "images": ["6", 0], "filename_prefix": "<NAME_NO_EXT>" } }
  }
}
```

Shared negative prompt: `text, watermark, logo, signature, readable text, letters,
words, blurry, distorted, deformed, ugly, low quality, people close-up, face close-up`
(plus per-asset additions, e.g. `no people` for interiors/abstracts).

## Procedure (sequential, one at a time)

1. `POST /prompt` → `prompt_id`
2. Poll `GET /history/{id}` every ~5 s (up to ~10 min) until outputs appear
3. Resolve `D:/ComfyUI/output/<filename>`; check non-zero bytes
4. Inspect: reject on visible artifacts / readable-text-where-forbidden / wrong subject; record rejection in manifest
5. Copy approved PNG → `apps/server/static/<lowercase-hyphen>.png`
6. Record seed, resolution, usage, status in `docs/ASSET_MANIFEST.md`
