extern crate image;
extern crate xcb;
extern crate xcb_util as xcbu;

use image::{GenericImage, Pixel};
use std::env::args_os;
use std::mem;

fn get_screen(conn: &xcb::Connection, display: usize) -> xcb::Screen {
    let setup = conn.get_setup();
    let mut screen_iter = setup.roots();
    let screen = screen_iter.nth(display).expect("Failed to get screen info");
    return screen
}

fn main() {
    let (conn, screen_num) = xcb::Connection::connect(None).expect("Failed to connect to X server");
    let screen = get_screen(&conn, screen_num as usize);
    let w = screen.width_in_pixels();
    let h = screen.height_in_pixels();

    let filename = args_os().nth(1).expect("No filename specified");
    let image = image::open(&filename).expect("Failed to open image");

    let mut shm = xcbu::image::shm::create(&conn, screen.root_depth(), w, h)
        .expect("Failed to create shm");

    for (x, y, pixel) in image.pixels() {
        shm.put(x, y,
                ((pixel[0] as u32) << 16) |
                ((pixel[1] as u32) << 8) |
                ((pixel[2] as u32) << 0));
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

    xcb::kill_client(&conn, xcb::KILL_ALL_TEMPORARY);
    xcb::set_close_down_mode(&conn, xcb::CLOSE_DOWN_RETAIN_TEMPORARY as u8);
    xcb::change_window_attributes(&conn, screen.root(), &[(xcb::CW_BACK_PIXMAP, pixmap_id)]);
    xcb::clear_area(&conn, false, screen.root(), 0, 0, w, h);
    conn.flush();
}
