// Video codecs comparation
//
// TODO add copyright
//

/*
TODOs:
[x] Allow move border or show only one video
[x] border fixed when zoom
[x] simple configure sources
[x] refactor mixer with a status (center_x, center_y, zoom, border)
[x] zoom_in_center_at
[x] status with any resolution
[x] config toml file
[ ] full configure sources
[ ] configure encoders
[ ] Bandwidth metrics
[ ] latency metrics
[ ] PSNR vs. SSIM metrics
[ ] Windows support
[ ] osX support
[ ] Fake sink

 */

mod settings;
mod status;

use gst::prelude::*;
use gst_video::video_event::NavigationEvent;
use settings::Settings;
use status::Status;
use std::sync::Mutex;

const HELP: &'static str = r#"
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

fn update_mixer(mixer_sink_0_pad: gst::Pad, mixer_sink_1_pad: gst::Pad, status: Status) {
    let (pos0, pos1) = status.get_positions();

    mixer_sink_0_pad.set_properties(&[
        ("width", &pos0.width),
        ("height", &pos0.height),
        ("xpos", &pos0.xpos),
        ("ypos", &pos0.ypos),
        ("crop-right", &pos0.crop_right),
    ]);

    mixer_sink_1_pad.set_properties(&[
        ("width", &pos1.width),
        ("height", &pos1.height),
        ("xpos", &pos1.xpos),
        ("ypos", &pos1.ypos),
        ("crop-left", &pos1.crop_left),
    ]);
}

fn main() -> Result<(), anyhow::Error> {
    let settings = Settings::new()?;

    gst::init()?;

    //TODO delete msg
    println!("Hello, video codec comparator\n{HELP}");

    let state = Mutex::new(MouseState::default());
    let status = Mutex::new(Status::new(settings.input.width, settings.input.height));

    gst::init().unwrap();

    let src = settings.get_pipeline_src();
    let enc0 = settings.get_pipeline_enc0();
    let enc1 = settings.get_pipeline_enc1();
    let sink = settings.get_pipeline_sink();

    //TODO(-100) handle no opengl pipelines with compositor and videotestsrc
    //TODO(-10) handle to use glimagesinkelement (no KeyPress) or gtk4paintablesink (Note no NavigationEvent and env var GST_GTK4_WINDOW=1 needed)
    let pipeline_srt = format!(
        r#"
        {src} ! queue ! tee name=tee_src
        tee_src.src_0 ! queue name=enc0 ! {enc0} ! queue name=dec0 !
        decodebin3 ! queue name=end0 ! mix.sink_0
        tee_src.src_1 ! queue name=enc1 ! {enc1} ! queue name=dec1 !
        decodebin3 ! queue name=end1 ! mix.sink_1
        glvideomixer name=mix  !
        {sink}
    "#
    );

    let pipeline = gst::parse::launch(&pipeline_srt)
        .unwrap()
        .downcast::<gst::Pipeline>()
        .unwrap();

    let mixer = pipeline.by_name("mix").unwrap();
    let mixer_src_pad = mixer.static_pad("src").unwrap();
    let mixer_sink_0_pad = mixer.static_pad("sink_0").unwrap();
    let mixer_sink_1_pad = mixer.static_pad("sink_1").unwrap();

    update_mixer(mixer_sink_0_pad, mixer_sink_1_pad, *status.lock().unwrap());

    let mixer_sink_0_pad_weak = mixer.static_pad("sink_0").unwrap().downgrade();
    let mixer_sink_1_pad_weak = mixer.static_pad("sink_1").unwrap().downgrade();

    // Probe added in the sink pad to get direct navigation events w/o transformation done by the zoom_mixer
    mixer_src_pad.add_probe(gst::PadProbeType::EVENT_UPSTREAM, move |_, probe_info| {
        let mixer_sink_0_pad = mixer_sink_0_pad_weak.upgrade().unwrap();
        let mixer_sink_1_pad = mixer_sink_1_pad_weak.upgrade().unwrap();
        let mut status = status.lock().unwrap();

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
                    status.move_pos(-10, 0);
                }
                "Right" => {
                    status.move_pos(10, 0);
                }
                "Up" => {
                    status.move_pos(0, -10);
                }
                "Down" => {
                    status.move_pos(0, 10);
                }
                "plus" => {
                    status.zoom_in();
                }
                "minus" => {
                    status.zoom_out();
                }
                "r" => {
                    status.reset();
                }
                "1" => {
                    status.move_border_to(0);
                }
                "2" => {
                    let w = status.width;
                    status.move_border_to(w);
                }
                "3" => {
                    status.reset_border();
                }
                "4" => {
                    status.move_border(-10);
                }
                "5" => {
                    status.move_border(10);
                }
                _ => (),
            },
            NavigationEvent::MouseMove { x, y, .. } => {
                let state = state.lock().unwrap();
                if state.clicked {
                    let new_xpos = (x - state.clicked_x) as i32 + state.clicked_xpos;
                    let new_ypos = (y - state.clicked_y) as i32 + state.clicked_ypos;

                    status.move_pos_to(new_xpos, new_ypos);
                }
            }
            NavigationEvent::MouseButtonPress { button, x, y, .. } => {
                if button == 1 || button == 272 {
                    let mut state = state.lock().unwrap();
                    state.clicked = true;
                    state.clicked_x = x;
                    state.clicked_y = y;
                    state.clicked_xpos = status.offset_x;
                    state.clicked_ypos = status.offset_y;

                    if y >= 600.0 {
                        status.move_border_to(x as i32);
                    }
                } else if button == 2 || button == 3 || button == 274 || button == 273 {
                    status.reset();
                } else if button == 4 {
                    status.zoom_in_center_at(x as i32, y as i32);
                } else if button == 5 {
                    status.zoom_out_center_at(x as i32, y as i32);
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
                    status.zoom_in_center_at(x as i32, y as i32);
                } else if delta_y < 0.0 {
                    status.zoom_out_center_at(x as i32, y as i32);
                }
            }
            _ => (),
        }

        //TODO only update if needed
        update_mixer(mixer_sink_0_pad, mixer_sink_1_pad, *status);

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

    pipeline
        .set_state(gst::State::Null)
        .expect("Unable to set the pipeline to the `Null` state");

    Ok(())
}
