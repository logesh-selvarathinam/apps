// What moves while music plays, as a function of the app's own clock (ms since the music started moving). No Api,
// no floats: the watch has no FPU. Host-tested.

use cmf_wp2_ui::motion::sin;

/// One animation step, 100 ms: the glow drifts slowly, so 10 updates a second look smooth and keep the redraw
/// (three large see-through layers) cheap.
pub const FRAME_MS: u32 = 100;

/// How far each glow drifts from its home, px.
pub const DRIFT: i32 = 44;
/// A full drift cycle per glow, ms: three different periods, so the pattern doesn't repeat for a long time.
pub const PERIODS: [u32; 3] = [6000, 7000, 5000];

/// The ring around play/pause: one turn per 2.4 s. The disc: one turn per 9 s.
pub const RING_MS: u32 = 2400;
pub const DISC_MS: u32 = 9000;

/// `65536 * t / period` as an angle in u16 turns.
fn turn(t: u32, period: u32) -> u16 {
    (((t % period) as u64 * 65536) / period as u64) as u16
}

/// Glow `i`'s offset from home at time `t`: a slow loop, x and y a quarter turn apart.
pub fn drift(i: usize, t: u32) -> (i16, i16) {
    let a = turn(t, PERIODS[i]).wrapping_add(i as u16 * 21845);
    let x = (DRIFT * sin(a)) >> 14;
    let y = (DRIFT * sin(a.wrapping_add(16384))) >> 14;
    (x as i16, (y * 3 / 4) as i16)
}

/// Rotation in degrees (0..360) for something that turns once per `period` ms.
pub fn degrees(t: u32, period: u32) -> u16 {
    ((t % period) as u64 * 360 / period as u64) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drift_stays_small_and_moves() {
        for i in 0..3 {
            let mut seen = std::collections::BTreeSet::new();
            for t in (0..20_000).step_by(FRAME_MS as usize) {
                let (x, y) = drift(i, t);
                assert!(x.abs() <= DRIFT as i16 && y.abs() <= DRIFT as i16, "{i} {t}: {x},{y}");
                seen.insert((x, y));
            }
            assert!(seen.len() > 30, "glow {i} barely moves");
        }
    }

    #[test]
    fn drift_steps_are_gentle() {
        // between two frames no glow jumps more than a few px, so 10 fps looks smooth
        for i in 0..3 {
            for t in (0..20_000).step_by(FRAME_MS as usize) {
                let (a, b) = (drift(i, t), drift(i, t + FRAME_MS));
                assert!((a.0 - b.0).abs() <= 6 && (a.1 - b.1).abs() <= 6, "{i} {t}");
            }
        }
    }

    #[test]
    fn rotations_wrap() {
        assert_eq!(degrees(0, RING_MS), 0);
        assert_eq!(degrees(RING_MS / 4, RING_MS), 90);
        assert_eq!(degrees(RING_MS, RING_MS), 0);
        assert!(degrees(u32::MAX, DISC_MS) < 360);
        assert_eq!(degrees(DISC_MS / 2, DISC_MS), 180);
    }
}
