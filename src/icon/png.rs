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
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_the_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn adler32_matches_a_known_vector() {
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn a_stored_block_carries_length_and_its_ones_complement() {
        let block = stored_deflate(b"hello");
        assert_eq!(block[0], 1);
        assert_eq!(&block[1..3], &5u16.to_le_bytes());
        assert_eq!(&block[3..5], &(!5u16).to_le_bytes());
        assert_eq!(&block[5..10], b"hello");
    }

    #[test]
    fn data_longer_than_one_block_is_split_and_every_block_but_the_last_is_not_final() {
        let data = vec![0x42u8; MAX_STORED_BLOCK + 10];
        let block = stored_deflate(&data);

        assert_eq!(block[0], 0, "the first block must not claim to be final");

        let first_block_len = 5 + MAX_STORED_BLOCK;
        assert_eq!(block[first_block_len], 1, "the second block must be final");
    }

    fn inflate_stored(mut deflate: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let is_final = deflate[0] == 1;
            let length = usize::from(u16::from_le_bytes([deflate[1], deflate[2]]));
            out.extend_from_slice(&deflate[5..5 + length]);
            deflate = &deflate[5 + length..];
            if is_final {
                return out;
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
        assert_eq!(inflate_stored(body), data);

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
    fn encode_round_trips_a_filled_image_through_the_stored_zlib_stream() {
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
        let raw = inflate_stored(&idat[2..idat.len() - 4]);

        let stride = usize::try_from(width).unwrap() * 4 + 1;
        assert_eq!(raw.len(), stride * usize::try_from(height).unwrap());
        for (row_index, row) in raw.chunks_exact(stride).enumerate() {
            assert_eq!(row[0], 0, "filter byte for row {row_index}");
            assert_eq!(
                &row[1..],
                &pixels[row_index * (stride - 1)..(row_index + 1) * (stride - 1)]
            );
        }
    }
}
