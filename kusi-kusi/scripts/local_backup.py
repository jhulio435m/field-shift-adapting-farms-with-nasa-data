"""Verified local backup: dump, restore in an isolated database, copy referenced immutable assets."""
import argparse, hashlib, json, os, shutil, subprocess, time, urllib.parse, uuid
from pathlib import Path
parser=argparse.ArgumentParser();parser.add_argument('--verify-restore',action='store_true');args=parser.parse_args()
os.chdir(Path(__file__).resolve().parent.parent)
env=dict(os.environ)
env.update(dict(line.split('=',1) for line in Path('.env').read_text().splitlines() if '=' in line and not line.startswith('#')))
u=urllib.parse.urlsplit(env['DATABASE_URL'])
if u.hostname not in ('127.0.0.1','localhost','::1'):raise SystemExit('Este comando solo opera contra PostgreSQL local.')
pg=dict(os.environ,PGHOST=u.hostname,PGPORT=str(u.port or 5432),PGDATABASE=u.path.lstrip('/'),PGUSER=urllib.parse.unquote(u.username or ''),PGPASSWORD=urllib.parse.unquote(u.password or ''))
pg.pop('PGOPTIONS',None)
if u.query:raise SystemExit('DATABASE_URL con opciones no soportada por el respaldo local.')
os.umask(0o077)
folder=Path('.local-backups')/(time.strftime('%Y%m%d-%H%M%S')+'-'+uuid.uuid4().hex[:8]);folder.mkdir(parents=True,mode=0o700);(folder/'assets').mkdir(mode=0o700)
tempdb='kusi_restore_'+uuid.uuid4().hex
source=Path(env.get('KUSI_LOCAL_STORAGE','.local-data'))/'assets'
def execute(argv,pg_env=pg):
    r=subprocess.run(argv,env=pg_env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    if r.returncode:raise RuntimeError(f'{argv[0]} falló; código {r.returncode}. Respaldo no verificado.')
    return r.stdout.strip()
def sql(query,p=pg):return execute(['psql','-X','-At','-v','ON_ERROR_STOP=1','-c',query],p)
created=False
try:
    execute(['pg_dump','--format=custom','--no-owner','--no-acl','--file',str(folder/'database.dump')])
    sql('CREATE DATABASE '+tempdb);created=True
    restored=dict(pg,PGDATABASE=tempdb)
    execute(['pg_restore','--exit-on-error','--no-owner','--no-acl','--dbname',tempdb,str(folder/'database.dump')],restored)
    rows=sql("SELECT storage_key||'|'||sha256 FROM local_assets ORDER BY storage_key",restored)
    assets=[]
    for line in rows.splitlines():
        key,digest=line.split('|')
        if Path(key).name!=key or key.startswith('.'):raise RuntimeError('Ruta de archivo inválida.')
        destination=folder/'assets'/key
        shutil.copyfile(source/key,destination)
        actual=hashlib.sha256(destination.read_bytes()).hexdigest()
        if actual!=digest:raise RuntimeError('Integridad de archivo incorrecta; no se acepta el respaldo.')
        assets.append(dict(file=key,sha256=actual,size=destination.stat().st_size))
    tables=['organizations','local_users','local_conversations','local_messages','creative_jobs','local_assets','local_publications','local_audit']
    counts={t:int(sql('SELECT count(*) FROM '+t,restored)) for t in tables}
    manifest=dict(format=1,created_at=int(time.time()),database_sha256=hashlib.sha256((folder/'database.dump').read_bytes()).hexdigest(),assets=assets,restored_table_counts=counts,restore_verified=True,includes_external_keyring=False)
    (folder/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2))
    if args.verify_restore:
        assert counts['local_assets']==len(assets)
        for a in assets:assert hashlib.sha256((folder/'assets'/a['file']).read_bytes()).hexdigest()==a['sha256']
    print(json.dumps(dict(status='verified',backup=str(folder),assets=len(assets),restored_table_counts=counts),ensure_ascii=False,indent=2))
except Exception:
    shutil.rmtree(folder,ignore_errors=True)
    raise
finally:
    if created:sql('DROP DATABASE '+tempdb+' WITH (FORCE)')
