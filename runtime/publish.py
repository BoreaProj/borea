#!/usr/bin/env python3
"""Publish a new, immutable-by-policy release only after all families built."""
import hashlib,json,os,subprocess,sys
from pathlib import Path
root=Path(sys.argv[1]);version=sys.argv[2]
manifest=[];assets=[]
for family in ['arch','debian','fedora','suse']:
 spec=json.loads((root/f'manifest-{family}.json').read_text())
 assert spec['family']==family and spec['version']==version
 archive=root/Path(spec['url']).name
 with archive.open('rb') as f:assert hashlib.file_digest(f,'sha256').hexdigest()==spec['sha256']
 assert archive.stat().st_size==spec['size']
 manifest.append(spec);assets.append(str(archive))
lock=root/'runtimes.lock.json';lock.write_text(json.dumps({'schema':1,'runtimes':manifest},indent=2)+'\n')
sums=root/'SHA256SUMS';sums.write_text(''.join(f"{s['sha256']}  {Path(s['url']).name}\n" for s in manifest))
if subprocess.run(['gh','release','view',version,'--repo','BoreaProj/borea'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode==0:raise SystemExit('Release exists: choose a new version; never overwrite trusted runtime assets')
subprocess.run(['gh','release','create',version,*assets,str(lock),str(sums),'--repo','BoreaProj/borea','--target',os.environ['GITHUB_SHA'],'--title',f'Independent tools: {version}','--notes','Experimental x86_64 family tool bundles for Borea. Includes checksummed root filesystems and a machine-readable lock. This is runtime delivery, not proof of a bootable installation.','--prerelease'],check=True)
