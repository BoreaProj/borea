# Borea design preview

A silent promo preview recorded directly from the current
[interactive HTML mockup](../flow/borea-flow.html). The animation and UI are the
actual design, with preview controls and pointer hidden for capture.
Installation progress and networking are simulated; this is a design preview,
not a recording of a working native installer.

- [1080p MP4](borea-design-preview.mp4): H.264, genuine 60 fps, no audio, 36 seconds.
- [Looping GIF](borea-design-preview.gif): 25 fps, a shorter highlight reel sized for GitHub embedding; see metadata for dimensions.
- [Family cards poster](borea-design-poster.png).
- [Restart poster](borea-restart-poster.png).
- [Export metadata and source fingerprint](preview.json).

The sequence shows the boot highlight loop, dot-to-arrow transformation, logo
spring into the corner, network screen, family colour transitions, Arch flavours,
Omarchy trim selections, install highlight progress, and matching restart logo.
The install section is accelerated for the preview. The full video has
short fades at the beginning and end. The GIF condenses the motion highlights
and loops through black.

The source HTML and locked design are unchanged by the recording. Typography is
rendered using the fonts/fallbacks available to the existing standalone mockup.

## Re-recording

`record-preview.cjs` uses Playwright, Chromium and system FFmpeg. It advances
the UI clock and animations one frame at a time, rendering all 2,160 frames
instead of relying on real-time recording. Slow capture cannot drop frames.
Set `PLAYWRIGHT_MODULE` if Playwright is installed outside normal Node module
resolution, and `CHROMIUM_PATH` if Chromium is not at
`/usr/bin/chromium`. Set `FFMPEG_PATH` if system FFmpeg is outside the command path.

Run from the repository root:

```sh
node design/promo/record-preview.cjs
python design/promo/encode-preview.py
```

The recorder writes a temporary 60 fps MP4 capture and records its location in
`capture.json`. The encoder uses system `ffmpeg` and `ffprobe` to create the final MP4,
GIF and `preview.json`. No motion interpolation or duplicated-frame upsampling
is used for the video. These tools are recording dependencies, not dependencies
of Borea's planned live system. No external service upload or publishing occurs.
