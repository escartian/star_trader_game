# Image-to-3D with local ComfyUI (TRELLIS.2)

Authoring-only tooling that turns a concept image into a Blender ship asset. It runs
ComfyUI's built-in Pixal3D/TRELLIS.2 template locally on Apple Silicon (MPS). Nothing
here is needed at runtime.

## Pipeline (AI courier)

1. **Generate** (ComfyUI, about 15 minutes on an M-series Mac with 64 GB): queue
   `trellis2-image-to-3d-api.json` (API prompt; it can also be dragged into ComfyUI).
   Its `LoadImage` node reads the concept image from ComfyUI's input folder (images
   are not kept in the repo). It runs structure, shape, a 1024 upsample, texture,
   vertex paint and GLB export. The raw output is
   about 10M triangles and 295 MB, so it stays outside the repo:
   `~/ComfyUI-Shared/output/ai-courier/ship-1024-fixed.glb`.
2. **Remesh and decimate** (ComfyUI's Python, about 20 s):
   `~/ComfyUI-Installs/ComfyUI/ComfyUI/.venv/bin/python3 tools/comfyui/remesh_scan.py ~/ComfyUI-Installs/ComfyUI/ComfyUI <raw.glb> courier.npz 60000`
   The "solid" remesh builds one watertight shell. The raw dual-grid surface has small
   holes and non-manifold edges, and a plain voxel remesh would give it a double wall.
   QEM decimation then reduces it to 60k triangles, keeping vertex colors.
3. **Blender asset**: `Blender -b -P tools/art/clean_ai_courier.py -- courier.npz`
   aligns the heading (the scan keeps the image's camera yaw), scales it to the
   hand-built ships' conventions (Z up, nose +Y, 4.3 units long), and writes
   `assets/source/ai-courier.blend` and renders `assets/sprites/scout.png` with the
   same camera and lights as the other ship sprites.

The result is a closed, vertex-colored 60k-triangle model, used as the scout's art.

## Mac requirements and past failures

ComfyUI's venv must run **torch 2.15.0 or newer**. Comfy Desktop's standalone install
shipped 2.12.1, which has two MPS bugs that broke this pipeline. If Desktop reinstalls
an older torch, upgrade it again in ComfyUI's venv:
`pip install torch==2.15.0 torchvision==0.29.2 torchaudio==2.11.1`.

1. **Integer sort was wrong on MPS.** `torch.sort`, `topk` and `unique` returned wrong
   values for integers above 2^24. TRELLIS hashes voxel coordinates into keys up to
   1024^3, so about a third of lookups missed. This produced fragmented, perforated
   meshes and broke the remesh, decimate and UV nodes.
2. **Sub-quadratic attention returned NaN at batch size 1.** When cfg drops to 1.0,
   ComfyUI skips the uncond pass. The shape latent then became all NaN, and the decode
   failed with `max(): Expected reduction dim 0 to have non-zero size`. Desktop also
   launches with `--use-pytorch-cross-attention` as a fallback.
