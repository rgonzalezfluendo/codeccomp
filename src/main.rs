// Video Codecs Comparation
//
// TODO add copyright
//

mod compositor;
mod pipeline;
mod settings;
mod ui;

use gst::prelude::*;

use compositor::Compositor;
use settings::Settings;
use std::sync::{Arc, Mutex};

const HELP: &str = include_str!("../doc/help.md");

fn main() -> Result<(), anyhow::Error> {
    let settings = Settings::new()?;

    pipeline::init()?;

    println!("Hello, video codec comparator\n{HELP}");
    if settings.debug {
        println!("settings:\n{:#?}", settings);
    }

    let state = Arc::new(Mutex::new(ui::MouseState::default()));
    let compositor_mode = if settings.sidebyside {
        compositor::Mode::SideBySide
    } else {
        compositor::Mode::Split
    };
    let compositor = Arc::new(Mutex::new(Compositor::new(
        compositor_mode,
        settings.input.width,
        settings.input.height,
    )));

    gst::init().unwrap();
    let pipeline_srt = pipeline::get_srt(&settings);

    let pipeline = gst::parse::launch(&pipeline_srt)
        .unwrap()
        .downcast::<gst::Pipeline>()
        .unwrap();

    pipeline
        .set_state(gst::State::Playing)
        .expect("Unable to set the pipeline to the `Playing` state");

    ui::add_ui_probe(&pipeline, state.clone(), compositor.clone(), &settings);

    let bus = pipeline.bus().unwrap();
    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => {
                println!("received eos");
                break;
            }
            MessageView::Error(err) => {
                println!(
                    "Error from {:?}: {} ({:?})",
                    err.src().map(|s| s.path_string()),
                    err.error(),
                    err.debug()
                );
                break;
            }
            _ => (),
        };
    }

    if std::env::var("GST_DEBUG_DUMP_DOT_DIR").as_deref().is_ok() {
        pipeline.debug_to_dot_file(gst::DebugGraphDetails::all(), "codeccomp");
    }

    pipeline
        .set_state(gst::State::Null)
        .expect("Unable to set the pipeline to the `Null` state");

    Ok(())
}
