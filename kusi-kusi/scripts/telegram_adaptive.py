from contextlib import closing
"""Telegram transport to the generic Rust application; no business intents or field names."""
import base64
import hashlib
import hmac
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import uuid

ROOT=Path(__file__).resolve().parents[1]
CONFIG=ROOT/'.local-data/adaptive/runtime.json'
MIGRATION=Path(__file__).resolve().parent/'migrations/001_adaptive_deliveries.sql'


def bound_chat():
    with closing(sqlite3.connect(ROOT/'.local-data/telegram-staging/ledger.sqlite')) as db, db:
        row=db.execute('SELECT chat FROM binding WHERE id=1').fetchone()
    return row[0] if row else None


def invoke(payload):
    try:
        if os.environ.get('KUSI_LOCAL_MODE') == 'true':
            return {'status':'blocked','text':'El modo local no permite consultas ni llamadas a proveedores externos.'}
        configuration=json.loads(CONFIG.read_text())
        chat=bound_chat()
        if not chat or not hmac.compare_digest(hashlib.sha256(chat.encode()).hexdigest(),configuration['binding_hash']):
            return {'status':'blocked','text':'El chat no tiene una delegación vigente para este entorno.'}
        environment={k:v for k,v in os.environ.items() if not k.startswith(('KUSI_','GROQ_','META_'))}
        allowed={key for key in [configuration['llm'].get('api_key_env')] if isinstance(key,str)}
        access=configuration.get('access')
        if access:
            allowed.add(access['database_url_env'])
            environment['KUSI_ADAPTIVE_REQUIRE_ACL']='true'
        elif os.environ.get('KUSI_ADAPTIVE_REQUIRE_ACL') == 'true':
            environment['KUSI_ADAPTIVE_REQUIRE_ACL']='true'
        for key in allowed:
            if key in os.environ:environment[key]=os.environ[key]
        for line in ((ROOT/'.env').read_text().splitlines() if (ROOT/'.env').exists() else []):
            if '=' in line and not line.lstrip().startswith('#'):
                key,value=line.split('=',1)
                if key.strip() in allowed:environment[key.strip()]=value.strip().strip('\"').strip("'")
        environment['KUSI_ADAPTIVE_STAGING_ACK']='authorized-environment'
        result=subprocess.run([str(ROOT/'target/debug/adaptive_assistant'),str(CONFIG)],input=json.dumps(payload),text=True,capture_output=True,env=environment,timeout=95)
        if result.returncode or len(result.stdout)>2097152:raise RuntimeError()
        return json.loads(result.stdout)
    except Exception:
        return {'status':'unavailable','text':'No pude completar la petición en el entorno autorizado. No voy a inventar datos; puedes volver a consultarme.'}


def respond(text, request_id=None):
    result=invoke({'message':text,'request_id':request_id or uuid.uuid4().hex})
    answer=result.get('text','No se obtuvo una respuesta verificable.')
    if len(answer)>3800:answer=answer[:3600]+'\n\nRespuesta abreviada. Puedes pedir un filtro o una parte del análisis.'
    return answer


def event(raw,signature,source,send):
    try:
        config=json.loads(CONFIG.read_text())
        registered=next((s for s in config['sources'] if s['id']==source),None)
        if not registered:return 404
        secret_file=registered.get('event_secret_file')
        if not secret_file:
            settings=json.loads(Path(registered['config_file']).read_text())
            secret_file=settings['event_secret_file']
        secret=Path(secret_file).read_text().strip()
        expected=base64.b64encode(hmac.new(secret.encode(),raw,hashlib.sha256).digest()).decode()
        if not hmac.compare_digest(expected.encode(),signature.encode()):return 401
        payload=json.loads(raw)
        if not isinstance(payload,dict) or set(payload)-{'test'}!={'resource','event_id','kind','values'}:return 400
        if not isinstance(payload['resource'],str) or not payload['resource'].startswith(source+'::'):return 400
        if not isinstance(payload['event_id'],str) or not 1<=len(payload['event_id'])<=200:return 400
        if not isinstance(payload['kind'],str) or not isinstance(payload['values'],dict) or type(payload.get('test',False)) is not bool:return 400
        chat=bound_chat()
        if not chat or hashlib.sha256(chat.encode()).hexdigest()!=config['binding_hash']:return 403
        with closing(sqlite3.connect(ROOT/'.local-data/telegram-staging/ledger.sqlite')) as db, db:
            db.executescript(MIGRATION.read_text())
        result=invoke({'event':payload})
        if result['status']=='notification':
            if bound_chat()!=chat:return 403
            # Application already claimed subscription/event. Never repeat an uncertain send.
            outcome='unknown'
            try:
                receipt=send(chat,result['text'])
                if receipt.get('message_id'):outcome='accepted'
            finally:
                with closing(sqlite3.connect(ROOT/'.local-data/telegram-staging/ledger.sqlite')) as db, db:
                    digest=hashlib.sha256((source+':'+payload['resource']+':'+payload['event_id']).encode()).hexdigest()
                    db.execute('INSERT OR REPLACE INTO adaptive_deliveries VALUES(?,?)',(digest,outcome))
            return 200
        if result['status']=='ignored':return 200
        if result['status']=='blocked':return 403
        return 503
    except Exception:return 503
