use adw::prelude::*;
use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    sync::mpsc,
    time::{SystemTime, UNIX_EPOCH},
};

mod asr;
mod audio;
mod state;
mod ui;
pub mod whisper_rs;
mod whisper_sys;

use audio::record::Recorder;
use state::{AppEvent, AppState};
use ui::transcript::Transcript;
use whisper_rs::WhisperContext;

const APP_STYLE: &str = include_str!("../assets/style.css");

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
            .decorated(false)
            .modal(false)
            .build();
        window.set_resizable(false);
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
            .build();
        top_bar.add_css_class("hud-topbar");

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

        let meter = gtk::ProgressBar::builder()
            .hexpand(true)
            .show_text(false)
            .fraction(0.0)
            .build();
        meter.add_css_class("hud-meter");

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

        top_bar.append(&mic_button);
        top_bar.append(&status_label);
        top_bar.append(&meter);
        top_bar.append(&right_controls);

        let transcript = Transcript::new();
        let transcript_view = transcript.view();

        let transcript_scroll = gtk::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&transcript_view)
            .build();
        transcript_scroll.add_css_class("hud-transcript-scroll");

        panel.append(&top_bar);
        panel.append(&transcript_scroll);

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
        apply_state(&state.borrow(), &status_label, &mic_button, &panel);

        {
            let state = Rc::clone(&state);
            let recorder = Rc::clone(&recorder);
            let status_label = status_label.clone();
            let meter = meter.clone();
            let mic_button = mic_button.clone();
            let mic_button_handler = mic_button.clone();
            let panel = panel.clone();
            let transcript = transcript.clone();

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

                    meter.set_fraction(0.0);
                    let next = state.borrow().on_event(AppEvent::StopListening);
                    set_state(&state, next, &status_label, &mic_button_handler, &panel);

                    let model_path = match asr::discover_model_path() {
                        Ok(path) => path,
                        Err(err) => {
                            status_label.set_label(&format!("Status: {err}"));
                            eprintln!("[asr] model discovery failed: {err}");
                            let idle = state.borrow().on_event(AppEvent::TranscriptionReady);
                            set_state(&state, idle, &status_label, &mic_button_handler, &panel);
                            return;
                        }
                    };

                    let (tx, rx) = mpsc::channel::<Result<String, String>>();
                    std::thread::spawn(move || {
                        let result = asr::transcribe(&wav_path, &model_path, &["-nt"])
                            .map_err(|err| err.to_string());
                        let _ = tx.send(result);
                    });

                    let state_done = Rc::clone(&state);
                    let status_done = status_label.clone();
                    let mic_done = mic_button_handler.clone();
                    let panel_done = panel.clone();
                    let transcript_done = transcript.clone();
                    glib::timeout_add_local(
                        std::time::Duration::from_millis(100),
                        move || match rx.try_recv() {
                            Ok(Ok(text)) => {
                                eprintln!("[asr] transcript:\n{text}");
                                let timestamp = match glib::DateTime::now_local() {
                                    Ok(dt) => dt
                                        .format("%H:%M:%S")
                                        .map(|ts| ts.to_string())
                                        .unwrap_or_else(|_| "unknown".to_string()),
                                    Err(_) => "unknown".to_string(),
                                };
                                let header = format!("Mic ({timestamp})");
                                transcript_done.append_block(&header, &text);
                                status_done.set_label("Status: Transcribed");
                                let idle =
                                    state_done.borrow().on_event(AppEvent::TranscriptionReady);
                                set_state(
                                    &state_done,
                                    idle,
                                    &status_done,
                                    &mic_done,
                                    &panel_done,
                                );
                                glib::ControlFlow::Break
                            }
                            Ok(Err(err)) => {
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
                                    &panel_done,
                                );
                                glib::ControlFlow::Break
                            }
                            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                            Err(mpsc::TryRecvError::Disconnected) => {
                                status_done.set_label("Status: ASR worker disconnected");
                                let idle =
                                    state_done.borrow().on_event(AppEvent::TranscriptionReady);
                                set_state(
                                    &state_done,
                                    idle,
                                    &status_done,
                                    &mic_done,
                                    &panel_done,
                                );
                                glib::ControlFlow::Break
                            }
                        },
                    );
                }
                    AppState::Idle => {
                        let meter_for_levels = meter.clone();
                        let wav_path = next_wav_path();
                        if let Err(err) = recorder.borrow_mut().start(&wav_path, move |level| {
                            meter_for_levels.set_fraction(level as f64)
                        }) {
                            status_label.set_label(&format!("Status: Mic error ({err})"));
                            return;
                        }
                        let next = state.borrow().on_event(AppEvent::StartListening);
                        set_state(&state, next, &status_label, &mic_button_handler, &panel);
                    }
                    AppState::Transcribing | AppState::TranscribingFile { .. } => {
                        status_label.set_label("Status: Transcribing (please wait)");
                    }
                    AppState::Error(_) => {
                        let meter_for_levels = meter.clone();
                        let wav_path = next_wav_path();
                        if let Err(err) = recorder.borrow_mut().start(&wav_path, move |level| {
                            meter_for_levels.set_fraction(level as f64)
                        }) {
                            status_label.set_label(&format!("Status: Mic error ({err})"));
                            return;
                        }
                        let next = state.borrow().on_event(AppEvent::StartListening);
                        set_state(&state, next, &status_label, &mic_button_handler, &panel);
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
    container: &gtk::Box,
) {
    *state.borrow_mut() = next.clone();
    apply_state(&next, status_label, mic_button, container);
}

fn apply_state(
    state: &AppState,
    status_label: &gtk::Label,
    mic_button: &gtk::Button,
    container: &gtk::Box,
) {
    status_label.set_label(&format!("Status: {}", state.status_text()));
    mic_button.set_sensitive(state.mic_enabled());
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
