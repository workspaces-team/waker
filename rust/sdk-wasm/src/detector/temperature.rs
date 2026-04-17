//! Temperature calibration for wake detection scores.

/// Apply temperature scaling to a sigmoid score.
///
/// Converts score → logit → scaled logit → sigmoid.
/// Matches the JS `applyTemperature` implementation.
pub fn apply_temperature(score: f32, temperature: Option<f32>) -> f32 {
    let temp = match temperature {
        Some(t) if (t - 1.0).abs() >= 1e-6 => t,
        _ => return score.clamp(1e-5, 1.0 - 1e-5),
    };

    let safe_score = score.clamp(1e-5, 1.0 - 1e-5);
    let logit = (safe_score / (1.0 - safe_score)).ln();
    let scaled_logit = logit / temp.max(1e-6);
    sigmoid(scaled_logit)
}

#[inline]
fn sigmoid(x: f32) -> f32 {
    let clamped = x.clamp(-40.0, 40.0);
    1.0 / (1.0 + (-clamped).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_temperature_passes_through() {
        let score = 0.7;
        let result = apply_temperature(score, None);
        assert!((result - score).abs() < 1e-4);
    }

    #[test]
    fn temperature_one_passes_through() {
        let score = 0.7;
        let result = apply_temperature(score, Some(1.0));
        assert!((result - score).abs() < 1e-4);
    }

    #[test]
    fn low_temperature_sharpens() {
        let score = 0.6;
        let sharpened = apply_temperature(score, Some(0.5));
        // Lower temperature pushes scores further from 0.5
        assert!(sharpened > score);
    }

    #[test]
    fn high_temperature_smooths() {
        let score = 0.8;
        let smoothed = apply_temperature(score, Some(2.0));
        // Higher temperature pushes scores toward 0.5
        assert!(smoothed < score);
    }
}
