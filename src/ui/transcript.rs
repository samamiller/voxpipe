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

    fn scroll_to_end(&self) {
        let mut iter = self.buffer.end_iter();
        self.buffer.place_cursor(&iter);
        self.view
            .scroll_to_iter(&mut iter, 0.0, false, 0.0, 1.0);
    }
}
