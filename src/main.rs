use adw::prelude::*;
use std::{cell::RefCell, rc::Rc};

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
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build();

        let status_label = gtk::Label::builder().xalign(0.0).build();
        let listen_button = gtk::Button::with_label("Listen");
        let stop_button = gtk::Button::with_label("Stop");
        let fail_button = gtk::Button::with_label("Simulate Error");
        let reset_button = gtk::Button::with_label("Reset Error");

        let controls = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .build();
        controls.append(&listen_button);
        controls.append(&stop_button);
        controls.append(&fail_button);
        controls.append(&reset_button);

        container.append(&status_label);
        container.append(&controls);
        window.set_content(Some(&container));

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

        let state = Rc::new(RefCell::new(AppState::Idle));
        apply_state(
            &state.borrow(),
            &status_label,
            &listen_button,
            &stop_button,
            &fail_button,
            &reset_button,
        );

        {
            let state = Rc::clone(&state);
            let status_label = status_label.clone();
            let listen_button = listen_button.clone();
            let listen_button_handler = listen_button.clone();
            let stop_button = stop_button.clone();
            let fail_button = fail_button.clone();
            let reset_button = reset_button.clone();

            listen_button.connect_clicked(move |_| {
                let next = state.borrow().on_event(AppEvent::StartListening);
                set_state(
                    &state,
                    next,
                    &status_label,
                    &listen_button_handler,
                    &stop_button,
                    &fail_button,
                    &reset_button,
                );
            });
        }

        {
            let state = Rc::clone(&state);
            let status_label = status_label.clone();
            let listen_button = listen_button.clone();
            let stop_button = stop_button.clone();
            let stop_button_handler = stop_button.clone();
            let fail_button = fail_button.clone();
            let reset_button = reset_button.clone();

            stop_button.connect_clicked(move |_| {
                let next = state.borrow().on_event(AppEvent::StopListening);
                set_state(
                    &state,
                    next,
                    &status_label,
                    &listen_button,
                    &stop_button_handler,
                    &fail_button,
                    &reset_button,
                );

                let next = state.borrow().on_event(AppEvent::BeginTranscription);
                set_state(
                    &state,
                    next,
                    &status_label,
                    &listen_button,
                    &stop_button_handler,
                    &fail_button,
                    &reset_button,
                );

                let state_timeout = Rc::clone(&state);
                let status_label_timeout = status_label.clone();
                let listen_button_timeout = listen_button.clone();
                let stop_button_timeout = stop_button_handler.clone();
                let fail_button_timeout = fail_button.clone();
                let reset_button_timeout = reset_button.clone();

                glib::timeout_add_seconds_local_once(1, move || {
                    let next = state_timeout
                        .borrow()
                        .on_event(AppEvent::TranscriptionFinished);
                    set_state(
                        &state_timeout,
                        next,
                        &status_label_timeout,
                        &listen_button_timeout,
                        &stop_button_timeout,
                        &fail_button_timeout,
                        &reset_button_timeout,
                    );
                });
            });
        }

        {
            let state = Rc::clone(&state);
            let status_label = status_label.clone();
            let listen_button = listen_button.clone();
            let stop_button = stop_button.clone();
            let fail_button = fail_button.clone();
            let fail_button_handler = fail_button.clone();
            let reset_button = reset_button.clone();

            fail_button.connect_clicked(move |_| {
                let next = state
                    .borrow()
                    .on_event(AppEvent::Fail("Mock failure".to_string()));
                set_state(
                    &state,
                    next,
                    &status_label,
                    &listen_button,
                    &stop_button,
                    &fail_button_handler,
                    &reset_button,
                );
            });
        }

        {
            let state = Rc::clone(&state);
            let status_label = status_label.clone();
            let listen_button = listen_button.clone();
            let stop_button = stop_button.clone();
            let fail_button = fail_button.clone();
            let reset_button = reset_button.clone();
            let reset_button_handler = reset_button.clone();

            reset_button.connect_clicked(move |_| {
                let next = state.borrow().on_event(AppEvent::Reset);
                set_state(
                    &state,
                    next,
                    &status_label,
                    &listen_button,
                    &stop_button,
                    &fail_button,
                    &reset_button_handler,
                );
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
    listen_button: &gtk::Button,
    stop_button: &gtk::Button,
    fail_button: &gtk::Button,
    reset_button: &gtk::Button,
) {
    *state.borrow_mut() = next;
    apply_state(
        &state.borrow(),
        status_label,
        listen_button,
        stop_button,
        fail_button,
        reset_button,
    );
}

fn apply_state(
    state: &AppState,
    status_label: &gtk::Label,
    listen_button: &gtk::Button,
    stop_button: &gtk::Button,
    fail_button: &gtk::Button,
    reset_button: &gtk::Button,
) {
    status_label.set_label(&format!("Status: {}", state.status_text()));
    listen_button.set_sensitive(state.can_start_listening());
    stop_button.set_sensitive(state.can_stop_listening());
    fail_button.set_sensitive(!matches!(state, AppState::Transcribing));
    reset_button.set_sensitive(state.can_reset());
}
