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

const EMERALD: [u8; 3] = [0x10, 0xb9, 0x81];
const AMBER: [u8; 3] = [0xf5, 0x9e, 0x0b];
const SKY: [u8; 3] = [0x38, 0xbd, 0xf8];
const RED: [u8; 3] = [0xef, 0x44, 0x44];
const WHITE: [u8; 3] = [0xff, 0xff, 0xff];
const DEEP: [u8; 3] = [0x33, 0x41, 0x55];

const WAVE_CYCLES: f32 = 1.6;
const WAVE_AMPLITUDE: f32 = 0.9;
const GLYPH_TOP: f32 = 4.7;
const GLYPH_BOTTOM: f32 = 27.3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconState {
    Idle,
    Updates,
    Busy,
    Error,
}

pub fn rgba(state: IconState, frame: u32, level: f32, size: u32) -> Vec<u8> {
    composite(&layers(state, frame, level), size)
}

fn layers(state: IconState, frame: u32, level: f32) -> Vec<Layer> {
    match state {
        IconState::Idle => badge(EMERALD, glyph()),
        IconState::Updates => badge(AMBER, glyph()),
        IconState::Error => badge(RED, glyph()),
        IconState::Busy => filling(frame, level),
    }
}

fn badge(color: [u8; 3], shapes: Vec<Shape>) -> Vec<Layer> {
    let mut layers = vec![Layer {
        shape: Shape::Disc {
            cx: 16.0,
            cy: 16.0,
            r: 15.0,
        },
        color,
        alpha: 1.0,
        clip: None,
    }];
    layers.extend(shapes.into_iter().map(|shape| Layer {
        shape,
        color: WHITE,
        alpha: 1.0,
        clip: None,
    }));
    layers
}

fn glyph() -> Vec<Shape> {
    vec![
        Shape::Arc {
            cx: 16.0,
            cy: 16.0,
            r: 9.0,
            width: 4.5,
            start: 0.25,
            end: std::f32::consts::TAU - 0.55,
        },
        Shape::Segment {
            from: (18.0, 17.0),
            to: (22.0, 17.0),
            width: 4.0,
        },
    ]
}

fn filling(frame: u32, level: f32) -> Vec<Layer> {
    let phase = std::f32::consts::TAU * (frame % BUSY_FRAMES) as f32 / BUSY_FRAMES as f32;
    let mut layers = vec![Layer {
        shape: Shape::Disc {
            cx: 16.0,
            cy: 16.0,
            r: 15.0,
        },
        color: DEEP,
        alpha: 1.0,
        clip: None,
    }];
    for shape in glyph() {
        layers.push(Layer {
            shape,
            color: WHITE,
            alpha: 0.28,
            clip: None,
        });
    }
    for shape in glyph() {
        layers.push(Layer {
            shape,
            color: SKY,
            alpha: 1.0,
            clip: Some(Wave { level, phase }),
        });
    }
    layers
}

struct Wave {
    level: f32,
    phase: f32,
}

impl Wave {
    fn covers(&self, x: f32, y: f32) -> bool {
        y >= self.surface_y(x)
    }

    fn surface_y(&self, x: f32) -> f32 {
        let span = GLYPH_BOTTOM - GLYPH_TOP;
        let level_y = GLYPH_BOTTOM - self.level.clamp(0.0, 1.0) * span;
        let angle = std::f32::consts::TAU * x / DESIGN * WAVE_CYCLES + self.phase;
        level_y + WAVE_AMPLITUDE * angle.sin()
    }
}

struct Layer {
    shape: Shape,
    color: [u8; 3],
    alpha: f32,
    clip: Option<Wave>,
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
                (angle - start).rem_euclid(std::f32::consts::TAU) <= end - start
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
                let alpha = coverage(layer, x, y, scale) * layer.alpha;
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

fn coverage(layer: &Layer, x: u32, y: u32, scale: f32) -> f32 {
    let step = 1.0 / SUPERSAMPLE as f32;
    let mut hits = 0;

    for row in 0..SUPERSAMPLE {
        for column in 0..SUPERSAMPLE {
            let sample_x = (x as f32 + (column as f32 + 0.5) * step) / scale;
            let sample_y = (y as f32 + (row as f32 + 0.5) * step) / scale;
            let inside = layer.shape.contains(sample_x, sample_y)
                && layer
                    .clip
                    .as_ref()
                    .is_none_or(|wave| wave.covers(sample_x, sample_y));
            if inside {
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
mod tests;
