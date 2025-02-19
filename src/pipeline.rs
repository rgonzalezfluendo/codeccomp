use crate::settings::BackendType;
use crate::Settings;
use crate::Status;

use gst::prelude::*;

pub fn get_srt(settings: Settings) -> String {
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

pub fn update_mixer(
    status: &Status,
    mixer_sink_0_pad: &gst::Pad,
    mixer_sink_1_pad: &gst::Pad,
    crop0: &gst::Element,
    crop1: &gst::Element,
    compositor_supports_crop: bool,
) {
    let (pos0, pos1) = status.get_positions();

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
        let mut status = Status::new(settings.input.width, settings.input.height);

        //TODO refactor this logic with main

        gst::init().unwrap();
        let pipeline_srt = get_srt(settings);

        let pipeline = gst::parse::launch(&pipeline_srt)
            .unwrap()
            .downcast::<gst::Pipeline>()
            .unwrap();

        let mixer = pipeline.by_name("mix").unwrap();
        let crop0 = pipeline.by_name("crop0").unwrap();
        let crop1 = pipeline.by_name("crop1").unwrap();
        let mixer_sink_0_pad = mixer.static_pad("sink_0").unwrap();
        let mixer_sink_1_pad = mixer.static_pad("sink_1").unwrap();

        let update_mixer_fn = |status: &Status| {
            update_mixer(
                status,
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
        status.zoom_in();
        update_mixer_fn(&status);
        assert!(wait(&bus));

        status.zoom_in();
        update_mixer_fn(&status);
        assert!(wait(&bus));

        //top left
        status.move_pos(-30, -10);
        status.zoom_in();
        update_mixer_fn(&status);
        assert!(wait(&bus));

        //bottom righ
        status.move_pos(status.width, status.height);
        update_mixer_fn(&status);
        assert!(wait(&bus));

        //bottom left
        status.move_pos(-1 * status.width, 34);
        update_mixer_fn(&status);
        assert!(wait(&bus));

        //top rigth
        status.move_pos(24, -1 * status.height);
        update_mixer_fn(&status);
        assert!(wait(&bus));

        // zoom out
        status.zoom_out();
        status.zoom_out();
        status.zoom_out();
        status.zoom_out();
        update_mixer_fn(&status);
        assert!(wait(&bus));

        //bottom left
        status.move_pos(-110, -100);
        update_mixer_fn(&status);
        assert!(wait(&bus));

        //top rigth
        status.move_pos(220, 200);
        update_mixer_fn(&status);
        assert!(wait(&bus));

        // 1
        status.move_border_to(0);
        update_mixer_fn(&status);
        assert!(wait(&bus));

        // 2
        let w = status.width;
        status.move_border_to(w);
        update_mixer_fn(&status);
        assert!(wait(&bus));

        // 3
        status.reset_border();
        update_mixer_fn(&status);
        assert!(wait(&bus));

        // 4
        status.move_border(-40);
        update_mixer_fn(&status);
        assert!(wait(&bus));

        // 5
        status.move_border(10);
        update_mixer_fn(&status);
        assert!(wait(&bus));

        // reset
        status.reset();
        update_mixer_fn(&status);
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
