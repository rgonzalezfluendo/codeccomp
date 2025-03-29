# GStreamer issues

## glvideomixer sink_0::width=0

command and error:
```
G_DEBUG=fatal-warnings gst-launch-1.0 -vm gltestsrc !  "video/x-raw(memory:GLMemory), width=1280, height=720" ! m.sink_0 glvideomixer name=m sink_0::crop-right=1280 sink_0::width=0  ! "video/x-raw, width=1280, height=720" ! xvimagesink
...
(gst-launch-1.0:2579028): GStreamer-Video-CRITICAL **: 00:29:36.982: gst_video_calculate_display_ratio: assertion 'num > 0' failed
```

tested with `main` (`d8a85e3793`) and `1.24.12` on Arch Linux

Also `sink_0::width=0` is not processed by glvideomixer, vacompositor or compositor.



## vacompositor wrong output

with a zoom_in, left video does not reach the half.

```
GST_DEBUG="*glvideomixer*:8" gst-launch-1.0  gltestsrc is-live=1 pattern=mandelbrot name=src num-buffers=1000 !  "video/x-raw(memory:GLMemory), width=1280, height=720" ! glcolorconvert ! gldownload ! queue ! x264enc tune=zerolatency speed-preset=ultrafast threads=4 key-int-max=2560 b-adapt=0 vbv-buf-capacity=120 ! avdec_h264 ! videocrop right=640 ! m.sink_0 vacompositor name=m sink_0::width=704 sink_0::height=792 sink_0::xpos=-64 sink_0::ypos=-36 ! "video/x-raw, width=1280, height=720" ! xvimagesink
```

or w/o enc/dec
```
GST_DEBUG="*glvideomixer*:8" gst-launch-1.0  gltestsrc is-live=1 pattern=mandelbrot name=src num-buffers=1000 !  "video/x-raw(memory:GLMemory), width=1280, height=720" ! glcolorconvert ! gldownload ! queue ! video/x-raw,format=Y444 ! videocrop right=640 ! m.sink_0 vacompositor name=m sink_0::width=704 sink_0::height=792 sink_0::xpos=-64 sink_0::ypos=-36 ! "video/x-raw, width=1280, height=720" ! xvimagesink
```


Issue with libva/mesa?

```
[58092.836463][ctx       none]=========vaMapBuffer ret = VA_STATUS_SUCCESS, success (no error)
[58092.836463][ctx 0x00000002]  element[0] =
[58092.836465][ctx 0x00000002]  --VAProcPipelineParameterBuffer
[58092.836466][ctx 0x00000002]    surface = 0x00000005
[58092.836466][ctx 0x00000002]    surface_region
[58092.836467][ctx 0x00000002]      x = 0
[58092.836467][ctx 0x00000002]      y = 0
[58092.836467][ctx 0x00000002]      width = 640
[58092.836468][ctx 0x00000002]      height = 720
[58092.836468][ctx 0x00000002]    surface_color_standard = 0
[58092.836468][ctx 0x00000002]    output_region
[58092.836469][ctx 0x00000002]      x = -64
[58092.836469][ctx 0x00000002]      y = -36
[58092.836470][ctx 0x00000002]      width = 704
[58092.836470][ctx 0x00000002]      height = 792
[58092.836471][ctx 0x00000002]    output_background_color = 0xff000000
[58092.836471][ctx 0x00000002]    output_color_standard = 0
[58092.836471][ctx 0x00000002]    pipeline_flags = 0x00000000
[58092.836472][ctx 0x00000002]    filter_flags = 0x00000000
[58092.836472][ctx 0x00000002]    num_filters = 0
[58092.836473][ctx 0x00000002]    filters = (nil)
[58092.836474][ctx 0x00000002]    num_forward_references = 0x00000000
[58092.836474][ctx 0x00000002]    num_backward_references = 0x00000000
```

Note issue only with `avdec_h264`. With `vah264dec` and `GST_VIDEO_CROP_META_API_TYPE` patch vacompositor works correctly.


Note issue only with `video/x-raw,format=Y444`. `video/x-raw,format=I420` fixes it.

https://gitlab.freedesktop.org/gstreamer/gstreamer/-/issues/4245

## compositor and vacompositor video out of the box

with vacompositor
```
GST_DEBUG="*glvideomixer*:8" gst-launch-1.0  gltestsrc is-live=1 pattern=mandelbrot name=src num-buffers=1000 !  "video/x-raw(memory:GLMemory), width=1280, height=720" ! glcolorconvert ! gldownload ! queue ! video/x-raw,format=Y444 ! videocrop right=640 ! m.sink_0 vacompositor name=m sink_0::width=704 sink_0::height=792 sink_0::xpos=-864 sink_0::ypos=-36 ! "video/x-raw, width=1280, height=720" ! xvimagesink
...
[Mon Feb 24 13:41:31 2025] amdgpu 0000:64:00.0: amdgpu: Dumping IP State
[Mon Feb 24 13:41:31 2025] amdgpu 0000:64:00.0: amdgpu: Dumping IP State Completed
[Mon Feb 24 13:41:31 2025] amdgpu 0000:64:00.0: amdgpu: ring gfx_0.0.0 timeout, signaled seq=54114692, emitted seq=54114694
[Mon Feb 24 13:41:31 2025] amdgpu 0000:64:00.0: amdgpu: Process information: process gst-launch-1.0 pid 3640855 thread gst-launch:cs0 pid 3640875
[Mon Feb 24 13:41:31 2025] amdgpu 0000:64:00.0: amdgpu: Starting gfx_0.0.0 ring reset
[Mon Feb 24 13:41:33 2025] amdgpu 0000:64:00.0: amdgpu: MES failed to respond to msg=RESET
[Mon Feb 24 13:41:33 2025] [drm:amdgpu_mes_reset_legacy_queue [amdgpu]] *ERROR* failed to reset legacy queue
[Mon Feb 24 13:41:33 2025] amdgpu 0000:64:00.0: amdgpu: Ring gfx_0.0.0 reset failure
[Mon Feb 24 13:41:33 2025] amdgpu 0000:64:00.0: amdgpu: GPU reset begin!
[Mon Feb 24 13:41:35 2025] amdgpu 0000:64:00.0: amdgpu: MES failed to respond to msg=REMOVE_QUEUE
[Mon Feb 24 13:41:35 2025] [drm:amdgpu_mes_unmap_legacy_queue [amdgpu]] *ERROR* failed to unmap legacy queue
[Mon Feb 24 13:41:36 2025] [drm:gfx_v11_0_hw_fini [amdgpu]] *ERROR* failed to halt cp gfx
[Mon Feb 24 13:41:36 2025] amdgpu 0000:64:00.0: amdgpu: MODE2 reset
[Mon Feb 24 13:41:36 2025] amdgpu 0000:64:00.0: amdgpu: GPU reset succeeded, trying to resume
[Mon Feb 24 13:41:36 2025] [drm] PCIE GART of 512M enabled (table at 0x0000008000900000).
[Mon Feb 24 13:41:36 2025] amdgpu 0000:64:00.0: amdgpu: SMU is resuming...
[Mon Feb 24 13:41:36 2025] amdgpu 0000:64:00.0: amdgpu: SMU is resumed successfully!
[Mon Feb 24 13:41:36 2025] [drm] DMUB hardware initialized: version=0x08004800
[Mon Feb 24 13:41:37 2025] amdgpu 0000:64:00.0: amdgpu: ring gfx_0.0.0 uses VM inv eng 0 on hub 0
[Mon Feb 24 13:41:37 2025] amdgpu 0000:64:00.0: amdgpu: ring comp_1.0.0 uses VM inv eng 1 on hub 0
[Mon Feb 24 13:41:37 2025] amdgpu 0000:64:00.0: amdgpu: ring comp_1.1.0 uses VM inv eng 4 on hub 0
[Mon Feb 24 13:41:37 2025] amdgpu 0000:64:00.0: amdgpu: ring comp_1.2.0 uses VM inv eng 6 on hub 0
[Mon Feb 24 13:41:37 2025] amdgpu 0000:64:00.0: amdgpu: ring comp_1.3.0 uses VM inv eng 7 on hub 0
[Mon Feb 24 13:41:37 2025] amdgpu 0000:64:00.0: amdgpu: ring comp_1.0.1 uses VM inv eng 8 on hub 0
[Mon Feb 24 13:41:37 2025] amdgpu 0000:64:00.0: amdgpu: ring comp_1.1.1 uses VM inv eng 9 on hub 0
[Mon Feb 24 13:41:37 2025] amdgpu 0000:64:00.0: amdgpu: ring comp_1.2.1 uses VM inv eng 10 on hub 0
[Mon Feb 24 13:41:37 2025] amdgpu 0000:64:00.0: amdgpu: ring comp_1.3.1 uses VM inv eng 11 on hub 0
[Mon Feb 24 13:41:37 2025] amdgpu 0000:64:00.0: amdgpu: ring sdma0 uses VM inv eng 12 on hub 0
[Mon Feb 24 13:41:37 2025] amdgpu 0000:64:00.0: amdgpu: ring vcn_unified_0 uses VM inv eng 0 on hub 8
[Mon Feb 24 13:41:37 2025] amdgpu 0000:64:00.0: amdgpu: ring jpeg_dec uses VM inv eng 1 on hub 8
[Mon Feb 24 13:41:37 2025] amdgpu 0000:64:00.0: amdgpu: ring mes_kiq_3.1.0 uses VM inv eng 13 on hub 0
[Mon Feb 24 13:41:37 2025] amdgpu 0000:64:00.0: amdgpu: GPU reset(4) succeeded!
[Mon Feb 24 13:41:37 2025] [drm:amdgpu_cs_ioctl [amdgpu]] *ERROR* Failed to initialize parser -125!
[Mon Feb 24 13:41:38 2025] rfkill: input handler enabled
[Mon Feb 24 13:41:39 2025] rfkill: input handler disabled
[Mon Feb 24 13:41:50 2025] rfkill: input handler enabled
[Mon Feb 24 13:41:50 2025] rfkill: input handler disabled
```

or with compositor (TO BE CHECKED)
```
gst-launch-1.0  gltestsrc is-live=1 pattern=mandelbrot name=src num-buffers=1000 !  "video/x-raw(memory:GLMemory), width=1280, height=720" ! glcolorconvert ! gldownload ! queue ! video/x-raw,format=Y444 ! videocrop right=1280 ! m.sink_0 compositor name=m sink_0::width=1280 sink_0::height=720 sink_0::xpos=1270 sink_0::ypos=603 ! "video/x-raw, width=1280, height=720" ! xvimagesink
...
WARNING: erroneous pipeline: could not link queue0 to videocrop0 with caps video/x-raw, format=(string)Y444
```

## d3d12compositor

TBD: It doesn't support in-place transform using crop meta as vacompositor and also negative offsets



## New vulkan backend

```
        gltestsrc is-live=1 pattern=mandelbrot name=src num-buffers=1000 ! video/x-raw(memory:GLMemory), framerate=30/1, width=1280, height=720, pixel-aspect-ratio=1/1 ! glcolorconvert ! gldownload ! queue ! tee name=tee_src
        tee_src.src_0 ! queue name=enc0 ! x264enc bitrate=256 tune=zerolatency speed-preset=ultrafast threads=4 key-int-max=2560 b-adapt=0 vbv-buf-capacity=120 ! video/x-h264,profile=high-4:4:4 ! queue name=dec0 !
        decodebin3 ! videocrop name=crop0 ! queue name=end0 ! vulkanupload ! vulkancolorconvert ! mix.sink_0
        tee_src.src_1 ! queue name=enc1 ! x264enc bitrate=2048 tune=zerolatency speed-preset=ultrafast threads=4 key-int-max=2560 b-adapt=0 vbv-buf-capacity=120 ! video/x-h264,profile=high-4:4:4 ! queue name=dec1 !
        decodebin3 ! videocrop name=crop1 ! queue name=end1 ! vulkanupload ! vulkancolorconvert ! mix.sink_1
        vulkanoverlaycompositor  name=mix  ! vulkancolorconvert ! vulkandownload !
        video/x-raw,framerate=30/1,width=1280, height=720, pixel-aspect-ratio=1/1 ! fpsdisplaysink video-sink=xvimagesink sync=false
```
Error: could not link vulkancolorconvert0 to mix


```
gstdump gst-launch-1.0 gltestsrc is-live=1 pattern=mandelbrot name=src num-buffers=1000 ! "video/x-raw(memory:GLMemory), framerate=30/1, width=1280, height=720, pixel-aspect-ratio=1/1" ! glcolorconvert ! gldownload ! queue ! x264enc bitrate=2048 tune=zerolatency speed-preset=ultrafast threads=4 key-int-max=2560 b-adapt=0 vbv-buf-capacity=120 ! queue ! decodebin3 ! queue ! vulkanupload ! vulkancolorconvert ! fakesink
ERROR: from element /GstPipeline:pipeline0/GstGLTestSrc:src: Internal data stream error.
Additional debug info:
../gstreamer/subprojects/gstreamer/libs/gst/base/gstbasesrc.c(3177): gst_base_src_loop (): /GstPipeline:pipeline0/GstGLTestSrc:src:
streaming stopped, reason not-negotiated (-4)
```

## originalbuffer: make originalbuffermeta avaliabe out of the crate

https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs/-/merge_requests/2139


# GStreamer notes

## decodebin3 ! videocrop ! vacompositor uses avdec_h264


```
gst-launch-1.0  gltestsrc is-live=1 pattern=mandelbrot name=src num-buffers=50 !  "video/x-raw(memory:GLMemory), width=1280, height=720" ! glcolorconvert ! gldownload ! queue ! x264enc tune=zerolatency speed-preset=ultrafast threads=4 key-int-max=2560 b-adapt=0 vbv-buf-capacity=120 ! decodebin3 ! videocrop right=640 ! m.sink_0 vacompositor name=m sink_0::width=704 sink_0::height=792 sink_0::xpos=-64 sink_0::ypos=-36 ! "video/x-raw, width=1280, height=720" ! xvimagesink
```

From logs
```
0:00:00.204321455 3677690 0x75155c000da0 LOG             decodebin3 gstdecodebin3.c:929:check_parser_caps_filter: Trying factory 2 vah264dec
0:00:00.204339375 3677690 0x75155c000da0 LOG             decodebin3 gstdecodebin3.c:942:check_parser_caps_filter:<decodebin3-0> Can NOT intersect video/x-h264, alignment=(string)au, stream-format=(string)byte-stream, parsed=(boolean)true, level=(string)3.1, profile=(string)high-4:4:4, width=(int)1280, height=(int)720, pixel-aspect-ratio=(fraction)1/1, framerate=(fraction)30/1, interlace-mode=(string)progressive, colorimetry=(string)bt709, chroma-site=(string)mpeg2, coded-picture-structure=(string)frame, chroma-format=(string)4:4:4, bit-depth-luma=(uint)8, bit-depth-chroma=(uint)8, lcevc=(boolean)false with video/x-h264, profile=(string){ constrained-baseline, baseline, main, extended, high, progressive-high, constrained-high }, width=(int)[ 1, 4096 ], height=(int)[ 1, 4096 ], alignment=(string)au, stream-format=(string){ avc, avc3, byte-stream }
```

Note: profile=high-4:4:4 vs profile=(string){ constrained-baseline, baseline, main, extended, high, progressive-high, constrained-high } supported by vah264

Solution add format before the x264enc or profile after it
```
gst-launch-1.0  gltestsrc is-live=1 pattern=mandelbrot name=src num-buffers=50 !  "video/x-raw(memory:GLMemory), width=1280, height=720" ! glcolorconvert ! gldownload ! queue ! video/x-raw,format=I420 ! x264enc tune=zerolatency speed-preset=ultrafast threads=4 key-int-max=2560 b-adapt=0 vbv-buf-capacity=120 ! decodebin3 ! videocrop right=640 ! m.sink_0 vacompositor name=m sink_0::width=704 sink_0::height=792 sink_0::xpos=-64 sink_0::ypos=-36 ! "video/x-raw, width=1280, height=720" ! xvimagesink
or
gst-launch-1.0  gltestsrc is-live=1 pattern=mandelbrot name=src num-buffers=50 !  "video/x-raw(memory:GLMemory), width=1280, height=720" ! glcolorconvert ! gldownload ! queue ! x264enc tune=zerolatency speed-preset=ultrafast threads=4 key-int-max=2560 b-adapt=0 vbv-buf-capacity=120 ! video/x-h264,profile=constrained-baseline ! decodebin3 ! videocrop right=640 ! m.sink_0 vacompositor name=m sink_0::width=704 sink_0::height=792 sink_0::xpos=-64 sink_0::ypos=-36 ! "video/x-raw, width=1280, height=720" ! xvimagesink
```

