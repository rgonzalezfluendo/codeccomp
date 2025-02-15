// Video codecs comparation
//
// TODO add copyright
//

/*
TOODs:
[x] Allow move border or show only one video
[x] border fixed when zoom
[x] simple configure sources
[ ] refactor mixer with a status (center_x, center_y, zoom, border)
[ ] config toml file
[ ] full configure sources
[ ] configure encoders
[ ] metrics
 */

use gst::prelude::*;
use gst_video::video_event::NavigationEvent;
use std::sync::Mutex;

const WIDTH: i32 = 1280;
const HEIGHT: i32 = 720;
const HALFWIDTH: i32 = WIDTH / 2;
const HELP: &'static str = r#"
User can change the video showed using the next keys:
 * 1: Only first video
 * 2: Only second video
 * 3: First and second videos side by side (default)
 * 4: Move side by side border left
 * 5: Move side by side border right
Also click in the botton of the video can be done too change the side by side border

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

fn move_border(mixer_sink_0_pad: gst::Pad, mixer_sink_1_pad: gst::Pad, x: i32) {
    if x <= 0 {
        mixer_sink_0_pad.set_property("zorder", 10u32);
        mixer_sink_0_pad.set_property("width", WIDTH);
        mixer_sink_0_pad.set_property("crop-right", 0);
        mixer_sink_1_pad.set_property("zorder", 0u32);
        mixer_sink_1_pad.set_property("width", WIDTH);
        mixer_sink_1_pad.set_property("crop-right", 0);
        mixer_sink_1_pad.set_property("xpos", 0);
        return
    }
    if x >= WIDTH {
        mixer_sink_0_pad.set_property("zorder", 0u32);
        mixer_sink_0_pad.set_property("width", WIDTH);
        mixer_sink_0_pad.set_property("crop-right", 0);
        mixer_sink_1_pad.set_property("zorder", 10u32);
        mixer_sink_1_pad.set_property("width", WIDTH);
        mixer_sink_1_pad.set_property("crop-left", 0);
        mixer_sink_1_pad.set_property("xpos", 0);
        return
    }

    mixer_sink_0_pad.set_property("zorder", 10u32);
    mixer_sink_0_pad.set_property("width", x);
    mixer_sink_0_pad.set_property("crop-right", WIDTH - x);
    mixer_sink_1_pad.set_property("zorder", 0u32);
    mixer_sink_1_pad.set_property("width", WIDTH - x);
    mixer_sink_1_pad.set_property("crop-left", x);
    mixer_sink_1_pad.set_property("xpos", x);
}

fn zoom(mixer_sink_pad: gst::Pad, x: i32, y: i32, zoom_in: bool) {
    let xpos = mixer_sink_pad.property::<i32>("xpos");
    let ypos = mixer_sink_pad.property::<i32>("ypos");
    let width = mixer_sink_pad.property::<i32>("width");
    let height = mixer_sink_pad.property::<i32>("height");

    let (width_offset, height_offset) = if zoom_in {
        (WIDTH / 10, HEIGHT / 10)
    } else {
        (-WIDTH / 10, -HEIGHT / 10)
    };

    if width_offset + width <= 0 {
        return;
    }

    mixer_sink_pad.set_property("width", width + width_offset);
    mixer_sink_pad.set_property("height", height + height_offset);

    let xpos_offset = ((x as f32 / WIDTH as f32) * width_offset as f32) as i32;
    let new_xpos = xpos - xpos_offset;
    let ypos_offset = ((y as f32 / HEIGHT as f32) * height_offset as f32) as i32;
    let new_ypos = ypos - ypos_offset;

    if new_xpos != xpos {
        mixer_sink_pad.set_property("xpos", new_xpos);
    }
    if new_ypos != ypos {
        mixer_sink_pad.set_property("ypos", new_ypos);
    }
}

fn reset_zoom(mixer_sink_pad: gst::Pad) {
    let xpos = mixer_sink_pad.property::<i32>("xpos");
    let ypos = mixer_sink_pad.property::<i32>("ypos");
    let width = mixer_sink_pad.property::<i32>("width");
    let height = mixer_sink_pad.property::<i32>("height");

    if 0 != xpos {
        mixer_sink_pad.set_property("xpos", 0);
    }
    if 0 != ypos {
        mixer_sink_pad.set_property("ypos", 0);
    }
    if WIDTH != width {
        mixer_sink_pad.set_property("width", WIDTH);
    }
    if HEIGHT != height {
        mixer_sink_pad.set_property("height", HEIGHT);
    }
}

fn main() -> Result<(), anyhow::Error> {
    gst::init()?;
    //TODO delete msg
    println!("Hello, video codec comparator\n{HELP}");

    let state = Mutex::new(MouseState::default());

    gst::init().unwrap();

    let use_testrc = std::env::var("CODECCOMP_TEST_SRC").as_deref() == Ok("1");
    let pipeline_src_srt = if use_testrc {
        format!("gltestsrc pattern=mandelbrot name=src num-buffers=1000 ! video/x-raw(memory:GLMemory),framerate=30/1,width={WIDTH},height={HEIGHT},pixel-aspect-ratio=1/1 ! glcolorconvert ! gldownload")
    } else {
        //TODO no fix caps use generic
        format!("v4l2src ! image/jpeg, width=1280, height=720, framerate=30/1 ! jpegdec ! videoconvertscale ! videorate ")
    };

    //TODO(-100) handle no opengl pipelines with compositor and videotestsrc
    //TODO handle num-buffers
    //TODO(-10) handle to use glimagesinkelement (no KeyPress) or gtk4paintablesink (Note no NavigationEvent and env var GST_GTK4_WINDOW=1 needed)
    let pipeline_srt = format!(
        r#"
        {pipeline_src_srt} ! video/x-raw,framerate=30/1,width={WIDTH},height={HEIGHT},pixel-aspect-ratio=1/1 ! queue ! tee name=tee_src
        tee_src.src_0 ! queue name=enc0 ! x264enc bitrate=2048 tune=zerolatency speed-preset=ultrafast threads=4 key-int-max=2560 b-adapt=0 vbv-buf-capacity=120 ! queue name=dec0 !
        decodebin3 ! queue name=end0 ! mix.sink_0
        tee_src.src_1 ! queue name=enc1 ! x264enc bitrate=200 tune=zerolatency speed-preset=ultrafast threads=4 key-int-max=2560 b-adapt=0 vbv-buf-capacity=120 ! queue name=dec1 !
        decodebin3 ! queue name=end1 ! mix.sink_1
        glvideomixer name=mix sink_0::zorder=100 sink_0::width={HALFWIDTH} sink_0::crop-right={HALFWIDTH} sink_1::width={HALFWIDTH} sink_1::crop-left={HALFWIDTH} sink_1::xpos={HALFWIDTH} !
        glvideomixer name=zoom background=1 sink_0::xpos=0 sink_0::ypos=0 sink_0::zorder=0 sink_0::width={WIDTH} sink_0::height={HEIGHT} ! xvimagesink
    "#
    );

    let pipeline = gst::parse::launch(&pipeline_srt)
        .unwrap()
        .downcast::<gst::Pipeline>()
        .unwrap();

    let zoom_mixer = pipeline.by_name("zoom").unwrap();
    let zoom_mixer_src_pad = zoom_mixer.static_pad("src").unwrap();
    let zoom_mixer_sink_pad_weak = zoom_mixer.static_pad("sink_0").unwrap().downgrade();

    let input_mixer = pipeline.by_name("mix").unwrap();
    let input_mixer_sink_0_pad_weak = input_mixer.static_pad("sink_0").unwrap().downgrade();
    let input_mixer_sink_1_pad_weak = input_mixer.static_pad("sink_1").unwrap().downgrade();

    // Probe added in the sink pad to get direct navigation events w/o transformation done by the zoom_mixer
    zoom_mixer_src_pad.add_probe(gst::PadProbeType::EVENT_UPSTREAM, move |_, probe_info| {
        let zoom_mixer_sink_pad = zoom_mixer_sink_pad_weak.upgrade().unwrap();
        let input_mixer_sink_0_pad = input_mixer_sink_0_pad_weak.upgrade().unwrap();
        let input_mixer_sink_1_pad = input_mixer_sink_1_pad_weak.upgrade().unwrap();

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
                    let xpos = zoom_mixer_sink_pad.property::<i32>("xpos");
                    zoom_mixer_sink_pad.set_property("xpos", xpos - 10);

                    // adjust border
                    let x: i32 = input_mixer_sink_0_pad.property::<i32>("width") + 10;
                    move_border(input_mixer_sink_0_pad, input_mixer_sink_1_pad, x);

                }
                "Right" => {
                    let xpos = zoom_mixer_sink_pad.property::<i32>("xpos");
                    zoom_mixer_sink_pad.set_property("xpos", xpos + 10);

                    // adjust border
                    let x: i32 = input_mixer_sink_0_pad.property::<i32>("width") - 10;
                    move_border(input_mixer_sink_0_pad, input_mixer_sink_1_pad, x);
                }
                "Up" => {
                    let ypos = zoom_mixer_sink_pad.property::<i32>("ypos");
                    zoom_mixer_sink_pad.set_property("ypos", ypos - 10);
                }
                "Down" => {
                    let ypos = zoom_mixer_sink_pad.property::<i32>("ypos");
                    zoom_mixer_sink_pad.set_property("ypos", ypos + 10);
                }
                "plus" => {
                    zoom(zoom_mixer_sink_pad, WIDTH / 2, HEIGHT / 2, true);
                }
                "minus" => {
                    zoom(zoom_mixer_sink_pad, WIDTH / 2, HEIGHT / 2, false);
                }
                "r" => {
                    reset_zoom(zoom_mixer_sink_pad);

                    // adjust border
                    move_border(input_mixer_sink_0_pad, input_mixer_sink_1_pad, HALFWIDTH);
                }
                "1" => {
                    move_border(input_mixer_sink_0_pad, input_mixer_sink_1_pad, 0);
                }
                "2" => {
                    move_border(input_mixer_sink_0_pad, input_mixer_sink_1_pad, WIDTH);
                }
                "3" => {
                    move_border(input_mixer_sink_0_pad, input_mixer_sink_1_pad, HALFWIDTH);
                }
                "4" => {
                    let x: i32 = input_mixer_sink_0_pad.property::<i32>("width") - 10;
                    move_border(input_mixer_sink_0_pad, input_mixer_sink_1_pad, x);
                }
                "5" => {
                    let x: i32 = input_mixer_sink_0_pad.property::<i32>("width") + 10;
                    move_border(input_mixer_sink_0_pad, input_mixer_sink_1_pad, x);
                }
                _ => (),
            },
            NavigationEvent::MouseMove { x, y, .. } => {
                let state = state.lock().unwrap();
                if state.clicked {
                    let xpos = zoom_mixer_sink_pad.property::<i32>("xpos");
                    let ypos = zoom_mixer_sink_pad.property::<i32>("ypos");

                    let new_xpos = state.clicked_xpos + (x - state.clicked_x) as i32;
                    let new_ypos = state.clicked_ypos + (y - state.clicked_y) as i32;

                    if new_xpos != xpos {
                        zoom_mixer_sink_pad.set_property("xpos", new_xpos);

                        // adjust border
                        let x: i32 = input_mixer_sink_0_pad.property::<i32>("width") - (new_xpos-xpos);
                        move_border(input_mixer_sink_0_pad, input_mixer_sink_1_pad, x);

                    }

                    if new_ypos != ypos {
                        zoom_mixer_sink_pad.set_property("ypos", new_ypos);
                    }
                }
            }
            NavigationEvent::MouseButtonPress { button, x, y, .. } => {
                // zoom
                if button == 1 || button == 272 {
                    let mut state = state.lock().unwrap();
                    state.clicked = true;
                    state.clicked_x = x;
                    state.clicked_y = y;
                    state.clicked_xpos = zoom_mixer_sink_pad.property("xpos");
                    state.clicked_ypos = zoom_mixer_sink_pad.property("ypos");
                } else if button == 2 || button == 3 || button == 274 || button == 273 {
                    reset_zoom(zoom_mixer_sink_pad);
                } else if button == 4 {
                    zoom(zoom_mixer_sink_pad, x as i32, y as i32, true);
                } else if button == 5 {
                    zoom(zoom_mixer_sink_pad, x as i32, y as i32, false);
                }

                // border
                // TODO remove magic number 650
                if (button == 1 || button == 272) && y > 650.0 {
                    let x: i32 = x as i32;
                    move_border(input_mixer_sink_0_pad, input_mixer_sink_1_pad, x);
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
                    zoom(zoom_mixer_sink_pad, x as i32, y as i32, true);
                } else if delta_y < 0.0 {
                    zoom(zoom_mixer_sink_pad, x as i32, y as i32, false);
                }
            }
            _ => (),
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

    pipeline
        .set_state(gst::State::Null)
        .expect("Unable to set the pipeline to the `Null` state");

    Ok(())
}
