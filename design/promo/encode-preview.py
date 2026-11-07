"""Export a genuine 60fps capture and a shorter 25fps GIF. Requires FFmpeg."""
from pathlib import Path
import hashlib,json,os,subprocess,sys
root=Path(__file__).resolve().parent
capture=json.loads((root/'capture.json').read_text())
raw=Path(sys.argv[1]) if len(sys.argv)>1 else Path(capture['rawVideo'])
def run(*args):subprocess.run([os.environ.get('FFMPEG_PATH','ffmpeg'),*args[1:]],check=True)
def probe(file):return json.loads(subprocess.check_output(['ffprobe','-v','error','-show_entries','format=duration:stream=width,height,r_frame_rate,nb_frames','-of','json',str(file)]))
info=probe(raw);trim=capture.get('trimStart',.5);duration=float(info['format']['duration'])-trim
fps=capture.get('fps',25)
if fps!=60:raise SystemExit('Re-record using the frame-by-frame 60fps recorder first; do not duplicate 25fps frames.')
mp4=root/'borea-design-preview.mp4';gif=root/'borea-design-preview.gif';palette=raw.parent/'palette.png';reel=raw.parent/'gif-reel.mp4'
run('ffmpeg','-hide_banner','-loglevel','error','-y','-ss',str(trim),'-i',str(raw),'-an','-vf',f'fade=t=in:st=0:d=0.2,fade=t=out:st={duration-.6:.3f}:d=0.6','-c:v','libx264','-preset','medium','-crf','17','-threads','4','-pix_fmt','yuv420p','-movflags','+faststart',str(mp4))
print('60fps MP4 encoded',flush=True)
# Keep the GIF light enough to embed in GitHub while retaining fluent motion.
clips=[(0,4.1),(6.9,9.1),(9.6,11),(17.1,19.2),(23.8,25.5),(28,30),(32.4,35.5)]
filters=[]
for i,(start,end) in enumerate(clips):filters.append(f'[0:v]trim=start={start}:end={end},setpts=PTS-STARTPTS[v{i}]')
gif_duration=sum(end-start for start,end in clips)
filters.append(''.join(f'[v{i}]' for i in range(len(clips)))+f'concat=n={len(clips)}:v=1:a=0,fps=25,fade=t=out:st={gif_duration-.4:.3f}:d=0.4[out]')
run('ffmpeg','-hide_banner','-loglevel','error','-y','-i',str(mp4),'-filter_complex',';'.join(filters),'-map','[out]','-c:v','libx264','-crf','17','-preset','fast','-threads','4',str(reel))
for width in [960,900,800]:
 run('ffmpeg','-hide_banner','-loglevel','error','-y','-i',str(reel),'-vf',f'scale={width}:-2:flags=lanczos,palettegen=max_colors=192:stats_mode=full:reserve_transparent=0','-frames:v','1','-threads','2',str(palette))
 run('ffmpeg','-hide_banner','-loglevel','error','-y','-i',str(reel),'-i',str(palette),'-filter_complex',f'[0:v]scale={width}:-2:flags=lanczos[v];[v][1:v]paletteuse=dither=bayer:bayer_scale=3:diff_mode=rectangle','-threads','2','-loop','0',str(gif))
 if gif.stat().st_size<9_900_000:break
v=probe(mp4);g=probe(gif);metadata={'source':'design/flow/borea-flow.html','source_sha256':hashlib.sha256((root/'../flow/borea-flow.html').read_bytes()).hexdigest(),'design_preview':True,'installation_simulated':True,'capture_mode':'frame-by-frame, no frame interpolation','audio':False,'duration_seconds':round(float(v['format']['duration']),2),'video':{'file':mp4.name,'width':1920,'height':1080,'fps':60,'frames':int(v['streams'][0]['nb_frames']),'codec':'H.264','bytes':mp4.stat().st_size},'gif':{'file':gif.name,'width':g['streams'][0]['width'],'height':g['streams'][0]['height'],'fps':25,'duration_seconds':round(float(g['format']['duration']),2),'loop':'forever','bytes':gif.stat().st_size},'scenes':[beat['label'] for beat in capture['beats']]}
(root/'preview.json').write_text(json.dumps(metadata,indent=2)+'\n')
print(json.dumps(metadata,indent=2))
