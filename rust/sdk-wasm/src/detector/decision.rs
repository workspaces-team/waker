//! Detection decision state: confirmation hit counting and cooldown enforcement.

/// Tracks the confirmation hit counter and cooldown timer for wake decisions.
pub struct DecisionState {
    confirmation_hits: u32,
    cooldown_until_ms: f64,
}

impl DecisionState {
    pub fn new() -> Self {
        Self {
            confirmation_hits: 0,
            cooldown_until_ms: 0.0,
        }
    }

    /// Observe a score and return whether a wake event should fire.
    ///
    /// `now_ms` is the current timestamp in milliseconds (e.g. from `Date.now()`).
    pub fn observe(
        &mut self,
        score: f32,
        threshold: f32,
        required_hits: u32,
        cooldown_seconds: f32,
        duplicate_suppression_seconds: f32,
        now_ms: f64,
    ) -> bool {
        if now_ms < self.cooldown_until_ms {
            if score < threshold {
                self.confirmation_hits = 0;
            }
            return false;
        }

        if score >= threshold {
            self.confirmation_hits += 1;
            if self.confirmation_hits >= required_hits.max(1) {
                self.confirmation_hits = 0;
                let suppression_seconds =
                    cooldown_seconds.max(duplicate_suppression_seconds).max(0.0);
                self.cooldown_until_ms = now_ms + (suppression_seconds as f64 * 1000.0);
                return true;
            }
            return false;
        }

        self.confirmation_hits = 0;
        false
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        self.confirmation_hits = 0;
        self.cooldown_until_ms = 0.0;
    }
}

impl Default for DecisionState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_hit_triggers() {
        let mut state = DecisionState::new();
        assert!(state.observe(0.8, 0.5, 1, 1.0, 1.0, 1000.0));
    }

    #[test]
    fn below_threshold_does_not_trigger() {
        let mut state = DecisionState::new();
        assert!(!state.observe(0.3, 0.5, 1, 1.0, 1.0, 1000.0));
    }

    #[test]
    fn cooldown_prevents_retrigger() {
        let mut state = DecisionState::new();
        assert!(state.observe(0.8, 0.5, 1, 1.0, 1.0, 1000.0));
        // Within cooldown window (1000ms)
        assert!(!state.observe(0.8, 0.5, 1, 1.0, 1.0, 1500.0));
        // After cooldown
        assert!(state.observe(0.8, 0.5, 1, 1.0, 1.0, 2100.0));
    }

    #[test]
    fn duplicate_suppression_can_extend_cooldown() {
        let mut state = DecisionState::new();
        assert!(state.observe(0.8, 0.5, 1, 1.0, 4.0, 1000.0));
        assert!(!state.observe(0.8, 0.5, 1, 1.0, 4.0, 4500.0));
        assert!(state.observe(0.8, 0.5, 1, 1.0, 4.0, 5100.0));
    }

    #[test]
    fn multi_hit_confirmation() {
        let mut state = DecisionState::new();
        assert!(!state.observe(0.8, 0.5, 3, 1.0, 1.0, 1000.0));
        assert!(!state.observe(0.8, 0.5, 3, 1.0, 1.0, 1100.0));
        assert!(state.observe(0.8, 0.5, 3, 1.0, 1.0, 1200.0));
    }

    #[test]
    fn miss_resets_hit_count() {
        let mut state = DecisionState::new();
        assert!(!state.observe(0.8, 0.5, 3, 1.0, 1.0, 1000.0));
        assert!(!state.observe(0.8, 0.5, 3, 1.0, 1.0, 1100.0));
        // Miss resets
        assert!(!state.observe(0.3, 0.5, 3, 1.0, 1.0, 1200.0));
        // Need 3 more hits now
        assert!(!state.observe(0.8, 0.5, 3, 1.0, 1.0, 1300.0));
    }
}
