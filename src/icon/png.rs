#![cfg(test)]

const CRC_POLYNOMIAL: u32 = 0xEDB8_8320;
const ADLER_MODULUS: u32 = 65521;
const MAX_STORED_BLOCK: usize = 65535;

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ CRC_POLYNOMIAL
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let mut low = 1u32;
    let mut high = 0u32;
    for &byte in data {
        low = (low + u32::from(byte)) % ADLER_MODULUS;
        high = (high + low) % ADLER_MODULUS;
    }
    (high << 16) | low
}

fn stored_deflate(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut offset = 0;
    loop {
        let end = (offset + MAX_STORED_BLOCK).min(data.len());
        let is_final = end == data.len();
        let chunk = &data[offset..end];
        out.push(u8::from(is_final));
        let length = u16::try_from(chunk.len()).expect("a stored block never exceeds 65535 bytes");
        out.extend_from_slice(&length.to_le_bytes());
        out.extend_from_slice(&(!length).to_le_bytes());
        out.extend_from_slice(chunk);
        if is_final {
            return out;
        }
        offset = end;
    }
}

fn zlib(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    out.extend(stored_deflate(data));
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn chunk(kind: [u8; 4], data: &[u8]) -> Vec<u8> {
    let mut crc_input = Vec::with_capacity(kind.len() + data.len());
    crc_input.extend_from_slice(&kind);
    crc_input.extend_from_slice(data);

    let mut out = Vec::with_capacity(12 + data.len());
    let length = u32::try_from(data.len()).expect("a PNG chunk for an icon never approaches 4 GiB");
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(&crc_input);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    out
}

pub(super) fn encode(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let stride = usize::try_from(width).unwrap() * 4;
    assert_eq!(rgba.len(), stride * usize::try_from(height).unwrap());

    let mut raw = Vec::with_capacity(rgba.len() + usize::try_from(height).unwrap());
    for row in rgba.chunks_exact(stride) {
        raw.push(0);
        raw.extend_from_slice(row);
    }

    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);

    let mut file = Vec::new();
    file.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    file.extend(chunk(*b"IHDR", &header));
    file.extend(chunk(*b"IDAT", &zlib(&raw)));
    file.extend(chunk(*b"IEND", &[]));
    file
}

#[cfg(test)]
mod tests;
