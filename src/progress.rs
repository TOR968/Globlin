use std::time::Duration;

const TAU_SECONDS: f32 = 3.0;
const WORKING_TAU_SECONDS: f32 = 12.0;
const WORKING_CEILING: f32 = 0.9;
const NEVER_QUITE_LANDED: f32 = 0.999_999;
const CELL_THRESHOLDS: [f32; 8] = [
    0.0625, 0.1875, 0.3125, 0.4375, 0.5625, 0.6875, 0.8125, 0.9375,
];

pub fn creep(elapsed: Duration) -> f32 {
    ramp(elapsed, TAU_SECONDS)
}

pub fn working(elapsed: Duration) -> f32 {
    ramp(elapsed, WORKING_TAU_SECONDS).min(WORKING_CEILING)
}

fn ramp(elapsed: Duration, tau: f32) -> f32 {
    (1.0 - (-elapsed.as_secs_f32() / tau).exp()).min(NEVER_QUITE_LANDED)
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
mod tests;
