extern crate clap;
extern crate image;
extern crate regex;
extern crate xcb;
extern crate xcb_util as xcbu;

mod xorg;

use clap::{App, Arg};
use image::*;
use std::u8;
use std::io::{Read, stdin};
use std::path::{Path, PathBuf};
use xorg::*;

#[derive(Clone, Copy)]
struct BackgroundOptions {
    mode: BackgroundMode,
    vflip: bool,
    hflip: bool,
}

#[derive(Clone, Copy)]
enum BackgroundMode {
    // Center,  // Center on background. Preserve aspect ratio.
             // If it's too small, surround with black.
             // See feh's --bg-center

    Stretch, // Force image to fit to screen. Do not
             // preserve aspect ratio. See feh's --bg-scale

    // Fill,    // Like Stretch, but preserve aspect ratio.
             // Scale image until it fits, and then center.
             // Either a horizontal or vertical section will
             // be cut off.

    // Tile,    // Put image in top-left of screen.
             // Repeat left-to-right if it is too small.
}

fn get_image_data(path: &Path,
                  options: BackgroundOptions,
                  w: u32,
                  h: u32) -> Result<image::DynamicImage, ImageError> {
    let dash = PathBuf::from("-");
    let mut image = if path == &dash {
        let mut buffer = Vec::new();
        let _ = stdin().read_to_end(&mut buffer);
        try!(load_from_memory(&buffer))
    } else {
        try!(open(path))
    };

    match options.mode {
        // BackgroundMode::Center => {},
        BackgroundMode::Stretch => {
                image = image.resize_exact(w, h, FilterType::Lanczos3);
        },
        // BackgroundMode::Fill => {
                // return Ok(image.resize(w, h, FilterType::Lanczos3))
        // }
        // BackgroundMode::Tile => {},
    }

    if options.vflip {
        image = image.flipv();
    }

    if options.hflip {
        image = image.fliph();
    }

    Ok(image)
}

fn is_valid_color(color: String) -> Result<(), String> {
    let regex = regex::Regex::new(r"#[:xdigit:]{6}").unwrap();
    match regex.is_match(&color) {
        true => Ok(()),
        false => Err(("Colors must be in the form of #rrggbb".to_owned())),
    }
}

fn get_solid_color(color_str: &str, w: u32, h:u32) -> Result<DynamicImage, ()> {
    let (r, g, b) = (
        u8::from_str_radix(&color_str[1..3], 16).unwrap(),
        u8::from_str_radix(&color_str[3..5], 16).unwrap(),
        u8::from_str_radix(&color_str[5..7], 16).unwrap(),
    );

    let color = Rgba::from_channels(r, g, b, 255);
    // println!("{}", &color_str[1..2]);
    // println!("{}", &color_str[3..4]);
    // println!("{}", &color_str[5..6]);
    // println!("{:?}", color);
    let mut image = DynamicImage::new_rgba8(w, h);
    for x in 0..image.width() {
        for y in 0..image.height() {
            image.put_pixel(x, y, color);
        }
    }

    Ok(image)
}

fn main() {
    let matches = App::new("wallpaper")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or("unknown"))
        .about("Sets the root window")
        .arg(Arg::with_name("display")
             .help("Which display to set the wallpaper of")
             .short("d")
             .long("display")
             .takes_value(true))
        .arg(Arg::with_name("vflip")
             .help("Flip the image vertically")
             .long("vflip"))
        .arg(Arg::with_name("hflip")
             .help("Flip the image horizontally")
             .long("hflip"))
        .arg(Arg::with_name("stretch")
             .help("Stretch image to fit to screen (default)")
             .short("s")
             .long("stretch"))
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

    if let Some(color_string) = matches.value_of("color") {
        if let Ok(color) = get_solid_color(&color_string, w as u32, h as u32) {
            set_background(&conn, &screen, &color);
        }
    } else if let Some(image) = matches.value_of_os("image") {
        let mode = if matches.is_present("stretch") {
                BackgroundMode::Stretch
        } else {
                BackgroundMode::Stretch
        };

        let bg_options = BackgroundOptions {
            mode : mode,
            vflip: matches.is_present("vflip"),
            hflip: matches.is_present("hflip"),
        };
        if let Ok(image) = get_image_data(&Path::new(image), bg_options, w as u32, h as u32) {
            set_background(&conn, &screen, &image);
        }
    }
}
