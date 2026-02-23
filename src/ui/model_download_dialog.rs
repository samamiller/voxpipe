use gtk::prelude::*;

use crate::asr;

struct ModelEntry {
    name: &'static str,
    file: &'static str,
    size: &'static str,
}

const MODELS: &[ModelEntry] = &[
    ModelEntry {
        name: "tiny.en (q5_1)",
        file: "ggml-tiny.en-q5_1.bin",
        size: "~31 MB",
    },
    ModelEntry {
        name: "base.en (q5_1)",
        file: "ggml-base.en-q5_1.bin",
        size: "~78 MB",
    },
    ModelEntry {
        name: "small.en (q5_1)",
        file: "ggml-small.en-q5_1.bin",
        size: "~255 MB",
    },
    ModelEntry {
        name: "medium.en (q5_1)",
        file: "ggml-medium.en-q5_1.bin",
        size: "~850 MB",
    },
];

pub fn show_model_download_dialog(parent: &impl IsA<gtk::Window>) {
    let dialog = gtk::Dialog::builder()
        .title("Download models")
        .transient_for(parent)
        .modal(true)
        .build();
    dialog.add_button("Close", gtk::ResponseType::Close);
    dialog.connect_response(|dialog, _| dialog.close());

    let content = dialog.content_area();
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);

    for entry in MODELS {
        let row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(8)
            .margin_end(8)
            .build();

        let title = gtk::Label::builder()
            .label(entry.name)
            .xalign(0.0)
            .hexpand(true)
            .build();
        let size = gtk::Label::builder()
            .label(entry.size)
            .xalign(1.0)
            .build();
        let installed = is_installed(entry.file);
        let status_label = gtk::Label::builder()
            .xalign(0.0)
            .label(if installed {
                "Installed"
            } else {
                "Not installed"
            })
            .build();
        let action_button = gtk::Button::builder()
            .label(if installed {
                "Installed"
            } else {
                "Download"
            })
            .build();
        if installed {
            action_button.set_sensitive(false);
        }

        let status_label_clone = status_label.clone();
        let action_button_clone = action_button.clone();
        action_button.connect_clicked(move |_| {
            status_label_clone.set_label("Queued");
            action_button_clone.set_sensitive(false);
        });

        row.append(&title);
        row.append(&size);
        row.append(&status_label);
        row.append(&action_button);

        let list_row = gtk::ListBoxRow::new();
        list_row.set_child(Some(&row));
        list.append(&list_row);
    }

    content.append(&list);
    dialog.show();
}

fn is_installed(file: &str) -> bool {
    asr::model_search_dirs()
        .iter()
        .any(|dir| dir.join(file).is_file())
}
