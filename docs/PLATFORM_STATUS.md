# Native interface spike status

## Implemented

Pinned direct dependencies: winit 0.30.13, wgpu 24.0.1, egui/egui-winit/
egui-wgpu 0.31.1, glam 0.30.10, pollster 0.4.0. Cargo.lock pins the complete graph.
This compatible cached set is a baseline, not a claim to use the latest releases.
The integration follows the upstream egui renderer and winit APIs:
https://docs.rs/egui-wgpu/0.31.1/egui_wgpu/

Desktop creates a native window and GPU surface, routes DPI-aware input through
egui-winit, reconfigures on resize and surface loss, ignores zero-size redraws,
and releases graphics resources on suspend. The event loop waits for input or
egui repaint deadlines while idle. No continuous animation loop is requested.
An orthographic map projects the three-system fixture using glam; the market
layout fixture scrolls horizontally and vertically in a narrow viewport.

## Evidence and remaining checks

Validated in this environment: workspace check, strict Clippy, formatting,
workspace tests (session snapshot isolation plus seven map tests), headless client startup,
and dedicated fixture startup all passed. The graphical launch reported macOS
LaunchServices / com.apple.hiservices-xpcservice connection failures and was
stopped; rendered output and interaction remain unverified in this environment.

See the README for reproducible build and smoke commands. Compilation does not
prove visual readability or mobile behavior. Manual checks still required:

- Select each star and rotate the map; route distance must stay constant.
- Resize to 320 × 480 logical pixels, scroll the market, and change display DPI.
- Exercise minimize/restore, GPU surface loss and desktop suspend/resume.
- Confirm idle redraw/battery behavior with platform profiling tools.
- Test overlap candidate selection and gestures with dense/overlap fixtures.

## Mobile feasibility: unresolved

Only desktop Rust targets are installed in this environment.
`xcrun --sdk iphoneos --show-sdk-path` failed because the iOS SDK cannot be
located. ANDROID_HOME and ANDROID_NDK_HOME are unset. No Android/iOS
shells, device runs, signing configuration, safe-area integration or touch
lifecycle evidence are included. Desktop suspend handling is groundwork, not
mobile validation. Add Android NativeActivity packaging and an iOS app shell,
then verify touch, suspend/resume and surface recreation on devices before
marking phase 1 complete or expanding the full interface.

## Map presentation and navigation follow-up

The map draws a static procedural nebula with a dim decorative starfield using
clipped egui geometry. Selectable stars have explicit domain classifications:
yellow/red dwarfs, blue/red giants and white dwarfs, with warm/cool colors,
different symbolic radii, radial soft halos, bright cores and selection rings.
There are no new dependencies or continuous animation requests.

Focus selection (button/F) centers the camera's orbit target. The Pan tool offers
primary-drag panning. Hover shows type details. A dedicated overlap chooser lists
only nearby candidates with types/distances. Selected labels have priority;
colliding ordinary labels are omitted, with all systems accessible in the list.

Use **Overlap test** in the client for five stars with the same XY position and
different Z coordinates. Front view aligns them; rotation separates them. Use
**Dense test** for a deterministic 64-star cluster containing all five types.
These are explicitly separate UI fixtures, and returning to Session restores its
selection. Reset restores the normal camera; Front reproduces the overlap.

Automated checks cover centering after focus across angles/zooms, all overlapping
candidates being pickable, route distance staying independent of projection,
session-selection preservation, and egui tessellation for all three scenes at
320 × 480 and 1000 × 720. These tests do not establish visual quality, actual
GPU rendering, mobile readability, touch handling or device lifecycle behavior.

## Galaxy / system navigation follow-up

Session snapshots now include fixed local destinations (rocky planet, gas giant,
and trade station). `LocalPosition` is a distinct kilometer-based type, and
`BodyId` is scoped to its parent `SystemId`. Destination snapshots are public
fixture data; there is still no discovery/hidden-state implementation.

Choose **Open selected system**, select a local destination by map or list, and
use **← Galaxy** or Escape to return. The system view has its own camera, focus,
pan tool, presets, orientation indicator and overlap chooser. Galaxy selection,
camera (including focus/pan/zoom) and candidate list remain intact. Navigation
fixtures with no local data show an empty catalogue rather than fabricated bodies.

Local map projection converts kilometers to display units explicitly; displayed
3D distance always uses the kilometer coordinates. Planet/station markers have
symbolic sizes, and fixed positions are neither real ephemerides nor an orbital
simulation. No travel, docking, fleet, fuel or economic rules are implied.

Checks added for local picking/focus, local overlaps, kilometer/view conversion,
and returning via Escape without changing galaxy context. Both galaxy and local
views tessellate at 320 × 480 and 1000 × 720 across all three scenes. Interactive
GPU/device checks and the previously documented SDK gaps remain unresolved.

## Trade-port art, modularity and depth slicing

Blender 5.2.3 generated the original trade-port `.blend` and transparent 512 × 512
PNG using the committed export script. The sandboxed Blender launch crashed before
executing Python; the permitted host launch completed successfully. The sprite was
visually inspected. The native map embeds it, retains a fallback marker on decode
failure and offers a larger selected-station preview. The illustrated station is
a billboard; it does not rotate as a 3D mesh with the camera. The client adds only
PNG decoding, with image 0.25.10 pinned and default features disabled.

Galaxy depth slicing is opt-in and operates on X/Y/Z world coordinates. The UI
shows the inclusive bounds in light-years and a visible-system count. Stars outside
the slice are hidden and excluded from picking. The system list marks hidden entries;
Focus recenters the slice if necessary, retaining its thickness and the selection.
The slice survives local-view navigation; Reset clears it.

Additional checks cover all-axis slice bounds, hidden-target picking, focus reveal,
slice preservation on return, PNG transparency/decoding and the selected station
inspector at desktop/phone sizes. The dense phone/desktop tessellation test now
exercises an enabled slice. `./tools/check.sh` additionally enforces source length,
formatting and strict Clippy. Interactive GPU, touch and device lifecycle checks
remain open, as do the previously documented mobile SDK gaps.
