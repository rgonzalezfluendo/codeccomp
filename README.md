# Video Codecs comparator


## GStreamer issues

### glvideomixer sink_0::width=0

check comit `a79556a`: glvideomixer: workaround to handle gst issue when width=0


### vacompositor wrong output

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
