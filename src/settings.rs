use config::{Config, ConfigError, Environment, File};
use serde_derive::Deserialize;

const WIDTH: i32 = 1280;
const HEIGHT: i32 = 720;
const FRAMERATE: &str = "30/1";

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Deserialize, PartialEq, Default)]
pub enum BackendType {
    #[default]
    GL,
    #[cfg(target_os = "linux")]
    VAAPI,
    CPU,
    #[cfg(target_os = "windows")]
    D3D12,
}

#[derive(Debug, Deserialize, PartialEq, Default)]
pub enum InputType {
    #[default]
    Test,
    Camera,
}

fn default_framerate() -> String {
    "30/1".to_string()
}
fn default_width() -> i32 {
    1280
}
fn default_height() -> i32 {
    720
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct Input {
    #[serde(default = "default_width")]
    pub width: i32,
    #[serde(default = "default_height")]
    pub height: i32,
    #[serde(default = "default_framerate")]
    pub framerate: String,
    pub format: Option<String>,
    #[serde(default)]
    pub input: InputType,
    pub pattern: Option<String>,
    pub num_buffers: Option<u32>,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            width: WIDTH,
            height: HEIGHT,
            framerate: FRAMERATE.to_string(),
            format: None,
            input: InputType::default(),
            pattern: None,
            num_buffers: None,
        }
    }
}

impl Input {
    fn is_test(&self) -> bool {
        self.input == InputType::Test
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize, Default)]
pub enum EncoderType {
    identity,
    custom,
    #[default]
    x264enc,
    x265enc,
    rav1enc,
    h266enc,
}

fn default_bitrate() -> u32 {
    2048
}

fn default_decoder() -> String {
    "decodebin3".to_string()
}

#[derive(Debug, Deserialize)]
pub struct Encoder {
    pub kind: EncoderType,
    #[serde(default = "default_bitrate")]
    pub bitrate: u32,
    pub custom: Option<String>,
    #[serde(default = "default_decoder")]
    pub decoder: String,
}
impl Default for Encoder {
    fn default() -> Self {
        Self {
            kind: EncoderType::default(),
            bitrate: default_bitrate(),
            custom: None,
            decoder: default_decoder(),
        }
    }
}
fn default_enc0() -> Encoder {
    Encoder {
        bitrate: 256,
        ..Default::default()
    }
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Settings {
    #[serde(default)]
    pub input: Input,
    #[serde(default = "default_enc0")]
    pub encoder0: Encoder,
    #[serde(default)]
    pub encoder1: Encoder,
    #[serde(default)]
    pub backend: BackendType,
    #[serde(default)]
    pub sidebyside: bool,
    #[serde(default)]
    pub nooutput: bool,
    #[serde(default)]
    pub debug: bool,
    #[serde(default = "default_true")]
    pub metrics: bool,
}
impl Default for Settings {
    fn default() -> Self {
        let input = Input::default();
        let encoder0 = default_enc0();
        let encoder1 = Encoder::default();
        let backend = BackendType::default();

        Self {
            input,
            encoder0,
            encoder1,
            backend,
            sidebyside: false,
            nooutput: false,
            debug: false,
            metrics: true,
        }
    }
}
impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name("config.toml").required(false))
            .add_source(Environment::with_prefix("CODECCOMP").separator("__"))
            .build()?;

        s.try_deserialize()
    }

    pub fn get_pipeline_src(&self) -> String {
        let width = self.input.width;
        let height = self.input.height;
        let framerate = &self.input.framerate;
        let format = self
            .input
            .format
            .clone()
            .map(|s| format!("! video/x-raw, format={}", s))
            .unwrap_or_default();
        let num_buffers = self
            .input
            .num_buffers
            .map(|s| format!(" num-buffers={}", s))
            .unwrap_or_default();

        if self.input.is_test() {
            // pattern=smpte
            let pattern = self
                .input
                .pattern
                .clone()
                .unwrap_or("mandelbrot".to_string());

            format!("gltestsrc is-live=1 pattern={pattern} {num_buffers} name=src  ! video/x-raw(memory:GLMemory), framerate={framerate}, width={width}, height={height}, pixel-aspect-ratio=1/1 ! glcolorconvert ! gldownload {format}")
        } else {
            let src = if cfg!(target_os = "linux") {
                "v4l2src"
            } else if cfg!(target_os = "windows") {
                "mfvideosrc"
            } else {
                unimplemented!()
            };

            format!("{src} {num_buffers} ! image/jpeg, width={width}, height={height}, framerate={framerate} ! jpegdec ! videoconvertscale ! videorate {format}")
        }
    }

    pub fn get_pipeline_enc0(&self) -> String {
        self.get_pipeline_enc(&self.encoder0)
    }

    pub fn get_pipeline_enc1(&self) -> String {
        self.get_pipeline_enc(&self.encoder1)
    }

    fn get_pipeline_enc(&self, enc: &Encoder) -> String {
        let bitrate = enc.bitrate;
        match enc.kind {
            EncoderType::identity => "identity".to_string(),
            EncoderType::custom => enc.custom.clone().expect("costom encoder w/o custom value"),
            EncoderType::x264enc => {
                format!("x264enc bitrate={bitrate} tune=zerolatency speed-preset=ultrafast threads=4 key-int-max=2560 b-adapt=0 vbv-buf-capacity=120")
                // constrained-baseline
            }
            EncoderType::x265enc => {
                format!("x265enc bitrate={bitrate} tune=zerolatency speed-preset=ultrafast key-int-max=2560")
            }
            EncoderType::rav1enc => {
                format!("rav1enc bitrate={bitrate} low-latency=1 max-key-frame-interval=715827882 speed-preset=10")
            }
            EncoderType::h266enc => {
                unimplemented!();
            }
        }
    }

    pub fn get_enc0_name(&self) -> String {
        self.get_enc_name(&self.encoder0)
    }

    pub fn get_enc1_name(&self) -> String {
        self.get_enc_name(&self.encoder1)
    }

    fn get_enc_name(&self, enc: &Encoder) -> String {
        let bitrate = enc.bitrate;
        match enc.kind {
            EncoderType::identity => "identity".to_string(),
            EncoderType::custom => {
                let c = enc.custom.clone().expect("custom encoder w/o custom value");
                format!("c {}", c.chars().take(10).collect::<String>())
            }

            EncoderType::x264enc => {
                format!("x264enc bitrate={bitrate}")
            }
            EncoderType::x265enc => {
                format!("x265enc bitrate={bitrate}")
            }
            EncoderType::rav1enc => {
                format!("rav1enc bitrate={bitrate}")
            }
            EncoderType::h266enc => {
                unimplemented!();
            }
        }
    }

    pub fn get_pipeline_dec0(&self) -> String {
        self.get_pipeline_dec(&self.encoder0)
    }

    pub fn get_pipeline_dec1(&self) -> String {
        self.get_pipeline_dec(&self.encoder1)
    }

    fn get_pipeline_dec(&self, enc: &Encoder) -> String {
        enc.decoder.clone()
    }

    pub fn get_pipeline_compositor(&self) -> &str {
        match self.backend {
            BackendType::GL => "glvideomixer",
            #[cfg(target_os = "linux")]
            BackendType::VAAPI => "vacompositor",
            BackendType::CPU => "compositor",
            #[cfg(target_os = "windows")]
            BackendType::D3D12 => "d3d12compositor",
        }
    }

    pub fn gst_pipeline_compositor_supports_crop(&self) -> bool {
        match self.backend {
            BackendType::GL => {
                // https://gitlab.freedesktop.org/gstreamer/gstreamer/-/merge_requests/2669
                true
            }
            _ => false,
        }
    }

    pub fn get_metrics_font(&self) -> String {
        "Consolas 10".to_string()
    }

    pub fn get_pipeline_sink(&self) -> String {
        let width = self.input.width;
        let height = self.input.height;
        let framerate = &self.input.framerate;
        let caps = format!("video/x-raw,framerate={framerate},width={width}, height={height}, pixel-aspect-ratio=1/1");

        let videosink = if cfg!(target_os = "linux") {
            "xvimagesink"
        } else if cfg!(target_os = "windows") {
            "d3d12videosink"
        } else {
            unimplemented!()
        };

        if self.nooutput {
            format!("{caps} ! fakesink sync=false")
        } else {
            format!("{caps} ! {videosink} sync=false")
        }
    }

    pub fn get_framerate(&self) -> (u64, u64) {
        let parts: Vec<&str> = self.input.framerate.split('/').collect();

        if parts.len() == 2 {
            if let (Ok(numerator), Ok(denominator)) =
                (parts[0].parse::<u64>(), parts[1].parse::<u64>())
            {
                return (numerator, denominator);
            }
        }

        panic!("framerate format must be num/den as \"30/1\"");
    }
}
// TODO: do settings.rs GStreamer agnostic

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_framerate() {
        let mut s = Settings::default();
        let (fps_n, fps_d) = s.get_framerate();

        assert_eq!(fps_n, 30, "framerate num");
        assert_eq!(fps_d, 1, "framerate den");

        s.input.framerate = "30000/1001".to_string();
        let (fps_n, fps_d) = s.get_framerate();

        assert_eq!(fps_n, 30000, "framerate num");
        assert_eq!(fps_d, 1001, "framerate den");
    }
}
