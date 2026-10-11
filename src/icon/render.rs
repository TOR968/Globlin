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
const FACE_ABOVE: u32 = 20;
const STICKER_FROM: u32 = 48;

const EMERALD: [u8; 3] = [0x10, 0xb9, 0x81];
const AMBER: [u8; 3] = [0xf5, 0x9e, 0x0b];
const SKY: [u8; 3] = [0x38, 0xbd, 0xf8];
const RED: [u8; 3] = [0xef, 0x44, 0x44];
const WHITE: [u8; 3] = [0xff, 0xff, 0xff];
const DEEP: [u8; 3] = [0x33, 0x41, 0x55];
const INK: [u8; 3] = [0x1d, 0x24, 0x30];
const GOLD: [u8; 3] = [0xff, 0xd1, 0x66];
const GREY: [u8; 3] = [0x5d, 0x66, 0x72];
const DROP: [u8; 3] = [0x8f, 0xd3, 0xff];

const WAVE_CYCLES: f32 = 1.6;
const WAVE_AMPLITUDE: f32 = 0.9;
const GLYPH_TOP: f32 = 7.0;
const GLYPH_BOTTOM: f32 = 28.0;

const EAR_SWING: f32 = 0.2;
const MOUTH_HOP: f32 = 1.0;
const OUTLINE: f32 = 1.25;

const LEFT_EAR: [(f32, f32); 3] = [(7.5, 12.5), (1.5, 4.5), (11.0, 8.5)];
const RIGHT_EAR: [(f32, f32); 3] = [(24.5, 12.5), (30.5, 4.5), (21.0, 8.5)];
const LEFT_EAR_PIVOT: (f32, f32) = (9.25, 10.5);
const RIGHT_EAR_PIVOT: (f32, f32) = (22.75, 10.5);
const HEAD_START: (f32, f32) = (5.0, 15.5);
const HEAD_CURVES: [((f32, f32), (f32, f32)); 4] = [
    ((5.0, 7.0), (16.0, 7.0)),
    ((27.0, 7.0), (27.0, 15.5)),
    ((27.0, 27.5), (16.0, 28.0)),
    ((5.0, 27.5), (5.0, 15.5)),
];
const FACE_CENTRE: (f32, f32) = (16.0, 17.5);
const SPARKLES: [(f32, f32, f32); 2] = [(28.5, 22.0, 2.4), (3.0, 23.5, 1.7)];
const SWEAT: [(f32, f32); 4] = [(25.5, 8.5), (26.9, 11.2), (25.5, 12.6), (24.1, 11.2)];

const EMPTY_GLASS: f32 = 0.35;
const BADGE: (f32, f32) = (26.0, 25.5);
const BADGE_RADIUS: f32 = 4.5;
const BADGE_GAP: f32 = 1.5;
const ALERT_STEM: [(f32, f32); 2] = [(26.0, 22.75), (26.0, 25.75)];
const ALERT_STEM_WIDTH: f32 = 1.75;
const ALERT_DOT: (f32, f32, f32) = (26.0, 28.25, 1.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconState {
    Idle,
    Updates,
    Busy,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Detail {
    Tiny,
    Tray,
    Sticker,
}

impl Detail {
    const fn for_size(size: u32) -> Self {
        if size >= STICKER_FROM {
            Self::Sticker
        } else if size > FACE_ABOVE {
            Self::Tray
        } else {
            Self::Tiny
        }
    }
}

struct Face {
    eyes: [(f32, f32); 2],
    eye_size: (f32, f32),
    glints: Vec<(f32, f32, f32)>,
    mouth: Option<([(f32, f32); 3], f32)>,
}

impl Face {
    fn for_detail(detail: Detail) -> Self {
        match detail {
            Detail::Tiny => Self {
                eyes: [(11.0, 17.5), (21.0, 17.5)],
                eye_size: (3.5, 4.25),
                glints: vec![(12.5, 15.5, 1.7), (22.5, 15.5, 1.7)],
                mouth: None,
            },
            Detail::Tray => Self {
                eyes: [(11.0, 17.0), (21.0, 17.0)],
                eye_size: (3.0, 3.75),
                glints: vec![(12.25, 15.5, 1.3), (22.25, 15.5, 1.3)],
                mouth: Some(([(13.5, 23.5), (16.0, 21.25), (18.5, 23.5)], 1.75)),
            },
            Detail::Sticker => Self {
                eyes: [(11.0, 17.0), (21.0, 17.0)],
                eye_size: (2.75, 3.5),
                glints: vec![
                    (12.0, 15.5, 1.1),
                    (22.0, 15.5, 1.1),
                    (10.25, 18.5, 0.5),
                    (20.25, 18.5, 0.5),
                ],
                mouth: Some(([(14.0, 23.0), (16.0, 21.25), (18.0, 23.0)], 1.3)),
            },
        }
    }
}

pub fn rgba(state: IconState, frame: u32, level: f32, size: u32) -> Vec<u8> {
    composite(&layers(state, frame, level, Detail::for_size(size)), size)
}

pub fn mono(state: IconState, frame: u32, level: f32, size: u32, ink: [u8; 3]) -> Vec<u8> {
    composite(
        &mono_layers(state, frame, level, Detail::for_size(size), ink),
        size,
    )
}

fn layers(state: IconState, frame: u32, level: f32, detail: Detail) -> Vec<Layer> {
    match state {
        IconState::Idle => still(EMERALD, detail, false),
        IconState::Updates => still(AMBER, detail, false),
        IconState::Error => still(RED, detail, true),
        IconState::Busy => working(frame, level, detail),
    }
}

fn still(color: [u8; 3], detail: Detail, worried: bool) -> Vec<Layer> {
    let mut layers = outline(detail, 0.0);
    layers.extend(painted(silhouette(0.0), color));
    layers.extend(features(detail, 0.0));
    if worried {
        layers.extend(painted(vec![Shape::Polygon(SWEAT.to_vec())], DROP));
    }
    if detail == Detail::Sticker {
        let sparkle = if worried { GREY } else { GOLD };
        layers.extend(painted(sparkles(0.0), sparkle));
    }
    layers
}

fn working(frame: u32, level: f32, detail: Detail) -> Vec<Layer> {
    let motion = Motion::at(frame);
    let mut layers = outline(detail, motion.swing);
    layers.extend(painted(silhouette(motion.swing), DEEP));
    layers.push(water(level, motion.phase, SKY));
    layers.extend(features(detail, motion.hop));
    if detail == Detail::Sticker {
        layers.extend(painted(sparkles(motion.phase), WHITE));
    }
    layers
}

fn mono_layers(
    state: IconState,
    frame: u32,
    level: f32,
    detail: Detail,
    ink: [u8; 3],
) -> Vec<Layer> {
    match state {
        IconState::Idle => mono_still(detail, ink),
        IconState::Updates => {
            let mut layers = mono_still(detail, ink);
            layers.extend(badge(ink, false));
            layers
        }
        IconState::Error => {
            let mut layers = mono_still(detail, ink);
            layers.extend(badge(ink, true));
            layers
        }
        IconState::Busy => mono_working(frame, level, detail, ink),
    }
}

fn mono_still(detail: Detail, ink: [u8; 3]) -> Vec<Layer> {
    let mut layers = painted(silhouette(0.0), ink);
    layers.extend(carved_face(detail, 0.0, ink));
    layers
}

fn mono_working(frame: u32, level: f32, detail: Detail, ink: [u8; 3]) -> Vec<Layer> {
    let motion = Motion::at(frame);
    let mut layers = tinted(silhouette(motion.swing), ink, EMPTY_GLASS);
    layers.push(water(level, motion.phase, ink));
    layers.extend(carved_face(detail, motion.hop, ink));
    layers
}

fn carved_face(detail: Detail, hop: f32, ink: [u8; 3]) -> Vec<Layer> {
    let mut layers = erased(face_ink(detail, hop));
    layers.extend(painted(face_glints(detail), ink));
    layers
}

fn badge(ink: [u8; 3], alert: bool) -> Vec<Layer> {
    let (cx, cy) = BADGE;
    let mut layers = erased(vec![Shape::Disc {
        cx,
        cy,
        r: BADGE_RADIUS + BADGE_GAP,
    }]);
    layers.extend(painted(
        vec![Shape::Disc {
            cx,
            cy,
            r: BADGE_RADIUS,
        }],
        ink,
    ));
    if alert {
        let (dot_x, dot_y, dot_r) = ALERT_DOT;
        layers.extend(erased(vec![
            Shape::Segment {
                from: ALERT_STEM[0],
                to: ALERT_STEM[1],
                width: ALERT_STEM_WIDTH,
            },
            Shape::Disc {
                cx: dot_x,
                cy: dot_y,
                r: dot_r,
            },
        ]));
    }
    layers
}

struct Motion {
    phase: f32,
    swing: f32,
    hop: f32,
}

impl Motion {
    fn at(frame: u32) -> Self {
        let phase = std::f32::consts::TAU * (frame % BUSY_FRAMES) as f32 / BUSY_FRAMES as f32;
        Self {
            phase,
            swing: EAR_SWING * phase.sin(),
            hop: MOUTH_HOP * phase.sin().max(0.0),
        }
    }
}

fn water(level: f32, phase: f32, color: [u8; 3]) -> Layer {
    Layer {
        shape: Shape::Polygon(head()),
        paint: Paint::Color(color),
        alpha: 1.0,
        clip: Some(Wave { level, phase }),
    }
}

fn outline(detail: Detail, swing: f32) -> Vec<Layer> {
    if detail != Detail::Sticker {
        return Vec::new();
    }
    let grown = silhouette(swing)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Polygon(points) => Some(Shape::Grown(points, OUTLINE)),
            _ => None,
        })
        .collect();
    painted(grown, WHITE)
}

fn painted(shapes: Vec<Shape>, color: [u8; 3]) -> Vec<Layer> {
    tinted(shapes, color, 1.0)
}

fn tinted(shapes: Vec<Shape>, color: [u8; 3], alpha: f32) -> Vec<Layer> {
    layered(shapes, Paint::Color(color), alpha)
}

fn erased(shapes: Vec<Shape>) -> Vec<Layer> {
    layered(shapes, Paint::Erase, 1.0)
}

fn layered(shapes: Vec<Shape>, paint: Paint, alpha: f32) -> Vec<Layer> {
    shapes
        .into_iter()
        .map(|shape| Layer {
            shape,
            paint,
            alpha,
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

fn features(detail: Detail, hop: f32) -> Vec<Layer> {
    let mut layers = painted(face_ink(detail, hop), INK);
    layers.extend(painted(face_glints(detail), WHITE));
    layers
}

fn face_ink(detail: Detail, hop: f32) -> Vec<Shape> {
    let face = Face::for_detail(detail);
    let (rx, ry) = face.eye_size;
    let mut ink: Vec<Shape> = face
        .eyes
        .iter()
        .map(|&(cx, cy)| Shape::Ellipse { cx, cy, rx, ry })
        .collect();
    if let Some((mouth, width)) = face.mouth {
        let raised = |point: (f32, f32)| (point.0, point.1 - hop);
        ink.push(Shape::Segment {
            from: raised(mouth[0]),
            to: raised(mouth[1]),
            width,
        });
        ink.push(Shape::Segment {
            from: raised(mouth[1]),
            to: raised(mouth[2]),
            width,
        });
    }
    ink
}

fn face_glints(detail: Detail) -> Vec<Shape> {
    Face::for_detail(detail)
        .glints
        .iter()
        .map(|&(cx, cy, r)| Shape::Disc { cx, cy, r })
        .collect()
}

fn sparkles(phase: f32) -> Vec<Shape> {
    SPARKLES
        .iter()
        .map(|&(x, y, r)| {
            let (cx, cy) = rotate_point((x, y), FACE_CENTRE, phase);
            Shape::Polygon(star(cx, cy, r))
        })
        .collect()
}

fn star(cx: f32, cy: f32, outer: f32) -> Vec<(f32, f32)> {
    let inner = outer * 0.35;
    (0..8)
        .map(|index| {
            let angle = std::f32::consts::FRAC_PI_4 * index as f32 - std::f32::consts::FRAC_PI_2;
            let radius = if index % 2 == 0 { outer } else { inner };
            (cx + radius * angle.cos(), cy + radius * angle.sin())
        })
        .collect()
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
    points
        .iter()
        .map(|&point| rotate_point(point, pivot, angle))
        .collect()
}

fn rotate_point(point: (f32, f32), pivot: (f32, f32), angle: f32) -> (f32, f32) {
    let (sin, cos) = angle.sin_cos();
    let (dx, dy) = (point.0 - pivot.0, point.1 - pivot.1);
    (pivot.0 + dx * cos - dy * sin, pivot.1 + dx * sin + dy * cos)
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
    paint: Paint,
    alpha: f32,
    clip: Option<Wave>,
}

#[derive(Clone, Copy)]
enum Paint {
    Color([u8; 3]),
    Erase,
}

enum Shape {
    Disc {
        cx: f32,
        cy: f32,
        r: f32,
    },
    Ellipse {
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
    },
    Segment {
        from: (f32, f32),
        to: (f32, f32),
        width: f32,
    },
    Polygon(Vec<(f32, f32)>),
    Grown(Vec<(f32, f32)>, f32),
}

impl Shape {
    fn contains(&self, x: f32, y: f32) -> bool {
        match self {
            Self::Disc { cx, cy, r } => (x - cx).powi(2) + (y - cy).powi(2) <= r * r,
            Self::Ellipse { cx, cy, rx, ry } => {
                ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) <= 1.0
            }
            Self::Segment { from, to, width } => {
                distance_to_segment(x, y, *from, *to) <= width / 2.0
            }
            Self::Polygon(points) => !points.is_empty() && inside_polygon(points, x, y),
            Self::Grown(points, by) => {
                !points.is_empty()
                    && (inside_polygon(points, x, y) || distance_to_outline(points, x, y) <= *by)
            }
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

fn distance_to_outline(points: &[(f32, f32)], x: f32, y: f32) -> f32 {
    let mut previous = points[points.len() - 1];
    let mut nearest = f32::MAX;
    for &current in points {
        nearest = nearest.min(distance_to_segment(x, y, previous, current));
        previous = current;
    }
    nearest
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
                    accumulated = match layer.paint {
                        Paint::Color(color) => source_over(color, alpha, accumulated),
                        Paint::Erase => destination_out(alpha, accumulated),
                    };
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

fn destination_out(alpha: f32, destination: [f32; 4]) -> [f32; 4] {
    let remaining = destination[3] * (1.0 - alpha);
    if remaining <= 0.0 {
        return [0.0; 4];
    }
    [destination[0], destination[1], destination[2], remaining]
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
