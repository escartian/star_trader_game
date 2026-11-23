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

## Browser preview

The initial `wasm32-unknown-unknown` path is configured in
`crates/game-client/Trunk.toml`. Trunk bundles the shared Rust client and embedded
art into static HTML/WASM; both `cargo check --manifest-path
crates/game-client/Cargo.toml --target wasm32-unknown-unknown` and
`cd crates/game-client && trunk build --release` succeeded. Serve the preview
with `cd crates/game-client && trunk serve`.

The prototype starts a fresh game. A local Chromium smoke check confirmed the
canvas renders and the Menu control responds. Browser save/preferences
persistence, other-browser compatibility and full interactive play checks
remain open; a successful bundle is not browser/device acceptance.

## Rendering evidence

The flat-map change has automated projection, picking and phone/desktop egui
tessellation checks. A fresh offscreen GPU capture was attempted but this sandbox
reported `No GPU adapter for capture`; its fixed-tilt layout and drag/pinch
behavior still need visual and interactive desktop checks. Mobile checks remain
deferred.

The startup chooser was tessellated at 390 × 844 points with an incompatible
schema-6 save and a valid second slot. A normal `cargo run -p game-client` with
the remembered schema-6 save reached the desktop runtime instead of exiting with
a load error. The sandbox's macOS desktop-service warnings prevented a reliable
window interaction check. Tests confirm the chooser does not autosave over the
incompatible primary or backup.

NPC galaxy markers now use the existing GLB ship models instead
of moving dots. The repainted scout GLB was re-exported from the checked-in
Blender source and visually inspected in a Blender render: its ivory hull and
teal wings are visible. Automated checks cover embedded model paint and battle
glide heading. Offscreen captures of the galaxy traffic and battle view succeeded
on the Apple M5 Pro and were visually inspected: trader markers have hull shapes,
and the scout battle model shows teal paint at its board position. Interactive
window and device checks remain open.
The scout source was levelled and its original-colour sprite re-exported at
512px. Freighter, patrol and prospector models gained visible detail and have
5,592, 4,336 and 3,676 triangles; the scout has 6,000. All four ship sprites
are now 512px. These new renders were inspected as files; fresh in-game GPU
captures of the revised battle scale remain open.

Hex battles (phone-first phase 4): offscreen captures at 1280 × 800, 1024 × 768
and 390 × 844 of a duel, the 7 v 7 scenario and a close-up after a climb and an
aim were visually inspected: the whole board fits both shapes, bands read by
height and shadow, and the phone layout stacks the controls under the board with
44-point Fire and End turn buttons. Automated tests cover board framing at wide
and phone sizes, hex/ship picking and aim clearing. Real touch input (tap, drag,
pinch) on a device, desktop mouse play and the wide view's unused space above
the board (perspective makes the near edge the binding fit) are not yet checked.

Campaign (phone-first phase 5): offscreen captures of the Victory panel (390 ×
844) and of a fleet orbiting the Raider Haven (1280 × 800, 1024 × 768, 390 ×
844) were visually inspected: road progress and the score are readable, the
Haven's fortress and garrison label sit beside its body, and the ASSAULT card
leads the sheet. Not yet seen on screen: the end card (victory, defeat or
campaign complete), the market's charter row and lane lines; their rules are
covered by core and server tests, and manual play remains open.

Original Blender 5.2.3 models and baked transparent sprites cover nine star classes,
seven rocky biomes, three gas giants, two moon looks, two asteroid looks and the
trade port. The new sprites were inspected in a contact sheet; a fresh in-game
GPU capture remains open. One shared cache preloads sprites per egui context; system views reuse
handles and refresh them when a graphics context is recreated. No animation frame
sets or runtime Blender dependency were introduced. Surface images are billboards;
star halos, galaxy markers, routes and selection remain procedural.

The actual system, market and save-slot views (see docs/previews/) were rendered to offscreen GPU
textures on the host Apple M5 Pro and visually inspected. This verified native
wgpu texture upload/render/readback, text, transparent sprites and those layouts.
The sandbox exposes no GPU adapter; the permitted host captures succeeded. The
local camera was adjusted after review to separate the port from the enlarged star.

The narrow market now stacks each commodity's stock, prices, margin and trade
actions in a card; its price comparison also stacks by commodity. A 320 px egui
layout test checks that the first card's text and actions fit. The earlier market
preview predates this layout; a fresh GPU capture was unavailable in the sandbox
(no GPU adapter).

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
Seeded-start tests check 1,024 reachable small worlds, orbital clearance and
varied star/body lists. Large-world tests check 150-system connectivity, save/load
and one navigation run; dense and sparse size extremes are also checked. A
150-system galaxy tessellates at a 390 × 844 phone viewport. The 15-object
system cap has orbital-clearance and largest-save checks. Two generated-world
schematic previews are reproducible with `tools/preview_generated_maps.py`.
The GPU capture attempt returned "No GPU adapter for capture"; procedural worlds
have not had a fresh GPU or device visual check.
Refreshes never create stock/money.

Save checks cover world/player/journey/economy round trips, schema/invariant
rejection, failed writes, previous-save backup recovery, and preserving a good backup
over a damaged primary. Client checks cover explicit navigation runs, save
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
and schema 1/2→3 migration. An egui pointer-input test clicks Plot course;
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

The navigation-run batch adds shared fleet progress, explicit Holds, attention
selection/map focus, queued-course save validation and deterministic contacts.
Tests cover two fleets sharing one clock advance, zero-time planning, readiness
rejection, long voyages/fuel charge once, reload-stable encounters and reward
replay rejection. A real save exported by the schema-4 executable migrated to
schema 5 with existing economy, ships, positions and elapsed ticks unchanged.
GPU captures of run review (unassigned Fleet 2 highlighted) and a salvage contact
(seed 2) were rendered on Apple M5 Pro and visually inspected. Physical-window
interaction and player pacing remain unverified; networking is still pending.

Local distances, cube extents and waypoint inputs display AU; galaxy distances
remain ly. Conversion tests cover signed/fractional rounding and invalid input.
Backward save compatibility is no longer a requirement during prototyping; this
batch's earlier migration checks are evidence, not a future maintenance promise.

Map-command checks cover actual egui fleet-marker clicks in the outer and local
maps, body destination intents, safe marker placement around projected stars,
automatic inner/outer transitions, full destination itineraries and stellar-core
crossing rejection. NPC checks demonstrate actual hauling over 16 navigation runs,
cash/commodity conservation after production, player-wallet isolation, save/reload
replay and rejection of player commands against NPC hulls. GPU destination/traffic
captures on Apple M5 Pro were visually inspected. Camera/test controls are collapsed
by default, and map canvas widths are clipped to the actual viewport. Native window
mouse/DPI/lifecycle checks remain separate, unverified acceptance work.

Flagship, contacts and setup: offscreen captures of the Session panel's new-game
setup (1280 × 800), the Fleets sheet with the flagship badge and secondary
fleets' standing orders (1280 × 800 and 390 × 844, `--scenario multi-fleet
--capture-view roster`) and a cargo-pod contact card (1280 × 800, `--seed 2
--capture-view encounter`) were visually inspected. Patrol tours, automatic
contacts and every contact kind are covered by server tests; patrols over many
runs and the balance of contact rewards have not been playtested.

Unlocks and icon: offscreen captures of a guide card (`--capture-view unlock`,
390 × 844) and the game chooser with the re-rendered app icon as its logo
(`--capture-view chooser`, 390 × 844) were visually inspected; the 1024 px
icon render was inspected directly. Unlock conditions, delivery counting, the
captain start and menu gating are covered by tests. Not yet checked: a full
playthrough of the unlock order, and the packaged macOS app's Dock icon after
this re-render (`tools/package_macos.sh` was not run).

Celestial art in generated systems: a contact sheet of every celestial sprite,
and offscreen system-view captures of eight systems of a default 60-system
galaxy (`--seed 3 --systems 60 --capture-view system:N`), were inspected: all
nine star kinds and every planet, giant, moon and asteroid variant render with
their own art. The captures showed generated moons drawn across the star (the
schematic moon spacing pushed them 40 million km from inner planets); fixed so
inward pushes stay short, checked in the captures and by a galaxy-wide test.

Faction police: offscreen captures of a patrol stopping a wanted fleet
(`--capture-view police`, 1280 × 800 and 390 × 844) and the Factions panel with
each faction's record (`factions`) were visually inspected. The market window's
wanted banner and Pay fine button were not captured; pursuit, fines, resisting
arrest, police against raider bands and replacement patrols are covered by
server and core tests. Wanted durations, fine sizes and patrol strength have not
been playtested.

Songbook: offscreen captures of the list view (`--capture-view menu-songbook`,
1280 × 800) and of the desktop transition map with a part shown as playing
(`menu-songbook@space_120`, 1280 × 800; `@cruise_b`, 1920 × 1080) were visually
inspected: the map card widens with the screen (up to 1200 points) and the map
fills it, with section outlines, tempo placement including the space ramps, and
the playing part's outgoing arrows with chances and incoming arrows. On a phone
(`@space_80`, 390 × 844) the Map toggle is hidden and the list shows. Dragging
the card was not captured; its size no longer depends on its position.
With the funk family added (`@funk_100`, 1920 × 1080; `@funk_60`, 1280 × 800)
the map fits a 1080-point-tall screen; on an 800-point screen the card scrolls
to reach the space rows and the explanation. Hover
highlighting, clicking a part and the pulsing "up next" ring need live audio and
pointer input and were not captured; the announced next part is covered by an
audio test.
