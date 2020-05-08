extern crate clap;
extern crate image;
extern crate palette;
extern crate xcb;
extern crate xcb_util as xcbu;

mod xorg;

use clap::{App, Arg, OsValues};
// use image::*;
use image::{
    DynamicImage,
    FilterType,
    GenericImage,
    ImageError,
    ImageFormat,
    load_from_memory,
    Pixel,
    Rgba as image_rgba,
};
use palette::Gradient;
use palette::Rgb as palette_rgb;
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

fn get_image_data(bg: BackgroundOptions) -> Result<DynamicImage, ImageError> {
    let colors = bg.colors.clone();
    let mut image = get_background(colors, bg.w, bg.h);

    if let Some(ref path) = bg.path {
        let mut buffer = Vec::new();
        let mut foreground =
            if path.as_os_str() == "-" {
                let _ = stdin().read_to_end(&mut buffer);
                load_from_memory(&buffer)?
            } else {
                let mut fin = match File::open(path) {
                    Ok(f) => f,
                    Err(e) => return Err(ImageError::IoError(e))
                };
                let _ = fin.read_to_end(&mut buffer);
                load_from_memory(&buffer)?
            };
        foreground = DynamicImage::ImageRgba8(foreground.to_rgba());

        let mut placeholder = image.clone();

        match bg.mode {
            BackgroundMode::Center => {
                let img_w = foreground.width();
                let img_h = foreground.height();

                let left: i32 = (bg.w as i32 - img_w as i32) / 2;
                let top: i32 = (bg.h as i32 - img_h as i32) / 2;

                foreground = DynamicImage::ImageRgba8(
                    foreground.sub_image(
                        if left < 0 { left.abs() as u32 } else { 0 },
                        if top < 0 { top.abs() as u32 } else { 0 },
                        if left < 0 { bg.w } else { img_w },
                        if top < 0 { bg.h } else { img_h },
                    ).to_image()
                );

                let x_offset = if left < 0 { 0 } else { left.abs() as u32 };
                let y_offset = if top < 0 { 0 } else { top.abs() as u32 };
                placeholder.copy_from(&foreground, x_offset, y_offset);
            },
            BackgroundMode::Stretch => {
                placeholder = foreground.resize_exact(bg.w, bg.h, FilterType::Lanczos3);
            },
            BackgroundMode::Fill => {
                foreground = foreground.resize(bg.w, bg.h, FilterType::Lanczos3);
                let offset = (bg.w - foreground.width()) / 2;
                placeholder.copy_from(&foreground, offset, 0);
            },
            BackgroundMode::Full => {
                foreground = foreground.crop(0, 0, bg.w, bg.h);
                placeholder.copy_from(&foreground, 0, 0);
            },
            BackgroundMode::Tile => {
                // To-Do: Use a SubImage rather than increasing the placeholder size?
                let img_w = foreground.width();
                let img_h = foreground.height();

                // I love it when I come across crazy things in source code that
                // make no sense and have no explanation! Just kidding.
                // Here's what this does. For both width and height:
                // If img > bg then the placeholder becomes the img size.
                // If bg > img then the placeholder becomes the lowest multiple of the
                // img size that's greater than the bg size.
                // This is so that copying to the placeholder image actually succeeds.

                placeholder = get_background(
                    bg.colors,
                    img_w * ((bg.w + img_w - 1) / img_w),
                    img_h * ((bg.h + img_h - 1) / img_h),
                );

                let mut vert_overlap = 0;
                while vert_overlap < bg.h {
                    let mut horiz_overlap = 0;
                    while horiz_overlap < bg.w {
                        placeholder.copy_from(&foreground, horiz_overlap, vert_overlap);
                        horiz_overlap += foreground.width();
                    }
                    vert_overlap += foreground.height();
                }

                placeholder = placeholder.crop(0, 0, bg.w, bg.h);
            },
        }

        if let Some(alpha) = bg.alpha {
            for pixel in placeholder.as_mut_rgba8().unwrap().pixels_mut() {
                pixel[3] = (pixel[3] as f32 * (alpha as f32 / 255f32)) as u8;
            }
        }

        let zipped = image.as_mut_rgba8().unwrap().pixels_mut().zip(
            placeholder.as_rgba8().unwrap().pixels()
        );

        for (bg_pix, fg_pix) in zipped {
            bg_pix.blend(fg_pix);
        }

    }

    if bg.vflip {
        image = image.flipv();
    }

    if bg.hflip {
        image = image.fliph();
    }

    if let Some(save_path) = bg.save_path {
        match File::create(&save_path) {
            Ok(mut file) => {
                if let Err(e) = image.save(&mut file, ImageFormat::PNG) {
                    let _ = writeln!(stderr(), "Error saving image: {}", e);
                }
            },
            Err(e) => {
                let _ = writeln!(stderr(), "Failed to save image: {}", e);
            },
        }
    }

    Ok(image)
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

fn get_background<'a>(colors: Option<OsValues<'a>>, w: u32, h: u32) -> DynamicImage {
    let colors_vec: Vec<_> =
        colors.iter().flat_map(|c| c.clone().into_iter()).collect::<Vec<_>>();
    match colors_vec.len() {
        0 => {
            get_gradient(&["#000000".as_ref()], w, h)
        },
        _ => {
            get_gradient(&colors_vec, w, h)
        },
    }
}

fn get_gradient(colors: &[&OsStr], w: u32, h: u32) -> DynamicImage {
    let mut image = DynamicImage::new_rgba8(w, h);
    let gradient = Gradient::new(
        colors.iter().map(|c|
            color_from_str(
                c.to_str().unwrap()
            )
        )
    );

    for (x, color) in (0..w).zip(gradient.take(w as usize)) {
        for y in 0..h {
            let foo = srgb(color);
            image.as_mut_rgba8().unwrap().put_pixel(x, y, foo);
        }
    }

    image
}

fn srgb(value: palette_rgb<f32>) -> image_rgba<u8> {
    image_rgba { data: [
        value.red as u8,
        value.green as u8,
        value.blue as u8,
        255,
    ] }
}

fn color_from_str(color_str: &str) -> palette_rgb<f32> {
    palette_rgb::new(
        u8::from_str_radix(&color_str[1..3], 16).unwrap() as f32,
        u8::from_str_radix(&color_str[3..5], 16).unwrap() as f32,
        u8::from_str_radix(&color_str[5..7], 16).unwrap() as f32,
    )
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
