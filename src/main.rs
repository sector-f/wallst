extern crate clap;
extern crate image;
extern crate xcb;
extern crate xcb_util as xcbu;

mod xorg;

use clap::{App, Arg};
use image::*;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, stderr, stdin, Write};
use std::path::Path;
use std::u8;
use xorg::*;

// #[derive(Clone, Copy)]
struct BackgroundOptions<'a> {
    path: Option<&'a Path>,
    color: Option<&'a str>,
    alpha: Option<u8>,
    w: u32,
    h: u32,
    mode: BackgroundMode,
    vflip: bool,
    hflip: bool,
    save_path: Option<&'a Path>,
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

    Tile,    // Put image in top-left of screen.
             // Repeat left-to-right if it is too small.
}

fn get_image_data(bg: BackgroundOptions) -> Result<DynamicImage, ImageError> {
    let bg_color = bg.color.unwrap_or("#000000");
    let mut image = get_solid_image(bg_color, bg.w, bg.h);

    if let Some(ref path) = bg.path {
        let mut buffer = Vec::new();
        let mut foreground =
            if path.as_os_str() == "-" {
                let _ = stdin().read_to_end(&mut buffer);
                try!(load_from_memory(&buffer))
            } else {
                let mut fin = match File::open(path) {
                    Ok(f) => f,
                    Err(e) => return Err(ImageError::IoError(e))
                };
                let _ = fin.read_to_end(&mut buffer);
                try!(load_from_memory(&buffer))
            };
        foreground = DynamicImage::ImageRgba8(foreground.to_rgba());

        let mut buffer = DynamicImage::new_rgba8(bg.w, bg.h);
        // let mut x_offset = 0;
        // let mut y_offset = 0;

        match bg.mode {
            BackgroundMode::Center => {
                // let img_w = image.width();
                // let img_h = image.height();

                // let bg_color = bg.color.unwrap_or("#000000");
                // let mut bg_image = get_solid_image(&bg_color, bg.w, bg.h);

                // let left: i32 = (bg.w as i32 - img_w as i32) / 2;
                // let top: i32 = (bg.h as i32 - img_h as i32) / 2;

                // let mut image_copy = image;
                // let sub_image = image_copy.sub_image(
                //     if left < 0 { left.abs() as u32 } else { 0 },
                //     if top < 0 { top.abs() as u32 } else { 0 },
                //     if left < 0 { bg.w } else { img_w },
                //     if top < 0 { bg.h } else { img_h },
                // );

                // bg_image.copy_from(&sub_image,
                //                    if left < 0 { 0 } else { left.abs() as u32 },
                //                    if top < 0 { 0 } else { top.abs() as u32 });
                // image = bg_image;
            },
            BackgroundMode::Stretch => {
                foreground = foreground.resize_exact(bg.w, bg.h, FilterType::Lanczos3);
            },
            BackgroundMode::Fill => {
                // let bg_color = bg.color.unwrap_or("#000000");
                // let mut bg_image = get_solid_image(&bg_color, bg.w, bg.h);

                // image = image.resize(bg.w, bg.h, FilterType::Lanczos3);
                // let offset = (bg.w - image.width()) / 2;
                // bg_image.copy_from(&image, offset, 0);
                // image = bg_image;
            },
            BackgroundMode::Full => {
                // let bg_color = bg.color.unwrap_or("#000000");
                // let mut bg_image = get_solid_image(&bg_color, bg.w, bg.h);

                // bg_image.copy_from(&image, 0, 0);
                // image = bg_image;
            },
            BackgroundMode::Tile => {
                // // To-Do: Use a SubImage rather than increasing the bg size
                // let mut bg_image = get_solid_image("#000000",
                //                                    bg.w + (image.width() % bg.w),
                //                                    bg.h + (image.height() % bg.h));

                // let mut vert_overlap = 0;
                // while vert_overlap < bg.h {
                //     let mut horiz_overlap = 0;
                //     while horiz_overlap < bg.w {
                //         bg_image.copy_from(&image, horiz_overlap, vert_overlap);
                //         horiz_overlap += image.width();
                //     }
                //     vert_overlap += image.height();
                // }

                // image = bg_image.crop(0, 0, bg.w, bg.h);
            },
        }

        buffer.copy_from(&foreground, 0, 0);

        if let Some(alpha) = bg.alpha {
            for pixel in buffer.as_mut_rgba8().unwrap().pixels_mut() {
                pixel[3] = alpha;
            }
        }

        let zipped = image.as_mut_rgba8().unwrap().pixels_mut().zip(buffer.as_rgba8().unwrap().pixels());

        for (bg_pix, fg_pix) in zipped {
            bg_pix.blend(fg_pix);
        }

        // And now I need to blend them. I think.

    }

    Ok(image)
}

fn _get_image_data(bg: BackgroundOptions) -> Result<DynamicImage, ImageError> {
    let mut image =
        match bg.path {
            Some(ref path) => {
                let mut buffer = Vec::new();
                if path.as_os_str() == "-" {
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

    image = DynamicImage::ImageRgba8(image.to_rgba());
    if let Some(alpha) = bg.alpha {
        for pixel in image.as_mut_rgba8().unwrap().pixels_mut() {
            pixel[3] = (alpha as f32 * 255f32) as u8;
        }
    }

    match bg.mode {
        BackgroundMode::Center => {
            let img_w = image.width();
            let img_h = image.height();

            let bg_color = bg.color.unwrap_or("#000000");
            let mut bg_image = get_solid_image(&bg_color, bg.w, bg.h);

            let left: i32 = (bg.w as i32 - img_w as i32) / 2;
            let top: i32 = (bg.h as i32 - img_h as i32) / 2;

            let mut image_copy = image;
            let sub_image = image_copy.sub_image(
                if left < 0 { left.abs() as u32 } else { 0 },
                if top < 0 { top.abs() as u32 } else { 0 },
                if left < 0 { bg.w } else { img_w },
                if top < 0 { bg.h } else { img_h },
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
            let bg_color = bg.color.unwrap_or("#000000");
            let mut bg_image = get_solid_image(&bg_color, bg.w, bg.h);

            image = image.resize(bg.w, bg.h, FilterType::Lanczos3);
            let offset = (bg.w - image.width()) / 2;
            bg_image.copy_from(&image, offset, 0);
            image = bg_image;
        },
        BackgroundMode::Full => {
            let bg_color = bg.color.unwrap_or("#000000");
            let mut bg_image = get_solid_image(&bg_color, bg.w, bg.h);

            bg_image.copy_from(&image, 0, 0);
            image = bg_image;
        },
        BackgroundMode::Tile => {
            // To-Do: Use a SubImage rather than increasing the bg size
            let mut bg_image = get_solid_image("#000000",
                                               bg.w + (image.width() % bg.w),
                                               bg.h + (image.height() % bg.h));

            let mut vert_overlap = 0;
            while vert_overlap < bg.h {
                let mut horiz_overlap = 0;
                while horiz_overlap < bg.w {
                    bg_image.copy_from(&image, horiz_overlap, vert_overlap);
                    horiz_overlap += image.width();
                }
                vert_overlap += image.height();
            }

            image = bg_image.crop(0, 0, bg.w, bg.h);
        },
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

fn get_solid_image(color_str: &str, w: u32, h:u32) -> DynamicImage {
    let (r, g, b) = (
        u8::from_str_radix(&color_str[1..3], 16).unwrap(),
        u8::from_str_radix(&color_str[3..5], 16).unwrap(),
        u8::from_str_radix(&color_str[5..7], 16).unwrap(),
    );

    let color = Rgba::from_channels(r, g, b, 255);
    DynamicImage::ImageRgba8(ImageBuffer::from_pixel(w, h, color))
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

    let color = matches.value_of("color");
    let alpha = matches.value_of("alpha").map(|a| a.parse::<u8>().unwrap());

    let bg_options = BackgroundOptions {
        path: path,
        color: color,
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
