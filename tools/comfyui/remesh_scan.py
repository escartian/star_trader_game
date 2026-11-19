"""Remesh and decimate a raw TRELLIS.2 GLB with ComfyUI's own mesh tools.

Run with ComfyUI's Python, so its remesher and QEM decimator are importable:
  ~/ComfyUI-Installs/ComfyUI/ComfyUI/.venv/bin/python3 tools/comfyui/remesh_scan.py \
      <ComfyUI dir> <raw.glb> <out.npz> [target_faces]

The raw dual-grid surface has small holes and non-manifold edges. The "solid"
remesh mode rebuilds one watertight shell (no inner double wall), then QEM
decimation reduces it to the target. Vertex colors are carried through both steps.
Output .npz holds vertices (N,3; glTF Y-up), faces (M,3) and colors (N,3|4; 0..1),
which tools/art/clean_ai_courier.py turns into the Blender asset.
"""
import json
import struct
import sys
from pathlib import Path

import numpy as np
import torch

REMESH_RESOLUTION = 512   # voxel grid over the [-0.5, 0.5] cube
HOLE_FILL_COST = 20.0     # "solid" mode: higher only closes small holes and cracks

_COMPONENTS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}
_DTYPES = {5120: np.int8, 5121: np.uint8, 5122: np.int16, 5123: np.uint16, 5125: np.uint32, 5126: np.float32}


def read_glb(path):
    """Minimal reader for a single-mesh GLB: positions, indices, COLOR_0."""
    data = Path(path).read_bytes()
    json_len = struct.unpack_from("<I", data, 12)[0]
    gltf = json.loads(data[20:20 + json_len])
    binary = data[20 + json_len + 8:]

    def accessor(index):
        acc = gltf["accessors"][index]
        view = gltf["bufferViews"][acc["bufferView"]]
        dtype = np.dtype(_DTYPES[acc["componentType"]])
        width = _COMPONENTS[acc["type"]]
        start = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
        stride = view.get("byteStride", 0) or dtype.itemsize * width
        rows = np.lib.stride_tricks.as_strided(
            np.frombuffer(binary, dtype=np.uint8, offset=start),
            shape=(acc["count"], dtype.itemsize * width), strides=(stride, 1))
        values = np.ascontiguousarray(rows).view(dtype).reshape(acc["count"], width)
        if acc.get("normalized") and dtype.kind in "iu":
            values = values.astype(np.float32) / np.iinfo(dtype).max
        return values

    primitive = gltf["meshes"][0]["primitives"][0]
    vertices = accessor(primitive["attributes"]["POSITION"]).astype(np.float32)
    faces = accessor(primitive["indices"]).reshape(-1, 3).astype(np.int64)
    colors = None
    if "COLOR_0" in primitive["attributes"]:
        colors = accessor(primitive["attributes"]["COLOR_0"]).astype(np.float32)
    return vertices, faces, colors


def main():
    comfy_dir, raw, out = sys.argv[1:4]
    target_faces = int(sys.argv[4]) if len(sys.argv) > 4 else 60_000
    sys.path.insert(0, comfy_dir)
    from comfy_extras.mesh3d.postprocess.qem_decimate import QEMConfig, qem_simplify
    from comfy_extras.mesh3d.postprocess.remesh import remesh_narrow_band_dc

    device = torch.device("mps" if torch.backends.mps.is_available() else "cpu")
    vertices, faces, colors = read_glb(raw)
    print(f"raw: {len(faces)} faces, colors={None if colors is None else colors.shape}")
    v = torch.from_numpy(vertices).to(device)
    f = torch.from_numpy(faces).to(device)
    c = torch.from_numpy(colors).to(device) if colors is not None else None

    band = 1.0
    scale = (REMESH_RESOLUTION + 3.0 * band) / REMESH_RESOLUTION  # same domain as the RemeshMesh node
    v, f, c = remesh_narrow_band_dc(
        v, f, resolution=REMESH_RESOLUTION, band=band, project_back=0.0, qef=False,
        sign_mode="solid", manifold=False, fill=HOLE_FILL_COST, fix_poles=False,
        smooth_iters=20, drop_small_components=0.01, drop_inverted_components=False,
        drop_enclosed_components=False, scale=scale,
        center=torch.zeros(3, device=device), colors=c)
    print(f"remeshed: {f.shape[0]} faces")
    v, f, c = qem_simplify(v, f.to(torch.int64), target_faces, colors=c, config=QEMConfig())
    print(f"decimated: {f.shape[0]} faces")
    np.savez_compressed(out, vertices=v.float().cpu().numpy(), faces=f.cpu().numpy().astype(np.int32),
                        colors=c.float().cpu().numpy() if c is not None else np.zeros((0, 3), np.float32))


if __name__ == "__main__":
    main()
