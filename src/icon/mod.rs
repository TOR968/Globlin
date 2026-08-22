use std::fs;
use std::path::Path;

use tray_icon::Icon;

use crate::Result;

mod ico;
mod render;

pub use render::{IconState, BUSY_FRAMES};

const TRAY_SIZE: u32 = 32;

pub const fn state_for(busy: bool, failed: bool, outdated: usize) -> IconState {
    if busy {
        IconState::Busy
    } else if failed {
        IconState::Error
    } else if outdated == 0 {
        IconState::Idle
    } else {
        IconState::Updates
    }
}

pub fn tray(state: IconState, frame: u32, level: f32) -> Result<Icon> {
    Ok(Icon::from_rgba(
        render::rgba(state, frame, level, TRAY_SIZE),
        TRAY_SIZE,
        TRAY_SIZE,
    )
    .map_err(|error| error.to_string())?)
}

pub fn write_app_icon(path: &Path) -> std::io::Result<()> {
    let images: Vec<(u32, Vec<u8>)> = ico::SIZES
        .iter()
        .map(|size| (*size, render::rgba(IconState::Idle, 0, 0.0, *size)))
        .collect();
    fs::write(path, ico::encode(&images))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn busy_outranks_every_other_state() {
        assert_eq!(state_for(true, true, 5), IconState::Busy);
        assert_eq!(state_for(true, false, 0), IconState::Busy);
    }

    #[test]
    fn a_failed_check_outranks_the_package_counts() {
        assert_eq!(state_for(false, true, 0), IconState::Error);
        assert_eq!(state_for(false, true, 3), IconState::Error);
    }

    #[test]
    fn the_settled_states_follow_the_outdated_count() {
        assert_eq!(state_for(false, false, 0), IconState::Idle);
        assert_eq!(state_for(false, false, 1), IconState::Updates);
    }

    #[test]
    fn every_state_and_frame_yields_a_usable_tray_icon() {
        for state in [
            IconState::Idle,
            IconState::Updates,
            IconState::Error,
            IconState::Busy,
        ] {
            for frame in 0..BUSY_FRAMES {
                assert!(tray(state, frame, 0.5).is_ok());
            }
        }
    }

    #[test]
    #[ignore = "writes .ico files for visual review: cargo test -- --ignored --exact icon::tests::dump_every_state_for_visual_review"]
    fn dump_every_state_for_visual_review() {
        let directory = std::env::temp_dir().join("npm-globals-tray-icons");
        fs::create_dir_all(&directory).unwrap();

        let mut sheet = vec![
            ("idle", IconState::Idle, 0, 0.0),
            ("updates", IconState::Updates, 0, 0.0),
            ("error", IconState::Error, 0, 0.0),
        ];
        for frame in 0..BUSY_FRAMES {
            sheet.push(("busy", IconState::Busy, frame, 0.45));
        }
        for (index, level) in [0.0, 0.25, 0.5, 0.75, 1.0].iter().enumerate() {
            sheet.push((
                "level",
                IconState::Busy,
                u32::try_from(index).unwrap_or(u32::MAX),
                *level,
            ));
        }

        for (label, state, frame, level) in sheet {
            let images: Vec<(u32, Vec<u8>)> = ico::SIZES
                .iter()
                .map(|size| (*size, render::rgba(state, frame, level, *size)))
                .collect();
            let name = if state == IconState::Busy {
                format!("{label}-{frame}.ico")
            } else {
                format!("{label}.ico")
            };
            fs::write(directory.join(name), ico::encode(&images)).unwrap();
        }

        println!("icons written to {}", directory.display());
    }

    #[test]
    fn the_app_icon_is_written_with_every_declared_size() {
        let path = std::env::temp_dir().join("npm-globals-tray-icon-test.ico");
        write_app_icon(&path).unwrap();

        let written = fs::read(&path).unwrap();
        assert_eq!(
            u16::from_le_bytes([written[4], written[5]]) as usize,
            ico::SIZES.len()
        );

        fs::remove_file(&path).ok();
    }

    #[test]
    fn a_stale_app_icon_is_overwritten_rather_than_kept() {
        let path = std::env::temp_dir().join("npm-globals-tray-stale-icon-test.ico");
        fs::write(&path, b"an icon from an older build").unwrap();

        write_app_icon(&path).unwrap();

        let written = fs::read(&path).unwrap();
        assert_eq!(&written[..2], &[0x00, 0x00]);
        assert_eq!(
            u16::from_le_bytes([written[4], written[5]]) as usize,
            ico::SIZES.len()
        );

        fs::remove_file(&path).ok();
    }
}
