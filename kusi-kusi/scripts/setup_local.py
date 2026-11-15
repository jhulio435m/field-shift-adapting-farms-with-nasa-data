"""Create a first local owner; keep its generated credential in an owner-only ignored file."""
import json, os, secrets, urllib.request, urllib.error
from pathlib import Path
os.chdir(Path(__file__).resolve().parent.parent)
env=dict(os.environ)
env.update(dict(line.split('=',1) for line in Path('.env').read_text().splitlines() if '=' in line and not line.startswith('#')))
base='http://'+env.get('BIND_ADDRESS','127.0.0.1:8080')
folder=Path(env.get('KUSI_LOCAL_STORAGE','.local-data'));folder.mkdir(mode=0o700,parents=True,exist_ok=True);folder.chmod(0o700)
path=folder/'login.json'
if path.exists():
    print('Credenciales existentes conservadas: '+str(path))
    raise SystemExit(0)
key=env.get('DEMO_ORGANIZATION_API_KEY') or env.get('KUSI_DEMO_API_KEY') or env.get('DEMO_API_KEY')
if not key:
    raise SystemExit('Falta DEMO_ORGANIZATION_API_KEY en .env; aprovisiona una organización con POST /v1/organizations y guarda su clave en esa variable.')
def call(route,method='GET',payload=None):
    headers={'Authorization':'Bearer '+key,'x-admin-key':env['KUSI_ADMIN_API_KEY']}
    if payload is not None:headers['Content-Type']='application/json'
    req=urllib.request.Request(base+route,method=method,headers=headers,data=json.dumps(payload).encode() if payload is not None else None)
    with urllib.request.urlopen(req,timeout=20) as response:return json.load(response)
org=call('/v1/organizations/current');password=secrets.token_urlsafe(24)
try:call('/v1/local/bootstrap','POST',dict(username='admin',display_name='Administrador local',password=password))
except urllib.error.HTTPError as e:raise SystemExit(f'Bootstrap rechazado ({e.code}); no se modificaron usuarios existentes.')
fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
with os.fdopen(fd,'w') as f:json.dump(dict(url='http://127.0.0.1:3000',organization=org['slug'],username='admin',password=password),f,indent=2)
print('Cuenta creada. Credenciales privadas: '+str(path))
