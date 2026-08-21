use std::time::Duration;

const TAU_SECONDS: f32 = 3.0;
const NEVER_QUITE_LANDED: f32 = 0.999_999;
const CELL_THRESHOLDS: [f32; 8] = [
    0.0625, 0.1875, 0.3125, 0.4375, 0.5625, 0.6875, 0.8125, 0.9375,
];

pub fn creep(elapsed: Duration) -> f32 {
    (1.0 - (-elapsed.as_secs_f32() / TAU_SECONDS).exp()).min(NEVER_QUITE_LANDED)
}

pub fn level(done: usize, total: usize, elapsed: Duration) -> f32 {
    if total == 0 {
        return creep(elapsed);
    }
    ((count(done) + creep(elapsed)) / count(total)).min(1.0)
}

fn count(value: usize) -> f32 {
    f32::from(u16::try_from(value).unwrap_or(u16::MAX))
}

pub fn bar(fraction: f32) -> String {
    let filled = CELL_THRESHOLDS
        .iter()
        .filter(|threshold| fraction >= **threshold)
        .count();
    let mut cells = String::new();
    for cell in 0..CELL_THRESHOLDS.len() {
        cells.push(if cell < filled { '█' } else { '░' });
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn the_water_never_reaches_the_next_mark_before_the_package_lands() {
        for seconds in [0, 1, 5, 30, 600] {
            let fraction = creep(Duration::from_secs(seconds));
            assert!(fraction < 1.0, "{seconds}s produced {fraction}");
        }
        assert_eq!(creep(Duration::ZERO).to_bits(), 0.0f32.to_bits());
    }

    #[test]
    fn the_creep_only_ever_rises() {
        let early = creep(Duration::from_millis(500));
        let later = creep(Duration::from_secs(3));
        let latest = creep(Duration::from_secs(9));

        assert!(early < later, "{early} !< {later}");
        assert!(later < latest, "{later} !< {latest}");
    }

    #[test]
    fn the_bar_keeps_a_fixed_cell_count_at_every_fraction() {
        for fraction in [-1.0, 0.0, 0.13, 0.5, 0.99, 1.0, 4.0] {
            assert_eq!(bar(fraction).chars().count(), 8, "fraction {fraction}");
        }
    }

    #[test]
    fn an_empty_bar_is_all_hollow_and_a_full_bar_is_all_solid() {
        assert_eq!(bar(0.0), "░░░░░░░░");
        assert_eq!(bar(1.0), "████████");
        assert_eq!(bar(0.5), "████░░░░");
    }

    #[test]
    fn a_landed_package_moves_the_level_to_its_full_share() {
        let fresh = level(1, 3, Duration::ZERO);
        assert!((fresh - 1.0 / 3.0).abs() < 0.001, "{fresh}");

        let later = level(1, 3, Duration::from_secs(30));
        assert!(later > fresh, "{later} !> {fresh}");
        assert!(later < 2.0 / 3.0 + 0.001, "{later} crossed the next mark");
    }

    #[test]
    fn an_empty_batch_never_divides_by_zero() {
        let value = level(0, 0, Duration::from_secs(2));
        assert!(value.is_finite(), "{value}");
        assert!((0.0..=1.0).contains(&value), "{value}");
    }

    #[test]
    fn a_finished_batch_stops_at_a_full_level() {
        for seconds in [0, 3, 600] {
            let value = level(3, 3, Duration::from_secs(seconds));
            assert!(
                (value - 1.0).abs() < f32::EPSILON,
                "{seconds}s produced {value}"
            );
        }
    }
}
