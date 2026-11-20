//! SplitMix64 stream for one engagement. The session draws a single seed from
//! its persisted navigation RNG, so a saved seed fully reproduces a fight.
pub struct CombatRng(u64);

impl CombatRng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The stream position, so a battle in progress can be saved and resumed.
    pub fn state(&self) -> u64 {
        self.0
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^ (value >> 31)
    }

    /// Uniform-enough value in 0..100 for percentage checks.
    pub fn percent(&mut self) -> u32 {
        (self.next_u64() % 100) as u32
    }
}
