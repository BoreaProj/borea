#!/usr/bin/env python3
"""Build/export family tools on a CI builder. Never called by plan or at runtime."""
import argparse,gzip,hashlib,json,os,re,shutil,subprocess
from pathlib import Path
parser=argparse.ArgumentParser()
parser.add_argument('--family',choices=['arch','debian','fedora','suse'],required=True)
parser.add_argument('--version',required=True)
parser.add_argument('--out',type=Path,required=True)
a=parser.parse_args()
if not re.fullmatch(r'[a-z0-9][a-z0-9.-]{0,63}',a.version):parser.error('invalid runtime version')
root=Path(__file__).resolve().parent.parent
images=json.loads((root/'recipes/families/runtime-images.json').read_text())
tools=json.loads((root/'runtime/tools.json').read_text())[a.family]
image=images[a.family]['image']
assert re.fullmatch(r'[a-z0-9./-]+@sha256:[0-9a-f]{64}',image)
a.out.mkdir(parents=True,exist_ok=True)
tag=f'borea-runtime-{a.family}:{a.version}'
subprocess.run(['docker','build','--platform','linux/amd64','--build-arg',f'BASE_IMAGE={image}','--tag',tag,'--file',str(root/f'runtime/Containerfile.{a.family}'),str(root/'runtime')],check=True)
# Names are separate positional args, never interpolated into shell source.
subprocess.run(['docker','run','--rm','--network=none','--read-only','--cap-drop=ALL',tag,'/bin/sh','-ec','for tool do command -v "$tool"; done','--',*tools],check=True)
name=f'borea-runtime-{a.family}-x86_64.tar.gz'
archive=a.out/name
cid=subprocess.check_output(['docker','create',tag],text=True).strip()
try:
 with archive.open('xb') as target:
  process=subprocess.Popen(['docker','export',cid],stdout=subprocess.PIPE)
  try:
   with gzip.GzipFile(fileobj=target,mode='wb',filename='',mtime=0,compresslevel=1) as output:shutil.copyfileobj(process.stdout,output,1024*1024)
   if process.wait()!=0:raise RuntimeError('docker export failed')
  finally:
   if process.poll() is None:process.kill();process.wait()
finally:subprocess.run(['docker','rm','--force',cid],check=True)
sha=hashlib.file_digest(archive.open('rb'),'sha256').hexdigest()
manifest={'schema':1,'family':a.family,'version':a.version,'arch':'x86_64','image':image,'url':f'https://github.com/BoreaProj/borea/releases/download/{a.version}/{name}','sha256':sha,'size':archive.stat().st_size,'max_unpacked_bytes':8*1024**3,'tools':tools}
(a.out/f'manifest-{a.family}.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps(manifest),flush=True)
