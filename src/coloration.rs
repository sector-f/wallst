extern crate clap;
extern crate picto;

use clap::OsValues;
use picto::buffer::{self, Buffer, Rgba as RgbaImage};
use picto::color::{Gradient, Rgb, Rgba};
use picto::Orientation::{Horizontal, Vertical};


pub fn get_background<'a>(colors: Option<OsValues<'a>>,
                  w: u32,
                  h: u32) -> RgbaImage {
    let colors_vec =
        colors.iter().flat_map(|c| c.clone().into_iter())
        .map(|c| color_from_str(&c.to_string_lossy()))
        .collect::<Vec<_>>();

    match colors_vec.len() {
        0 => {
            let black = Rgba::new_u8(0, 0, 0, 0);
            Buffer::from_pixel(w, h, &black)
        },
        1 => {
            Buffer::from_pixel(w, h, &colors_vec[0])
        },
        _ => {
            let bg_gradient = Gradient::new(colors_vec);
            Buffer::from_gradient(w, h, Horizontal, bg_gradient)
        }
    }
}

fn color_from_str(color_str: &str) -> Rgba {
    let red = u8::from_str_radix(&color_str[1..3], 16).unwrap();
    let green = u8::from_str_radix(&color_str[3..5], 16).unwrap();
    let blue = u8::from_str_radix(&color_str[5..7], 16).unwrap();
    Rgba::new_u8(red, green, blue, 255)
}
