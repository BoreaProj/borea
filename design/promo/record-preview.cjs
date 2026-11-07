// Render genuine 60fps frames from the real HTML; wall-clock speed cannot drop frames.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const fs=require('node:fs'),path=require('node:path'),os=require('node:os');
const {spawn}=require('node:child_process');
const {once}=require('node:events');
(async()=>{
 const output=__dirname,raw=fs.mkdtempSync(path.join(os.tmpdir(),'borea-promo-smooth-'));
 const fps=60,seconds=36,rawVideo=path.join(raw,'capture.mp4');
 const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH||'/usr/bin/chromium',args:['--no-sandbox']});
 const p=await browser.newPage({viewport:{width:1920,height:1080},deviceScaleFactor:1}),errors=[];p.on('pageerror',e=>errors.push(e.message));
 await p.clock.install({time:0});
 await p.evaluate(()=>{
  window.__frameTime=0;window.__motions=[];window.__knownMotions=new WeakSet();
  const track=a=>{if(__knownMotions.has(a))return a;__knownMotions.add(a);a.pause();__motions.push({a,start:__frameTime,finished:false});return a};
  const original=Element.prototype.animate;Element.prototype.animate=function(...args){return track(original.apply(this,args))};
  window.__renderAnimations=()=>{
   document.getAnimations().forEach(track);
   for(const m of __motions){if(m.finished||m.a.playState==='idle')continue;const elapsed=__frameTime-m.start,end=m.a.effect.getComputedTiming().endTime;if(Number.isFinite(end)&&elapsed>=end){m.a.finish();m.finished=true}else m.a.currentTime=elapsed}
  };
 });
 await p.setContent(fs.readFileSync(path.join(__dirname,'../flow/borea-flow.html'),'utf8'));
 await p.addStyleTag({content:`html,body{padding:0!important;margin:0!important;overflow:hidden!important;background:#0b0f14!important}#preview-toolbar{display:none!important}body>#scene{position:fixed!important;inset:0!important;width:100vw!important;height:100vh!important;max-width:none!important;border-radius:0!important}*{cursor:none!important}*:focus-visible{outline:none!important}.install-ink,.install-text-progress{transition:none!important}`});
 await p.evaluate(()=>{document.getElementById('scen').value='eth';fitWindow();run()});
 const beats=[];
 const events=[
  [4.2,()=>signal(),'Boot: highlight loop and dot-to-arrow'],
  [9.6,()=>{if(!bootCanContinue)throw Error('Boot not ready');transition()},'Logo springs into the corner / Get online'],
  [17,()=>{if(page!=='fam')throw Error('Family not reached');fpick(1)},'Choose your family / colour and card motion'],
  [17.85,()=>fpick(2)],[18.7,()=>fpick(3)],[19.55,()=>fpick(0)],
  [20.5,()=>fnext(),'Arch flavours / Omarchy'],[21.5,()=>selectArch(1)],[22.25,()=>archNext()],
  [23.7,()=>document.querySelector('.lf[data-x="Chromium"]').click(),'Trim Omarchy / individual and group selection'],
  [24.2,()=>document.querySelector('.lf[data-x="Signal"]').click()],
  [24.7,()=>document.querySelector('.lf[data-x="Spotify"]').click()],
  [25.2,()=>document.querySelector('[data-all="2"]').click()],
  [26.6,()=>{seedPreview('install');installPercent=0;showNextPage('install');stopInstall()},'Silent install / highlight as progress'],
  [32.4,()=>showNextPage('done'),'Restart / exact boot wordmark, dot-to-arrow and prompt']
 ];
 const encoder=spawn(process.env.FFMPEG_PATH||'ffmpeg',['-hide_banner','-loglevel','error','-y','-f','image2pipe','-vcodec','mjpeg','-framerate',String(fps),'-i','pipe:0','-an','-c:v','libx264','-preset','fast','-crf','17','-threads','4','-pix_fmt','yuv420p','-movflags','+faststart',rawVideo],{stdio:['pipe','inherit','inherit']});
 const encoded=once(encoder,'close');let event=0;
 for(let frame=0;frame<seconds*fps;frame++){
  const t=frame/fps;await p.evaluate(t=>window.__frameTime=t,t*1000);
  while(event<events.length&&t>=events[event][0]){const [at,fn,label]=events[event++];await p.evaluate(fn);if(label){beats.push({seconds:at,label});console.log(label)}}
  await p.clock.runFor(1000/fps);
  if(t>=27.25&&t<=31.95)await p.evaluate(t=>{const fraction=Math.min(100,(t-27.25)/4.65*100);installPercent=Math.round(fraction);updateProgress();paintInstallLogo(document.getElementById('install-progress'),fraction)},t);
  await p.evaluate(()=>__renderAnimations());
  const jpeg=await p.screenshot({type:'jpeg',quality:95});if(!encoder.stdin.write(jpeg))await once(encoder.stdin,'drain');
  if(frame===16.5*fps)await p.screenshot({path:path.join(output,'borea-design-poster.png')});
  if(frame===35*fps)await p.screenshot({path:path.join(output,'borea-restart-poster.png')});
  if(frame%300===0)console.log('Rendered '+frame+'/'+seconds*fps+' frames');
 }
 encoder.stdin.end();const [code]=await encoded;await browser.close();if(code!==0)throw Error('FFmpeg failed: '+code);if(errors.length)throw Error(errors.join('\n'));
 fs.writeFileSync(path.join(output,'capture.json'),JSON.stringify({source:'design/flow/borea-flow.html',simulated:true,captureMode:'frame-by-frame',fps,trimStart:0,viewport:{width:1920,height:1080},rawVideo,beats},null,2)+'\n');
 console.log('RAW_VIDEO='+rawVideo);
})().catch(e=>{console.error(e);process.exit(1)});
