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
    let directory = std::env::temp_dir().join("globlin-icons");
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
#[ignore = "writes PNGs for the README into docs/img: cargo test -- --ignored --exact icon::tests::dump_readme_images"]
fn dump_readme_images() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/img");
    fs::create_dir_all(&directory).unwrap();

    let logo_size = 128;
    let logo = render::rgba(IconState::Idle, 0, 0.0, logo_size);
    fs::write(
        directory.join("logo.png"),
        png::encode(logo_size, logo_size, &logo),
    )
    .unwrap();

    let icon_size = 32;
    let states = [
        ("icon-idle", IconState::Idle, 0, 0.0),
        ("icon-updates", IconState::Updates, 0, 0.0),
        ("icon-error", IconState::Error, 0, 0.0),
        ("icon-busy", IconState::Busy, BUSY_FRAMES / 2, 0.6),
    ];
    for (name, state, frame, level) in states {
        let pixels = render::rgba(state, frame, level, icon_size);
        let png = png::encode(icon_size, icon_size, &pixels);
        fs::write(directory.join(format!("{name}.png")), png).unwrap();
    }

    println!("README images written to {}", directory.display());
}

#[test]
fn the_app_icon_is_written_with_every_declared_size() {
    let path = std::env::temp_dir().join("globlin-icon-test.ico");
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
    let path = std::env::temp_dir().join("globlin-stale-icon-test.ico");
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

const OG_WIDTH: u32 = 1200;
const OG_HEIGHT: u32 = 630;
const OG_BACKGROUND: [u8; 4] = [0x0d, 0x11, 0x17, 0xff];
const OG_LOGO_SIZE: u32 = 288;
const OG_STATE_SIZE: u32 = 72;
const OG_STATE_GAP: u32 = 48;

fn blit(canvas: &mut [u8], canvas_width: u32, patch: &[u8], patch_size: u32, left: u32, top: u32) {
    let width = usize::try_from(canvas_width).unwrap();
    let size = usize::try_from(patch_size).unwrap();
    let left = usize::try_from(left).unwrap();
    let top = usize::try_from(top).unwrap();

    for row in 0..size {
        for column in 0..size {
            let source = (row * size + column) * 4;
            let alpha = u32::from(patch[source + 3]);
            if alpha == 0 {
                continue;
            }
            let target = ((top + row) * width + left + column) * 4;
            for channel in 0..3 {
                let over = u32::from(patch[source + channel]) * alpha;
                let under = u32::from(canvas[target + channel]) * (255 - alpha);
                canvas[target + channel] = u8::try_from((over + under) / 255).unwrap();
            }
        }
    }
}

fn open_graph_card() -> Vec<u8> {
    let pixels = usize::try_from(OG_WIDTH * OG_HEIGHT).unwrap();
    let mut canvas = OG_BACKGROUND.repeat(pixels);

    let logo = render::rgba(IconState::Idle, 0, 0.0, OG_LOGO_SIZE);
    blit(
        &mut canvas,
        OG_WIDTH,
        &logo,
        OG_LOGO_SIZE,
        (OG_WIDTH - OG_LOGO_SIZE) / 2,
        96,
    );

    let states = [
        (IconState::Idle, 0, 0.0),
        (IconState::Updates, 0, 0.0),
        (IconState::Error, 0, 0.0),
        (IconState::Busy, BUSY_FRAMES / 2, 0.6),
    ];
    let row_width = 4 * OG_STATE_SIZE + 3 * OG_STATE_GAP;
    let row_left = (OG_WIDTH - row_width) / 2;
    for (index, (state, frame, level)) in states.into_iter().enumerate() {
        let offset = u32::try_from(index).unwrap() * (OG_STATE_SIZE + OG_STATE_GAP);
        let patch = render::rgba(state, frame, level, OG_STATE_SIZE);
        blit(
            &mut canvas,
            OG_WIDTH,
            &patch,
            OG_STATE_SIZE,
            row_left + offset,
            456,
        );
    }

    canvas
}

#[test]
fn the_open_graph_card_is_the_size_social_networks_expect() {
    let card = open_graph_card();

    assert_eq!(
        card.len(),
        usize::try_from(OG_WIDTH * OG_HEIGHT * 4).unwrap()
    );
    assert_eq!(&card[..4], &OG_BACKGROUND);
}

#[test]
fn the_open_graph_card_paints_the_logo_over_the_background() {
    let card = open_graph_card();
    let centre = usize::try_from((OG_HEIGHT / 3) * OG_WIDTH + OG_WIDTH / 2).unwrap() * 4;

    assert_ne!(&card[centre..centre + 4], &OG_BACKGROUND);
}

#[test]
#[ignore = "writes the landing-page PNGs into site/img: cargo test -- --ignored --exact icon::tests::dump_site_images"]
fn dump_site_images() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("site/img");
    fs::create_dir_all(&directory).unwrap();

    let logo_size = 256;
    let logo = render::rgba(IconState::Idle, 0, 0.0, logo_size);
    fs::write(
        directory.join("logo.png"),
        png::encode(logo_size, logo_size, &logo),
    )
    .unwrap();

    let icon_size = 64;
    let states = [
        ("icon-idle", IconState::Idle, 0, 0.0),
        ("icon-updates", IconState::Updates, 0, 0.0),
        ("icon-error", IconState::Error, 0, 0.0),
        ("icon-busy", IconState::Busy, BUSY_FRAMES / 2, 0.6),
    ];
    for (name, state, frame, level) in states {
        let pixels = render::rgba(state, frame, level, icon_size);
        fs::write(
            directory.join(format!("{name}.png")),
            png::encode(icon_size, icon_size, &pixels),
        )
        .unwrap();
    }

    fs::write(
        directory.join("og.png"),
        png::encode(OG_WIDTH, OG_HEIGHT, &open_graph_card()),
    )
    .unwrap();

    println!("site images written to {}", directory.display());
}
