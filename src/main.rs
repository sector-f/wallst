extern crate clap;
extern crate picto;
extern crate xcb;
extern crate xcb_util as xcbu;

mod xorg;

use clap::{App, Arg, OsValues};
use picto::Buffer;
use picto::color::{Alpha, Gradient, Rgb};
use picto::Orientation::{Horizontal, Vertical};
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, stderr, stdin, Write};
use std::path::Path;
use std::u8;
use xorg::*;

struct BackgroundOptions<'a> {
    path: Option<&'a Path>,
    colors: Option<OsValues<'a>>,
    alpha: Option<u8>,
    w: u32,
    h: u32,
    mode: BackgroundMode,
    vflip: bool,
    hflip: bool,
    save_path: Option<&'a Path>,
}

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

    Tile,    // Put image in top-left of screen.
             // Repeat left-to-right if it is too small.
}

type rgba_image = picto::Buffer<u8, Rgb, Vec<u8>>;

fn get_image_data(bg: BackgroundOptions) -> Result<rgba_image, picto::Error> {
    if let Some(_) = bg.path {
        unimplemented!();
    }

    let colors = bg.colors.clone();
    let mut background = get_background(colors, bg.w, bg.h);

    if let Some(save_path) = bg.save_path {
        match File::create(&save_path) {
            Ok(mut file) => {
                if let Err(e) = picto::write::png(&file, &background, |_|{()}) {
                    let _ = writeln!(stderr(), "Error saving image: {}", e);
               }
            },
            Err(e) => {
                let _ = writeln!(stderr(), "Failed to save image: {}", e);
            },
        }
    }

    Ok(background)
}

fn color_from_str(color_str: &str) -> Rgb {
    let red = u8::from_str_radix(&color_str[1..3], 16).unwrap();
    let green = u8::from_str_radix(&color_str[3..5], 16).unwrap();
    let blue = u8::from_str_radix(&color_str[5..7], 16).unwrap();
    Rgb::new_u8(red, green, blue) as Rgb<f32>
}

fn get_background<'a>(colors: Option<OsValues<'a>>,
                  w: u32,
                  h: u32) -> rgba_image {
    let colors_vec =
        colors.iter().flat_map(|c| c.clone().into_iter())
        .map(|c| color_from_str(&c.to_string_lossy()))
        .collect::<Vec<_>>();

    let bg_color = match colors_vec.len() {
        0 => {
            let black: Rgb = Rgb::new_u8(0, 0, 0);
            Gradient::new(vec![black])
        },
        _ => {
            Gradient::new(colors_vec)
        }
    };

    Buffer::from_gradient(w, h, Horizontal, bg_color)
}

fn is_valid_color(color: String) -> Result<(), String> {
    fn err() -> Result<(),String> {
        Err(String::from("Colors must be in the form #RRGGBB"))
    }

    if color.len() != 7 /* || color.len() != 9 */ {
        return err();
    }

    let mut chars = color.chars();
    if chars.next() != Some('#') {
        return err();
    }

    for c in chars {
        if ! c.is_digit(16) {
            return err();
        }
    }

    Ok(())
}

fn is_alpha(alpha: String) -> Result<(), String> {
    match alpha.parse::<u8>() {
        Ok(_) => Ok(()),
        Err(_) => Err(String::from("Alpha must be an integer from 0-255")),
    }
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
             .possible_values(&["center", "fill", "full", "stretch", "tile"]))
        .arg(Arg::with_name("vflip")
             .help("Flip the image vertically")
             .long("vflip"))
        .arg(Arg::with_name("hflip")
             .help("Flip the image horizontally")
             .long("hflip"))
        .arg(Arg::with_name("alpha")
             .help("The images alpha channel (0 - 255)")
             .short("a")
             .long("alpha")
             .takes_value(true)
             .validator(is_alpha))
        .arg(Arg::with_name("output")
             .help("The path to save the resulting (PNG) image as")
             .short("o")
             .long("output")
             .takes_value(true))
        .arg(Arg::with_name("color")
             .help("Set a solid color as the background")
             .short("c")
             .long("color")
             .validator(is_valid_color)
             .multiple(true)
             .number_of_values(1)
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

    let path = matches.value_of_os("image").map(OsStr::as_ref);

    let mode = if let Some(mode) = matches.value_of("mode") {
        if mode == "center" {
            BackgroundMode::Center
        } else if mode == "fill" {
            BackgroundMode::Fill
        } else if mode == "full" {
            BackgroundMode::Full
        } else if mode == "stretch" {
            BackgroundMode::Stretch
        } else if mode == "tile" {
            BackgroundMode::Tile
        } else {
            unreachable!()
        }
    } else {
        BackgroundMode::Full
    };

    let alpha = matches.value_of("alpha").map(|a| a.parse::<u8>().unwrap());

    let bg_options = BackgroundOptions {
        path: path,
        colors: matches.values_of_os("color"),
        alpha: alpha,
        w: w as u32,
        h: h as u32,
        mode : mode,
        vflip: matches.is_present("vflip"),
        hflip: matches.is_present("hflip"),
        save_path: matches.value_of_os("output").map(Path::new),
    };

    match get_image_data(bg_options) {
        Ok(image) => set_background(&conn, &screen, &image),
        Err(e) => { let _ = writeln!(stderr(), "Error: {}", e); },
    }
}
