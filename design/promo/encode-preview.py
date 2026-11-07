"""Encode a captured mockup into shareable MP4/GIF previews. Requires FFmpeg."""
from pathlib import Path
import hashlib,json,subprocess,sys
root=Path(__file__).resolve().parent
capture=json.loads((root/'capture.json').read_text())
raw=Path(sys.argv[1]) if len(sys.argv)>1 else Path(capture['rawVideo'])
def run(*args):subprocess.run(args,check=True)
info=json.loads(subprocess.check_output(['ffprobe','-v','error','-show_entries','format=duration','-of','json',str(raw)]))
duration=float(info['format']['duration'])-.5
mp4=root/'borea-design-preview.mp4'; gif=root/'borea-design-preview.gif'; palette=raw.parent/'palette.png'
run('ffmpeg','-hide_banner','-loglevel','error','-y','-ss','0.5','-i',str(raw),'-an','-vf',f'fps=25,fade=t=in:st=0:d=0.25,fade=t=out:st={duration-.65:.3f}:d=0.65','-c:v','libx264','-preset','medium','-crf','18','-threads','4','-pix_fmt','yuv420p','-movflags','+faststart',str(mp4))
print('MP4 encoded',flush=True)
run('ffmpeg','-hide_banner','-loglevel','error','-y','-i',str(mp4),'-vf','fps=12,scale=960:-1:flags=lanczos,palettegen=stats_mode=full:reserve_transparent=0','-frames:v','1','-threads','2',str(palette))
run('ffmpeg','-hide_banner','-loglevel','error','-y','-i',str(mp4),'-i',str(palette),'-filter_complex','[0:v]fps=12,scale=960:-1:flags=lanczos[v];[v][1:v]paletteuse=dither=bayer:bayer_scale=3:diff_mode=rectangle','-threads','2','-loop','0',str(gif))
metadata={'source':'design/flow/borea-flow.html','source_sha256':hashlib.sha256((root/'../flow/borea-flow.html').read_bytes()).hexdigest(),'design_preview':True,'installation_simulated':True,'audio':False,'duration_seconds':round(duration,2),'video':{'file':mp4.name,'width':1920,'height':1080,'fps':25,'codec':'H.264','bytes':mp4.stat().st_size},'gif':{'file':gif.name,'width':960,'height':540,'fps':12,'loop':'forever','bytes':gif.stat().st_size},'scenes':[beat['label'] for beat in capture['beats'] if beat['label']!='End']}
(root/'preview.json').write_text(json.dumps(metadata,indent=2)+'\n')
print(json.dumps(metadata,indent=2))
