use super::*;

#[test]
fn crc32_matches_the_standard_check_value() {
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
}

#[test]
fn adler32_matches_a_known_vector() {
    assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
}

struct BitReader<'a> {
    data: &'a [u8],
    position: usize,
}

impl BitReader<'_> {
    fn bit(&mut self) -> u32 {
        let byte = self.data[self.position / 8];
        let bit = (byte >> (self.position % 8)) & 1;
        self.position += 1;
        u32::from(bit)
    }

    fn bits(&mut self, count: u32) -> u32 {
        let mut value = 0;
        for index in 0..count {
            value |= self.bit() << index;
        }
        value
    }

    fn code(&mut self, count: u32) -> u32 {
        let mut value = 0;
        for _ in 0..count {
            value = (value << 1) | self.bit();
        }
        value
    }
}

fn inflate_fixed(data: &[u8]) -> Vec<u8> {
    let mut reader = BitReader { data, position: 0 };
    assert_eq!(reader.bits(1), 1, "the stream is one final block");
    assert_eq!(reader.bits(2), 1, "the block uses fixed Huffman codes");

    let mut out = Vec::new();
    loop {
        let mut code = reader.code(7);
        let symbol = if code <= 0x17 {
            code + 256
        } else {
            code = (code << 1) | reader.bit();
            if code <= 0xbf {
                code - 0x30
            } else if code <= 0xc7 {
                code - 0xc0 + 280
            } else {
                code = (code << 1) | reader.bit();
                code - 0x190 + 144
            }
        };

        if symbol == END_OF_BLOCK {
            return out;
        }

        if symbol < END_OF_BLOCK {
            out.push(u8::try_from(symbol).unwrap());
            continue;
        }

        let (_, base, extra_bits) = LENGTH_CODES
            .iter()
            .find(|(length_code, _, _)| *length_code == symbol)
            .copied()
            .expect("every emitted length symbol is in the table");
        let length = base + usize::try_from(reader.bits(extra_bits)).unwrap();

        let distance_code = reader.code(5);
        assert_eq!(distance_code, 0, "the encoder only ever emits distance 1");
        for _ in 0..length {
            let byte = out[out.len() - 1];
            out.push(byte);
        }
    }
}

#[test]
fn zlib_wraps_a_stream_that_inflates_back_to_the_original_bytes() {
    let data = b"the quick brown fox jumps over the lazy dog".repeat(200);
    let wrapped = zlib(&data);

    assert_eq!(wrapped[0], 0x78);
    assert_eq!(wrapped[1], 0x01);
    assert_eq!(
        (u16::from(wrapped[0]) * 256 + u16::from(wrapped[1])) % 31,
        0
    );

    let body = &wrapped[2..wrapped.len() - 4];
    assert_eq!(inflate_fixed(body), data);

    let checksum = &wrapped[wrapped.len() - 4..];
    assert_eq!(
        u32::from_be_bytes(checksum.try_into().unwrap()),
        adler32(&data)
    );
}

#[test]
fn encode_produces_a_valid_signature_and_ihdr_for_a_tiny_image() {
    let pixels = vec![0xFFu8; 2 * 2 * 4];
    let png = encode(2, 2, &pixels);

    assert_eq!(
        &png[0..8],
        &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
    );
    assert_eq!(&png[12..16], b"IHDR");
    assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 2);
    assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 2);
    assert_eq!(png[24], 8, "bit depth");
    assert_eq!(png[25], 6, "color type: truecolor with alpha");
    assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
}

#[test]
fn encode_round_trips_a_filled_image_through_the_compressed_zlib_stream() {
    let width = 4u32;
    let height = 3u32;
    let pixels: Vec<u8> = (0..width * height * 4)
        .map(|index| u8::try_from(index % 256).unwrap())
        .collect();
    let png = encode(width, height, &pixels);

    let ihdr_chunk_end = 8 + 4 + 4 + 13 + 4;
    let idat_length = usize::try_from(u32::from_be_bytes(
        png[ihdr_chunk_end..ihdr_chunk_end + 4].try_into().unwrap(),
    ))
    .unwrap();
    let idat_start = ihdr_chunk_end + 8;
    let idat = &png[idat_start..idat_start + idat_length];
    let raw = inflate_fixed(&idat[2..idat.len() - 4]);

    let stride = usize::try_from(width).unwrap() * 4;
    assert_eq!(raw.len(), (stride + 1) * usize::try_from(height).unwrap());

    let mut restored: Vec<u8> = Vec::with_capacity(pixels.len());
    for (row_index, row) in raw.chunks_exact(stride + 1).enumerate() {
        let expected_filter = if row_index == 0 { 1 } else { 2 };
        assert_eq!(row[0], expected_filter, "filter byte for row {row_index}");

        for column in 0..stride {
            let reference = if row_index == 0 {
                if column >= 4 {
                    restored[column - 4]
                } else {
                    0
                }
            } else {
                restored[(row_index - 1) * stride + column]
            };
            restored.push(row[1 + column].wrapping_add(reference));
        }
    }

    assert_eq!(restored, pixels);
}

#[test]
fn a_uniform_image_compresses_to_a_fraction_of_its_raw_size() {
    let width = 64;
    let height = 64;
    let pixels = vec![0x11u8; usize::try_from(width * height).unwrap() * 4];

    let png = encode(width, height, &pixels);

    assert!(
        png.len() * 20 < pixels.len(),
        "a uniform image encoded to {} bytes from {} raw",
        png.len(),
        pixels.len()
    );
}

#[test]
fn a_run_longer_than_the_largest_length_code_is_split_across_several_matches() {
    let width = 512u32;
    let height = 1u32;
    let pixels = vec![0x00u8; usize::try_from(width).unwrap() * 4];

    let png = encode(width, height, &pixels);

    let ihdr_chunk_end = 8 + 4 + 4 + 13 + 4;
    let idat_length = usize::try_from(u32::from_be_bytes(
        png[ihdr_chunk_end..ihdr_chunk_end + 4].try_into().unwrap(),
    ))
    .unwrap();
    let idat_start = ihdr_chunk_end + 8;
    let idat = &png[idat_start..idat_start + idat_length];

    let raw = inflate_fixed(&idat[2..idat.len() - 4]);

    assert_eq!(raw.len(), usize::try_from(width).unwrap() * 4 + 1);
    assert_eq!(raw[0], 1);
    assert!(raw[1..].iter().all(|byte| *byte == 0));
}
