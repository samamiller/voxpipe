use gst::prelude::*;
use gstreamer as gst;

pub struct Monitor {
    running: Option<RunningMonitor>,
}

struct RunningMonitor {
    pipeline: gst::Pipeline,
    _watch: gst::bus::BusWatchGuard,
}

impl Monitor {
    pub fn new() -> Self {
        Self { running: None }
    }

    pub fn start<F>(&mut self, on_level: F) -> Result<(), String>
    where
        F: Fn(f32) + 'static,
    {
        self.stop();

        gst::init().map_err(|err| format!("gstreamer init failed: {err}"))?;

        let element = gst::parse::launch(
            "pipewiresrc ! audioconvert ! audioresample ! level interval=100000000 post-messages=true ! fakesink",
        )
        .map_err(|err| format!("pipeline creation failed: {err}"))?;

        let pipeline = element
            .dynamic_cast::<gst::Pipeline>()
            .map_err(|_| "pipeline type mismatch".to_string())?;

        let bus = pipeline
            .bus()
            .ok_or_else(|| "pipeline bus missing".to_string())?;

        let watch = bus
            .add_watch_local(move |_, message| {
                if let gst::MessageView::Element(element) = message.view() {
                    if let Some(structure) = element.structure() {
                        if structure.name() == "level" {
                            if let Some(level) = parse_level(structure) {
                                on_level(level);
                            }
                        }
                    }
                }
                glib::ControlFlow::Continue
            })
            .map_err(|err| format!("bus watch setup failed: {err}"))?;

        pipeline
            .set_state(gst::State::Playing)
            .map_err(|err| format!("unable to start monitor: {err}"))?;

        self.running = Some(RunningMonitor {
            pipeline,
            _watch: watch,
        });

        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(running) = self.running.take() {
            let _ = running.pipeline.set_state(gst::State::Null);
        }
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.stop();
    }
}

fn parse_level(structure: &gst::StructureRef) -> Option<f32> {
    let peaks = structure.get::<gst::List>("peak").ok()?;
    let db = peaks
        .as_slice()
        .iter()
        .filter_map(|value| value.get::<f64>().ok())
        .fold(f64::NEG_INFINITY, f64::max);

    if !db.is_finite() {
        return Some(0.0);
    }

    let linear = 10_f64.powf(db / 20.0);
    Some(linear.clamp(0.0, 1.0) as f32)
}
