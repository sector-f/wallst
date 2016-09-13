extern crate clap;
extern crate image;
extern crate regex;
extern crate xcb;
extern crate xcb_util as xcbu;

mod xorg;

use clap::{App, Arg};
use image::*;
use std::fs::File;
use std::io::{Read, stdin};
use std::path::PathBuf;
use std::u8;
use xorg::*;

// #[derive(Clone, Copy)]
struct BackgroundOptions {
    path: Option<PathBuf>,
    color: Option<String>,
    w: u32,
    h: u32,
    mode: BackgroundMode,
    vflip: bool,
    hflip: bool,
    // save_path: Option<PathBuf>,
}

// #[derive(Clone, Copy)]
enum BackgroundMode {
    Center,  // Center on background. Preserve aspect ratio.
             // If it's too small, surround with background color.
             // See feh's --bg-center

    Stretch, // Force image to fit to screen. Do not
             // preserve aspect ratio. See feh's --bg-scale

    Fill,    // Like Stretch, but preserve aspect ratio.
             // Scale image until it fits, and then center.
             // Either a horizontal or vertical section will
             // be cut off.

    Full,    // Place image in top-left of screen with no
             // modifications.

    // Tile,    // Put image in top-left of screen.
             // Repeat left-to-right if it is too small.
}

fn get_image_data(bg: &BackgroundOptions) -> Result<image::DynamicImage, ImageError> {
    let mut image =
        match bg.path {
            Some(ref path) => {
                let mut buffer = Vec::new();
                if path == &PathBuf::from("-") {
                    let _ = stdin().read_to_end(&mut buffer);
                    try!(load_from_memory(&buffer))
                } else {
                    let mut fin = match File::open(path) {
                        Ok(f) => f,
                        Err(e) => return Err(ImageError::IoError(e))
                    };
                    let _ = fin.read_to_end(&mut buffer);
                    try!(load_from_memory(&buffer))
                }
            },
            None => {
                if let Some(ref color) = bg.color {
                    get_solid_image(&color, bg.w, bg.h)
                } else {
                    unreachable!()
                }
            },
        };

    match bg.mode {
        BackgroundMode::Center => {
            let img_w = image.width();
            let img_h = image.height();
            let bg_w = bg.w;
            let bg_h = bg.h;

            let bg_color = bg.color.to_owned().unwrap_or("#000000".to_owned());
            let mut bg_image = get_solid_image(&bg_color, bg.w, bg.h);

            let left: i32 = (bg_w as i32 - img_w as i32) / 2;
            let top: i32 = (bg_h as i32 - img_h as i32) / 2;

            let mut image_copy = image;
            let sub_image = image_copy.sub_image(
                if left < 0 { left.abs() as u32 } else { 0 },
                if top < 0 { top.abs() as u32 } else { 0 },
                if left < 0 { bg_w } else { img_w },
                if top < 0 { bg_h } else { img_h },
            );

            bg_image.copy_from(&sub_image,
                               if left < 0 { 0 } else { left.abs() as u32 },
                               if top < 0 { 0 } else { top.abs() as u32 });
            image = bg_image;
        },
        BackgroundMode::Stretch => {
            image = image.resize_exact(bg.w, bg.h, FilterType::Lanczos3);
        },
        BackgroundMode::Fill => {
            let bg_color = bg.color.to_owned().unwrap_or("#000000".to_owned());
            let mut bg_image = get_solid_image(&bg_color, bg.w, bg.h);

            image = image.resize(bg.w, bg.h, FilterType::Lanczos3);
            let offset = (bg.w - image.width()) / 2;
            bg_image.copy_from(&image, offset, 0);
            image = bg_image;
        },
        BackgroundMode::Full => {
            let bg_color = bg.color.to_owned().unwrap_or("#000000".to_owned());
            let mut bg_image = get_solid_image(&bg_color, bg.w, bg.h);

            bg_image.copy_from(&image, 0, 0);
            image = bg_image;
        },
        // BackgroundMode::Tile => {},
    }

    if bg.vflip {
        image = image.flipv();
    }

    if bg.hflip {
        image = image.fliph();
    }

    Ok(image)
}

fn is_valid_color(color: String) -> Result<(), String> {
    let regex = regex::Regex::new(r"^#[:xdigit:]{6}$").unwrap();
    match regex.is_match(&color) {
        true => Ok(()),
        false => Err(("Colors must be in the form of #rrggbb".to_owned())),
    }
}

fn get_solid_image(color_str: &str, w: u32, h:u32) -> DynamicImage {
    let (r, g, b) = (
        u8::from_str_radix(&color_str[1..3], 16).unwrap(),
        u8::from_str_radix(&color_str[3..5], 16).unwrap(),
        u8::from_str_radix(&color_str[5..7], 16).unwrap(),
    );

    let color = Rgb::from_channels(r, g, b, 255);
    DynamicImage::ImageRgb8(ImageBuffer::from_pixel(w, h, color))
}

fn main() {
    let matches = App::new("wallst")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or("unknown"))
        .about("Sets the root window")
        .arg(Arg::with_name("display")
             .help("Which display to set the wallpaper of")
             .short("d")
             .long("display")
             .takes_value(true))
        .arg(Arg::with_name("mode")
             .help("Sets the image mode")
             .short("m")
             .long("mode")
             .takes_value(true)
             .possible_values(&["center", "fill", "full", "stretch"]))
        .arg(Arg::with_name("vflip")
             .help("Flip the image vertically")
             .long("vflip"))
        .arg(Arg::with_name("hflip")
             .help("Flip the image horizontally")
             .long("hflip"))
        .arg(Arg::with_name("color")
             .help("Set a solid color as the background")
             .short("c")
             .long("color")
             .validator(is_valid_color)
             .takes_value(true))
        .arg(Arg::with_name("image")
             .help("The image to use as the background. Use - for stdin")
             .required_unless("color")
             .index(1))
        .get_matches();

    let (conn, screen_num) = xcb::Connection::connect(matches.value_of("display"))
        .expect("Failed to connect to X server");
    let screen = get_screen(&conn, screen_num as usize);
    let w = screen.width_in_pixels();
    let h = screen.height_in_pixels();

    let path = matches.value_of_os("image").map(|p| PathBuf::from(p));

    let mode = if let Some(mode) = matches.value_of("mode") {
        if mode == "center" {
            BackgroundMode::Center
        } else if mode == "fill" {
            BackgroundMode::Fill
        } else if mode == "full" {
            BackgroundMode::Full
        } else if mode == "stretch" {
            BackgroundMode::Stretch
        } else {
            unreachable!()
        }
    } else {
        BackgroundMode::Fill
    };

    let color = matches.value_of("color").map(|c| c.to_string());

    let bg_options = BackgroundOptions {
        path: path,
        color: color,
        w: w as u32,
        h: h as u32,
        mode : mode,
        vflip: matches.is_present("vflip"),
        hflip: matches.is_present("hflip"),
        // save_path: None,
    };

    if let Ok(image) = get_image_data(&bg_options) {
        set_background(&conn, &screen, &image);
    }
}
