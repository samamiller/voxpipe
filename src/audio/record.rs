use gst::prelude::*;
use gstreamer as gst;
use std::path::{Path, PathBuf};

pub struct Recorder {
    running: Option<RunningRecorder>,
}

struct RunningRecorder {
    wav_path: PathBuf,
    record_pipeline: gst::Pipeline,
    level_pipeline: gst::Pipeline,
    _watch: gst::bus::BusWatchGuard,
}

impl Recorder {
    pub fn new() -> Self {
        Self { running: None }
    }

    pub fn start<F>(&mut self, wav_path: impl AsRef<Path>, on_level: F) -> Result<(), String>
    where
        F: Fn(f32) + 'static,
    {
        self.stop();
        gst::init().map_err(|err| format!("gstreamer init failed: {err}"))?;

        let wav_path = wav_path.as_ref().to_path_buf();
        let record_pipeline = build_record_pipeline(&wav_path)?;
        let (level_pipeline, watch) = build_level_pipeline(on_level)?;

        level_pipeline
            .set_state(gst::State::Playing)
            .map_err(|err| format!("unable to start level monitor: {err}"))?;
        if let Err(err) = record_pipeline.set_state(gst::State::Playing) {
            let _ = level_pipeline.set_state(gst::State::Null);
            return Err(format!("unable to start recorder: {err}"));
        }

        self.running = Some(RunningRecorder {
            wav_path,
            record_pipeline,
            level_pipeline,
            _watch: watch,
        });
        Ok(())
    }

    pub fn stop(&mut self) -> Option<PathBuf> {
        let running = self.running.take()?;
        let _ = running.record_pipeline.set_state(gst::State::Null);
        let _ = running.level_pipeline.set_state(gst::State::Null);
        Some(running.wav_path)
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.stop();
    }
}

fn build_record_pipeline(wav_path: &Path) -> Result<gst::Pipeline, String> {
    let wav_path_str = wav_path
        .to_str()
        .ok_or_else(|| format!("wav path is not UTF-8: {}", wav_path.display()))?;

    let pipeline = gst::Pipeline::new();
    let src = gst::ElementFactory::make("pipewiresrc")
        .build()
        .map_err(|_| "failed to create pipewiresrc".to_string())?;
    let convert = gst::ElementFactory::make("audioconvert")
        .build()
        .map_err(|_| "failed to create audioconvert".to_string())?;
    let resample = gst::ElementFactory::make("audioresample")
        .build()
        .map_err(|_| "failed to create audioresample".to_string())?;
    let wavenc = gst::ElementFactory::make("wavenc")
        .build()
        .map_err(|_| "failed to create wavenc".to_string())?;
    let sink = gst::ElementFactory::make("filesink")
        .build()
        .map_err(|_| "failed to create filesink".to_string())?;
    sink.set_property("location", wav_path_str);

    pipeline
        .add_many([&src, &convert, &resample, &wavenc, &sink])
        .map_err(|err| format!("unable to build record pipeline: {err}"))?;
    gst::Element::link_many([&src, &convert, &resample, &wavenc, &sink])
        .map_err(|err| format!("unable to link record pipeline: {err}"))?;

    Ok(pipeline)
}

fn build_level_pipeline<F>(on_level: F) -> Result<(gst::Pipeline, gst::bus::BusWatchGuard), String>
where
    F: Fn(f32) + 'static,
{
    let pipeline = gst::Pipeline::new();
    let src = gst::ElementFactory::make("pipewiresrc")
        .build()
        .map_err(|_| "failed to create pipewiresrc".to_string())?;
    let convert = gst::ElementFactory::make("audioconvert")
        .build()
        .map_err(|_| "failed to create audioconvert".to_string())?;
    let resample = gst::ElementFactory::make("audioresample")
        .build()
        .map_err(|_| "failed to create audioresample".to_string())?;
    let level = gst::ElementFactory::make("level")
        .build()
        .map_err(|_| "failed to create level".to_string())?;
    level.set_property("interval", 100_000_000i64);
    level.set_property("post-messages", true);
    let sink = gst::ElementFactory::make("fakesink")
        .build()
        .map_err(|_| "failed to create fakesink".to_string())?;

    pipeline
        .add_many([&src, &convert, &resample, &level, &sink])
        .map_err(|err| format!("unable to build level pipeline: {err}"))?;
    gst::Element::link_many([&src, &convert, &resample, &level, &sink])
        .map_err(|err| format!("unable to link level pipeline: {err}"))?;

    let bus = pipeline
        .bus()
        .ok_or_else(|| "level pipeline bus missing".to_string())?;
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

    Ok((pipeline, watch))
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
