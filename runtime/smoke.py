#!/usr/bin/env python3
import argparse,json,subprocess,os
from pathlib import Path
parser=argparse.ArgumentParser(description='Smoke-test cached Borea runtimes without target disks or host package changes')
parser.add_argument('--binary',type=Path,default=Path('target/release/borea'))
parser.add_argument('--cache',type=Path,required=True)
parser.add_argument('--out',type=Path,default=Path('target/validation/runtime'))
a=parser.parse_args();cache=str(a.cache.resolve());out=a.out.resolve();out.mkdir(parents=True,exist_ok=True)
marker=out/'host-only-marker';marker.write_text('must not be mounted in the runtime\n')
binary=str(a.binary.resolve());report=[]
for family,version in [('arch',['pacstrap','-h']),('debian',['debootstrap','--version']),('fedora',['dnf5','--version']),('suse',['zypper','--version'])]:
 def run(args,label,env=None):
  result=subprocess.run([binary,'runtime',*args],text=True,capture_output=True,env=env)
  (out/f'{family}-{label}.log').write_text(result.stdout+result.stderr)
  if result.returncode:print(result.stderr,flush=True);raise SystemExit(result.returncode)
  return result.stdout
 print(f'{family}: namespace probe',flush=True)
 state=json.loads(run(['probe','--family',family,'--cache',cache,'--json'],'probe'))
 assert state['verified'] and state['isolation_verified']
 run(['fetch','--family',family,'--cache',cache,'--offline','--json'],'offline')
 text=run(['exec','--family',family,'--cache',cache,'--',*version],'version')
 env=dict(os.environ,BOREA_HOST_SENTINEL='must-not-leak')
 script='test -z "${BOREA_HOST_SENTINEL:-}"; test ! -e "$1"; test ! -e /dev/sda; test ! -e /dev/vda; test ! -S /var/run/docker.sock; test ! -S /run/user/1000/bus; if touch /borea-root-write 2>/dev/null; then exit 21; fi; touch /tmp/borea-private-probe /var/log/borea-private-probe /var/cache/borea-private-probe; cat /etc/os-release; readlink /proc/self/ns/net'
 text2=run(['exec','--family',family,'--cache',cache,'--','/bin/sh','-ec',script,'--',str(marker)],'isolation',env)
 host_network=os.readlink('/proc/self/ns/net');assert host_network not in text2
 row={'family':family,'verified':True,'isolated':True,'tools':len(state['tools']),'offline_reuse':True,'bootstrap_command':version,'bootstrap_output':text.splitlines()[:3],'host_home_disks_env_and_socket_hidden':True,'root_readonly':True,'network_namespace_private':True,'os_release':text2.splitlines()}
 report.append(row);(out/'summary.json').write_text(json.dumps({'schema':1,'runtimes':report,'host_package_manager_invoked':False,'target_installations_tested':False},indent=2)+'\n')
 print(f'{family}: PASS ({len(state["tools"])} tools, bootstrap execution, offline reuse and isolation)',flush=True)
marker.unlink()
