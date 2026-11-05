# Coding standard

Keep code readable and split by responsibility. Source files should stay below
500 lines; tests count too. Split before reaching the limit rather than squeezing
statements together or hiding behavior in large closures. Neighboring modules for
rendering, panels, assets, rules and tests make changes easier to review.

Use explicit units, typed IDs and small functions with clear names. Keep
simulation/session authority separate from UI, graphics, storage and transport.
Comments should explain units, invariants or decisions rather than repeat code.

## Local check

```sh
./tools/check.sh
```

This runs the maintained-source length check, rustfmt, Clippy with warnings denied,
and workspace tests. Cargo uses `--locked` so dependency updates are explicit.
Set `CARGO_NET_OFFLINE=true` for cached/offline validation. On a clean machine,
Cargo needs network access once to fetch the locked dependencies.

The line check scans Rust, Python and shell sources under `crates/` and `tools/`;
assets, lockfiles and build outputs are excluded. The GitHub workflow runs the
same check on push and pull requests. Its hosted run remains unverified until
this repository is pushed and the workflow actually executes.

`AGENTS.md` records these rules for future coding sessions. Clippy is the Rust
linter; rustfmt handles formatting. Focused tests validate behavior, while visual,
touch, platform lifecycle and device performance require separate manual evidence.
