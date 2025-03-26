use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use gst::prelude::*;
use human_bytes::human_bytes;
#[cfg(target_os = "linux")]
use procfs::process::Process;

use crate::Settings;
use gstoriginalbuffer::originalbuffermeta::OriginalBufferMeta;

use rust_vmaf::{
    model::{VmafModel, VmafModelConfig},
    picture::{Picture, Yuv420Planar},
    ContextConfig, PollMethod, VmafContext,
};

static VMAF_MODEL: Lazy<VmafModel> = Lazy::new(|| {
    VmafModel::model_load("vmaf_v0.6.1", VmafModelConfig::default())
        .expect("Failed to load VMAF model")
});

#[derive(Default)]
pub struct EncMetrics {
    name: String,
    num_buffers: u64,
    num_bytes: u64,
    time_last_buffers: VecDeque<Instant>,
    max_buffers_inside: usize,
    total_processing_time: Duration,
    threads_utime: u64,
    threads_stime: u64,
    vmaf_ctx: Option<VmafContext<rust_vmaf::Process>>,
    vmaf_score: f64,
}

#[derive(Default)]
pub struct Metrics {
    fps_n: u64,
    fps_d: u64,
    enc0: EncMetrics,
    enc1: EncMetrics,
}

impl Metrics {
    pub fn new(s: &Settings) -> Self {
        let (fps_n, fps_d) = s.get_framerate();

        let cfg = ContextConfig::default();
        let ctx0 = VmafContext::new(cfg).unwrap();
        let vmaf_ctx0 = ctx0
            .use_features_from_model(&VMAF_MODEL)
            .unwrap()
            .start_processing();

        let ctx1 = VmafContext::new(cfg).unwrap();
        let vmaf_ctx1 = ctx1
            .use_features_from_model(&VMAF_MODEL)
            .unwrap()
            .start_processing();

        let enc0 = EncMetrics {
            name: s.get_enc0_name(),
            time_last_buffers: VecDeque::with_capacity(25),
            vmaf_ctx: Some(vmaf_ctx0),
            ..Default::default()
        };
        let enc1 = EncMetrics {
            name: s.get_enc1_name(),
            time_last_buffers: VecDeque::with_capacity(25),
            vmaf_ctx: Some(vmaf_ctx1),
            ..Default::default()
        };

        Self {
            fps_n,
            fps_d,
            enc0,
            enc1,
        }
    }

    fn for_enc(&mut self, id: usize) -> &mut EncMetrics {
        match id {
            0 => &mut self.enc0,
            1 => &mut self.enc1,
            _ => unreachable!(),
        }
    }
}

impl EncMetrics {
    pub fn buffer_in(&mut self) {
        self.time_last_buffers.push_back(Instant::now());
        if self.time_last_buffers.len() > self.max_buffers_inside {
            self.max_buffers_inside = self.time_last_buffers.len();
        }
    }

    pub fn buffer_out(&mut self) {
        // Metric calculation does not require input-output buffer association
        if let Some(arrive) = self.time_last_buffers.pop_front() {
            let diff = arrive.elapsed();
            self.total_processing_time += diff;
        } else {
            panic!("output buffer w/o input");
        }
    }

    pub fn avg_processing_time(&self) -> Duration {
        if self.num_buffers != 0 {
            self.total_processing_time / self.num_buffers as u32
        } else {
            Duration::ZERO
        }
    }
}

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.fps_d != 1 {
            unimplemented!();
        }

        writeln!(
            f,
            "{:->20}{:>37}{:->20}",
            &self.enc0.name, "", &self.enc1.name
        )?;
        writeln!(
            f,
            "{:->14}{:>3}{:>3}{:>37}{:->14}{:>3}{:>3}",
            self.enc0.num_buffers,
            self.enc0.max_buffers_inside,
            self.enc0.time_last_buffers.len(),
            "",
            self.enc1.num_buffers,
            self.enc1.max_buffers_inside,
            self.enc1.time_last_buffers.len()
        )?;
        let num_bytes0 = human_bytes(self.enc0.num_bytes as f64);
        let num_bytes1 = human_bytes(self.enc1.num_bytes as f64);
        writeln!(f, "{:->20}{:>37}{:->20}", num_bytes0, "", num_bytes1)?;

        let bitrate0 =
            human_bytes((self.fps_n * self.enc0.num_bytes) as f64 / self.enc0.num_buffers as f64);
        let bitrate1 =
            human_bytes((self.fps_n * self.enc1.num_bytes) as f64 / self.enc1.num_buffers as f64);
        writeln!(f, "{:->18}/s{:>37}{:->18}/s", bitrate0, "", bitrate1)?;

        let processing_time0 = self.enc0.avg_processing_time();
        let processing_time1 = self.enc1.avg_processing_time();
        writeln!(
            f,
            "{:->20?}{:>37}{:->20?}",
            processing_time0, "", processing_time1
        )?;

        let cpu_time0 = self.enc0.threads_utime + self.enc0.threads_stime;
        let cpu_time1 = self.enc1.threads_utime + self.enc1.threads_stime;
        writeln!(
            f,
            "{:->8} clock ticks{:>37}{:->8} clock ticks",
            cpu_time0, "", cpu_time1
        )?;

        write!(
            f,
            "{:->20.4}{:>37}{:->20.4}",
            self.enc0.vmaf_score, "", self.enc1.vmaf_score
        )
    }
}

pub fn add_probe(pipeline: &gst::Pipeline, metrics: Arc<Mutex<Metrics>>, settings: &Settings) {
    add_raw_identity_probe(pipeline, metrics.clone(), settings);
    add_vmaf_probes(pipeline, metrics.clone());
    add_encoder_probes(pipeline, metrics.clone());
}

fn add_raw_identity_probe(
    pipeline: &gst::Pipeline,
    metrics: Arc<Mutex<Metrics>>,
    settings: &Settings,
) {
    //TODO add a setting to disable textoverlay
    let textoverlay = pipeline.by_name("metrics").unwrap();

    let i0 = pipeline.by_name("i0").unwrap();
    let i1 = pipeline.by_name("i1").unwrap();

    let mixer = pipeline.by_name("mix").unwrap();
    let mixer_src_pad = mixer.static_pad("src").unwrap();

    let (fps_n, fps_d) = settings.get_framerate();
    if fps_d != 1 {
        unimplemented!();
    }
    let settings_debug = settings.debug;

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
        if num_buffers1 % fps_n == 0 {
            let (total_utime0, total_stime0, total_utime1, total_stime1) = get_cpu_usage();

            let mut metrics = metrics.lock().unwrap();
            metrics.enc0.threads_utime = total_utime0;
            metrics.enc0.threads_stime = total_stime0;
            metrics.enc1.threads_utime = total_utime1;
            metrics.enc1.threads_stime = total_stime1;

            metrics.enc0.num_bytes = num_bytes0;
            metrics.enc0.num_buffers = num_buffers0;

            metrics.enc1.num_bytes = num_bytes1;
            metrics.enc1.num_buffers = num_buffers1;

            let metrics_string = format!("{metrics}");
            if settings_debug {
                println!("{}", metrics);
            }
            textoverlay.set_property("text", metrics_string);
        }

        gst::PadProbeReturn::Ok
    });
}

fn add_vmaf_probe(pad: &gst::Pad, metrics: Arc<Mutex<Metrics>>, enc_id: usize) {
    let metrics = metrics.clone();
    pad.add_probe(gst::PadProbeType::BUFFER, move |_, probe_info| {
        let Some(buffer) = probe_info.buffer() else {
            dbg!("no buffer");
            return gst::PadProbeReturn::Ok;
        };

        let Some(ometa) = buffer.meta::<OriginalBufferMeta>() else {
            //gst::element_warning!(self, gst::StreamError::Failed, ["Buffer {} is missing the GstOriginalBufferMeta, put originalbuffersave upstream in your pipeline", buffer]);
            dbg!("no meta");
            return gst::PadProbeReturn::Ok;
        };

        let outbuf = ometa.original().copy();

        let map = buffer
            .map_readable()
            .map_err(|_| {
                dbg!("error map");
                gst::FlowError::Error
            })
            .unwrap();

        let outmap = outbuf
            .map_readable()
            .map_err(|_| {
                dbg!("error map");
                gst::FlowError::Error
            })
            .unwrap();

        //TODO avoid hardcoded resolution
        let len = 1280 * 720 + 1280 * 720 / 2;

        let reference = Picture::try_from(
            Yuv420Planar::new_with_combined_planes(&outmap.as_slice()[0..len], 1280, 720).unwrap(),
        )
        .unwrap();

        let target = Picture::try_from(
            Yuv420Planar::new_with_combined_planes(&map.as_slice()[0..len], 1280, 720).unwrap(),
        )
        .unwrap();

        let mut metrics = metrics.lock().unwrap();
        let enc_metrics = metrics.for_enc(enc_id);

        let ctx = enc_metrics.vmaf_ctx.as_mut().unwrap();
        ctx.read_pictures(Some((reference, target))).unwrap();
        let num = enc_metrics.num_buffers as u32;
        if num > 60 {
            let range = Some((num - 15)..(num - 5));
            enc_metrics.vmaf_score = ctx
                .score_pooled(&VMAF_MODEL, PollMethod::Mean, range)
                .unwrap();
        }

        gst::PadProbeReturn::Ok
    });
}

fn add_vmaf_probes(pipeline: &gst::Pipeline, metrics: Arc<Mutex<Metrics>>) {
    let dec0 = pipeline.by_name("ia0").unwrap();
    let dec1 = pipeline.by_name("ia1").unwrap();
    let dec0_sink_pad = dec0.static_pad("sink").unwrap();
    let dec1_sink_pad = dec1.static_pad("sink").unwrap();

    add_vmaf_probe(&dec0_sink_pad, metrics.clone(), 0);
    add_vmaf_probe(&dec1_sink_pad, metrics.clone(), 1);
}

fn add_encoder_probes(pipeline: &gst::Pipeline, metrics: Arc<Mutex<Metrics>>) {
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
            metrics.enc0.buffer_in();

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
            metrics.enc0.buffer_out();

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
            metrics.enc1.buffer_in();

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
            metrics.enc1.buffer_out();

            gst::PadProbeReturn::Ok
        });
    }
}

#[cfg(target_os = "linux")]
fn get_cpu_usage() -> (u64, u64, u64, u64) {
    let my_pid = std::process::id() as i32;
    let process = Process::new(my_pid).unwrap();

    let mut total_utime0: u64 = 0;
    let mut total_stime0: u64 = 0;
    let mut total_utime1: u64 = 0;
    let mut total_stime1: u64 = 0;

    for thread in process.tasks().unwrap().flatten() {
        let stat = thread.stat().unwrap();
        //TODO no hardcode thread names
        if stat.comm == "enc0:src" {
            total_utime0 += stat.utime;
            total_stime0 += stat.stime;
        } else if stat.comm == "enc1:src" {
            total_utime1 += stat.utime;
            total_stime1 += stat.stime;
        }
    }

    (total_utime0, total_stime0, total_utime1, total_stime1)
}

#[cfg(not(target_os = "linux"))]
fn get_cpu_usage() -> (u64, u64, u64, u64) {
    (0, 0, 0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compositor_default() {
        let metrics = Metrics::default();

        assert_eq!(metrics.fps_n, 0, "metrics.fps_n");
        assert_eq!(metrics.fps_d, 0, "metrics.fps_d");
        assert_eq!(metrics.enc0.name, "", "metrics.enc0.name");
        assert_eq!(metrics.enc0.num_buffers, 0, "metrics.enc0.num_buffers");
        assert_eq!(metrics.enc0.num_bytes, 0, "metrics.enc0.num_bytes");
        assert_eq!(
            metrics.enc0.time_last_buffers.len(),
            0,
            "metrics.enc0.time_last_buffers.len == 0"
        );
        assert_eq!(
            metrics.enc0.time_last_buffers.capacity(),
            0,
            "metrics.enc0.time_last_buffers.capacity == 0"
        );
        assert_eq!(
            metrics.enc0.total_processing_time,
            Duration::ZERO,
            "metrics.enc0.total_processing_time"
        );
        assert_eq!(
            metrics.enc0.avg_processing_time(),
            Duration::ZERO,
            "metrics.enc0.total_processing_time"
        );
        assert_eq!(metrics.enc0.threads_utime, 0, "metrics.enc0.threads_utime");
        assert_eq!(metrics.enc0.threads_stime, 0, "metrics.enc0.threads_stime");

        assert_eq!(metrics.enc1.name, "", "metrics.enc0.name");
        assert_eq!(metrics.enc1.num_buffers, 0, "metrics.enc1.num_buffers");
        assert_eq!(metrics.enc1.num_bytes, 0, "metrics.enc1.num_bytes");
        assert_eq!(
            metrics.enc1.time_last_buffers.len(),
            0,
            "metrics.enc1.time_last_buffers.len == 0"
        );
        assert_eq!(
            metrics.enc1.time_last_buffers.capacity(),
            0,
            "metrics.enc1.time_last_buffers.capacity == 0"
        );
        assert_eq!(
            metrics.enc1.total_processing_time,
            Duration::ZERO,
            "metrics.enc1.total_processing_time"
        );
        assert_eq!(metrics.enc1.threads_utime, 0, "metrics.enc1.threads_utime");
        assert_eq!(metrics.enc1.threads_stime, 0, "metrics.enc1.threads_stime");
    }

    #[test]
    fn test_compositor_new() {
        let s = Settings::default();
        let metrics = Metrics::new(&s);

        assert_eq!(metrics.fps_n, 30, "metrics.fps_n");
        assert_eq!(metrics.fps_d, 1, "metrics.fps_d");

        assert_ne!(metrics.enc1.name, "", "metrics.enc0.name not empty");
        assert_eq!(metrics.enc0.num_buffers, 0, "metrics.enc0.num_buffers");
        assert_eq!(metrics.enc0.num_bytes, 0, "metrics.enc0.num_bytes");
        assert_eq!(
            metrics.enc0.time_last_buffers.len(),
            0,
            "metrics.enc0.time_last_buffers.len == 0"
        );
        assert_eq!(
            metrics.enc0.time_last_buffers.capacity(),
            25,
            "metrics.enc0.time_last_buffers.capacity == 25"
        );
        assert_eq!(
            metrics.enc0.total_processing_time,
            Duration::ZERO,
            "metrics.enc0.total_processing_time"
        );
        assert_eq!(metrics.enc0.threads_utime, 0, "metrics.enc0.threads_utime");
        assert_eq!(metrics.enc0.threads_stime, 0, "metrics.enc0.threads_stime");

        assert_ne!(metrics.enc1.name, "", "metrics.enc0.name not empty");
        assert_eq!(metrics.enc1.num_buffers, 0, "metrics.enc1.num_buffers");
        assert_eq!(metrics.enc1.num_bytes, 0, "metrics.enc1.num_bytes");
        assert_eq!(
            metrics.enc1.time_last_buffers.len(),
            0,
            "metrics.enc1.time_last_buffers.len == 0"
        );
        assert_eq!(
            metrics.enc1.time_last_buffers.capacity(),
            25,
            "metrics.enc1.time_last_buffers.capacity == 25"
        );
        assert_eq!(
            metrics.enc1.total_processing_time,
            Duration::ZERO,
            "metrics.enc1.total_processing_time"
        );
        assert_eq!(metrics.enc1.threads_utime, 0, "metrics.enc1.threads_utime");
        assert_eq!(metrics.enc1.threads_stime, 0, "metrics.enc1.threads_stime");
    }

    #[test]
    #[should_panic]
    fn test_buffer_out_no_in() {
        let mut metrics = Metrics::default();

        metrics.enc0.buffer_out();
    }

    #[test]
    #[should_panic]
    fn test_buffer_in_out_out() {
        let mut metrics = Metrics::default();

        metrics.enc0.buffer_in();
        metrics.enc0.buffer_out();
        metrics.enc0.buffer_out();
    }

    #[test]
    fn test_buffer_in_and_out_no() {
        let mut metrics = Metrics::default();

        assert_eq!(
            metrics.enc0.time_last_buffers.len(),
            0,
            "metrics.enc0.time_last_buffers.len == 0"
        );
        assert_eq!(
            metrics.enc0.time_last_buffers.capacity(),
            0,
            "metrics.enc0.time_last_buffers.capacity == 0"
        );
        assert_eq!(
            metrics.enc0.total_processing_time,
            Duration::ZERO,
            "metrics.enc0.total_processing_time"
        );
        assert_eq!(
            metrics.enc1.time_last_buffers.len(),
            0,
            "metrics.enc1.time_last_buffers.len == 0"
        );
        assert_eq!(
            metrics.enc1.time_last_buffers.capacity(),
            0,
            "metrics.enc1.time_last_buffers.capacity == 0"
        );
        assert_eq!(
            metrics.enc1.total_processing_time,
            Duration::ZERO,
            "metrics.enc0.total_processing_time"
        );

        metrics.enc0.buffer_in();

        assert_eq!(
            metrics.enc0.time_last_buffers.len(),
            1,
            "metrics.enc0.time_last_buffers.len == 1"
        );
        assert_ne!(
            metrics.enc0.time_last_buffers.capacity(),
            0,
            "metrics.enc0.time_last_buffers.capacity != 0"
        );
        assert_eq!(
            metrics.enc0.total_processing_time,
            Duration::ZERO,
            "metrics.enc0.total_processing_time"
        );
        assert_eq!(
            metrics.enc1.time_last_buffers.len(),
            0,
            "metrics.enc1.time_last_buffers.len == 0"
        );
        assert_eq!(
            metrics.enc1.time_last_buffers.capacity(),
            0,
            "metrics.enc1.time_last_buffers.capacity == 0"
        );
        assert_eq!(
            metrics.enc1.total_processing_time,
            Duration::ZERO,
            "metrics.enc0.total_processing_time"
        );

        metrics.enc0.buffer_out();
        let t1 = metrics.enc0.total_processing_time;

        assert_eq!(
            metrics.enc0.time_last_buffers.len(),
            0,
            "metrics.enc0.time_last_buffers.len == 0"
        );
        assert_ne!(
            metrics.enc0.time_last_buffers.capacity(),
            0,
            "metrics.enc0.time_last_buffers.capacity != 0"
        );
        assert!(
            metrics.enc0.total_processing_time > Duration::ZERO,
            "metrics.enc0.total_processing_time"
        );
        assert_eq!(
            metrics.enc1.time_last_buffers.len(),
            0,
            "metrics.enc1.time_last_buffers.len == 0"
        );
        assert_eq!(
            metrics.enc1.time_last_buffers.capacity(),
            0,
            "metrics.enc1.time_last_buffers.capacity == 0"
        );
        assert_eq!(
            metrics.enc1.total_processing_time,
            Duration::ZERO,
            "metrics.enc0.total_processing_time"
        );

        metrics.enc0.buffer_in();
        metrics.enc0.buffer_in();
        metrics.enc0.buffer_in();
        assert_eq!(
            metrics.enc0.time_last_buffers.len(),
            3,
            "metrics.enc0.time_last_buffers.len == 0"
        );
        assert_ne!(
            metrics.enc0.time_last_buffers.capacity(),
            0,
            "metrics.enc0.time_last_buffers.capacity != 0"
        );
        assert_eq!(
            metrics.enc0.total_processing_time, t1,
            "metrics.enc0.total_processing_time"
        );
        assert_eq!(
            metrics.enc1.time_last_buffers.len(),
            0,
            "metrics.enc1.time_last_buffers.len == 0"
        );
        assert_eq!(
            metrics.enc1.time_last_buffers.capacity(),
            0,
            "metrics.enc1.time_last_buffers.capacity == 0"
        );
        assert_eq!(
            metrics.enc1.total_processing_time,
            Duration::ZERO,
            "metrics.enc0.total_processing_time"
        );

        metrics.enc0.buffer_out();
        assert!(
            metrics.enc0.total_processing_time > t1,
            "metrics.enc0.total_processing_time"
        );
        let t1 = metrics.enc0.total_processing_time;

        metrics.enc0.buffer_out();
        assert!(
            metrics.enc0.total_processing_time > t1,
            "metrics.enc0.total_processing_time"
        );
        let t1 = metrics.enc0.total_processing_time;

        metrics.enc0.buffer_out();

        assert_eq!(
            metrics.enc0.time_last_buffers.len(),
            0,
            "metrics.enc0.count_buffers_inside == 0"
        );
        assert_ne!(
            metrics.enc0.time_last_buffers.capacity(),
            0,
            "metrics.enc0.time_last_buffers.capacity != 0"
        );
        assert!(
            metrics.enc0.total_processing_time > t1,
            "metrics.enc0.total_processing_time"
        );
        assert_eq!(
            metrics.enc0.max_buffers_inside, 3,
            "metrics.enc0.total_processing_time"
        );
        assert_eq!(
            metrics.enc1.time_last_buffers.len(),
            0,
            "metrics.enc1.count_buffers_inside == 0"
        );
        assert_eq!(
            metrics.enc1.time_last_buffers.capacity(),
            0,
            "metrics.enc1.time_last_buffers.capacity == 0"
        );
        assert_eq!(
            metrics.enc1.total_processing_time,
            Duration::ZERO,
            "metrics.enc0.total_processing_time"
        );
        assert_eq!(
            metrics.enc1.max_buffers_inside, 0,
            "metrics.enc0.total_processing_time"
        );
    }
}
