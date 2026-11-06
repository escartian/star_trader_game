# Native prototype verification

## Implemented desktop path

The native client uses pinned winit 0.30.13, wgpu 24.0.1, egui integrations 0.31.1,
glam 0.30.10 and image 0.25.10 with PNG-only decoding. Cargo.lock pins the graph.
Core/session JSON uses serde 1.0.229 and serde_json 1.0.154; temporary-file save
replacement uses tempfile 3.27.0. No game engine or network runtime is present.

The window handles resize/DPI events, nonzero surface reconfiguration and
surface-loss recovery. Graphics are released on suspend and recreated on resume.
World state is owned separately. Movement actions alone advance ticks;
autosave runs at safe close/suspend points. Failed close-saves keep the window open.
Waiting and UI refreshes do not advance ticks. Travel resolves as an action.

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

Reproduce with `--new --capture PATH --capture-view system|market|saves|fleets|galaxy`.
This does not verify a desktop window, mouse/touch interaction, DPI transitions,
OS suspend/resume, surface loss or idle power behavior. An earlier sandboxed window
launch reported macOS desktop-service errors. Manual interactive checks remain open.

## Behavioral and engineering checks

`./tools/check.sh` enforces the 500-line source-file limit, formatting, strict
Clippy and workspace tests. Tests cover map projection/picking/focus, overlapping
candidates, view-state preservation, double-click target decisions,
texture decoding/cache reuse, and UI tessellation at 320 × 480 and 1000 × 720.

Travel checks cover exact range/fuel boundaries, ownership, stale commands,
rejection atomicity and host-driven arrival. Economics checks cover per-unit bulk
pricing, anti-arbitrage across small/large market targets, bounded prices/stock,
batched-versus-stepped tick equivalence and real-command trading through upgrade.
Seeded-start tests check 256 reachable starts. Refreshes never create stock/money.

Save checks cover world/player/journey/economy round trips, schema/invariant
rejection, failed writes, previous-save backup recovery, and preserving a good backup
over a damaged primary. Client checks cover movement-triggered ticks, save
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

## Navigation simplification and return-to-game follow-up

Depth slicing has been removed at user request. Moon diagram spacing is exaggerated
without changing save/world coordinates. Markers, picking, Focus and inspection
share one layout; labels avoid marker bounds. A test checks that an existing
fixture moon remains separately pickable/focusable without mutating its state.

Separate schema-2 `data/settings.json` now remembers save path/slot, active ship,
guide visibility and autosave. Preferences use validated atomic JSON replacement;
invalid preferences report a fallback to defaults without modifying game saves.
CLI `--save` overrides remembered selection. Capture/headless commands do not
write settings. Checks cover preference round trips,
invalid data, preserving a last-good settings file and restoring slot/control state
without altering simulation state.

The trader guide is explicitly labelled as such. Spy, soldier and pirate mission
routes are planned in PLAYER_SPECIALIZATIONS.md; no quest/role system is claimed.
Planet rotation remains deferred pending lighting-consistent surface animation.

Fleet/action-time checks now cover purchases, splits/merges, per-ship cargo
transfers, group travel rollback, unauthorized/stale operations, fleet save round
trips and schema-1 migration of active journeys. Ship Blender sources and sprites
cover freighter, scout and patrol. Markers now follow authoritative local-body/waypoint/deep-space locations.

The updated system and fleet-management captures were rendered on Apple M5 Pro
and visually inspected. The system capture shows the freighter beside its trade
port. Fleet UI capture validates layout, not clicking/OS lifecycle behavior.

Movement now has local/body, cube exit, deep-space and explicit entry flows.
Checks cover exact fuel/range bounds, star intersection, extreme coordinates,
whole-fleet rejection, timing/market advancement, saved positions at each stage,
and schema 1/2→3 migration. An egui pointer-input test clicks Confirm movement;
a preview-only or invalid preview leaves time unchanged. This is not OS window
or physical-device interaction evidence. Offscreen navigation, entry and planet
ship-marker captures were rendered on Apple M5 Pro and visually inspected.

Planetary economy checks cover market isolation, specialization production, finite
funds, cash conservation, hull depletion/restocking, last-ship/cargo sale rejection,
unique IDs, fleet naming and market/metadata save validation. A save exported by
the prior schema-3 executable was migrated through the new CLI: player state and
station goods matched, with no new cash/hulls or duplicated planetary goods.
New planetary market/shipyard GPU captures on Apple M5 Pro were visually inspected;
OS window/lifecycle and player pacing remain unverified.
