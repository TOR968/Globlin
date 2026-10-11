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
    assert!(hits(6.7, 8.5), "the left ear is missing");
    assert!(hits(25.3, 8.5), "the right ear is missing");
    assert!(!hits(16.0, 5.0), "nothing should sit between the ears");
    assert!(
        !hits(16.0, 30.0),
        "the chin should end above the bottom edge"
    );
}

#[test]
fn an_ellipse_is_wider_than_it_is_tall_when_told_so() {
    let ellipse = Shape::Ellipse {
        cx: 16.0,
        cy: 16.0,
        rx: 6.0,
        ry: 3.0,
    };

    assert!(ellipse.contains(21.0, 16.0));
    assert!(!ellipse.contains(16.0, 20.0));
}

#[test]
fn a_grown_polygon_reaches_past_its_edge_by_the_given_distance() {
    let square = vec![(10.0, 10.0), (20.0, 10.0), (20.0, 20.0), (10.0, 20.0)];
    let grown = Shape::Grown(square.clone(), 2.0);

    assert!(grown.contains(15.0, 15.0));
    assert!(grown.contains(21.5, 15.0));
    assert!(!grown.contains(23.0, 15.0));
    assert!(!Shape::Polygon(square).contains(21.5, 15.0));
}

fn has_shape_colour(pixels: &[u8], colour: [u8; 3]) -> bool {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .any(|pixel| pixel[3] == 255 && pixel[..3] == colour)
}

#[test]
fn only_sticker_sizes_get_the_white_outline_and_the_sparkles() {
    let sticker = rgba(IconState::Idle, 0, 0.0, 128);
    let tray = rgba(IconState::Idle, 0, 0.0, 32);

    assert!(has_shape_colour(&sticker, GOLD), "the sparkles are missing");
    assert!(
        !has_shape_colour(&tray, GOLD),
        "the tray should have no sparkles"
    );
    let ring = ((128 * 25 + 64) * 4) as usize;
    assert_eq!(&sticker[ring..ring + 4], &[255, 255, 255, 255]);
    assert_eq!(tray[((32 * 6 + 16) * 4 + 3) as usize], 0);
}

#[test]
fn the_smallest_face_keeps_its_eyes_and_drops_the_mouth() {
    assert!(Face::for_detail(Detail::Tiny).mouth.is_none());
    assert!(Face::for_detail(Detail::Tray).mouth.is_some());
    assert_eq!(Detail::for_size(16), Detail::Tiny);
    assert_eq!(Detail::for_size(32), Detail::Tray);
    assert_eq!(Detail::for_size(48), Detail::Sticker);
}

#[test]
fn an_error_sweats() {
    let error = rgba(IconState::Error, 0, 0.0, 64);

    assert!(has_shape_colour(&error, DROP));
    assert!(!has_shape_colour(&rgba(IconState::Idle, 0, 0.0, 64), DROP));
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
