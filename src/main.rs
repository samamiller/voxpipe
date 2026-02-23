use adw::prelude::*;
use gtk::gio;
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    sync::mpsc,
    time::{SystemTime, UNIX_EPOCH},
};

mod asr;
mod audio;
mod models;
mod state;
mod ui;
pub mod whisper_rs;
mod whisper_sys;

use audio::record::Recorder;
use state::{AppEvent, AppState};
use ui::transcript::Transcript;
use whisper_rs::WhisperContext;

const APP_STYLE: &str = include_str!("../assets/style.css");

fn segments_to_text(segments: &[asr::ConfidenceSegment]) -> String {
    segments
        .iter()
        .map(|segment| segment.text.trim_end())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id("io.voxpipe.App")
        .build();

    app.connect_activate(|app| {
        let system_info = whisper_rs::system_info();
        eprintln!("[whisper] system_info: {system_info}");

        let css_provider = gtk::CssProvider::new();
        css_provider.load_from_data(APP_STYLE);
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &css_provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Voxpipe")
            .default_width(420)
            .default_height(360)
            .decorated(true)
            .modal(false)
            .build();
        window.add_css_class("hud-window");

        let panel = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(10)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(12)
            .margin_end(12)
            .build();
        panel.add_css_class("hud-body");

        let top_bar = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .hexpand(true)
            .build();
        top_bar.add_css_class("hud-topbar");

        let file_button = gtk::Button::builder()
            .icon_name("audio-x-generic-symbolic")
            .tooltip_text("Transcribe file")
            .build();
        file_button.add_css_class("hud-control");

        let mic_button = gtk::Button::builder()
            .icon_name("audio-input-microphone-symbolic")
            .tooltip_text("Start listening")
            .build();
        mic_button.add_css_class("hud-mic");

        let status_label = gtk::Label::builder()
            .xalign(0.0)
            .hexpand(false)
            .label("Status: Idle")
            .build();
        status_label.add_css_class("hud-status");

        let model_menu = gio::Menu::new();
        model_menu.append(Some("Change model…"), Some("app.change_model"));
        model_menu.append(Some("Download models…"), Some("app.download_models"));
        model_menu.append(Some("Open models folder"), Some("app.open_models_folder"));

        let model_menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&model_menu)
            .build();
        model_menu_button.add_css_class("hud-control");

        let model_info_button = gtk::Button::builder()
            .icon_name("dialog-information-symbolic")
            .tooltip_text("Model info")
            .build();
        model_info_button.add_css_class("hud-control");

        let model_info_popover = gtk::Popover::new();
        model_info_popover.set_has_arrow(true);
        model_info_popover.set_parent(&model_info_button);

        let model_info_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(12)
            .margin_end(12)
            .build();
        let model_info_name = gtk::Label::builder().xalign(0.0).build();
        let model_info_path = gtk::Label::builder().xalign(0.0).build();
        let model_info_size = gtk::Label::builder().xalign(0.0).build();
        let model_info_sys = gtk::Label::builder().xalign(0.0).wrap(true).build();
        model_info_box.append(&model_info_name);
        model_info_box.append(&model_info_path);
        model_info_box.append(&model_info_size);
        model_info_box.append(&model_info_sys);
        model_info_popover.set_child(Some(&model_info_box));


        let right_controls = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(4)
            .build();
        right_controls.add_css_class("hud-controls");
        let debug_button = gtk::Button::builder()
            .icon_name("document-edit-symbolic")
            .tooltip_text("Append test transcript")
            .build();
        debug_button.add_css_class("hud-control");

        let minimize_button = gtk::Button::builder()
            .icon_name("window-minimize-symbolic")
            .tooltip_text("Minimize")
            .build();
        minimize_button.add_css_class("hud-control");
        let close_button = gtk::Button::builder()
            .icon_name("window-close-symbolic")
            .tooltip_text("Close")
            .build();
        close_button.add_css_class("hud-control");
        right_controls.append(&debug_button);
        right_controls.append(&minimize_button);
        right_controls.append(&close_button);

        top_bar.append(&model_menu_button);
        top_bar.append(&model_info_button);
        top_bar.append(&file_button);
        top_bar.append(&mic_button);
        top_bar.append(&status_label);

        let top_spacer = gtk::Box::builder()
            .hexpand(true)
            .halign(gtk::Align::Fill)
            .build();
        top_bar.append(&top_spacer);
        top_bar.append(&right_controls);

        let transcript = Transcript::new();
        let transcript_view = transcript.view();
        let confidence_colors = Rc::new(Cell::new(false));

        let transcript_copy_button = gtk::Button::builder()
            .icon_name("edit-copy-symbolic")
            .tooltip_text("Copy all")
            .action_name("app.transcript_copy")
            .build();
        transcript_copy_button.add_css_class("hud-control");

        let transcript_confidence_toggle = gtk::ToggleButton::builder()
            .icon_name("format-text-color-symbolic")
            .tooltip_text("Confidence colors")
            .build();
        transcript_confidence_toggle.add_css_class("hud-control");
        {
            let confidence_colors = Rc::clone(&confidence_colors);
            transcript_confidence_toggle.connect_toggled(move |toggle| {
                confidence_colors.set(toggle.is_active());
            });
        }

        let transcript_clear_button = gtk::Button::builder()
            .icon_name("edit-clear-symbolic")
            .tooltip_text("Clear")
            .action_name("app.transcript_clear")
            .build();
        transcript_clear_button.add_css_class("hud-control");

        let transcript_header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .build();
        transcript_header.add_css_class("hud-transcript-header");
        let transcript_title = gtk::Label::builder()
            .label("Transcript")
            .xalign(0.0)
            .hexpand(true)
            .build();
        transcript_title.add_css_class("hud-transcript-title");
        transcript_header.append(&transcript_title);
        transcript_header.append(&transcript_confidence_toggle);
        transcript_header.append(&transcript_copy_button);
        transcript_header.append(&transcript_clear_button);

        let transcript_scroll = gtk::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&transcript_view)
            .build();
        transcript_scroll.add_css_class("hud-transcript-scroll");

        panel.append(&top_bar);
        panel.append(&transcript_header);
        panel.append(&transcript_scroll);

        {
            let model_info_popover = model_info_popover.clone();
            let model_info_name = model_info_name.clone();
            let model_info_path = model_info_path.clone();
            let model_info_size = model_info_size.clone();
            let model_info_sys = model_info_sys.clone();
            model_info_button.connect_clicked(move |_| {
                update_model_info(
                    &model_info_name,
                    &model_info_path,
                    &model_info_size,
                    &model_info_sys,
                );
                model_info_popover.popup();
            });
        }

        {
            let window = window.clone();
            let action = gio::SimpleAction::new("change_model", None);
            action.connect_activate(move |_, _| {
                let dialog = gtk::FileChooserNative::builder()
                    .title("Select a Whisper model")
                    .accept_label("Use Model")
                    .modal(true)
                    .build();
                dialog.set_transient_for(Some(&window));
                dialog.set_action(gtk::FileChooserAction::Open);

                if let Some(dir) = asr::model_default_dir() {
                    let _ = dialog.set_current_folder(Some(&gio::File::for_path(dir)));
                }

                let filter = gtk::FileFilter::new();
                filter.set_name(Some("Whisper model (*.bin)"));
                filter.add_pattern("*.bin");
                dialog.add_filter(&filter);
                dialog.set_filter(&filter);

                dialog.connect_response(move |dialog, response| {
                    if response != gtk::ResponseType::Accept {
                        dialog.destroy();
                        return;
                    }

                    let Some(file) = dialog.file() else {
                        dialog.destroy();
                        return;
                    };

                    let Some(path) = file.path() else {
                        dialog.destroy();
                        return;
                    };

                    unsafe {
                        std::env::set_var("ASR_MODEL", &path);
                    }
                    dialog.destroy();
                });

                dialog.show();
            });
            app.add_action(&action);
        }

        {
            let window = window.clone();
            let action = gio::SimpleAction::new("download_models", None);
            action.connect_activate(move |_, _| {
                ui::model_download_dialog::show_model_download_dialog(&window);
            });
            app.add_action(&action);
        }

        {
            let action = gio::SimpleAction::new("open_models_folder", None);
            action.connect_activate(move |_, _| {
                let dir = asr::cache_models_dir();
                let uri = gio::File::for_path(&dir).uri();
                if let Err(err) = gio::AppInfo::launch_default_for_uri(
                    &uri,
                    gio::AppLaunchContext::NONE,
                ) {
                    eprintln!("[ui] failed to open models folder: {err}");
                }
            });
            app.add_action(&action);
        }


        {
            let transcript = transcript.clone();
            let action = gio::SimpleAction::new("transcript_clear", None);
            action.connect_activate(move |_, _| {
                transcript.clear();
            });
            app.add_action(&action);
        }

        {
            let transcript = transcript.clone();
            let action = gio::SimpleAction::new("transcript_copy", None);
            action.connect_activate(move |_, _| {
                let text = transcript.text();
                if let Some(display) = gtk::gdk::Display::default() {
                    display.clipboard().set_text(&text);
                }
            });
            app.add_action(&action);
        }

        let drag_gesture = gtk::GestureClick::builder().button(0).build();
        {
            let top_bar = top_bar.clone();
            let window = window.clone();
            drag_gesture.connect_pressed(move |gesture, _, x, y| {
                let button = gesture.current_button() as i32;
                let timestamp = gesture.current_event_time();
                let on_control = top_bar
                    .pick(x, y, gtk::PickFlags::DEFAULT)
                    .map(|widget| {
                        widget.is::<gtk::Button>()
                            || widget
                                .ancestor(gtk::Button::static_type())
                                .is_some()
                    })
                    .unwrap_or(false);

                if on_control {
                    return;
                }

                if let (Some(device), Some(surface)) =
                    (gesture.current_event_device(), window.surface())
                {
                    if let Ok(toplevel) = surface.dynamic_cast::<gtk::gdk::Toplevel>() {
                        toplevel.begin_move(&device, button, x, y, timestamp);
                    }
                }
            });
        }
        top_bar.add_controller(drag_gesture);
        window.set_content(Some(&panel));

        let state = Rc::new(RefCell::new(AppState::Idle));
        let recorder = Rc::new(RefCell::new(Recorder::new()));
        apply_state(&state.borrow(), &status_label, &mic_button, &file_button, &panel);

        {
            let state = Rc::clone(&state);
            let recorder = Rc::clone(&recorder);
            let status_label = status_label.clone();
            let mic_button = mic_button.clone();
            let mic_button_handler = mic_button.clone();
            let panel = panel.clone();
            let transcript = transcript.clone();
            let file_button_handler = file_button.clone();
            let confidence_colors = Rc::clone(&confidence_colors);

            mic_button.connect_clicked(move |_| {
                let current_state = state.borrow().clone();
                match current_state {
                    AppState::Listening => {
                    let maybe_path = recorder.borrow_mut().stop();
                    let wav_path = if let Some(path) = maybe_path {
                        eprintln!("[audio] saved recording: {}", path.display());
                        path
                    } else {
                        status_label.set_label("Status: Recording stop failed (no WAV path)");
                        return;
                    };

                    let next = state.borrow().on_event(AppEvent::StopListening);
                    set_state(
                        &state,
                        next,
                        &status_label,
                        &mic_button_handler,
                        &file_button_handler,
                        &panel,
                    );

                    let model_path = match asr::discover_model_path() {
                        Ok(path) => path,
                        Err(err) => {
                            status_label.set_label(&format!("Status: {err}"));
                            eprintln!("[asr] model discovery failed: {err}");
                            let idle = state.borrow().on_event(AppEvent::TranscriptionReady);
                            set_state(
                                &state,
                                idle,
                                &status_label,
                                &mic_button_handler,
                                &file_button_handler,
                                &panel,
                            );
                            return;
                        }
                    };

                    let use_confidence_colors = confidence_colors.get();
                    let (tx, rx) = mpsc::channel::<Result<asr::ConfidenceSegments, String>>();
                    let handle = std::thread::spawn(move || {
                        let result = asr::transcribe_with_confidence(&wav_path, &model_path)
                            .map_err(|err| err.to_string());
                        let _ = tx.send(result);
                    });

                    let state_done = Rc::clone(&state);
                    let status_done = status_label.clone();
                    let mic_done = mic_button_handler.clone();
                    let file_done = file_button_handler.clone();
                    let panel_done = panel.clone();
                    let transcript_done = transcript.clone();
                    let use_confidence_colors = use_confidence_colors;
                    let mut handle = Some(handle);
                    glib::timeout_add_local(
                        std::time::Duration::from_millis(100),
                        move || match rx.try_recv() {
                            Ok(Ok(segments)) => {
                                if let Some(handle) = handle.take() {
                                    let _ = handle.join();
                                }
                                let text = segments_to_text(&segments);
                                eprintln!("[asr] transcript:\n{text}");
                                let timestamp = match glib::DateTime::now_local() {
                                    Ok(dt) => dt
                                        .format("%H:%M:%S")
                                        .map(|ts| ts.to_string())
                                        .unwrap_or_else(|_| "unknown".to_string()),
                                    Err(_) => "unknown".to_string(),
                                };
                                let header = format!("Mic ({timestamp})");
                                if use_confidence_colors {
                                    transcript_done.append_confidence_block(&header, &segments);
                                } else {
                                    transcript_done.append_block(&header, &text);
                                }
                                status_done.set_label("Status: Transcribed");
                                let idle =
                                    state_done.borrow().on_event(AppEvent::TranscriptionReady);
                                set_state(
                                    &state_done,
                                    idle,
                                    &status_done,
                                    &mic_done,
                                    &file_done,
                                    &panel_done,
                                );
                                glib::ControlFlow::Break
                            }
                            Ok(Err(err)) => {
                                if let Some(handle) = handle.take() {
                                    let _ = handle.join();
                                }
                                status_done.set_label(&format!("Status: ASR error ({err})"));
                                eprintln!("[asr] transcription failed: {err}");
                                transcript_done.append_error(&err);
                                let idle =
                                    state_done.borrow().on_event(AppEvent::TranscriptionReady);
                                set_state(
                                    &state_done,
                                    idle,
                                    &status_done,
                                    &mic_done,
                                    &file_done,
                                    &panel_done,
                                );
                                glib::ControlFlow::Break
                            }
                            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                            Err(mpsc::TryRecvError::Disconnected) => {
                                if let Some(handle) = handle.take() {
                                    let _ = handle.join();
                                }
                                status_done.set_label("Status: ASR worker disconnected");
                                let idle =
                                    state_done.borrow().on_event(AppEvent::TranscriptionReady);
                                set_state(
                                    &state_done,
                                    idle,
                                    &status_done,
                                    &mic_done,
                                    &file_done,
                                    &panel_done,
                                );
                                glib::ControlFlow::Break
                            }
                        },
                    );
                }
                    AppState::Idle => {
                        let wav_path = next_wav_path();
                        if let Err(err) = recorder.borrow_mut().start(&wav_path, move |level| {
                            let _ = level;
                        }) {
                            status_label.set_label(&format!("Status: Mic error ({err})"));
                            return;
                        }
                        let next = state.borrow().on_event(AppEvent::StartListening);
                        set_state(
                            &state,
                            next,
                            &status_label,
                            &mic_button_handler,
                            &file_button_handler,
                            &panel,
                        );
                    }
                    AppState::Transcribing | AppState::TranscribingFile { .. } => {
                        status_label.set_label("Status: Transcribing (please wait)");
                    }
                    AppState::Error(_) => {
                        let wav_path = next_wav_path();
                        if let Err(err) = recorder.borrow_mut().start(&wav_path, move |level| {
                            let _ = level;
                        }) {
                            status_label.set_label(&format!("Status: Mic error ({err})"));
                            return;
                        }
                        let next = state.borrow().on_event(AppEvent::StartListening);
                        set_state(
                            &state,
                            next,
                            &status_label,
                            &mic_button_handler,
                            &file_button_handler,
                            &panel,
                        );
                    }
                }
            });
        }

        {
            let window = window.clone();
            minimize_button.connect_clicked(move |_| {
                window.minimize();
            });
        }

        {
            let window = window.clone();
            let state = Rc::clone(&state);
            let status_label = status_label.clone();
            let mic_button_handler = mic_button.clone();
            let file_button_handler = file_button.clone();
            let panel = panel.clone();
            let transcript = transcript.clone();
            let confidence_colors = Rc::clone(&confidence_colors);

            file_button.connect_clicked(move |_| {
                let dialog = gtk::FileChooserNative::builder()
                    .title("Transcribe audio file")
                    .accept_label("Transcribe")
                    .modal(true)
                    .build();
                dialog.set_transient_for(Some(&window));
                dialog.set_action(gtk::FileChooserAction::Open);

                let ffmpeg_enabled = asr::ffmpeg_enabled();
                let filter = gtk::FileFilter::new();
                if ffmpeg_enabled {
                    filter.set_name(Some("Audio files"));
                    filter.add_mime_type("audio/*");
                    for pattern in [
                        "*.wav", "*.mp3", "*.flac", "*.m4a", "*.aac", "*.opus", "*.ogg", "*.oga",
                    ] {
                        filter.add_pattern(pattern);
                    }
                } else {
                    filter.set_name(Some("WAV audio"));
                    filter.add_pattern("*.wav");
                }
                dialog.add_filter(&filter);
                dialog.set_filter(&filter);

                let state = Rc::clone(&state);
                let status_label = status_label.clone();
                let mic_button_handler = mic_button_handler.clone();
                let file_button_handler = file_button_handler.clone();
                let panel = panel.clone();
                let transcript = transcript.clone();
                let ffmpeg_enabled = ffmpeg_enabled;
                let confidence_colors = Rc::clone(&confidence_colors);

                dialog.connect_response(move |dialog, response| {
                    if response != gtk::ResponseType::Accept {
                        dialog.destroy();
                        return;
                    }

                    let Some(file) = dialog.file() else {
                        status_label.set_label("Status: File path unavailable");
                        dialog.destroy();
                        return;
                    };

                    let Some(path) = file.path() else {
                        status_label.set_label("Status: File path unavailable");
                        dialog.destroy();
                        return;
                    };

                    let is_wav = path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext.eq_ignore_ascii_case("wav"))
                        .unwrap_or(false);
                    if !ffmpeg_enabled && !is_wav {
                        let err = "Unsupported audio format. Rebuild with VOXPIPE_WHISPER_FFMPEG=1 and install FFmpeg dev packages.";
                        status_label.set_label("Status: Unsupported audio format");
                        transcript.append_error(err);
                        dialog.destroy();
                        return;
                    }

                    let next =
                        state
                            .borrow()
                            .on_event(AppEvent::StartFileTranscription(path.clone()));
                    set_state(
                        &state,
                        next,
                        &status_label,
                        &mic_button_handler,
                        &file_button_handler,
                        &panel,
                    );

                    let model_path = match asr::discover_model_path() {
                        Ok(path) => path,
                        Err(err) => {
                            status_label.set_label(&format!("Status: {err}"));
                            eprintln!("[asr] model discovery failed: {err}");
                            let idle = state.borrow().on_event(AppEvent::TranscriptionReady);
                            set_state(
                                &state,
                                idle,
                                &status_label,
                                &mic_button_handler,
                                &file_button_handler,
                                &panel,
                            );
                            return;
                        }
                    };

                    let use_confidence_colors = confidence_colors.get();
                    let (tx, rx) = mpsc::channel::<Result<asr::ConfidenceSegments, String>>();
                    let path_for_worker = path.clone();
                    let handle = std::thread::spawn(move || {
                        let result = asr::transcribe_with_confidence(&path_for_worker, &model_path)
                            .map_err(|err| err.to_string());
                        let _ = tx.send(result);
                    });

                    let state_done = Rc::clone(&state);
                    let status_done = status_label.clone();
                    let mic_done = mic_button_handler.clone();
                    let file_done = file_button_handler.clone();
                    let panel_done = panel.clone();
                    let transcript_done = transcript.clone();
                    let path_for_header = path.clone();
                    let use_confidence_colors = use_confidence_colors;
                    let mut handle = Some(handle);
                    glib::timeout_add_local(
                        std::time::Duration::from_millis(100),
                        move || match rx.try_recv() {
                            Ok(Ok(segments)) => {
                                if let Some(handle) = handle.take() {
                                    let _ = handle.join();
                                }
                                let name = path_for_header
                                    .file_name()
                                    .and_then(|name| name.to_str())
                                    .unwrap_or("file");
                                let header = format!("File: {name}");
                                if use_confidence_colors {
                                    transcript_done.append_confidence_block(&header, &segments);
                                } else {
                                    let text = segments_to_text(&segments);
                                    transcript_done.append_block(&header, &text);
                                }
                                status_done.set_label("Status: Transcribed");
                                let idle =
                                    state_done.borrow().on_event(AppEvent::TranscriptionReady);
                                set_state(
                                    &state_done,
                                    idle,
                                    &status_done,
                                    &mic_done,
                                    &file_done,
                                    &panel_done,
                                );
                                glib::ControlFlow::Break
                            }
                            Ok(Err(err)) => {
                                if let Some(handle) = handle.take() {
                                    let _ = handle.join();
                                }
                                status_done.set_label(&format!("Status: ASR error ({err})"));
                                transcript_done.append_error(&err);
                                let idle =
                                    state_done.borrow().on_event(AppEvent::TranscriptionReady);
                                set_state(
                                    &state_done,
                                    idle,
                                    &status_done,
                                    &mic_done,
                                    &file_done,
                                    &panel_done,
                                );
                                glib::ControlFlow::Break
                            }
                            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                            Err(mpsc::TryRecvError::Disconnected) => {
                                if let Some(handle) = handle.take() {
                                    let _ = handle.join();
                                }
                                status_done.set_label("Status: ASR worker disconnected");
                                let idle =
                                    state_done.borrow().on_event(AppEvent::TranscriptionReady);
                                set_state(
                                    &state_done,
                                    idle,
                                    &status_done,
                                    &mic_done,
                                    &file_done,
                                    &panel_done,
                                );
                                glib::ControlFlow::Break
                            }
                        },
                    );
                    dialog.destroy();
                });

                dialog.show();
            });
        }

        {
            let transcript = transcript.clone();
            debug_button.connect_clicked(move |_| {
                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |dur| dur.as_secs());
                let header = format!("Debug {ts}");
                let body = format!("Test line at {ts}");
                transcript.append_block(&header, &body);
            });
        }

        {
            let window = window.clone();
            close_button.connect_clicked(move |_| {
                window.close();
            });
        }

        {
            let status_label = status_label.clone();
            let key_controller = gtk::EventControllerKey::new();
            key_controller.connect_key_pressed(move |_, key, _, state| {
                let ctrl_shift = state.contains(gtk::gdk::ModifierType::CONTROL_MASK)
                    && state.contains(gtk::gdk::ModifierType::SHIFT_MASK);
                let dev_shortcut = key == gtk::gdk::Key::W || key == gtk::gdk::Key::w;
                if !(ctrl_shift && dev_shortcut) {
                    return glib::Propagation::Proceed;
                }

                let model_path = match std::env::var("VOXPIPE_WHISPER_MODEL") {
                    Ok(path) if !path.trim().is_empty() => path,
                    _ => {
                        status_label
                            .set_label("Status: Whisper smoke failed (set VOXPIPE_WHISPER_MODEL)");
                        return glib::Propagation::Stop;
                    }
                };

                match WhisperContext::new(&model_path) {
                    Ok(ctx) => {
                        drop(ctx);
                        status_label.set_label("Status: Whisper smoke ok");
                        eprintln!("[whisper] smoke context init succeeded: {model_path}");
                    }
                    Err(err) => {
                        status_label.set_label(&format!("Status: Whisper smoke failed ({err})"));
                        eprintln!("[whisper] smoke context init failed: {err}");
                    }
                }

                glib::Propagation::Stop
            });
            window.add_controller(key_controller);
        }

        window.present();
    });

    app.run()
}

fn set_state(
    state: &Rc<RefCell<AppState>>,
    next: AppState,
    status_label: &gtk::Label,
    mic_button: &gtk::Button,
    file_button: &gtk::Button,
    container: &gtk::Box,
) {
    *state.borrow_mut() = next.clone();
    apply_state(&next, status_label, mic_button, file_button, container);
}

fn apply_state(
    state: &AppState,
    status_label: &gtk::Label,
    mic_button: &gtk::Button,
    file_button: &gtk::Button,
    container: &gtk::Box,
) {
    status_label.set_label(&format!("Status: {}", state.status_text()));
    mic_button.set_sensitive(state.mic_enabled());
    file_button.set_sensitive(state.file_enabled());
    if matches!(state, AppState::Listening) {
        mic_button.set_icon_name("media-playback-stop-symbolic");
        mic_button.set_tooltip_text(Some("Stop listening"));
        container.add_css_class("listening");
    } else {
        mic_button.set_icon_name("audio-input-microphone-symbolic");
        mic_button.set_tooltip_text(Some("Start listening"));
        container.remove_css_class("listening");
    }
}

fn next_wav_path() -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |dur| dur.as_millis());
    std::env::temp_dir().join(format!("voxpipe-{ts}.wav"))
}

fn update_model_info(
    name_label: &gtk::Label,
    path_label: &gtk::Label,
    size_label: &gtk::Label,
    sys_label: &gtk::Label,
) {
    match asr::discover_model_path() {
        Ok(path) => {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("model");
            name_label.set_label(&format!("Name: {name}"));
            path_label.set_label(&format!("Path: {}", path.display()));
            let size = std::fs::metadata(&path)
                .map(|meta| meta.len())
                .map(|bytes| format!("{bytes} bytes"))
                .unwrap_or_else(|_| "unknown".to_string());
            size_label.set_label(&format!("Size: {size}"));
            sys_label.set_label(&format!("Whisper: {}", whisper_rs::system_info()));
        }
        Err(err) => {
            name_label.set_label("Name: missing");
            path_label.set_label(&format!("Error: {err}"));
            size_label.set_label("Size: unavailable");
            sys_label.set_label("");
        }
    }
}
