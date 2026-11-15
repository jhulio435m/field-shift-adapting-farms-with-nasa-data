"""Start a temporary API image on loopback; never replace the running local panel."""
import json, os, secrets, subprocess, tempfile, time, urllib.parse, urllib.request, uuid
import sys
from pathlib import Path
if '--isolated' in sys.argv:
    subprocess.run([sys.executable,str(Path(__file__).with_name('verify_container_isolated.py')),*[arg for arg in sys.argv[1:] if arg!='--isolated']],check=True)
    raise SystemExit(0)
os.chdir(Path(__file__).resolve().parent.parent)
env=dict(line.split('=',1) for line in Path('.env').read_text().splitlines() if '=' in line and not line.startswith('#'))
def run(args):return subprocess.check_output(args,text=True,stderr=subprocess.PIPE).strip()
postgres=run(['docker','compose','ps','-q','postgres'])
networks=json.loads(run(['docker','inspect',postgres]))[0]['NetworkSettings']['Networks']
network=next(iter(networks));u=urllib.parse.urlsplit(env['DATABASE_URL'])
if u.hostname not in ('127.0.0.1','localhost','::1'):raise SystemExit('Only local development database is supported.')
container_url=urllib.parse.urlunsplit(u._replace(netloc=u.netloc.rsplit('@',1)[0]+'@postgres:5432'))
name='kusi-imagecheck-'+uuid.uuid4().hex[:12];base='http://127.0.0.1:18089'
with tempfile.TemporaryDirectory(prefix='kusi-imagecheck-',dir='.local-data') as tmp:
    path=Path(tmp)/'runtime.env';path.write_text('DATABASE_URL='+container_url+'\nKUSI_ADMIN_API_KEY='+secrets.token_hex(32)+'\nBIND_ADDRESS=0.0.0.0:8080\n');path.chmod(0o600)
    started=False;web_started=False;web_name=name+"-web"
    try:
        run(['docker','run','--rm','--detach','--name',name,'--network',network,'--publish','127.0.0.1:18089:8080','--env-file',str(path),'kusi-kusi-api']);started=True
        for _ in range(100):
            try:
                with urllib.request.urlopen(base+'/health/ready',timeout=1) as response:assert json.load(response)['status']=='ready'
                break
            except OSError:time.sleep(.2)
        else:raise RuntimeError('Container did not become ready.')
        with urllib.request.urlopen(base+'/openapi.yaml') as response:assert b'/v1/local/publications' in response.read()
        filters=run(['docker','exec',name,'ffmpeg','-hide_banner','-filters']);encoders=run(['docker','exec',name,'ffmpeg','-hide_banner','-encoders'])
        assert 'drawtext' in filters and 'libx264' in encoders
        run(['docker','exec',name,'test','-f','/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'])
        run(['docker','run','--rm','--detach','--name',web_name,'--network',network,'--publish','127.0.0.1:18090:3000','--env','KUSI_API_ORIGIN=http://'+name+':8080','kusi-kusi-web']);web_started=True
        web_base='http://127.0.0.1:18090'
        for _ in range(100):
            try:
                with urllib.request.urlopen(web_base+'/health/ready',timeout=1) as response:assert json.load(response)['status']=='ready'
                break
            except OSError:time.sleep(.2)
        else:raise RuntimeError('Next container proxy did not become ready.')
        with urllib.request.urlopen(web_base) as response:html=response.read().decode();assert '<html lang="es"' in html
        import re
        stylesheet=re.search(r'href="([^"]+\.css)"',html);assert stylesheet,'Next stylesheet not found'
        with urllib.request.urlopen(web_base+stylesheet.group(1)) as response:assert response.status==200 and len(response.read())>1000
        print(json.dumps(dict(status='passed',checks=['Rust container boot','PostgreSQL readiness','served OpenAPI','FFmpeg drawtext and libx264','DejaVu font','Next standalone container boot','Next proxy to API','Next SSR HTML and built CSS'],ports=[18089,18090]),indent=2))
    finally:
        if web_started:run(['docker','stop',web_name])
        if started:run(['docker','stop',name])
