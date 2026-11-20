//! Seeds for new, unscripted maps. The generated world itself is deterministic
//! for a seed, so a saved world and `--seed` remain reproducible.

/// Zero is the fixed baseline fixture used by captures and explicit `--seed 0`.
pub fn new_seed() -> u64 {
    fastrand::u64(..).max(1)
}
