#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]

pub const SIZES: [u32; 5] = [16, 32, 48, 64, 128];

const DIRECTORY_HEADER: usize = 6;
const DIRECTORY_ENTRY: usize = 16;
const INFO_HEADER: u32 = 40;

pub fn encode(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let bodies: Vec<Vec<u8>> = images
        .iter()
        .map(|(size, pixels)| device_independent_bitmap(*size, pixels))
        .collect();

    let mut file = Vec::new();
    file.extend_from_slice(&0u16.to_le_bytes());
    file.extend_from_slice(&1u16.to_le_bytes());
    file.extend_from_slice(&(images.len() as u16).to_le_bytes());

    let mut offset = DIRECTORY_HEADER + images.len() * DIRECTORY_ENTRY;
    for ((size, _), body) in images.iter().zip(&bodies) {
        file.push(stored_dimension(*size));
        file.push(stored_dimension(*size));
        file.push(0);
        file.push(0);
        file.extend_from_slice(&1u16.to_le_bytes());
        file.extend_from_slice(&32u16.to_le_bytes());
        file.extend_from_slice(&(body.len() as u32).to_le_bytes());
        file.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += body.len();
    }

    for body in bodies {
        file.extend_from_slice(&body);
    }
    file
}

const fn stored_dimension(size: u32) -> u8 {
    if size >= 256 {
        0
    } else {
        size as u8
    }
}

fn device_independent_bitmap(size: u32, pixels: &[u8]) -> Vec<u8> {
    let mask_stride = size.div_ceil(32) * 4;
    let mut body = Vec::new();

    body.extend_from_slice(&INFO_HEADER.to_le_bytes());
    body.extend_from_slice(&(size as i32).to_le_bytes());
    body.extend_from_slice(&((size * 2) as i32).to_le_bytes());
    body.extend_from_slice(&1u16.to_le_bytes());
    body.extend_from_slice(&32u16.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes());
    body.extend_from_slice(&(size * size * 4 + mask_stride * size).to_le_bytes());
    body.extend_from_slice(&0i32.to_le_bytes());
    body.extend_from_slice(&0i32.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes());

    for y in (0..size).rev() {
        for x in 0..size {
            let offset = ((y * size + x) * 4) as usize;
            body.push(pixels[offset + 2]);
            body.push(pixels[offset + 1]);
            body.push(pixels[offset]);
            body.push(pixels[offset + 3]);
        }
    }
    body.resize(body.len() + (mask_stride * size) as usize, 0);
    body
}

#[cfg(test)]
mod tests;
