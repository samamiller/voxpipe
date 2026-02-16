use adw::prelude::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

mod state;

use state::{AppEvent, AppState};

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id("io.voxpipe.App")
        .build();

    app.connect_activate(|app| {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Voxpipe")
            .default_width(420)
            .default_height(72)
            .decorated(false)
            .modal(true)
            .build();
        window.set_resizable(false);

        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(12)
            .margin_end(12)
            .build();

        let mic_button = gtk::Button::builder()
            .icon_name("audio-input-microphone-symbolic")
            .tooltip_text("Start listening")
            .build();

        let status_label = gtk::Label::builder()
            .xalign(0.0)
            .hexpand(false)
            .label("Status: Idle")
            .build();

        let meter = gtk::ProgressBar::builder()
            .hexpand(true)
            .show_text(false)
            .fraction(0.0)
            .build();

        let right_controls = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(4)
            .build();
        let minimize_button = gtk::Button::builder()
            .icon_name("window-minimize-symbolic")
            .tooltip_text("Minimize")
            .build();
        let close_button = gtk::Button::builder()
            .icon_name("window-close-symbolic")
            .tooltip_text("Close")
            .build();
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
        apply_state(&state.borrow(), &status_label, &mic_button);

        {
            let state = Rc::clone(&state);
            let status_label = status_label.clone();
            let mic_button = mic_button.clone();
            let mic_button_handler = mic_button.clone();

            mic_button.connect_clicked(move |_| {
                let next = if matches!(*state.borrow(), AppState::Listening) {
                    state.borrow().on_event(AppEvent::StopListening)
                } else {
                    state.borrow().on_event(AppEvent::StartListening)
                };
                set_state(&state, next, &status_label, &mic_button_handler);
            });
        }

        {
            let state = Rc::clone(&state);
            let meter = meter.clone();
            let phase = Rc::new(RefCell::new(0.0_f64));
            let phase_for_tick = Rc::clone(&phase);

            glib::timeout_add_local(Duration::from_millis(80), move || {
                if matches!(*state.borrow(), AppState::Listening) {
                    let mut phase = phase_for_tick.borrow_mut();
                    *phase += 0.22;
                    let value = ((*phase).sin() * 0.5 + 0.5) as f64;
                    meter.set_fraction(value.clamp(0.02, 0.98));
                } else {
                    *phase_for_tick.borrow_mut() = 0.0;
                    meter.set_fraction(0.0);
                }
                glib::ControlFlow::Continue
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

        window.present();
    });

    app.run()
}

fn set_state(
    state: &Rc<RefCell<AppState>>,
    next: AppState,
    status_label: &gtk::Label,
    mic_button: &gtk::Button,
) {
    *state.borrow_mut() = next;
    apply_state(&state.borrow(), status_label, mic_button);
}

fn apply_state(state: &AppState, status_label: &gtk::Label, mic_button: &gtk::Button) {
    status_label.set_label(&format!("Status: {}", state.status_text()));
    if matches!(state, AppState::Listening) {
        mic_button.set_icon_name("media-playback-stop-symbolic");
        mic_button.set_tooltip_text(Some("Stop listening"));
    } else {
        mic_button.set_icon_name("audio-input-microphone-symbolic");
        mic_button.set_tooltip_text(Some("Start listening"));
    }
}
