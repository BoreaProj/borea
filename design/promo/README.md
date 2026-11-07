# Borea design preview

A silent promo preview recorded directly from the current
[interactive HTML mockup](../flow/borea-flow.html). The animation and UI are the
actual design, with preview controls and pointer hidden for capture.
Installation progress and networking are simulated; this is a design preview,
not a recording of a working native installer.

- [1080p MP4](borea-design-preview.mp4): H.264, 25 fps, no audio, approximately 35 seconds.
- [Looping GIF](borea-design-preview.gif): 960 × 540, 12 fps.
- [Family cards poster](borea-design-poster.png).
- [Restart poster](borea-restart-poster.png).
- [Export metadata and source fingerprint](preview.json).

The sequence shows the boot highlight loop, dot-to-arrow transformation, logo
spring into the corner, network screen, family colour transitions, Arch flavours,
Omarchy trim selections, install highlight progress, and matching restart logo.
The install section is accelerated for the preview. The full-frame exports have
short fades at the beginning and end; the GIF loops through black.

The source HTML and locked design are unchanged by the recording. Typography is
rendered using the fonts/fallbacks available to the existing standalone mockup.

## Re-recording

`record-preview.cjs` uses Playwright, Chromium and Playwright-compatible FFmpeg
capture support. Set `PLAYWRIGHT_MODULE` if Playwright is installed outside normal
Node module resolution, and `CHROMIUM_PATH` if Chromium is not at
`/usr/bin/chromium`. Playwright's video encoder must be installed or made available
through its supported browser cache path.

Run from the repository root:

```sh
node design/promo/record-preview.cjs
python design/promo/encode-preview.py
```

The recorder writes a temporary WebM capture and records its location in
`capture.json`. The encoder uses system `ffmpeg` and `ffprobe` to create the MP4,
GIF and `preview.json`. These tools are recording dependencies, not dependencies
of Borea's planned live system. No external service upload or publishing occurs.
