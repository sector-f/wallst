extern crate clap;
extern crate image;
extern crate rand;
extern crate xcb;
extern crate xcb_util as xcbu;

use clap::{App, Arg};
use image::*;
use rand::{thread_rng, Rng};
use std::env::args_os;
use std::fs::read_dir;
use std::io::{stderr, Write};
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

const ATOMS: &'static [&'static str] = &[
    "_XROOTPMAP_ID",
    "_XSETROOT_ID",
    "ESETROOT_PMAP_ID"
];

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

fn clean_root_atoms(conn: &xcb::Connection,
                  screen: &xcb::Screen) {
    let ids = ATOMS.iter().map(|atom| {
        let reply = xcb::get_property(conn, false, screen.root(),
            xcb::intern_atom(conn, false, atom).get_reply().expect("failed to intern atom").atom(),
            xcb::ATOM_PIXMAP, 0, 1).get_reply();

        match reply {
            Ok(ref reply) if reply.type_() == xcb::ATOM_PIXMAP => {
                Some(reply.value()[0])
            },
            _ => None,
        }
    }).collect::<Vec<Option<xcb::Pixmap>>>();

    if ids.iter().all(Option::is_some) && ids.iter().all(|id| id == ids.first().unwrap()) {
        xcb::kill_client(conn, ids.first().unwrap().unwrap());
    }

    xcb::kill_client(conn, xcb::KILL_ALL_TEMPORARY);
    xcb::set_close_down_mode(conn, xcb::CLOSE_DOWN_RETAIN_TEMPORARY as u8);
}

fn set_background(conn: &xcb::Connection,
                  screen: &xcb::Screen,
                  image: &image::DynamicImage) {
    let w = screen.width_in_pixels();
    let h = screen.height_in_pixels();

    let mut shm = xcbu::image::shm::create(&conn, screen.root_depth(), w, h)
        .expect("Failed to create shm");

    for (x, y, pixel) in image.pixels() {
        let r = pixel[0] as u32;
        let g = pixel[1] as u32;
        let b = pixel[2] as u32;
        shm.put(x, y, ((r << 16) | (g << 8) | (b << 0)));
    }

    let pixmap_id = conn.generate_id();
    xcb::create_pixmap(&conn, screen.root_depth(), pixmap_id, screen.root(), w, h);

    let context = conn.generate_id();
    xcb::create_gc(&conn, context, pixmap_id, &[]);

    xcbu::image::shm::put(&conn,
                          pixmap_id,
                          context,
                          &shm,
                          0,
                          0,
                          0,
                          0,
                          w as u16,
                          h as u16,
                          false).unwrap();

    clean_root_atoms(&conn, &screen);
    for atom in ATOMS {
            xcb::change_property(conn, xcb::PROP_MODE_REPLACE as u8, screen.root(),
                xcb::intern_atom(conn, false, atom).get_reply().expect("failed to intern atom").atom(),
                xcb::ATOM_PIXMAP, 32, &[pixmap_id]);
    }
    xcb::kill_client(&conn, xcb::KILL_ALL_TEMPORARY);
    xcb::set_close_down_mode(&conn, xcb::CLOSE_DOWN_RETAIN_TEMPORARY as u8);
    xcb::change_window_attributes(&conn, screen.root(), &[(xcb::CW_BACK_PIXMAP, pixmap_id)]);
    xcb::clear_area(&conn, false, screen.root(), 0, 0, w, h);
    conn.flush();
}

fn get_image_data(path: &Path,
                  mode: &BackgroundMode,
                  w: u32,
                  h: u32) -> Result<image::DynamicImage, ImageError> {
    let image = try!(open(path));
    match mode {
        &BackgroundMode::Center => {},
        &BackgroundMode::Stretch => {
                return Ok(image.resize_exact(w, h, FilterType::Lanczos3))
        },
        &BackgroundMode::Fill => {
                return Ok(image.resize(w, h, FilterType::Lanczos3))
        }
        &BackgroundMode::Tile => {},
    }

    Ok(image)
}

fn set_random_loop(mut paths: Vec<PathBuf>,
                   mode: &BackgroundMode,
                   delay: u64,
                   conn: &xcb::Connection,
                   screen: &xcb::Screen) {
    let mut rng = thread_rng();

    let w = screen.width_in_pixels();
    let h = screen.height_in_pixels();
    loop {
        rng.shuffle(&mut paths);
        for path in &paths {
            if let Ok(image) = get_image_data(path, mode, w as u32, h as u32) {
                set_background(conn, screen, &image);
                sleep(Duration::from_secs(delay));
            }
        }
    }
}

fn set_in_loop(paths: Vec<PathBuf>,
               mode: &BackgroundMode,
               delay: u64,
               conn: &xcb::Connection,
               screen: &xcb::Screen) {
    let w = screen.width_in_pixels();
    let h = screen.height_in_pixels();
    loop {
        for path in &paths {
            if let Ok(image) = get_image_data(path, mode, w as u32, h as u32) {
                set_background(conn, screen, &image);
                sleep(Duration::from_secs(delay));
            }
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

    let mode = BackgroundMode::Stretch;

    match images.len() {
        0 => {
            let _ = writeln!(stderr(), "No images found");
        },
        1 => {
            if let Ok(image) = get_image_data(&images[0],
                                              &mode,
                                              screen.width_in_pixels() as u32,
                                              screen.height_in_pixels() as u32) {
                set_background(&conn, &screen, &image,);
            }
        },
        _ => {
            set_in_loop(images, &mode, 1, &conn, &screen);
            // set_random_loop(images, &mode, 1, &conn, &screen);
        },
    }
}
