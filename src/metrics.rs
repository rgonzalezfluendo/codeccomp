use std::sync::{Arc, Mutex};

use gst::prelude::*;
use human_bytes::human_bytes;

use crate::Settings;

#[derive(Default)]
pub struct EncMetrics {
    num_buffers: u64,
    num_bytes: u64,
}

#[derive(Default)]
pub struct Metrics {
    enc0: EncMetrics,
    enc1: EncMetrics,
}

pub fn add_probe(pipeline: &gst::Pipeline, metrics: Arc<Mutex<Metrics>>, settings: &Settings) {
    let i0 = pipeline.by_name("i0").unwrap();
    let i1 = pipeline.by_name("i1").unwrap();

    let mixer = pipeline.by_name("mix").unwrap();
    let mixer_src_pad = mixer.static_pad("src").unwrap();
    let enc0_name = settings.get_enc0_name();
    let enc1_name = settings.get_enc1_name();
    let fps = 30; //TODO

    //TODO use other pad ?
    mixer_src_pad.add_probe(gst::PadProbeType::BUFFER, move |_, probe_info| {
        let Some(_) = probe_info.buffer() else {
            return gst::PadProbeReturn::Ok;
        };

        let stats0 = i0.property::<gst::Structure>("stats");
        let num_bytes0 = stats0.get::<u64>("num-bytes").unwrap();
        let num_buffers0 = stats0.get::<u64>("num-buffers").unwrap();

        let stats1 = i1.property::<gst::Structure>("stats");
        let num_bytes1 = stats1.get::<u64>("num-bytes").unwrap();
        let num_buffers1 = stats1.get::<u64>("num-buffers").unwrap();

        // TODO no hardcode metrics every second
        if num_buffers1 % fps == 0 {
            let mut metrics = metrics.lock().unwrap();
            metrics.enc0.num_bytes = num_bytes0;
            metrics.enc0.num_buffers = num_buffers0;

            metrics.enc1.num_bytes = num_bytes1;
            metrics.enc1.num_buffers = num_buffers1;

            let bitrate0 = human_bytes((fps * num_bytes0) as f64 / num_buffers0 as f64);
            let mut num_bytes0 = human_bytes(num_bytes0 as f64);

            let bitrate1 = human_bytes((fps * num_bytes1) as f64 / num_buffers1 as f64);
            let mut num_bytes1 = human_bytes(num_bytes1 as f64);

            let text = format!(
                r#"
{:->20}{:>37}{:->20}
{:->20}{:>37}{:->20}
{:->18}/s{:>37}{:->18}/s
"#,
                enc0_name, "", enc1_name, num_bytes0, "", num_bytes1, bitrate0, "", bitrate1,
            );
            println!("{}", text);
        }

        gst::PadProbeReturn::Ok
    });
}
