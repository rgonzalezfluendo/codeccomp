use config::{Config, ConfigError, Environment, File};
use serde_derive::Deserialize;

const WIDTH: i32 = 1280;
const HEIGHT: i32 = 720;
const FRAMERATE: &str = "30/1";

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Deserialize, PartialEq)]
#[derive(Default)]
pub enum BackendType {
    #[default]
    GL,
    VAAPI,
    CPU,
    D3D11,
    D3D12,
}

#[derive(Debug, Deserialize, PartialEq)]
#[derive(Default)]
pub enum InputType {
    #[default]
    Test,
    Camera,
}

#[derive(Debug, Deserialize)]
pub struct Input {
    pub width: i32,
    pub height: i32,
    #[serde(default)]
    pub framerate: String,
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
#[derive(Debug, Deserialize)]
#[derive(Default)]
pub enum EncoderType {
    #[default]
    x264enc,
    x265enc,
    rav1enc,
    h266enc,
}

#[derive(Debug, Deserialize)]
pub struct Encoder {
    pub kind: EncoderType,
    pub bitrate: u32,
}
impl Default for Encoder {
    fn default() -> Self {
        Self {
            kind: EncoderType::default(),
            bitrate: 2048,
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
    pub output: bool,
}
impl Default for Settings {
    fn default() -> Self {
        let input = Input::default();
        let encoder0 = default_enc0();
        let encoder1 = Encoder::default();
        let backend = BackendType::default();
        let output = true;

        Self {
            input,
            encoder0,
            encoder1,
            backend,
            output,
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

        if self.input.is_test() {
            // pattern=smpte
            let pattern = self
                .input
                .pattern
                .clone()
                .unwrap_or("mandelbrot".to_string());
            let num_buffers = self.input.num_buffers.unwrap_or(1000);

            format!("gltestsrc is-live=1 pattern={pattern} name=src num-buffers={num_buffers} ! video/x-raw(memory:GLMemory), framerate={framerate}, width={width}, height={height}, pixel-aspect-ratio=1/1 ! glcolorconvert ! gldownload")
        } else {
            //TODO no fix caps use generic
            format!("v4l2src ! image/jpeg, width={width}, height={height}, framerate={framerate} ! jpegdec ! videoconvertscale ! videorate ")
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
            EncoderType::x264enc => {
                format!("x264enc bitrate={bitrate} tune=zerolatency speed-preset=ultrafast threads=4 key-int-max=2560 b-adapt=0 vbv-buf-capacity=120")
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

    pub fn get_pipeline_compositor(&self) -> &str {
        match self.backend {
            BackendType::GL => "glvideomixer",
            BackendType::VAAPI => "vacompositor",
            BackendType::CPU => "compositor",
            BackendType::D3D11 => {
                unimplemented!();
            }
            BackendType::D3D12 => {
                unimplemented!();
            }
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

    pub fn get_pipeline_sink(&self) -> String {
        let width = self.input.width;
        let height = self.input.height;
        format!("video/x-raw,framerate=30/1,width={width}, height={height}, pixel-aspect-ratio=1/1 ! xvimagesink sync=false")
    }
}
