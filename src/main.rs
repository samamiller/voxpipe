use adw::prelude::*;
use std::{cell::RefCell, rc::Rc};

mod audio;
mod state;
mod whisper_sys;
pub mod whisper_rs;

use audio::monitor::Monitor;
use state::{AppEvent, AppState};
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
            .default_height(72)
            .decorated(false)
            .modal(true)
            .build();
        window.set_resizable(false);
        window.add_css_class("hud-window");

        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(12)
            .margin_end(12)
            .build();
        container.add_css_class("hud-body");

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
        right_controls.append(&minimize_button);
        right_controls.append(&close_button);

        container.append(&mic_button);
        container.append(&status_label);
        container.append(&meter);
        container.append(&right_controls);

        let drag_gesture = gtk::GestureClick::builder().button(0).build();
        {
            let window = window.clone();
            drag_gesture.connect_pressed(move |gesture, _, x, y| {
                let button = gesture.current_button() as i32;
                let timestamp = gesture.current_event_time();

                if let (Some(device), Some(surface)) = (gesture.current_event_device(), window.surface()) {
                    if let Ok(toplevel) = surface.dynamic_cast::<gtk::gdk::Toplevel>() {
                        toplevel.begin_move(&device, button, x, y, timestamp);
                    }
                }
            });
        }
        container.add_controller(drag_gesture);
        window.set_content(Some(&container));

        let state = Rc::new(RefCell::new(AppState::Idle));
        let monitor = Rc::new(RefCell::new(Monitor::new()));
        apply_state(&state.borrow(), &status_label, &mic_button, &container);

        {
            let state = Rc::clone(&state);
            let monitor = Rc::clone(&monitor);
            let status_label = status_label.clone();
            let meter = meter.clone();
            let mic_button = mic_button.clone();
            let mic_button_handler = mic_button.clone();
            let container = container.clone();

            mic_button.connect_clicked(move |_| {
                let next = if matches!(*state.borrow(), AppState::Listening) {
                    monitor.borrow_mut().stop();
                    meter.set_fraction(0.0);
                    state.borrow().on_event(AppEvent::StopListening)
                } else {
                    let meter_for_levels = meter.clone();
                    if let Err(err) = monitor.borrow_mut().start(move |level| {
                        meter_for_levels.set_fraction(level as f64);
                    }) {
                        status_label.set_label(&format!("Status: Mic error ({err})"));
                        return;
                    }
                    state.borrow().on_event(AppEvent::StartListening)
                };
                set_state(&state, next, &status_label, &mic_button_handler, &container);
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
                        status_label.set_label(
                            "Status: Whisper smoke failed (set VOXPIPE_WHISPER_MODEL)",
                        );
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
                        status_label
                            .set_label(&format!("Status: Whisper smoke failed ({err})"));
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
    *state.borrow_mut() = next;
    apply_state(&state.borrow(), status_label, mic_button, container);
}

fn apply_state(
    state: &AppState,
    status_label: &gtk::Label,
    mic_button: &gtk::Button,
    container: &gtk::Box,
) {
    status_label.set_label(&format!("Status: {}", state.status_text()));
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
