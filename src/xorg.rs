use xcb_util as xcbu;

use image::{DynamicImage, GenericImage};

const ATOMS: &'static [&'static str] = &[
    "_XROOTPMAP_ID",
    "_XSETROOT_ID",
    "ESETROOT_PMAP_ID"
];

pub fn clean_root_atoms(conn: &xcb::Connection,
                        screen: &xcb::Screen) {
    let ids = ATOMS.iter().map(|atom| {
        let reply = xcb::get_property(&conn, false, screen.root(),
            xcb::intern_atom(&conn, false, atom).get_reply().expect("failed to intern atom").atom(),
            xcb::ATOM_PIXMAP, 0, 1).get_reply();

        match reply {
            Ok(ref reply) if reply.type_() == xcb::ATOM_PIXMAP => {
                Some(reply.value()[0])
            },
            _ => None,
        }
    }).collect::<Vec<Option<xcb::Pixmap>>>();

    if ids.iter().all(Option::is_some) && ids.iter().all(|id| id == ids.first().unwrap()) {
        xcb::kill_client(&conn, ids.first().unwrap().unwrap());
    }

    xcb::kill_client(&conn, xcb::KILL_ALL_TEMPORARY);
    xcb::set_close_down_mode(&conn, xcb::CLOSE_DOWN_RETAIN_TEMPORARY as u8);
}

pub fn set_background(conn: &xcb::Connection,
                      screen: &xcb::Screen,
                      image: &DynamicImage) {
    let w = screen.width_in_pixels();
    let h = screen.height_in_pixels();

    let mut shm = xcbu::image::shm::create(&conn, screen.root_depth(), w, h)
        .expect("Failed to create shm");

    for (x, y, pixel) in image.pixels() {
        let r = pixel[0] as u32;
        let g = pixel[1] as u32;
        let b = pixel[2] as u32;
        shm.put(x, y, (r << 16) | (g << 8) | (b << 0));
    }

    let pixmap_id = conn.generate_id();
    xcb::create_pixmap(&conn, screen.root_depth(), pixmap_id, screen.root(), w, h);

    let context = conn.generate_id();
    xcb::create_gc(&conn, context, pixmap_id, &[]);

    xcbu::image::shm::put(&conn, pixmap_id, context, &shm, 0, 0, 0, 0, w as u16, h as u16, false).expect("Failed to draw to pixmap");

    clean_root_atoms(&conn, &screen);
    for atom in ATOMS {
            xcb::change_property(&conn, xcb::PROP_MODE_REPLACE as u8, screen.root(),
                xcb::intern_atom(&conn, false, atom).get_reply().expect("failed to intern atom").atom(),
                xcb::ATOM_PIXMAP, 32, &[pixmap_id]);
    }

    xcb::kill_client(&conn, xcb::KILL_ALL_TEMPORARY);
    xcb::set_close_down_mode(&conn, xcb::CLOSE_DOWN_RETAIN_TEMPORARY as u8);
    xcb::change_window_attributes(&conn, screen.root(), &[(xcb::CW_BACK_PIXMAP, pixmap_id)]);
    xcb::clear_area(&conn, false, screen.root(), 0, 0, w, h);
    &conn.flush();
}

pub fn get_screen(conn: &xcb::Connection, display: usize) -> xcb::Screen {
    let setup = &conn.get_setup();
    let mut screen_iter = setup.roots();
    let screen = screen_iter.nth(display).expect("Failed to get screen info");
    return screen
}

