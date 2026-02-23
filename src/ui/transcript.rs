use gtk::gdk;
use gtk::prelude::*;

#[derive(Clone)]
pub struct Transcript {
    buffer: gtk::TextBuffer,
    view: gtk::TextView,
}

impl Transcript {
    pub fn new() -> Self {
        let buffer = gtk::TextBuffer::new(None);
        let view = gtk::TextView::builder()
            .buffer(&buffer)
            .editable(false)
            .cursor_visible(false)
            .wrap_mode(gtk::WrapMode::WordChar)
            .build();
        view.add_css_class("hud-transcript");

        Self { buffer, view }
    }

    pub fn view(&self) -> gtk::TextView {
        self.view.clone()
    }

    pub fn append_line(&self, text: &str) {
        self.append_text(text);
        self.append_text("\n");
    }

    pub fn append_block(&self, header: &str, body: &str) {
        self.append_line(header);
        if !body.is_empty() {
            self.append_text(body);
            self.append_text("\n");
        }
        self.append_text("\n");
    }

    #[allow(dead_code)]
    pub fn append_error(&self, err: &str) {
        self.append_block("Error", err);
    }

    pub fn append_ansi_block(&self, header: &str, body: &str) {
        self.append_line(header);
        if !body.is_empty() {
            self.append_ansi_text(body);
            self.append_text("\n");
        }
        self.append_text("\n");
    }

    #[allow(dead_code)]
    pub fn clear(&self) {
        self.buffer.set_text("");
    }

    pub fn text(&self) -> String {
        let start = self.buffer.start_iter();
        let end = self.buffer.end_iter();
        self.buffer.text(&start, &end, true).to_string()
    }

    fn append_text(&self, text: &str) {
        let mut iter = self.buffer.end_iter();
        self.buffer.insert(&mut iter, text);
        self.scroll_to_end();
    }

    fn append_ansi_text(&self, text: &str) {
        let mut iter = self.buffer.end_iter();
        let mut current_tag: Option<gtk::TextTag> = None;
        let mut start = 0usize;
        let bytes = text.as_bytes();
        let len = bytes.len();

        while start < len {
            let mut idx = start;
            while idx < len && bytes[idx] != 0x1b {
                idx += 1;
            }

            if idx > start {
                self.insert_with_tag(&mut iter, &text[start..idx], current_tag.as_ref());
            }

            if idx >= len {
                break;
            }

            let remainder = &text[idx..];
            if remainder.starts_with("\u{1b}[0m") {
                current_tag = None;
                start = idx + 4;
                continue;
            }

            if remainder.starts_with("\u{1b}[38;5;") {
                if let Some(end_idx) = remainder.find('m') {
                    let code_text = &remainder[7..end_idx];
                    if let Ok(code) = code_text.parse::<u16>() {
                        let code = code.min(255) as u8;
                        current_tag = Some(self.tag_for_color(code));
                    } else {
                        current_tag = None;
                    }
                    start = idx + end_idx + 1;
                    continue;
                }
            }

            start = idx + 1;
        }

        self.scroll_to_end();
    }

    fn insert_with_tag(
        &self,
        iter: &mut gtk::TextIter,
        text: &str,
        tag: Option<&gtk::TextTag>,
    ) {
        if text.is_empty() {
            return;
        }
        if let Some(tag) = tag {
            self.buffer.insert_with_tags(iter, text, &[tag]);
        } else {
            self.buffer.insert(iter, text);
        }
    }

    fn tag_for_color(&self, code: u8) -> gtk::TextTag {
        let tag_name = format!("vox-color-{code}");
        if let Some(tag) = self.buffer.tag_table().lookup(&tag_name) {
            return tag;
        }

        let (r, g, b) = xterm_256_to_rgb(code);
        let rgba = gdk::RGBA::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            1.0,
        );
        let tag = match self.buffer.create_tag(Some(&tag_name), &[]) {
            Some(tag) => tag,
            None => {
                let tag = gtk::TextTag::new(Some(&tag_name));
                self.buffer.tag_table().add(&tag);
                tag
            }
        };
        tag.set_property("foreground-rgba", &rgba);
        tag
    }

    fn scroll_to_end(&self) {
        let mut iter = self.buffer.end_iter();
        self.buffer.place_cursor(&iter);
        self.view
            .scroll_to_iter(&mut iter, 0.0, false, 0.0, 1.0);
    }
}

fn xterm_256_to_rgb(code: u8) -> (u8, u8, u8) {
    match code {
        0 => (0x00, 0x00, 0x00),
        1 => (0x80, 0x00, 0x00),
        2 => (0x00, 0x80, 0x00),
        3 => (0x80, 0x80, 0x00),
        4 => (0x00, 0x00, 0x80),
        5 => (0x80, 0x00, 0x80),
        6 => (0x00, 0x80, 0x80),
        7 => (0xc0, 0xc0, 0xc0),
        8 => (0x80, 0x80, 0x80),
        9 => (0xff, 0x00, 0x00),
        10 => (0x00, 0xff, 0x00),
        11 => (0xff, 0xff, 0x00),
        12 => (0x00, 0x00, 0xff),
        13 => (0xff, 0x00, 0xff),
        14 => (0x00, 0xff, 0xff),
        15 => (0xff, 0xff, 0xff),
        16..=231 => {
            let idx = code - 16;
            let r = idx / 36;
            let g = (idx / 6) % 6;
            let b = idx % 6;
            let map = |v| if v == 0 { 0 } else { 55 + v * 40 };
            (map(r), map(g), map(b))
        }
        232..=255 => {
            let gray = 8 + (code - 232) * 10;
            (gray, gray, gray)
        }
    }
}
