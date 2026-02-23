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

    pub fn append_confidence_block(&self, header: &str, segments: &[Vec<(String, f32)>]) {
        self.append_line(header);
        for segment in segments {
            for (text, confidence) in segment {
                let tag = self.tag_for_confidence(*confidence);
                self.insert_with_tag(text, tag.as_ref());
            }
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

    fn insert_with_tag(&self, text: &str, tag: Option<&gtk::TextTag>) {
        if text.is_empty() {
            return;
        }
        let mut iter = self.buffer.end_iter();
        if let Some(tag) = tag {
            self.buffer.insert_with_tags(&mut iter, text, &[tag]);
        } else {
            self.buffer.insert(&mut iter, text);
        }
        self.scroll_to_end();
    }

    fn tag_for_confidence(&self, confidence: f32) -> Option<gtk::TextTag> {
        let (name, color) = if confidence < 0.33 {
            ("vox-confidence-low", "#f06b6b")
        } else if confidence < 0.66 {
            ("vox-confidence-mid", "#f6c35c")
        } else {
            ("vox-confidence-high", "#9ad57d")
        };
        Some(self.ensure_tag(name, color))
    }

    fn ensure_tag(&self, name: &str, color: &str) -> gtk::TextTag {
        if let Some(tag) = self.buffer.tag_table().lookup(name) {
            return tag;
        }
        let tag = gtk::TextTag::new(Some(name));
        tag.set_property("foreground", &color);
        self.buffer.tag_table().add(&tag);
        tag
    }

    fn scroll_to_end(&self) {
        let mut iter = self.buffer.end_iter();
        self.buffer.place_cursor(&iter);
        self.view
            .scroll_to_iter(&mut iter, 0.0, false, 0.0, 1.0);
    }
}
