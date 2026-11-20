# ComfyUI ship image to 3D

This is an experimental art workflow for Star Trader. Blender is the authoring
tool; the game currently uses exported ship sprites, not runtime 3D models. Save
source images, ComfyUI workflows, cleaned Blender files, and export scripts under
`assets/source/`.

## Run the medium ship

1. Open ComfyUI Desktop. Drag
   `assets/source/ai-medium-ship-comfy-workflow.json` onto its canvas.
2. In **Load Image**, upload the ship concept image. The original reference
   images are no longer kept in the repository; the generated GLB and preview
   below are the record of that run.
3. Check that **Switch to Trellis2** is `true`. The workflow uses the installed
   TRELLIS.2 checkpoint, the shared shape and texture VAEs, DINOv3, and the
   background removal model. The shape target is 1024; remeshing is set to 256.
4. Queue with **Cmd+Enter**. Wait for **Save 3D (Advanced)** to finish. With the
   standard Desktop output folder, look under
   `~/ComfyUI-Shared/output/ai-medium-ship/` for a `.glb` file.
5. Import the GLB in Blender with **File → Import → glTF 2.0**. Rotate around it
   and inspect the silhouette, back, underside, disconnected pieces, holes, and
   colors. Keep the generated file as an experiment until that inspection passes.

The medium ship run on macOS 26.7.3, ComfyUI 0.46.0, and PyTorch 2.15.0
produced `assets/source/ai-medium-ship-generated.glb`. Blender imported it
and rendered `assets/source/ai-medium-ship-preview.png`; the inspected view
shows a coherent ship. The back and underside still need an artist's review.
The reproducible workflow is beside the GLB.

To reproduce the preview on a Mac with Blender installed at the usual path:

```sh
/Applications/Blender.app/Contents/MacOS/Blender --background \
  --python assets/source/render_glb_preview.py -- \
  assets/source/ai-medium-ship-generated.glb \
  assets/source/ai-medium-ship-preview.png
```

The saved workflow exports **vertex color** on a GLB. It skips mesh decimation
and UV baking because those steps failed on this Mac's Apple GPU in earlier
runs. A final game asset still needs Blender cleanup and a reproducible sprite
export; generated GLBs are not loaded by the game.

The game's freighter, patrol and prospector are instead built by the
scripts in `tools/art/` (see `assets/README.md`); the AI concept images that
inspired them have been removed.

## Other ships

For another ship, duplicate the workflow, select its image in **Load Image**,
and change **Save 3D (Advanced) → filename_prefix** so the output does not mix
with the medium ship. Start with one centered object on a transparent PNG. Keep the
edited workflow under `assets/source/`.

If a run fails, inspect the failing node in ComfyUI's error report. If it
exports a mesh that looks fragmented in Blender, a successful queue status has
not established that the asset is usable. Record the ComfyUI, PyTorch, and macOS
versions with the image and workflow when comparing results.
