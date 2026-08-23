use super::*;

fn read_u16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn read_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn one_image(size: u32) -> Vec<(u32, Vec<u8>)> {
    vec![(size, vec![0x11u8; (size * size * 4) as usize])]
}

#[test]
fn the_directory_header_declares_an_icon_and_the_image_count() {
    let file = encode(&one_image(16));

    assert_eq!(read_u16(&file, 0), 0);
    assert_eq!(read_u16(&file, 2), 1);
    assert_eq!(read_u16(&file, 4), 1);
}

#[test]
fn every_entry_points_at_a_body_that_lies_inside_the_file() {
    let images: Vec<(u32, Vec<u8>)> = SIZES
        .iter()
        .map(|size| (*size, vec![0x22u8; (size * size * 4) as usize]))
        .collect();
    let file = encode(&images);

    assert_eq!(read_u16(&file, 4) as usize, SIZES.len());
    for (index, size) in SIZES.iter().enumerate() {
        let entry = DIRECTORY_HEADER + index * DIRECTORY_ENTRY;
        let length = read_u32(&file, entry + 8) as usize;
        let offset = read_u32(&file, entry + 12) as usize;

        assert_eq!(file[entry], *size as u8);
        assert_eq!(read_u16(&file, entry + 6), 32);
        assert_eq!(
            offset + length,
            if index + 1 == SIZES.len() {
                file.len()
            } else {
                read_u32(&file, entry + DIRECTORY_ENTRY + 12) as usize
            }
        );
    }
}

#[test]
fn a_body_carries_a_doubled_height_for_the_mask_and_the_mask_bytes() {
    let size = 16u32;
    let file = encode(&one_image(size));
    let offset = read_u32(&file, DIRECTORY_HEADER + 12) as usize;

    assert_eq!(read_u32(&file, offset), INFO_HEADER);
    assert_eq!(read_u32(&file, offset + 4), size);
    assert_eq!(read_u32(&file, offset + 8), size * 2);

    let expected = INFO_HEADER as usize + (size * size * 4) as usize + (4 * size) as usize;
    assert_eq!(file.len() - offset, expected);
}

#[test]
fn pixels_are_written_bottom_up_as_bgra() {
    let mut pixels = vec![0u8; 16 * 16 * 4];
    pixels[0..4].copy_from_slice(&[0x01, 0x02, 0x03, 0x04]);
    let file = encode(&[(16, pixels)]);

    let offset = read_u32(&file, DIRECTORY_HEADER + 12) as usize + INFO_HEADER as usize;
    let last_row = offset + 15 * 16 * 4;

    assert_eq!(&file[last_row..last_row + 4], &[0x03, 0x02, 0x01, 0x04]);
}

#[test]
fn a_dimension_of_256_is_stored_as_zero() {
    assert_eq!(stored_dimension(16), 16);
    assert_eq!(stored_dimension(255), 255);
    assert_eq!(stored_dimension(256), 0);
}
