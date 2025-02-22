// Video codecs comparation
//
// TODO add copyright
//

mod pipeline;
mod settings;
mod compositor;

use gst::prelude::*;
use gst_video::video_event::NavigationEvent;
use settings::Settings;
use compositor::Compositor;
use std::sync::Mutex;

const HELP: &str = r#"
User can change the video showed using the next keys:
 * 1: Only first video
 * 2: Only second video
 * 3: First and second videos side by side (default)
 * 4: Move side by side border left
 * 5: Move side by side border right
Also click in the botton of the video can be done to change the side by side border

User can change the video player zoom using the next keys:
 * +: Zoom in
 * -: Zoom out
 * Up/Down/Right/Left: Move the frame
 * r: reset the zoom
Also mouse navigation events can be used for a better UX.

"#;

#[derive(Default)]
struct MouseState {
    clicked: bool,
    clicked_x: f64,
    clicked_y: f64,
    clicked_xpos: i32,
    clicked_ypos: i32,
}

fn main() -> Result<(), anyhow::Error> {
    let settings = Settings::new()?;
    let compositor_supports_crop: bool = settings.gst_pipeline_compositor_supports_crop();

    gst::init()?;

    println!("Hello, video codec comparator\n{HELP}");
    if settings.debug {
        println!("settings:\n{:#?}", settings);
    }

    let state = Mutex::new(MouseState::default());
    let compositor = Mutex::new(Compositor::new(settings.input.width, settings.input.height));

    gst::init().unwrap();
    let pipeline_srt = pipeline::get_srt(settings);

    let pipeline = gst::parse::launch(&pipeline_srt)
        .unwrap()
        .downcast::<gst::Pipeline>()
        .unwrap();

    let mixer = pipeline.by_name("mix").unwrap();
    let crop0 = pipeline.by_name("crop0").unwrap();
    let crop1 = pipeline.by_name("crop1").unwrap();
    let mixer_src_pad = mixer.static_pad("src").unwrap();
    let mixer_sink_0_pad = mixer.static_pad("sink_0").unwrap();
    let mixer_sink_1_pad = mixer.static_pad("sink_1").unwrap();

    pipeline::update_mixer(
        &compositor.lock().unwrap(),
        &mixer_sink_0_pad,
        &mixer_sink_1_pad,
        &crop0,
        &crop1,
        compositor_supports_crop,
    );

    // Probe added in the sink pad to get direct navigation events w/o transformation done by the zoom_mixer
    mixer_src_pad.add_probe(gst::PadProbeType::EVENT_UPSTREAM, move |_, probe_info| {
        let mut compositor = compositor.lock().unwrap();
        let original_compositor = compositor.clone();

        let Some(ev) = probe_info.event() else {
            return gst::PadProbeReturn::Ok;
        };

        if ev.type_() != gst::EventType::Navigation {
            return gst::PadProbeReturn::Ok;
        };

        let Ok(nav_event) = NavigationEvent::parse(ev) else {
            return gst::PadProbeReturn::Ok;
        };

        match nav_event {
            NavigationEvent::KeyPress { key, .. } => match key.as_str() {
                "Left" => {
                    compositor.move_pos(-10, 0);
                }
                "Right" => {
                    compositor.move_pos(10, 0);
                }
                "Up" => {
                    compositor.move_pos(0, -10);
                }
                "Down" => {
                    compositor.move_pos(0, 10);
                }
                "plus" => {
                    compositor.zoom_in();
                }
                "minus" => {
                    compositor.zoom_out();
                }
                "r" => {
                    compositor.reset_position();
                }
                "Shift_R" => {
                    compositor.reset();
                }
                "1" => {
                    compositor.move_border_to(0);
                }
                "2" => {
                    let w = compositor.width;
                    compositor.move_border_to(w);
                }
                "3" => {
                    compositor.reset_border();
                }
                "4" => {
                    compositor.move_border(-10);
                }
                "5" => {
                    compositor.move_border(10);
                }
                _ => (),
            },
            NavigationEvent::MouseMove { x, y, .. } => {
                let state = state.lock().unwrap();
                if state.clicked {
                    let new_xpos = (x - state.clicked_x) as i32 + state.clicked_xpos;
                    let new_ypos = (y - state.clicked_y) as i32 + state.clicked_ypos;

                    compositor.move_pos_to(new_xpos, new_ypos);
                }
            }
            NavigationEvent::MouseButtonPress { button, x, y, .. } => {
                if button == 1 || button == 272 {
                    let mut state = state.lock().unwrap();
                    state.clicked = true;
                    state.clicked_x = x;
                    state.clicked_y = y;
                    state.clicked_xpos = compositor.offset_x;
                    state.clicked_ypos = compositor.offset_y;

                    if y >= 600.0 {
                        compositor.move_border_to(x as i32);
                    }
                } else if button == 2 || button == 3 || button == 274 || button == 273 {
                    compositor.reset();
                } else if button == 4 {
                    compositor.zoom_in_center_at(x as i32, y as i32);
                } else if button == 5 {
                    compositor.zoom_out_center_at(x as i32, y as i32);
                }
            }
            NavigationEvent::MouseButtonRelease { button, .. } => {
                if button == 1 || button == 272 {
                    let mut state = state.lock().unwrap();
                    state.clicked = false;
                }
            }
            NavigationEvent::MouseScroll { x, y, delta_y, .. } => {
                if delta_y > 0.0 {
                    compositor.zoom_in_center_at(x as i32, y as i32);
                } else if delta_y < 0.0 {
                    compositor.zoom_out_center_at(x as i32, y as i32);
                }
            }
            _ => (),
        }

        if original_compositor != *compositor {
            pipeline::update_mixer(
                &compositor,
                &mixer_sink_0_pad,
                &mixer_sink_1_pad,
                &crop0,
                &crop1,
                compositor_supports_crop,
            );
        }

        gst::PadProbeReturn::Ok
    });

    pipeline
        .set_state(gst::State::Playing)
        .expect("Unable to set the pipeline to the `Playing` state");

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
