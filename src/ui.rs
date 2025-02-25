///  * No UI (Only Gstreamer NavigationEvent)
use std::sync::{Arc, Mutex};

use gst::prelude::*;
use gst_video::NavigationEvent;

use crate::pipeline;
use crate::Compositor;
use crate::Settings;

#[derive(Default)]
pub struct MouseState {
    clicked: bool,
    clicked_x: f64,
    clicked_y: f64,
    clicked_xpos: i32,
    clicked_ypos: i32,
}

pub fn add_probe(
    pipeline: &gst::Pipeline,
    state: Arc<Mutex<MouseState>>,
    compositor: Arc<Mutex<Compositor>>,
    settings: &Settings,
) {
    let compositor_supports_crop: bool = settings.gst_pipeline_compositor_supports_crop();

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
        let Some(ev) = probe_info.event() else {
            return gst::PadProbeReturn::Ok;
        };

        if ev.type_() != gst::EventType::Navigation {
            return gst::PadProbeReturn::Ok;
        };

        let Ok(nav_event) = NavigationEvent::parse(ev) else {
            return gst::PadProbeReturn::Ok;
        };

        let mut compositor = compositor.lock().unwrap();
        let original_compositor = *compositor;

        match nav_event {
            NavigationEvent::KeyPress { key, .. } => match key.as_str() {
                "Left" | "FLECHA IZQUIERDA" => {
                    compositor.move_pos(-10, 0);
                }
                "Right" | "FLECHA DERECHA" => {
                    compositor.move_pos(10, 0);
                }
                "Up" | "FLECHA ARRIBA" => {
                    compositor.move_pos(0, -10);
                }
                "Down" | "FLECHA ABAJO" => {
                    compositor.move_pos(0, 10);
                }
                "plus" | "+" => {
                    compositor.zoom_in();
                }
                "minus" | "-" => {
                    compositor.zoom_out();
                }
                "r" => {
                    compositor.reset_position();
                }
                "Shift_R" | "R" => {
                    compositor.reset();
                }
                "1" => {
                    compositor.split_mode();
                    compositor.move_border_to(0);
                }
                "2" => {
                    compositor.split_mode();
                    let w = compositor.width;
                    compositor.move_border_to(w);
                }
                "3" => {
                    compositor.split_mode();
                    compositor.reset_border();
                }
                "4" => {
                    compositor.side_by_side_mode();
                }
                "5" => {
                    compositor.split_mode();
                    compositor.move_border(-10);
                }
                "6" => {
                    compositor.split_mode();
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
}
