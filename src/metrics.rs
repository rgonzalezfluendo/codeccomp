use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use gst::prelude::*;
use human_bytes::human_bytes;

use crate::Settings;

#[derive(Default)]
pub struct EncMetrics {
    num_buffers: u64,
    num_bytes: u64,
    time_last_buffer: Option<SystemTime>,
    total_processing_time: Duration,
}

#[derive(Default)]
pub struct Metrics {
    enc0: EncMetrics,
    enc1: EncMetrics,
}

pub fn add_probe(pipeline: &gst::Pipeline, metrics: Arc<Mutex<Metrics>>, settings: &Settings) {
    add_raw_identity_probe(pipeline, metrics.clone(), settings);
    add_encoder_probes(pipeline, metrics.clone(), settings);
}

fn add_raw_identity_probe(
    pipeline: &gst::Pipeline,
    metrics: Arc<Mutex<Metrics>>,
    settings: &Settings,
) {
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
            let num_bytes0 = human_bytes(num_bytes0 as f64);
            let processing_time0 = metrics.enc0.total_processing_time / num_buffers0 as u32;

            let bitrate1 = human_bytes((fps * num_bytes1) as f64 / num_buffers1 as f64);
            let num_bytes1 = human_bytes(num_bytes1 as f64);
            let processing_time1 = metrics.enc1.total_processing_time / num_buffers1 as u32;

            let text = format!(
                r#"
{:->20}{:>37}{:->20}
{:->20}{:>37}{:->20}
{:->18}/s{:>37}{:->18}/s
{:->20?}{:>37}{:->20?}
"#,
                enc0_name,
                "",
                enc1_name,
                num_bytes0,
                "",
                num_bytes1,
                bitrate0,
                "",
                bitrate1,
                processing_time0,
                "",
                processing_time1
            );
            println!("{}", text);
        }

        gst::PadProbeReturn::Ok
    });
}

fn add_encoder_probes(pipeline: &gst::Pipeline, metrics: Arc<Mutex<Metrics>>, settings: &Settings) {
    let enc0 = pipeline.by_name("enc0").unwrap();
    let dec0 = pipeline.by_name("dec0").unwrap();
    let enc1 = pipeline.by_name("enc1").unwrap();
    let dec1 = pipeline.by_name("dec1").unwrap();

    let enc0_src_pad = enc0.static_pad("src").unwrap();
    let enc1_src_pad = enc1.static_pad("src").unwrap();
    let dec0_sink_pad = dec0.static_pad("sink").unwrap();
    let dec1_sink_pad = dec1.static_pad("sink").unwrap();

    {
        let metrics = metrics.clone();
        enc0_src_pad.add_probe(gst::PadProbeType::BUFFER, move |_, probe_info| {
            let Some(_) = probe_info.buffer() else {
                return gst::PadProbeReturn::Ok;
            };

            let mut metrics = metrics.lock().unwrap();
            metrics.enc0.time_last_buffer = Some(SystemTime::now());

            gst::PadProbeReturn::Ok
        });
    }

    {
        let metrics = metrics.clone();

        dec0_sink_pad.add_probe(gst::PadProbeType::BUFFER, move |_, probe_info| {
            let Some(_) = probe_info.buffer() else {
                return gst::PadProbeReturn::Ok;
            };

            let mut metrics = metrics.lock().unwrap();
            let diff = metrics.enc0.time_last_buffer.unwrap().elapsed().unwrap();
            metrics.enc0.total_processing_time += diff;

            gst::PadProbeReturn::Ok
        });
    }

    //TODO add test and refactor instead of copy&paste
    {
        let metrics = metrics.clone();
        enc1_src_pad.add_probe(gst::PadProbeType::BUFFER, move |_, probe_info| {
            let Some(_) = probe_info.buffer() else {
                return gst::PadProbeReturn::Ok;
            };

            let mut metrics = metrics.lock().unwrap();
            metrics.enc1.time_last_buffer = Some(SystemTime::now());

            gst::PadProbeReturn::Ok
        });
    }

    {
        let metrics = metrics.clone();

        dec1_sink_pad.add_probe(gst::PadProbeType::BUFFER, move |_, probe_info| {
            let Some(_) = probe_info.buffer() else {
                return gst::PadProbeReturn::Ok;
            };

            let mut metrics = metrics.lock().unwrap();
            let diff = metrics.enc1.time_last_buffer.unwrap().elapsed().unwrap();
            metrics.enc1.total_processing_time += diff;

            gst::PadProbeReturn::Ok
        });
    }
}
