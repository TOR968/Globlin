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
const CURVE_STEPS: u32 = 12;
const DETAIL_ABOVE: u32 = 20;

const EMERALD: [u8; 3] = [0x10, 0xb9, 0x81];
const AMBER: [u8; 3] = [0xf5, 0x9e, 0x0b];
const SKY: [u8; 3] = [0x38, 0xbd, 0xf8];
const RED: [u8; 3] = [0xef, 0x44, 0x44];
const WHITE: [u8; 3] = [0xff, 0xff, 0xff];
const DEEP: [u8; 3] = [0x33, 0x41, 0x55];

const WAVE_CYCLES: f32 = 1.6;
const WAVE_AMPLITUDE: f32 = 0.9;
const GLYPH_TOP: f32 = 5.0;
const GLYPH_BOTTOM: f32 = 28.0;

const EAR_SWING: f32 = 0.2;
const MOUTH_HOP: f32 = 1.2;

const LEFT_EAR: [(f32, f32); 3] = [(8.5, 12.5), (2.0, 5.0), (10.5, 9.5)];
const RIGHT_EAR: [(f32, f32); 3] = [(23.5, 12.5), (30.0, 5.0), (21.5, 9.5)];
const LEFT_EAR_PIVOT: (f32, f32) = (9.5, 11.0);
const RIGHT_EAR_PIVOT: (f32, f32) = (22.5, 11.0);
const HEAD_START: (f32, f32) = (6.5, 13.5);
const HEAD_CURVES: [((f32, f32), (f32, f32)); 4] = [
    ((6.5, 9.0), (16.0, 9.0)),
    ((25.5, 9.0), (25.5, 13.5)),
    ((27.5, 23.0), (16.0, 28.0)),
    ((4.5, 23.0), (6.5, 13.5)),
];
const EYES: [(f32, f32); 2] = [(12.0, 15.0), (20.0, 15.0)];
const EYE_RADIUS: f32 = 1.6;
const MOUTH: [(f32, f32); 3] = [(11.5, 22.5), (16.0, 18.5), (20.5, 22.5)];
const MOUTH_WIDTH: f32 = 2.5;
const SMALL_MOUTH: [(f32, f32); 3] = [(11.0, 21.5), (16.0, 17.0), (21.0, 21.5)];
const SMALL_MOUTH_WIDTH: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconState {
    Idle,
    Updates,
    Busy,
    Error,
}

pub fn rgba(state: IconState, frame: u32, level: f32, size: u32) -> Vec<u8> {
    composite(&layers(state, frame, level, size > DETAIL_ABOVE), size)
}

fn layers(state: IconState, frame: u32, level: f32, detailed: bool) -> Vec<Layer> {
    match state {
        IconState::Idle => goblin(EMERALD, detailed),
        IconState::Updates => goblin(AMBER, detailed),
        IconState::Error => goblin(RED, detailed),
        IconState::Busy => working(frame, level, detailed),
    }
}

fn goblin(color: [u8; 3], detailed: bool) -> Vec<Layer> {
    let mut layers = painted(silhouette(0.0), color);
    layers.extend(painted(face(detailed, 0.0), WHITE));
    layers
}

fn working(frame: u32, level: f32, detailed: bool) -> Vec<Layer> {
    let phase = std::f32::consts::TAU * (frame % BUSY_FRAMES) as f32 / BUSY_FRAMES as f32;
    let swing = EAR_SWING * phase.sin();
    let hop = MOUTH_HOP * phase.sin().max(0.0);
    let mut layers = painted(silhouette(swing), DEEP);
    for shape in silhouette(swing) {
        layers.push(Layer {
            shape,
            color: SKY,
            alpha: 1.0,
            clip: Some(Wave { level, phase }),
        });
    }
    layers.extend(painted(face(detailed, hop), WHITE));
    layers
}

fn painted(shapes: Vec<Shape>, color: [u8; 3]) -> Vec<Layer> {
    shapes
        .into_iter()
        .map(|shape| Layer {
            shape,
            color,
            alpha: 1.0,
            clip: None,
        })
        .collect()
}

fn silhouette(swing: f32) -> Vec<Shape> {
    vec![
        Shape::Polygon(rotated(&LEFT_EAR, LEFT_EAR_PIVOT, -swing)),
        Shape::Polygon(rotated(&RIGHT_EAR, RIGHT_EAR_PIVOT, swing)),
        Shape::Polygon(head()),
    ]
}

fn face(detailed: bool, hop: f32) -> Vec<Shape> {
    let (mouth, width) = if detailed {
        (MOUTH, MOUTH_WIDTH)
    } else {
        (SMALL_MOUTH, SMALL_MOUTH_WIDTH)
    };
    let raised = |point: (f32, f32)| (point.0, point.1 - hop);
    let mut shapes = vec![
        Shape::Segment {
            from: raised(mouth[0]),
            to: raised(mouth[1]),
            width,
        },
        Shape::Segment {
            from: raised(mouth[1]),
            to: raised(mouth[2]),
            width,
        },
    ];
    if detailed {
        shapes.extend(EYES.iter().map(|&(cx, cy)| Shape::Disc {
            cx,
            cy,
            r: EYE_RADIUS,
        }));
    }
    shapes
}

fn head() -> Vec<(f32, f32)> {
    let mut points = vec![HEAD_START];
    let mut from = HEAD_START;
    for (control, to) in HEAD_CURVES {
        for step in 1..=CURVE_STEPS {
            let t = step as f32 / CURVE_STEPS as f32;
            let u = 1.0 - t;
            points.push((
                u * u * from.0 + 2.0 * u * t * control.0 + t * t * to.0,
                u * u * from.1 + 2.0 * u * t * control.1 + t * t * to.1,
            ));
        }
        from = to;
    }
    points
}

fn rotated(points: &[(f32, f32)], pivot: (f32, f32), angle: f32) -> Vec<(f32, f32)> {
    let (sin, cos) = angle.sin_cos();
    points
        .iter()
        .map(|&(x, y)| {
            let (dx, dy) = (x - pivot.0, y - pivot.1);
            (pivot.0 + dx * cos - dy * sin, pivot.1 + dx * sin + dy * cos)
        })
        .collect()
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
    Polygon(Vec<(f32, f32)>),
}

impl Shape {
    fn contains(&self, x: f32, y: f32) -> bool {
        match self {
            Self::Disc { cx, cy, r } => (x - cx).powi(2) + (y - cy).powi(2) <= r * r,
            Self::Segment { from, to, width } => {
                distance_to_segment(x, y, *from, *to) <= width / 2.0
            }
            Self::Polygon(points) => inside_polygon(points, x, y),
        }
    }
}

fn inside_polygon(points: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut inside = false;
    let mut previous = points[points.len() - 1];
    for &current in points {
        let crosses = (current.1 > y) != (previous.1 > y);
        if crosses {
            let edge_x =
                current.0 + (y - current.1) * (previous.0 - current.0) / (previous.1 - current.1);
            if x < edge_x {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
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
