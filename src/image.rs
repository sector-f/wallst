extern crate picto;

use picto::buffer::{self, Buffer, Rgba as RgbaImage};
use picto::color::{Alpha, Gradient, Rgb, Rgba};
use std::path::Path;
use std::fs::File;
use std::io::{stderr, Write};

pub fn save_image(path: &Path, image: &RgbaImage) {
    match File::create(&path) {
        Ok(mut file) => {
            if let Err(e) = picto::write::png(&file, &image, |_|{()}) {
                let _ = writeln!(stderr(), "Error saving image: {}", e);
           }
        },
        Err(e) => {
            let _ = writeln!(stderr(), "Failed to save image: {}", e);
        },
    }
}

pub fn center_image(bg: &mut RgbaImage, fg: &mut RgbaImage, w: u32, h: u32) {
    unimplemented!();
}

pub fn stretch_image(bg: &mut RgbaImage, fg: &mut RgbaImage, w: u32, h: u32) {
    unimplemented!();
}

pub fn fill_image(bg: &mut RgbaImage, fg: &mut RgbaImage, w: u32, h: u32) {
    unimplemented!();
}

pub fn full_image(bg: &mut RgbaImage, fg: &mut RgbaImage, w: u32, h: u32) {
    unimplemented!();
}

pub fn tile_image(bg: &mut RgbaImage, fg: &mut RgbaImage, w: u32, h: u32) {
    unimplemented!();
}
