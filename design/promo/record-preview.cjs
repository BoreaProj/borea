// Capture the real mockup. Requires Playwright + Chromium + FFmpeg.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const fs=require('node:fs');
const path=require('node:path');
const os=require('node:os');
(async()=>{
 const output=path.resolve(__dirname),raw=fs.mkdtempSync(path.join(os.tmpdir(),'borea-promo-'));
 const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH||'/usr/bin/chromium',args:['--no-sandbox']});
 const context=await browser.newContext({viewport:{width:1920,height:1080},deviceScaleFactor:1,recordVideo:{dir:raw,size:{width:1920,height:1080}}});
 const p=await context.newPage(),errors=[];p.on('pageerror',e=>errors.push(e.message));
 await p.setContent(fs.readFileSync(path.join(__dirname,'../flow/borea-flow.html'),'utf8'));
 await p.addStyleTag({content:`html,body{padding:0!important;margin:0!important;overflow:hidden!important;background:#0b0f14!important}#preview-toolbar{display:none!important}body>#scene{position:fixed!important;inset:0!important;width:100vw!important;height:100vh!important;max-width:none!important;border-radius:0!important}*{cursor:none!important}*:focus-visible{outline:none!important}`});
 await p.evaluate(()=>{document.getElementById('scen').value='eth';fitWindow();run()});
 const beats=[],start=Date.now(),wait=ms=>p.waitForTimeout(ms),mark=label=>{beats.push({seconds:Number(((Date.now()-start)/1000).toFixed(2)),label});console.log(label)};
 mark('Boot: highlight loop and dot-to-arrow');
 await wait(4200);await p.evaluate(()=>signal());await p.waitForFunction(()=>bootCanContinue,null,{timeout:15000});await wait(650);
 mark('Logo springs into the corner / Get online');await p.evaluate(()=>transition());await p.waitForFunction(()=>page==='fam',null,{timeout:15000});
 await wait(1200);mark('Choose your family / colour and card motion');
 await p.screenshot({path:path.join(output,'borea-design-poster.png')});
 for(const i of [1,2,3,0]){await p.evaluate(i=>fpick(i),i);await wait(850)}
 mark('Arch flavours / Omarchy');await p.evaluate(()=>fnext());await wait(1000);await p.evaluate(()=>selectArch(1));await wait(750);await p.evaluate(()=>archNext());await wait(1000);
 mark('Trim Omarchy / individual and group selection');
 for(const label of ['Chromium','Signal','Spotify']){await p.locator('.lf[data-x="'+label+'"]').click();await wait(500)}
 await p.locator('[data-all="2"]').click();await wait(1500);
 mark('Silent install / highlight as progress');
 await p.evaluate(()=>{seedPreview('install');installPercent=0;showNextPage('install');stopInstall()});await wait(650);
 for(let value=4;value<=100;value+=4){await p.evaluate(value=>{installPercent=value;updateProgress()},value);await wait(150)}
 await wait(500);mark('Restart / exact boot wordmark, dot-to-arrow and prompt');await p.evaluate(()=>showNextPage('done'));await wait(3100);
 await p.screenshot({path:path.join(output,'borea-restart-poster.png')});
 mark('End');if(errors.length)throw Error(errors.join('\n'));
 const video=p.video();await context.close();await video.saveAs(path.join(raw,'capture.webm'));await browser.close();
 fs.writeFileSync(path.join(output,'capture.json'),JSON.stringify({source:'design/flow/borea-flow.html',simulated:true,viewport:{width:1920,height:1080},rawVideo:path.join(raw,'capture.webm'),beats},null,2)+'\n');
 console.log('RAW_VIDEO='+path.join(raw,'capture.webm'));
})().catch(e=>{console.error(e);process.exit(1)});
