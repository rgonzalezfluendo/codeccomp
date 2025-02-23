use crate::compositor::Position;
use crate::Compositor;
use crate::Settings;

use gst::prelude::*;

pub fn get_srt(settings: &Settings) -> String {
    let src = settings.get_pipeline_src();
    let enc0 = settings.get_pipeline_enc0();
    let enc1 = settings.get_pipeline_enc1();
    let sink = settings.get_pipeline_sink();
    let compositor = settings.get_pipeline_compositor();

    //TODO(-100) handle no opengl pipelines with compositor and videotestsrc
    //TODO(-10) handle to use glimagesinkelement (no KeyPress) or gtk4paintablesink (Note no NavigationEvent and env var GST_GTK4_WINDOW=1 needed)
    let pipeline_srt = format!(
        r#"
        {src} ! queue ! tee name=tee_src
        tee_src.src_0 ! queue name=enc0 ! {enc0} ! queue name=dec0 !
        decodebin3 ! videocrop name=crop0 ! queue name=end0 ! mix.sink_0
        tee_src.src_1 ! queue name=enc1 ! {enc1} ! queue name=dec1 !
        decodebin3 ! videocrop name=crop1 ! queue name=end1 ! mix.sink_1
        {compositor} name=mix  !
        {sink}
    "#
    );

    if settings.debug {
        println!("pipeline:\n{}", &pipeline_srt);
    }

    pipeline_srt
}

fn fix_pos(pos: &mut Position, width: i32, compositor_supports_crop: bool) {
    // workaround to handle gst issue when width==0 with any video mixers
    // see `glvideomixer sink_0::width=0` in README.md
    if pos.width == 0 {
        pos.width = width;
        pos.xpos = width;
    }

    // workaround to handle gst issue when crop==total_width with compositor and vacompositor
    // see `compositor and vacompositor video out of the box` in README.md
    if !compositor_supports_crop {
        if pos.crop_right == width {
            pos.crop_right = width - 10;
        }

        if pos.crop_left == width {
            pos.crop_left = width - 10;
        }
    }
}

pub fn update_mixer(
    compositor: &Compositor,
    mixer_sink_0_pad: &gst::Pad,
    mixer_sink_1_pad: &gst::Pad,
    crop0: &gst::Element,
    crop1: &gst::Element,
    compositor_supports_crop: bool,
) {
    let (mut pos0, mut pos1) = compositor.get_positions();

    fix_pos(&mut pos0, compositor.width, compositor_supports_crop);
    fix_pos(&mut pos1, compositor.width, compositor_supports_crop);

    //TODO refactor avoid copy and paste
    if compositor_supports_crop {
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
    } else {
        mixer_sink_0_pad.set_properties(&[
            ("width", &pos0.width),
            ("height", &pos0.height),
            ("xpos", &pos0.xpos),
            ("ypos", &pos0.ypos),
        ]);

        mixer_sink_1_pad.set_properties(&[
            ("width", &pos1.width),
            ("height", &pos1.height),
            ("xpos", &pos1.xpos),
            ("ypos", &pos1.ypos),
        ]);

        crop0.set_property("right", pos0.crop_right);
        crop1.set_property("left", pos1.crop_left);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::BackendType;

    #[test]
    fn test_fix_pos_with_0() {
        let width: i32 = 720;
        let mut pos = Position {
            xpos: 0,
            ypos: 216,
            width: 0,
            height: 288,
            crop_right: 720,
            crop_left: 0,
        };

        fix_pos(&mut pos, width, true);
        assert_eq!(pos.xpos, width);
        assert_eq!(pos.ypos, 216);
        assert_eq!(pos.width, width);
        assert_eq!(pos.height, 288);
        assert_eq!(pos.crop_right, 720);
        assert_eq!(pos.crop_left, 0);
    }

    #[test]
    fn test_fix_pos_video_out_of_box() {
        let width: i32 = 720;
        let mut pos = Position {
            xpos: 0,
            ypos: 216,
            width: 0,
            height: 288,
            crop_right: 720,
            crop_left: 0,
        };

        fix_pos(&mut pos, width, false);
        assert_eq!(pos.xpos, width);
        assert_eq!(pos.ypos, 216);
        assert_eq!(pos.width, width);
        assert_eq!(pos.height, 288);
        assert_eq!(pos.crop_right, 710); // width - 10
        assert_eq!(pos.crop_left, 0);
    }

    fn wait(bus: &gst::Bus) -> bool {
        for msg in bus.iter_timed(gst::ClockTime::SECOND) {
            use gst::MessageView;

            match msg.view() {
                MessageView::Eos(..) => {
                    println!("received eos");
                    return false;
                }
                MessageView::Error(err) => {
                    println!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    );
                    return false;
                }
                _ => (),
            };
        }
        true
    }

    fn test_tour(backend: BackendType) {
        let mut settings = Settings::default();
        settings.backend = backend;
        let compositor_supports_crop: bool = settings.gst_pipeline_compositor_supports_crop();
        let mut compositor = Compositor::new_split(settings.input.width, settings.input.height);

        //TODO refactor this logic with main

        gst::init().unwrap();
        let pipeline_srt = get_srt(&settings);

        let pipeline = gst::parse::launch(&pipeline_srt)
            .unwrap()
            .downcast::<gst::Pipeline>()
            .unwrap();

        let mixer = pipeline.by_name("mix").unwrap();
        let crop0 = pipeline.by_name("crop0").unwrap();
        let crop1 = pipeline.by_name("crop1").unwrap();
        let mixer_sink_0_pad = mixer.static_pad("sink_0").unwrap();
        let mixer_sink_1_pad = mixer.static_pad("sink_1").unwrap();

        let update_mixer_fn = |compositor: &Compositor| {
            update_mixer(
                compositor,
                &mixer_sink_0_pad,
                &mixer_sink_1_pad,
                &crop0,
                &crop1,
                compositor_supports_crop,
            );
        };

        pipeline
            .set_state(gst::State::Playing)
            .expect("Unable to set the pipeline to the `Playing` state");

        let bus = pipeline.bus().unwrap();
        assert!(wait(&bus));

        //zoom in
        compositor.zoom_in();
        update_mixer_fn(&compositor);
        assert!(wait(&bus));

        compositor.zoom_in();
        update_mixer_fn(&compositor);
        assert!(wait(&bus));

        //top left
        compositor.move_pos(-30, -10);
        compositor.zoom_in();
        update_mixer_fn(&compositor);
        assert!(wait(&bus));

        //bottom righ
        compositor.move_pos(compositor.width, compositor.height);
        update_mixer_fn(&compositor);
        assert!(wait(&bus));

        //bottom left
        compositor.move_pos(-1 * compositor.width, 34);
        update_mixer_fn(&compositor);
        assert!(wait(&bus));

        //top rigth
        compositor.move_pos(24, -1 * compositor.height);
        update_mixer_fn(&compositor);
        assert!(wait(&bus));

        // zoom out
        compositor.zoom_out();
        compositor.zoom_out();
        compositor.zoom_out();
        compositor.zoom_out();
        update_mixer_fn(&compositor);
        assert!(wait(&bus));

        //bottom left
        compositor.move_pos(-110, -100);
        update_mixer_fn(&compositor);
        assert!(wait(&bus));

        //top rigth
        compositor.move_pos(220, 200);
        update_mixer_fn(&compositor);
        assert!(wait(&bus));

        // 1
        compositor.move_border_to(0);
        update_mixer_fn(&compositor);
        assert!(wait(&bus));

        // 2
        let w = compositor.width;
        compositor.move_border_to(w);
        update_mixer_fn(&compositor);
        assert!(wait(&bus));

        // 3
        compositor.reset_border();
        update_mixer_fn(&compositor);
        assert!(wait(&bus));

        // 4
        compositor.move_border(-40);
        update_mixer_fn(&compositor);
        assert!(wait(&bus));

        // 5
        compositor.move_border(10);
        update_mixer_fn(&compositor);
        assert!(wait(&bus));

        // reset
        compositor.reset();
        update_mixer_fn(&compositor);
        assert!(wait(&bus));

        pipeline
            .set_state(gst::State::Null)
            .expect("Unable to set the pipeline to the `Null` state");
    }

    #[test]
    fn test_tour_gl() {
        test_tour(BackendType::GL);
    }

    #[test]
    #[cfg_attr(not(feature = "expensive_tests"), ignore)]
    fn test_tour_vaapi() {
        test_tour(BackendType::VAAPI);
    }

    #[test]
    #[cfg_attr(not(feature = "expensive_tests"), ignore)]
    fn test_tour_cpu() {
        test_tour(BackendType::CPU);
    }
}
