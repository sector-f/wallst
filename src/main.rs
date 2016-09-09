extern crate image;
extern crate rand;
extern crate xcb;
extern crate xcb_util;

use xcb_util::image::{self as xcb_image, Image as XcbImage};
// extern crate byteorder;

// use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use image::*;
use rand::{thread_rng, Rng};
use std::env::args_os;
use std::fs::read_dir;
use std::io::{stderr, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use std::thread::sleep;
use std::time::Duration;

enum BackgroundMode {
    Center,  // Center on background. Preserve aspect ratio.
             // If it's too small, surround with black.
             // See feh's --bg-center

    Stretch, // Force image to fit to screen. Do not
             // preserve aspect ratio. See feh's --bg-scale

    Fill,    // Like Stretch, but preserve aspect ratio.
             // Zoom image until it fits. Either a horizontal
             // or vertical section will be cut off.

    Tile,    // Put image in top-left of screen.
             // Repeat left-to-right if it is too small.
}

// #[derive(Debug)]
// struct ScreenInfo {
//     root_window_id: u32,
//     root_window_depth: u8,
//     x: u32,
//     y: u32,
// }

fn get_image_data(path: &Path,
                  mode: BackgroundMode,
                  screen_x: u16,
                  screen_y: u16) -> Result<XcbImage, ImageError> {
    let image = try!(open(path));
    match mode {
        BackgroundMode::Center => {},
        BackgroundMode::Stretch => {
            let new_image =
                imageops::resize(&image, screen_x as u32, screen_y as u32, FilterType::Nearest);
            return Ok(xcb_image::create(&new_image.into_raw(), screen_y as u32, screen_x as u32));
        },
        BackgroundMode::Fill => {},
        BackgroundMode::Tile => {},
    }

    Ok(xcb_image::create(&image.raw_pixels(), screen_y as u32, screen_x as u32))
}

fn set_image(data: &[u8],
             conn: &xcb::Connection,
             screen: &xcb::Screen) {
}

// fn set_image(path: &Path,
//              conn: &xcb::Connection,
//              screen: &xcb::Screen) -> Result<(), xcb::GenericError> {
//     let image_result = get_image_data(path, BackgroundMode::Stretch, screen.width_in_pixels(), screen.height_in_pixels());
//     match image_result {
//         Ok(image) => {
//         },
//         Err(e) => {
//             let _ = writeln!(stderr(), "Error opening {}: {}", path.display(), e);
//         },
//     }
//     Ok(())
// }

fn set_random_loop(mut paths: Vec<PathBuf>, delay: u64, conn: &xcb::Connection) {
    let mut rng = thread_rng();

    loop {
        rng.shuffle(&mut paths);
        for path in &paths {
            // set_image(path, conn);
            sleep(Duration::from_secs(delay));
        }
    }
}

fn get_images_vec(args_vec: &[PathBuf]) -> Vec<PathBuf> {
    let mut images: Vec<PathBuf> = Vec::new();

    for path in args_vec {
        if path.is_file() {
            images.push(path.to_owned());
        } else if path.is_dir() {
            if let Ok(contents) = read_dir(path) {
                for direntry_result in contents {
                    if let Ok(direntry) = direntry_result {
                        images.push(direntry.path());
                    }
                }
            }
        }
    }
    images
}

fn get_screen(conn: &xcb::Connection, display: usize) -> xcb::Screen {
    let setup = conn.get_setup();
    let mut screen_iter = setup.roots();
    let screen = screen_iter.nth(display).expect("Failed to get screen info");
    return screen
}

fn main() {
    let (conn, screen_num) = xcb::Connection::connect(None).expect("Failed to connect to X server");
    let screen = get_screen(&conn, screen_num as usize);

    let arguments = args_os().skip(1).map(PathBuf::from).collect::<Vec<PathBuf>>();
    let images = get_images_vec(&arguments);

    match images.len() {
        0 => {
            let _ = writeln!(stderr(), "No images found");
            drop(&conn);
            exit(1);
        },
        1 => {
            // set_image(&images[0], &conn, &screen);
        },
        _ => {
            // set_random_loop(images, 30, &conn);
        },
    }
}
