use super::*;

const STATES: [IconState; 4] = [
    IconState::Idle,
    IconState::Updates,
    IconState::Busy,
    IconState::Error,
];

fn alpha_at(pixels: &[u8], size: u32, x: u32, y: u32) -> u8 {
    pixels[((y * size + x) * 4 + 3) as usize]
}

#[test]
fn every_state_fills_a_complete_rgba_buffer_at_every_size() {
    for state in STATES {
        for size in [16, 32, 48, 128] {
            assert_eq!(rgba(state, 0, 0.0, size).len() as u32, size * size * 4);
        }
    }
}

#[test]
fn corners_stay_transparent_and_the_centre_is_opaque() {
    let size = 32;
    let pixels = rgba(IconState::Idle, 0, 0.0, size);

    assert_eq!(alpha_at(&pixels, size, 0, 0), 0);
    assert_eq!(alpha_at(&pixels, size, size - 1, size - 1), 0);
    assert_eq!(alpha_at(&pixels, size, 16, 16), 255);
}

#[test]
fn edges_are_antialiased_rather_than_hard() {
    let size = 32;
    let pixels = rgba(IconState::Idle, 0, 0.0, size);
    let partial = (0..size * size)
        .map(|index| pixels[(index * 4 + 3) as usize])
        .filter(|alpha| *alpha > 0 && *alpha < 255)
        .count();

    assert!(
        partial > 20,
        "expected soft edges, found {partial} blended pixels"
    );
}

#[test]
fn each_state_renders_a_distinct_image() {
    let rendered: Vec<Vec<u8>> = STATES
        .iter()
        .map(|state| rgba(*state, 0, 0.0, 32))
        .collect();

    for left in 0..rendered.len() {
        for right in left + 1..rendered.len() {
            assert_ne!(
                rendered[left], rendered[right],
                "states {left} and {right} match"
            );
        }
    }
}

fn sky_pixels(level: f32) -> usize {
    let pixels = rgba(IconState::Busy, 0, level, 32);
    pixels
        .chunks_exact(4)
        .filter(|pixel| pixel[3] > 128 && pixel[2] > pixel[0] + 40)
        .count()
}

#[test]
fn the_water_covers_more_of_the_glyph_as_the_level_rises() {
    let empty = sky_pixels(0.0);
    let half = sky_pixels(0.5);
    let full = sky_pixels(1.0);

    assert!(half > empty, "half {half} !> empty {empty}");
    assert!(full > half, "full {full} !> half {half}");
}

#[test]
fn the_surface_keeps_moving_while_the_level_stands_still() {
    let first = rgba(IconState::Busy, 0, 0.5, 32);
    let second = rgba(IconState::Busy, 1, 0.5, 32);

    assert_ne!(first, second, "the wave did not advance");
}

#[test]
fn every_state_frame_and_level_yields_a_filled_buffer() {
    for state in STATES {
        for frame in 0..BUSY_FRAMES {
            for level in [0.0, 0.5, 1.0] {
                assert_eq!(rgba(state, frame, level, 32).len(), 32 * 32 * 4);
            }
        }
    }
}

#[test]
fn still_states_ignore_the_frame_counter() {
    for state in [IconState::Idle, IconState::Updates, IconState::Error] {
        assert_eq!(rgba(state, 0, 0.0, 32), rgba(state, 5, 0.0, 32));
    }
}

#[test]
fn a_disc_contains_its_centre_but_not_a_far_corner() {
    let disc = Shape::Disc {
        cx: 16.0,
        cy: 16.0,
        r: 15.0,
    };
    assert!(disc.contains(16.0, 16.0));
    assert!(!disc.contains(0.0, 0.0));
}

#[test]
fn a_segment_is_thick_along_its_length_only() {
    let segment = Shape::Segment {
        from: (10.0, 16.0),
        to: (22.0, 16.0),
        width: 4.0,
    };
    assert!(segment.contains(16.0, 17.5));
    assert!(!segment.contains(16.0, 19.0));
    assert!(!segment.contains(26.0, 16.0));
}

#[test]
fn an_arc_contains_its_span_and_not_the_gap() {
    let arc = Shape::Arc {
        cx: 16.0,
        cy: 16.0,
        r: 6.0,
        width: 4.0,
        start: std::f32::consts::PI,
        end: std::f32::consts::TAU,
    };

    assert!(arc.contains(16.0, 10.0), "the top of the arc is missing");
    assert!(!arc.contains(16.0, 22.0), "the bottom should be a gap");
    assert!(!arc.contains(16.0, 16.0), "the centre should be hollow");
    assert!(!arc.contains(30.0, 10.0), "outside the radius");
    assert!(arc.contains(22.0, 16.0), "the span should close at its end");
    assert!(
        arc.contains(10.0, 16.0),
        "the span should close at its start"
    );
}

#[test]
fn the_glyph_covers_the_ring_and_the_crossbar() {
    let shapes = glyph();
    let hits = |x: f32, y: f32| shapes.iter().any(|shape| shape.contains(x, y));

    assert!(hits(16.0, 7.0), "the top of the ring is missing");
    assert!(hits(20.0, 17.0), "the crossbar is missing");
    assert!(!hits(16.0, 16.0), "the ring's counter should be hollow");
    assert!(
        !hits(25.0, 16.0),
        "the gap on the right of the ring should be open"
    );
}
