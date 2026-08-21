#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::many_single_char_names
)]

pub const DESIGN: f32 = 32.0;
pub const BUSY_FRAMES: u32 = 8;

const SUPERSAMPLE: u32 = 4;

const SLATE: [u8; 3] = [0x64, 0x74, 0x8b];
const AMBER: [u8; 3] = [0xf5, 0x9e, 0x0b];
const SKY: [u8; 3] = [0x38, 0xbd, 0xf8];
const RED: [u8; 3] = [0xef, 0x44, 0x44];
const WHITE: [u8; 3] = [0xff, 0xff, 0xff];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconState {
    Idle,
    Updates,
    Busy,
    Error,
}

pub fn rgba(state: IconState, frame: u32, size: u32) -> Vec<u8> {
    composite(&layers(state, frame), size)
}

fn layers(state: IconState, frame: u32) -> Vec<Layer> {
    match state {
        IconState::Idle => badge(SLATE, glyph()),
        IconState::Updates => badge(AMBER, glyph()),
        IconState::Error => badge(RED, glyph()),
        IconState::Busy => spinner(frame),
    }
}

fn badge(color: [u8; 3], glyph: Vec<Shape>) -> Vec<Layer> {
    let mut layers = vec![Layer {
        shape: Shape::Disc {
            cx: 16.0,
            cy: 16.0,
            r: 15.0,
        },
        color,
        alpha: 1.0,
    }];
    layers.extend(glyph.into_iter().map(|shape| Layer {
        shape,
        color: WHITE,
        alpha: 1.0,
    }));
    layers
}

fn glyph() -> Vec<Shape> {
    vec![
        Shape::Segment {
            from: (10.0, 9.5),
            to: (10.0, 23.0),
            width: 4.4,
        },
        Shape::Arc {
            cx: 16.0,
            cy: 15.5,
            r: 6.0,
            width: 4.4,
            start: std::f32::consts::PI,
            end: std::f32::consts::TAU,
        },
        Shape::Segment {
            from: (22.0, 15.5),
            to: (22.0, 23.0),
            width: 4.4,
        },
    ]
}

fn spinner(frame: u32) -> Vec<Layer> {
    const ORBIT: f32 = 10.6;
    const DOT: f32 = 3.4;

    (0..BUSY_FRAMES)
        .map(|dot| {
            let behind = (dot + BUSY_FRAMES - frame % BUSY_FRAMES) % BUSY_FRAMES;
            let angle = std::f32::consts::TAU * dot as f32 / BUSY_FRAMES as f32
                - std::f32::consts::FRAC_PI_2;
            Layer {
                shape: Shape::Disc {
                    cx: 16.0 + ORBIT * angle.cos(),
                    cy: 16.0 + ORBIT * angle.sin(),
                    r: DOT,
                },
                color: SKY,
                alpha: 1.0 - 0.8 * (behind as f32 / (BUSY_FRAMES - 1) as f32),
            }
        })
        .collect()
}

struct Layer {
    shape: Shape,
    color: [u8; 3],
    alpha: f32,
}

enum Shape {
    Disc {
        cx: f32,
        cy: f32,
        r: f32,
    },
    Segment {
        from: (f32, f32),
        to: (f32, f32),
        width: f32,
    },
    Arc {
        cx: f32,
        cy: f32,
        r: f32,
        width: f32,
        start: f32,
        end: f32,
    },
}

impl Shape {
    fn contains(&self, x: f32, y: f32) -> bool {
        match *self {
            Self::Disc { cx, cy, r } => (x - cx).powi(2) + (y - cy).powi(2) <= r * r,
            Self::Segment { from, to, width } => distance_to_segment(x, y, from, to) <= width / 2.0,
            Self::Arc {
                cx,
                cy,
                r,
                width,
                start,
                end,
            } => {
                let (dx, dy) = (x - cx, y - cy);
                let distance = (dx * dx + dy * dy).sqrt();
                if (distance - r).abs() > width / 2.0 {
                    return false;
                }
                let angle = dy.atan2(dx).rem_euclid(std::f32::consts::TAU);
                angle >= start && angle <= end
            }
        }
    }
}

fn composite(layers: &[Layer], size: u32) -> Vec<u8> {
    let scale = size as f32 / DESIGN;
    let mut pixels = vec![0u8; (size * size * 4) as usize];

    for y in 0..size {
        for x in 0..size {
            let mut accumulated = [0.0f32; 4];
            for layer in layers {
                let alpha = coverage(&layer.shape, x, y, scale) * layer.alpha;
                if alpha > 0.0 {
                    accumulated = source_over(layer.color, alpha, accumulated);
                }
            }
            let offset = ((y * size + x) * 4) as usize;
            for channel in 0..4 {
                pixels[offset + channel] = to_byte(accumulated[channel]);
            }
        }
    }
    pixels
}

fn coverage(shape: &Shape, x: u32, y: u32, scale: f32) -> f32 {
    let step = 1.0 / SUPERSAMPLE as f32;
    let mut hits = 0;

    for row in 0..SUPERSAMPLE {
        for column in 0..SUPERSAMPLE {
            let sample_x = (x as f32 + (column as f32 + 0.5) * step) / scale;
            let sample_y = (y as f32 + (row as f32 + 0.5) * step) / scale;
            if shape.contains(sample_x, sample_y) {
                hits += 1;
            }
        }
    }
    hits as f32 / (SUPERSAMPLE * SUPERSAMPLE) as f32
}

fn source_over(color: [u8; 3], alpha: f32, destination: [f32; 4]) -> [f32; 4] {
    let below = destination[3];
    let combined = alpha + below * (1.0 - alpha);
    if combined <= 0.0 {
        return [0.0; 4];
    }
    let mix = |source: f32, under: f32| (source * alpha + under * below * (1.0 - alpha)) / combined;
    [
        mix(color[0] as f32 / 255.0, destination[0]),
        mix(color[1] as f32 / 255.0, destination[1]),
        mix(color[2] as f32 / 255.0, destination[2]),
        combined,
    ]
}

fn to_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn distance_to_segment(x: f32, y: f32, from: (f32, f32), to: (f32, f32)) -> f32 {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length_squared = dx * dx + dy * dy;
    let position = if length_squared <= f32::EPSILON {
        0.0
    } else {
        (((x - from.0) * dx + (y - from.1) * dy) / length_squared).clamp(0.0, 1.0)
    };
    let nearest = (from.0 + position * dx, from.1 + position * dy);
    ((x - nearest.0).powi(2) + (y - nearest.1).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
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
                assert_eq!(rgba(state, 0, size).len() as u32, size * size * 4);
            }
        }
    }

    #[test]
    fn corners_stay_transparent_and_the_centre_is_opaque() {
        let size = 32;
        let pixels = rgba(IconState::Idle, 0, size);

        assert_eq!(alpha_at(&pixels, size, 0, 0), 0);
        assert_eq!(alpha_at(&pixels, size, size - 1, size - 1), 0);
        assert_eq!(alpha_at(&pixels, size, 16, 16), 255);
    }

    #[test]
    fn edges_are_antialiased_rather_than_hard() {
        let size = 32;
        let pixels = rgba(IconState::Idle, 0, size);
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
        let rendered: Vec<Vec<u8>> = STATES.iter().map(|state| rgba(*state, 0, 32)).collect();

        for left in 0..rendered.len() {
            for right in left + 1..rendered.len() {
                assert_ne!(
                    rendered[left], rendered[right],
                    "states {left} and {right} match"
                );
            }
        }
    }

    #[test]
    fn the_spinner_changes_on_every_frame_and_repeats_after_a_full_turn() {
        let frames: Vec<Vec<u8>> = (0..BUSY_FRAMES)
            .map(|frame| rgba(IconState::Busy, frame, 32))
            .collect();

        for frame in 1..frames.len() {
            assert_ne!(
                frames[frame - 1],
                frames[frame],
                "frame {frame} did not move"
            );
        }
        assert_eq!(rgba(IconState::Busy, BUSY_FRAMES, 32), frames[0]);
    }

    #[test]
    fn the_spinner_has_one_fully_bright_leading_dot() {
        let brightest = spinner(0)
            .iter()
            .filter(|layer| layer.alpha >= 0.999)
            .count();
        assert_eq!(brightest, 1);
    }

    #[test]
    fn still_states_ignore_the_frame_counter() {
        for state in [IconState::Idle, IconState::Updates, IconState::Error] {
            assert_eq!(rgba(state, 0, 32), rgba(state, 5, 32));
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
    }

    #[test]
    fn the_glyph_covers_both_stems_and_the_shoulder() {
        let shapes = glyph();
        let hits = |x: f32, y: f32| shapes.iter().any(|shape| shape.contains(x, y));

        assert!(hits(10.0, 21.0), "the left stem is missing");
        assert!(hits(22.0, 21.0), "the right stem is missing");
        assert!(hits(16.0, 9.5), "the shoulder is missing");
        assert!(
            !hits(16.0, 21.0),
            "the gap under the shoulder should be open"
        );
    }
}
