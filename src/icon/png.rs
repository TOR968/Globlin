#![cfg(test)]

const CRC_POLYNOMIAL: u32 = 0xEDB8_8320;
const ADLER_MODULUS: u32 = 65521;
const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;
const END_OF_BLOCK: u32 = 256;

const LENGTH_CODES: [(u32, usize, u32); 29] = [
    (257, 3, 0),
    (258, 4, 0),
    (259, 5, 0),
    (260, 6, 0),
    (261, 7, 0),
    (262, 8, 0),
    (263, 9, 0),
    (264, 10, 0),
    (265, 11, 1),
    (266, 13, 1),
    (267, 15, 1),
    (268, 17, 1),
    (269, 19, 2),
    (270, 23, 2),
    (271, 27, 2),
    (272, 31, 2),
    (273, 35, 3),
    (274, 43, 3),
    (275, 51, 3),
    (276, 59, 3),
    (277, 67, 4),
    (278, 83, 4),
    (279, 99, 4),
    (280, 115, 4),
    (281, 131, 5),
    (282, 163, 5),
    (283, 195, 5),
    (284, 227, 5),
    (285, 258, 0),
];

struct BitWriter {
    bytes: Vec<u8>,
    buffer: u32,
    filled: u32,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            buffer: 0,
            filled: 0,
        }
    }

    fn write(&mut self, value: u32, bits: u32) {
        self.buffer |= value << self.filled;
        self.filled += bits;
        while self.filled >= 8 {
            self.bytes
                .push(u8::try_from(self.buffer & 0xff).expect("a masked byte fits in u8"));
            self.buffer >>= 8;
            self.filled -= 8;
        }
    }

    fn write_code(&mut self, code: u32, bits: u32) {
        for index in (0..bits).rev() {
            self.write((code >> index) & 1, 1);
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.filled > 0 {
            self.bytes
                .push(u8::try_from(self.buffer & 0xff).expect("a masked byte fits in u8"));
        }
        self.bytes
    }
}

fn write_symbol(writer: &mut BitWriter, symbol: u32) {
    if symbol < 144 {
        writer.write_code(0x30 + symbol, 8);
    } else if symbol < 256 {
        writer.write_code(0x190 + symbol - 144, 9);
    } else if symbol < 280 {
        writer.write_code(symbol - END_OF_BLOCK, 7);
    } else {
        writer.write_code(0xc0 + symbol - 280, 8);
    }
}

fn write_match(writer: &mut BitWriter, length: usize) {
    let (symbol, base, extra_bits) = LENGTH_CODES
        .iter()
        .rev()
        .find(|(_, base, _)| *base <= length)
        .copied()
        .expect("a match is never shorter than the smallest length code");

    write_symbol(writer, symbol);
    if extra_bits > 0 {
        let extra = u32::try_from(length - base).expect("a length offset fits in u32");
        writer.write(extra, extra_bits);
    }
    writer.write_code(0, 5);
}

fn deflate(data: &[u8]) -> Vec<u8> {
    let mut writer = BitWriter::new();
    writer.write(1, 1);
    writer.write(1, 2);

    let mut index = 0;
    while index < data.len() {
        let mut run = 0;
        if index > 0 {
            while run < MAX_MATCH
                && index + run < data.len()
                && data[index + run] == data[index + run - 1]
            {
                run += 1;
            }
        }

        if run >= MIN_MATCH {
            write_match(&mut writer, run);
            index += run;
        } else {
            write_symbol(&mut writer, u32::from(data[index]));
            index += 1;
        }
    }

    write_symbol(&mut writer, END_OF_BLOCK);
    writer.finish()
}

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

fn zlib(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    out.extend(deflate(data));
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
    let mut previous: Option<&[u8]> = None;
    for row in rgba.chunks_exact(stride) {
        match previous {
            None => {
                raw.push(1);
                for (column, byte) in row.iter().enumerate() {
                    let left = if column >= 4 { row[column - 4] } else { 0 };
                    raw.push(byte.wrapping_sub(left));
                }
            }
            Some(above) => {
                raw.push(2);
                for (column, byte) in row.iter().enumerate() {
                    raw.push(byte.wrapping_sub(above[column]));
                }
            }
        }
        previous = Some(row);
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
