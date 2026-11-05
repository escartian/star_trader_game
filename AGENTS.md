# Coding standard

- Keep source files at or below 500 lines, ideally comfortably below that limit.
  Split by responsibility before reaching it; do not compress code to pass the check.
  Tests count toward the limit and can live in a neighboring test module.
- Prefer readable names, small focused functions and explicit imports. Keep view
  state, drawing, UI panels, simulation rules and I/O in separate modules.
- Explain units and non-obvious invariants. Use distinct types across coordinate
  spaces. Authoritative mutations belong in simulation/session code, never in UI.
- Use Rust's standard formatter and Clippy. Warnings fail the check; resolve them
  instead of adding broad lint suppressions.
- Run `./tools/check.sh` before committing. Add focused behavior tests where rules
  or interactions change. Report device/visual checks honestly and separately.
- Preserve the dependency boundaries and milestone criteria in ROADMAP.md and
  docs/ARCHITECTURE.md. Update documentation when implemented behavior changes.
- Keep original art sources and reproducible export scripts alongside runtime
  assets. Blender is an authoring tool, not a runtime dependency.

Mobile build/device work is deferred at the user’s request. Continue the desktop
implementation roadmap while keeping those validation gaps documented.
