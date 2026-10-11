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
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel[3] > 128 && u16::from(pixel[2]) > u16::from(pixel[0]) + 40)
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
fn a_polygon_contains_its_inside_and_not_its_outside() {
    let triangle = Shape::Polygon(vec![(4.0, 4.0), (28.0, 4.0), (16.0, 28.0)]);

    assert!(triangle.contains(16.0, 10.0));
    assert!(!triangle.contains(5.0, 26.0));
    assert!(!triangle.contains(30.0, 30.0));
}

#[test]
fn the_goblin_has_a_head_two_ears_and_a_gap_between_the_ears() {
    let shapes = silhouette(0.0);
    let hits = |x: f32, y: f32| shapes.iter().any(|shape| shape.contains(x, y));

    assert!(hits(16.0, 16.0), "the head is missing");
    assert!(hits(7.0, 9.0), "the left ear is missing");
    assert!(hits(25.0, 9.0), "the right ear is missing");
    assert!(!hits(16.0, 5.0), "nothing should sit between the ears");
    assert!(
        !hits(16.0, 30.0),
        "the chin should end above the bottom edge"
    );
}

#[test]
fn the_face_drops_its_eyes_at_tray_sizes_but_keeps_the_mouth() {
    let detailed = face(true, 0.0);
    let small = face(false, 0.0);
    let eye = |shapes: &[Shape]| shapes.iter().any(|shape| shape.contains(12.0, 15.0));

    assert!(eye(&detailed));
    assert!(!eye(&small));
    assert!(small.iter().any(|shape| shape.contains(16.0, 17.5)));
}

#[test]
fn the_ears_swing_and_the_mouth_hops_while_busy() {
    let resting = rgba(IconState::Busy, 0, 0.5, 64);
    let swung = rgba(IconState::Busy, BUSY_FRAMES / 4, 0.5, 64);

    assert_ne!(resting, swung);
    assert_ne!(
        rotated(&LEFT_EAR, LEFT_EAR_PIVOT, -EAR_SWING),
        LEFT_EAR.to_vec()
    );
}
