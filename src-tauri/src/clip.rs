// Clipboard snapshot/restore around our own cut and paste, so the user's clipboard survives.

use arboard::{Clipboard, ImageData};

pub enum Saved {
    Text(String),
    Image(ImageData<'static>),
    Empty,
}

pub fn snapshot(cb: &mut Clipboard) -> Saved {
    if let Ok(t) = cb.get_text() {
        return Saved::Text(t);
    }
    if let Ok(img) = cb.get_image() {
        return Saved::Image(img.to_owned_img());
    }
    Saved::Empty
}

pub fn restore(cb: &mut Clipboard, saved: Saved) {
    let _ = match saved {
        Saved::Text(t) => cb.set_text(t),
        Saved::Image(img) => cb.set_image(img),
        Saved::Empty => cb.clear(),
    };
}
