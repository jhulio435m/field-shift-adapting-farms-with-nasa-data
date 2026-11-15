"""Real Kusi worker + signed local channel events + local model + Vendure data.

Creates a disposable Kusi database; never changes the Vendure catalog or calls
WhatsApp/Telegram APIs. Questions go through the existing generic planner.
"""
from contextlib import contextmanager
import hashlib
import hmac
import json
import os
from pathlib import Path
import secrets
import subprocess
import time
import urllib.request
import urllib.error
import uuid
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

from local_acceptance import ROOT, postgres, free_port, sql
from verify_external_access import call
from adaptive_graphql_bridge import Bridge

ACCOUNTS=Path('/home/blink/Projects/mubarak-vendure/apps/mubarak/var/kusi-accounts.json')

def write(path,value):
    path.parent.mkdir(mode=0o700,parents=True,exist_ok=True)
    path.write_text(json.dumps(value,ensure_ascii=False,indent=2));path.chmod(0o600)

@contextmanager
def server(env,root,channels):
    base='http://127.0.0.1:'+str(free_port())
    settings=dict(env,KUSI_ADMIN_API_KEY=secrets.token_hex(32),KUSI_LOCAL_MODE='true',KUSI_LOCAL_ORIGIN=base,KUSI_LOCAL_STORAGE=str(root/'storage'),BIND_ADDRESS=base.removeprefix('http://'),KUSI_CREATIVE_SIMULATION='true',KUSI_CREATIVE_WORKER='false',KUSI_OAUTH_REFRESH_WORKER='false',RUST_LOG='error')
    for key in ['LLM_PROFILES','AUDIO_PROFILES','CREATIVE_RECIPES','HTTP_DESTINATIONS','OAUTH_PROFILES','SYSTEM_PROFILES']:settings['KUSI_'+key]='{}'
    settings.update(KUSI_CHANNELS=json.dumps(channels),LOCAL_TG_SECRET='local-telegram-'+secrets.token_hex(16),LOCAL_WA_SECRET=secrets.token_hex(32),LOCAL_WA_VERIFY=secrets.token_hex(16))
    with (root/'server.log').open('a') as log:
        process=subprocess.Popen([str(ROOT/'target/debug/kusi-server')],cwd=root,env=settings,stdout=log,stderr=log)
        try:
            for _ in range(100):
                if process.poll() is not None:raise RuntimeError('Kusi local server exited; inspect private acceptance log')
                try:call(base,'/health/ready');break
                except (OSError,RuntimeError):time.sleep(.1)
            else:raise RuntimeError('Kusi local server not ready')
            yield base,settings
        finally:
            process.terminate()
            try:process.wait(timeout=10)
            except subprocess.TimeoutExpired:process.kill();process.wait()

def wait(env,query,timeout=70):
    deadline=time.monotonic()+timeout
    while time.monotonic()<deadline:
        value=sql(env,query)
        if value:return value
        time.sleep(.3)
    raise RuntimeError('No durable adaptive response within local timeout')

def emit(base,settings,channel,provider,sender,message,ident):
    if provider=='whatsapp':
        payload={'object':'whatsapp_business_account','entry':[{'changes':[{'value':{'metadata':{'phone_number_id':'123456'},'messages':[{'id':str(ident),'from':sender,'type':'text','text':{'body':message}}]}}]}]}
        raw=json.dumps(payload,separators=(',',':')).encode()
        headers={'X-Hub-Signature-256':'sha256='+hmac.new(settings['LOCAL_WA_SECRET'].encode(),raw,hashlib.sha256).hexdigest()}
    else:
        payload={'update_id':ident,'message':{'message_id':ident,'from':{'id':int(sender),'is_bot':False,'first_name':'Synthetic'},'chat':{'id':int(sender),'type':'private'},'text':message}}
        raw=json.dumps(payload,separators=(',',':')).encode();headers={'X-Telegram-Bot-Api-Secret-Token':settings['LOCAL_TG_SECRET']}
    request=urllib.request.Request(base+'/webhooks/'+channel,data=raw,headers={'Content-Type':'application/json',**headers})
    with urllib.request.urlopen(request,timeout=5) as response:assert response.status==200


class FixturePlanner(BaseHTTPRequestHandler):
    """A plan fixture verifies execution, never claims natural-language understanding."""
    def log_message(self,*_args): pass
    def do_POST(self):
        body=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        prompt=json.loads(body['messages'][-1]['content'])
        resource=prompt['catalog']['resources'][0]['id']
        decision={'type':'query','plan':{'version':prompt['catalog']['version'],'queries':[{'resource':resource,'filters':[],'analyses':[{'op':'count'}]}]}}
        data=json.dumps({'choices':[{'finish_reason':'stop','message':{'content':json.dumps(decision)}}]}).encode()
        self.send_response(200);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)

def verify():
    model=HTTPServer(('127.0.0.1',0),FixturePlanner)
    threading.Thread(target=model.serve_forever,daemon=True).start()
    accounts=json.loads(ACCOUNTS.read_text())
    observations=[]
    with postgres() as (env,temporary):
        tenants=[]
        with server(env,temporary,{}) as (base,settings):
            for account in accounts:
                created=call(base,'/v1/organizations',{'name':'Adaptive '+account['name'],'slug':'mubarak-'+account['slug']},{'x-admin-key':settings['KUSI_ADMIN_API_KEY'],'Origin':base},201)
                org=created['organization']['id'];password=secrets.token_urlsafe(24)
                call(base,'/v1/local/bootstrap',{'username':'seller','display_name':'Synthetic seller','password':password},{'Authorization':'Bearer '+created['api_key'],'x-admin-key':settings['KUSI_ADMIN_API_KEY'],'Origin':base})
                session=call(base,'/local/auth/login',{'organization':created['organization']['slug'],'username':'seller','password':password},{'Origin':base})
                tenants.append({'org':org,'user':session['user']['id'],'account':account})
        channels={};profiles=[]
        for tenant in tenants:
            for provider,sender in [('whatsapp','51900000001'),('telegram','71001')]:
                channel=str(uuid.uuid4())
                channels[channel]={'organization_id':tenant['org'],'provider':provider,'external_id':'123456' if provider=='whatsapp' else 'fixture-bot','secret_env':'LOCAL_WA_SECRET' if provider=='whatsapp' else 'LOCAL_TG_SECRET','verify_token_env':'LOCAL_WA_VERIFY' if provider=='whatsapp' else None}
                profiles.append({**tenant,'provider':provider,'sender':sender,'channel':channel})
        with server(env,temporary,channels) as (base,settings):
            for index,profile in enumerate(profiles):
                org,user,channel,sender=profile['org'],profile['user'],profile['channel'],profile['sender']
                code=secrets.token_hex(18)
                # Fixture provisioning inserts only a one-use link code; actual binding is via the signed webhook.
                sql(env,f"INSERT INTO local_link_codes(code_hash,organization_id,channel_id,user_id,expires_at) VALUES('{hashlib.sha256(code.encode()).hexdigest()}','{org}','{channel}','{user}',now()+interval '10 minutes')")
                emit(base,settings,channel,profile['provider'],sender,'/vincular '+code,100+index)
                wait(env,f"SELECT user_id FROM local_channel_bindings WHERE channel_id='{channel}' AND sender='{sender}' AND revoked_at IS NULL")
                credential=temporary/(user+'-credential.json');write(credential,{'username':profile['account']['username'],'password':profile['account']['password']})
                source={'enabled':True,'label':'Mubarak / '+profile['account']['name'],'endpoint':'http://127.0.0.1:3100/admin-api','channel_token':profile['account']['channelToken'],'credential_file':str(credential),'timezone':'America/Lima','resources':['products','productVariants','orders'],'categorical_fields':{'orders':['state']}}
                source_file=temporary/(user+'-source.json');write(source_file,source)
                binding=hashlib.sha256((channel+':'+sender).encode()).hexdigest()
                runtime={'organization':org,'principal':user,'read':True,'subscribe':False,'sources':[{'id':'store','label':source['label'],'kind':'graphql_loopback','config_file':str(source_file)}],'state_directory':str(temporary/'state'/binding),'bridge_script':str(ROOT/'scripts/adaptive_graphql_bridge.py'),'binding_hash':binding,'llm':{'provider':'openai_compatible','model':'fixture-planner-not-nlp-acceptance','api_key_env':None,'endpoint':f'http://127.0.0.1:{model.server_port}/v1/chat/completions','max_output_tokens':1500,'timeout_ms':5000,'tools_supported':False,'anthropic_version':None,'routing_providers':None}}
                write(temporary/'storage/adaptive-bindings'/org/user/(binding+'.json'),runtime)
                wait(env,f"SELECT id FROM local_outbox WHERE channel_id='{channel}'")
                before=int(sql(env,f"SELECT count(*) FROM local_outbox WHERE channel_id='{channel}'"))
                emit(base,settings,channel,profile['provider'],sender,'¿Cuántos productos tengo en mi negocio?',1000+index)
                answer=wait(env,f"SELECT content FROM local_outbox WHERE channel_id='{channel}' ORDER BY created_at DESC LIMIT 1 OFFSET 0")
                # Wait for the new reply rather than reusing the account-link acknowledgement.
                wait(env,f"SELECT count(*) FROM local_outbox WHERE channel_id='{channel}' HAVING count(*)>{before}")
                answer=sql(env,f"SELECT content FROM local_outbox WHERE channel_id='{channel}' ORDER BY created_at DESC LIMIT 1")
                receipts=list((temporary/'state'/binding).rglob('receipt-*.json'))
                completed=[json.loads(p.read_text())['data']['reply'] for p in receipts]
                result=next((r for r in completed if r.get('status')=='completed'),None)
                observations.append({'seller':profile['account']['slug'],'provider':profile['provider'],'status':completed[-1]['status'] if completed else 'no-receipt','reply':answer})
                if not result:
                    write(ROOT/'.local-data/mubarak-adaptive/partial.json',{'observations':observations,'receipts':completed})
                    raise RuntimeError('Local model did not produce a completed plan; inspect report')
                assert result['evidence'][0]['observed_rows']==1
                assert all(x['status']=='queued' for x in json.loads(sql(env,f"SELECT json_agg(json_build_object('status',status)) FROM local_outbox WHERE channel_id='{channel}'")))
                assert int(sql(env,f"SELECT count(*) FROM local_outbox WHERE channel_id='{channel}'"))==before+1
                emit(base,settings,channel,profile['provider'],sender,'¿Cuántos productos tengo en mi negocio?',1000+index)
                time.sleep(.4)
                assert int(sql(env,f"SELECT count(*) FROM local_outbox WHERE channel_id='{channel}'"))==before+1
    model.shutdown();model.server_close()
    return observations

if __name__=='__main__':
    output=ROOT/'.local-data/mubarak-adaptive/channel-acceptance.json'
    try:
        results=verify();write(output,{'status':'passed','model':'fixture-planner-not-nlp-acceptance','delivery':'local-outbox-only','observations':results})
        print('Passed: two sellers × WhatsApp/Telegram signed local webhooks, generic adaptive engine with fixture plans, real Vendure reads, replay deduplication; zero provider sends.')
    except Exception as error:
        write(output,{'status':'failed','reason':str(error)})
        raise
