//! Linear-amplitude ↔ decibel conversion for the `audio/*` wire format.

use kira::Decibels;

/// Converts a linear amplitude into kira's own decibel scale.
///
/// - Wire convention: every `audio/*` volume field is linear amplitude
///   (`0.0` silent, `1.0` unity gain), not decibels.
/// - Why this exists: `Decibels` has `as_amplitude` for the reverse
///   direction but no public constructor the other way.
/// - How: the standard amplitude-to-dB formula, floored at
///   `Decibels::SILENCE` so a non-positive `volume` doesn't compute `-inf`.
pub(crate) fn linear_to_decibels(volume: f32) -> Decibels {
    if volume <= 0.0 {
        Decibels::SILENCE
    } else {
        Decibels((20.0 * volume.log10()).max(Decibels::SILENCE.0))
    }
}
