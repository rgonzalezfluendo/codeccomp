# Video Codecs Comparator

![screenshot.png](./doc/screenshot.png)


Simple:

 * Simple is better than complex.
 * No UI (Only Gstreamer NavigationEvent) as  [this sample](https://gitlab.freedesktop.org/gstreamer/gstreamer-rs/-/blob/main/examples/src/bin/zoom.rs)
 * Only one pipeline (sorry for the strings algebra)
 * Versatile and extensible configuration file

![pipeline.png](./doc/pipeline.png)

Nerd version (use `debug=1` to print in stdout):
```
  v4l2src ! image/jpeg, width=1280, height=720, framerate=30/1 ! jpegdec ! videoconvertscale ! videorate  !
  queue ! originalbuffersave ! tee name=tee_src
  tee_src.src_0 ! queue name=enc0 ! x264enc bitrate=256 tune=zerolatency speed-preset=ultrafast threads=4 key-int-max=2560 b-adapt=0 vbv-buf-capacity=120 ! video/x-h264,profile=high-4:4:4 ! queue name=dec0 !
  identity name=i0 ! decodebin3 ! videocrop name=crop0 ! queue name=end0 ! mix.sink_0
  tee_src.src_1 ! queue name=enc1 ! x265enc bitrate=2048 tune=zerolatency speed-preset=ultrafast key-int-max=2560 ! queue name=dec1 !
  identity name=i1 ! decodebin3 ! videocrop name=crop1 ! queue name=end1 ! mix.sink_1
  glvideomixer name=mix  !
  textoverlay name=metrics valignment=bottom font-desc="Consolas 10" !
  video/x-raw,framerate=30/1,width=1280, height=720, pixel-aspect-ratio=1/1 ! xvimagesink sync=false
```

## Configuration Structure

The configuration is structured as follows:

```toml
[input]
width = 1280
height = 720
framerate = "30/1"
format = "YUV"            # If not defined the pipeline negociates the format
input = "Test"            # Values "Test"|"Camera"
pattern = "smpte"         # Only for input Test, check gst-inspect-1.0 gltestsrc for more values
num_buffers = 100         # No num_buffers if not defined

[encoder0]
kind = "x264enc"          # Values "identity"|"custom"|"x264enc"|"x265enc"|"rav1enc"|"h266enc"
bitrate = 2028
custom = ""               # GStreamer encoder with properties to use when kind is "custom"
decoder = "decodebin3"    # GStreamer decoder to use with this encoder. decodebin3 by default.

[encoder1]
kind = "x265enc"
bitrate = 2048
decoder = "decodebin3"

[backend]
backend = "GL"            # Values "GL"|"VAAPI"|"CPU"|"D3D12"

[options]
sidebyside = false        # Initial layout used
nooutput = false          # To use fake sink
debug = false             # To print usefule info in the stdout
metrics = true            # To disable the metrics
```

You can override settings using environment variables:

```
$ CODECCOMP__INPUT__INPUT=Camera CODECCOMP__DEBUG=1 ./codeccomp
```
## Links

### Internal

* [Help](./doc/help.md)

### Alternatives

* https://gitlab.gnome.org/YaLTeR/identity
* https://github.com/IENT/YUView

### GStreamer

* [GStremaer issues](./doc/gst_issues.md)
* https://gitlab.freedesktop.org/gstreamer/gstreamer/-/merge_requests/8537
* https://gitlab.freedesktop.org/gstreamer/cerbero/-/merge_requests/1728

## TODOs:

 * [ ] support diff v4l2src resolutions
 * [ ] more documentation
 * [ ] create status with MouseState and compositor
 * [ ] PSNR and SSIM metrics
 * [ ] VMAF metrics
 * [ ] fix and enable test_tour_vaapi and test_tour_cpu
 * [ ] more tour test with identity encoder and 8x8 input with real assert of the output in the CI (see this commit msg)
 * [ ] CI for Windows

## Future TODOs:

 * [ ] Move logic to GStremaer elements (like a `smartcompositor`)
 * [ ] a new `onlydecodebin` without queues (parsebin and multiqueue from decodebin3 are not needed)
 * [ ] Add vulan backend. Check `vulkanoverlaycompositor`


