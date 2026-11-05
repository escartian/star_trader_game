# Native prototype verification

## Implemented desktop path

The native client uses pinned winit 0.30.13, wgpu 24.0.1, egui integrations 0.31.1,
glam 0.30.10 and image 0.25.10 with PNG-only decoding. Cargo.lock pins the graph.
Core/session JSON uses serde 1.0.229 and serde_json 1.0.154; temporary-file save
replacement uses tempfile 3.27.0. No game engine or network runtime is present.

The window handles resize/DPI events, nonzero surface reconfiguration and
surface-loss recovery. Graphics are released on suspend and recreated on resume.
World state is owned separately. The offline clock resets at suspend/resume;
autosave runs at safe close/suspend points. Failed close-saves keep the window open.
While travelling, tick progress schedules redraws. Idle stock/ticks catch up on
input without a continuous render loop. Pause and speed affect scheduling only.

## Rendering evidence

Original Blender 5.2.3 models and baked transparent sprites cover five star classes,
a rocky planet, gas giant, moon, asteroid and trade port. Their images were visually
inspected. One shared cache preloads sprites per egui context; system views reuse
handles and refresh them when a graphics context is recreated. No animation frame
sets or runtime Blender dependency were introduced. Surface images are billboards;
star halos, galaxy markers, routes and selection remain procedural.

The actual system, market and save-slot views (see docs/previews/) were rendered to offscreen GPU
textures on the host Apple M5 Pro and visually inspected. This verified native
wgpu texture upload/render/readback, text, transparent sprites and those layouts.
The sandbox exposes no GPU adapter; the permitted host captures succeeded. The
local camera was adjusted after review to separate the port from the enlarged star.

Reproduce with `--new --capture PATH --capture-view system|market|saves|galaxy`.
This does not verify a desktop window, mouse/touch interaction, DPI transitions,
OS suspend/resume, surface loss or idle power behavior. An earlier sandboxed window
launch reported macOS desktop-service errors. Manual interactive checks remain open.

## Behavioral and engineering checks

`./tools/check.sh` enforces the 500-line source-file limit, formatting, strict
Clippy and workspace tests. Tests cover map projection/picking/focus, overlapping
candidates, slice reveal, view-state preservation, double-click target decisions,
texture decoding/cache reuse, and UI tessellation at 320 × 480 and 1000 × 720.

Travel checks cover exact range/fuel boundaries, ownership, stale commands,
rejection atomicity and host-driven arrival. Economics checks cover per-unit bulk
pricing, anti-arbitrage across small/large market targets, bounded prices/stock,
batched-versus-stepped tick equivalence and real-command trading through upgrade.
Seeded-start tests check 256 reachable starts. Refreshes never create stock/money.

Save checks cover world/player/journey/economy round trips, schema/invariant
rejection, failed writes, previous-save backup recovery, and preserving a good backup
over a damaged primary. Client checks cover fractional ticks, speed/pause, save
restore and a failed load retaining the live session. See docs/DATA_FORMATS.md.

The 20–30 minute first-upgrade target and whether routes are enjoyable require
observed playtesting. Automated route completion establishes mechanics only.
CI is configured but a hosted workflow run is not claimed here.

## Mobile: deliberately deferred

The user has asked to continue implementation while ignoring mobile build work.
Only desktop Rust targets were installed when inspected. The iOS SDK lookup failed;
ANDROID_HOME/ANDROID_NDK_HOME were unset. No mobile shells, signing, touch-device,
safe-area or device lifecycle evidence is provided. These remain open milestones.
Desktop lifecycle checks and Windows/Linux packaging also remain unverified.
