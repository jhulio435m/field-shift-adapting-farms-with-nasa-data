"""Isolated local product checks: no external network providers; real PostgreSQL/FFmpeg."""
import hashlib, hmac, http.cookiejar, json, os, secrets, struct, subprocess, tempfile, time, urllib.error, urllib.parse, urllib.request, uuid, zlib
from pathlib import Path
root=Path(__file__).resolve().parent.parent
os.chdir(root)
env=dict(os.environ)
if Path('.env').exists():
    env.update(dict(line.split('=',1) for line in Path('.env').read_text().splitlines() if '=' in line and not line.startswith('#')))
u=urllib.parse.urlsplit(env['DATABASE_URL'])
pg=dict(os.environ,PGHOST=u.hostname or '127.0.0.1',PGPORT=str(u.port or 5432),PGDATABASE=u.path.lstrip('/'),PGUSER=urllib.parse.unquote(u.username or ''),PGPASSWORD=urllib.parse.unquote(u.password or ''))
pg.pop('PGOPTIONS',None)
def sql(q):
    try:return subprocess.check_output(['psql','-X','-At','-v','ON_ERROR_STOP=1','-c',q],env=pg,text=True,stderr=subprocess.PIPE).strip()
    except subprocess.CalledProcessError as exc:raise RuntimeError('isolated PostgreSQL check failed: '+exc.stderr.strip()) from exc
schema='local_api_'+uuid.uuid4().hex
sql('CREATE SCHEMA '+schema)
pg['PGOPTIONS']='-csearch_path='+schema
q=dict(urllib.parse.parse_qsl(u.query));q['options']='-csearch_path='+schema
env['DATABASE_URL']=urllib.parse.urlunsplit(u._replace(query=urllib.parse.urlencode(q)))
env.update(KUSI_ADMIN_API_KEY=secrets.token_hex(32),KUSI_LOCAL_MODE='true',KUSI_CREATIVE_SIMULATION='true',KUSI_CREATIVE_WORKER='true',KUSI_OAUTH_REFRESH_WORKER='false',BIND_ADDRESS='127.0.0.1:18088',KUSI_LOCAL_ORIGIN='http://127.0.0.1:18088')
for name in ['KUSI_CHANNELS','KUSI_LLM_PROFILES','KUSI_AUDIO_PROFILES','KUSI_CREATIVE_RECIPES','KUSI_HTTP_DESTINATIONS','KUSI_OAUTH_PROFILES']:env[name]='{}'
base='http://'+env['BIND_ADDRESS'];process=None
class Client:
    def __init__(self):self.jar=http.cookiejar.CookieJar();self.opener=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(self.jar));self.csrf=''
    def call(self,path,method='GET',body=None,expected=200,headers=None,raw=False):
        h={'Origin':base,'x-csrf-token':self.csrf};h.update(headers or {})
        if body is not None and not isinstance(body,bytes):body=json.dumps(body).encode();h['Content-Type']='application/json'
        req=urllib.request.Request(base+path,data=body,method=method,headers=h)
        try:r=self.opener.open(req,timeout=10);code=r.status;data=r.read()
        except urllib.error.HTTPError as e:code=e.code;data=e.read()
        assert code==expected,(method,path,code,expected,data[:300])
        if raw:return data
        return json.loads(data) if data else None
    def login(self,org,user,password):
        r=self.call('/local/auth/login','POST',dict(organization=org,username=user,password=password));self.csrf=r['csrf_token'];return r['user']
def wait_for(fn,timeout=30):
    until=time.time()+timeout
    while time.time()<until:
        v=fn()
        if v:return v
        time.sleep(.4)
    raise AssertionError('timed out waiting for durable worker')
def whatsapp_event(channel, secret, event_id, sender, text):
    payload={'object':'whatsapp_business_account','entry':[{'changes':[{'value':{'metadata':{'phone_number_id':'123456'},'messages':[{'id':event_id,'from':sender,'type':'text','text':{'body':text}}]}}]}]}
    body=json.dumps(payload,separators=(',',':')).encode()
    signature=hmac.new(secret.encode(),body,hashlib.sha256).hexdigest()
    request=urllib.request.Request(base+'/webhooks/'+channel,data=body,headers={'Content-Type':'application/json','X-Hub-Signature-256':'sha256='+signature})
    with urllib.request.urlopen(request,timeout=10) as response: assert response.status==200
def whatsapp_image_event(channel, secret, event_id, sender):
    payload={'object':'whatsapp_business_account','entry':[{'changes':[{'value':{'metadata':{'phone_number_id':'123456'},'messages':[{'id':event_id,'from':sender,'type':'image','image':{'id':'fixture-media-private-id','mime_type':'image/jpeg','sha256':'0'*64}}]}}]}]}
    body=json.dumps(payload,separators=(',',':')).encode()
    signature=hmac.new(secret.encode(),body,hashlib.sha256).hexdigest()
    request=urllib.request.Request(base+'/webhooks/'+channel,data=body,headers={'Content-Type':'application/json','X-Hub-Signature-256':'sha256='+signature})
    with urllib.request.urlopen(request,timeout=10) as response: assert response.status==200
def png():
    def chunk(t,d):return struct.pack('!I',len(d))+t+d+struct.pack('!I',zlib.crc32(t+d)&0xffffffff)
    return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('!2I5B',64,64,8,2,0,0,0))+chunk(b'IDAT',zlib.compress((b'\0'+b'\x40\x90\x60'*64)*64))+chunk(b'IEND',b'')
checks=[]
with tempfile.TemporaryDirectory(prefix='kusi-local-check-') as tmp:
    env['KUSI_LOCAL_STORAGE']=tmp+'/storage'
    log=open(tmp+'/server.log','w+')
    def start():
        global process
        process=subprocess.Popen(['target/debug/kusi-server'],env=env,stdout=log,stderr=log)
        for _ in range(100):
            try:urllib.request.urlopen(base+'/health/ready',timeout=1);return
            except (urllib.error.URLError,TimeoutError):
                if process.poll() is not None:log.seek(0);raise AssertionError('server exited: '+log.read()[-4000:])
                time.sleep(.15)
        raise AssertionError('server not ready')
    try:
        start();admin=Client();password=secrets.token_urlsafe(24)
        demo_payments=admin.call('/v1/demo/payments')['payments']
        assert len(demo_payments)==2 and all(row['synthetic'] for row in demo_payments)
        assert all('dni' not in str(row).lower() and 'student_user' not in row for row in demo_payments)
        dni_lookup=admin.call('/v1/demo/payments/by-dni','POST',dict(dni='70000001'))
        assert dni_lookup['student']['synthetic'] is True
        assert dni_lookup['student']['dni_masked']=='70***001'
        assert len(dni_lookup['payments'])==len(demo_payments)
        empty_history=admin.call('/v1/demo/payments/by-dni','POST',dict(dni='70000002'))
        assert empty_history['payments']==[]
        admin.call('/v1/demo/payments/by-dni','POST',dict(dni='70000003'),404)
        admin.call('/v1/demo/payments/by-dni','POST',dict(dni='123'),400)
        matched=admin.call('/v1/demo/payments/validate','POST',dict(code='V999-00004821'))['payment']
        assert matched['payment_code']=='UNCP-EXP-004821' and matched['amount_cents']==19800
        receipt=Path('web/admin/public/demo/recibo-sintetico.png').read_bytes()
        photo_match=admin.call('/v1/demo/payments/validate-receipt','POST',receipt,headers={'Content-Type':'image/png'})
        assert photo_match['validation_method']=='local_ocr'
        assert photo_match['payment']['payment_code']=='UNCP-EXP-004821'
        assert photo_match['detected_reference']=='V***-***4821' and 'ocr_text' not in photo_match
        admin.call('/v1/demo/payments/validate','POST',dict(code='UNCP-EXP-999999'),404)
        admin.call('/v1/demo/payments/validate','POST',dict(code='UNCP-EXP-004821'),403,{'Origin':'https://example.invalid'})
        checks.append('no-login loopback demo lists synthetic payments, demonstrates fixed fake-DNI lookup, and validates typed/OCR receipt codes against the local platform clone')
        assert admin.call('/v1/auth/mode')=={'mode':'local','oidc_enabled':False}
        admin.call('/v1/admin/auth/oidc/start',expected=404)
        admin.call('/v1/admin/auth/session',expected=401)
        checks.append('local mode reports OIDC disabled and rejects admin login routes')
        keys=[]
        for slug in ['fixture-local-a','fixture-local-b']:
            org=admin.call('/v1/organizations','POST',dict(name=slug,slug=slug),201,{'x-admin-key':env['KUSI_ADMIN_API_KEY']})
            headers={'x-admin-key':env['KUSI_ADMIN_API_KEY'],'Authorization':'Bearer '+org['api_key']}
            admin.call('/v1/local/bootstrap','POST',dict(username='owner',display_name='Fixture Owner',password=password),headers=headers)
            keys.append(org)
        owner=Client();user=owner.login('fixture-local-a','owner',password)
        other=Client();other.login('fixture-local-b','owner',password)
        editor=Client();viewer=Client();approver=Client()
        users={}
        for role,client in [('editor',editor),('viewer',viewer),('approver',approver)]:
            users[role]=owner.call('/v1/local/users','POST',dict(username=role,display_name=role,password=password,role=role));client.login('fixture-local-a',role,password)
        owner_current=owner.call('/v1/local/sessions')['sessions']
        assert len(owner_current)==4 and sum(session['current'] for session in owner_current)==1
        assert set(owner_current[0])=={'id','user_id','display_name','username','created_at','expires_at','current'}
        owner_second=Client();owner_second.login('fixture-local-a','owner',password)
        editor_second=Client();editor_second.login('fixture-local-a','editor',password)
        owner_sessions=owner.call('/v1/local/sessions')['sessions']
        assert len(owner_sessions)==6 and sum(s['current'] for s in owner_sessions)==1
        assert sum(s['user_id']==user['id'] for s in owner_sessions)==2
        other_session=other.call('/v1/local/sessions')['sessions'][0]
        assert all(s['id']!=other_session['id'] for s in owner_sessions)
        editor_sessions=editor.call('/v1/local/sessions')['sessions']
        assert len(editor_sessions)==2 and sum(s['current'] for s in editor_sessions)==1
        assert all(s['user_id']==users['editor']['id'] for s in editor_sessions)
        owner_current_id=next(s['id'] for s in owner_sessions if s['current'])
        owner.call('/v1/local/sessions/'+owner_current_id,'DELETE',expected=409)
        owner.call('/v1/local/sessions/'+other_session['id'],'DELETE',expected=404)
        editor.call('/v1/local/sessions/'+owner_current_id,'DELETE',expected=404)
        owner_target=next(s for s in owner_sessions if s['user_id']==user['id'] and not s['current'])
        owner.call('/v1/local/sessions/'+owner_target['id'],'DELETE',expected=204)
        owner_second.call('/v1/local/sessions',expected=401)
        editor_target=next(s for s in editor_sessions if not s['current'])
        editor.call('/v1/local/sessions/'+editor_target['id'],'DELETE',expected=204)
        editor_second.call('/v1/local/sessions',expected=401)
        assert len(owner.call('/v1/local/sessions')['sessions'])==4
        security_user=owner.call('/v1/local/users','POST',dict(username='session_subject',display_name='Session Subject',password=password,role='viewer'))
        security_client=Client();security_client.login('fixture-local-a','session_subject',password)
        prior_sessions=security_client.call('/v1/local/sessions')['sessions']
        assert len(prior_sessions)==1 and prior_sessions[0]['current']
        replacement_password=secrets.token_urlsafe(24)
        owner.call('/v1/local/users/'+security_user['id']+'/password','POST',dict(password=replacement_password),204)
        security_client.call('/v1/local/sessions',expected=401)
        assert sql(f"SELECT bool_and(revoked_at IS NOT NULL) FROM local_sessions WHERE user_id='{security_user['id']}'")=='t'
        security_client.login('fixture-local-a','session_subject',replacement_password)
        owner.call('/v1/local/users/'+security_user['id'],'PATCH',dict(role='viewer',disabled=True))
        security_client.call('/v1/local/sessions',expected=401)
        assert sql(f"SELECT bool_and(revoked_at IS NOT NULL) FROM local_sessions WHERE user_id='{security_user['id']}'")=='t'
        assert all(s['user_id']!=security_user['id'] for s in owner.call('/v1/local/sessions')['sessions'])
        checks.append('session inventory scopes, management UUIDs, current-session protection, atomic revocation, tenant isolation, password reset and account disable')
        checks.append('login Argon2, opaque sessions, organization isolation and four roles')
        case_list=viewer.call('/v1/local/cases')['cases']
        assert len(case_list)==1 and case_list[0]['workflow_key']=='uncp_titulacion_demo'
        case_id=case_list[0]['id']
        case=viewer.call('/v1/local/cases/'+case_id)['case']
        assert case['payment_amount_cents']==19800 and case['payment_reference_masked']=='****4821'
        assert case['health_interruption_summary']=='Interrumpido por motivos de salud'
        sources=viewer.call('/v1/local/cases/'+case_id+'/sources')['sources']
        assert {s['key'] for s in sources}=={'PA57002335','PA570004D8','PA57002563'}
        assert all(s['validity']=='needs_confirmation' for s in sources)
        guidance=viewer.call('/v1/local/cases/'+case_id+'/guidance','POST',dict(question='¿Cómo retomo el trámite?'))['guidance']
        assert guidance['assistant_mode']=='deterministic_demo' and len(guidance['citations'])==3
        assert 'debe confirmarse con Tesorería' in guidance['summary']
        viewer.call('/v1/local/cases/'+case_id+'/guidance','POST',dict(question='¿Cómo retomo?'),403,{'x-csrf-token':''})
        viewer.call('/v1/local/cases/'+case_id+'/requests','POST',dict(recipient_unit='Tesorería',body='Consulta de prueba',include_receipt_fixture=True),403,{'Origin':'https://example.invalid'})
        draft=viewer.call('/v1/local/cases/'+case_id+'/requests','POST',dict(recipient_unit='Tesorería - UNCP',body='Solicito confirmar estado y tratamiento del pago de PA57002335.',include_receipt_fixture=True))['request']
        viewer.call('/v1/local/cases/'+case_id+'/requests/'+draft['id']+'/simulate-send','POST',expected=403)
        viewer.call('/v1/local/cases/'+case_id+'/requests/'+draft['id']+'/approve','POST',dict(version=draft['version'],digest='0'*64,confirm_share=True),409)
        approved=viewer.call('/v1/local/cases/'+case_id+'/requests/'+draft['id']+'/approve','POST',dict(version=draft['version'],digest=draft['digest'],confirm_share=True))['request']
        assert approved['consented'] and approved['state']=='approved'
        simulated=viewer.call('/v1/local/cases/'+case_id+'/requests/'+draft['id']+'/simulate-send','POST')['request']
        assert simulated['state']=='simulated_sent'
        assert viewer.call('/v1/local/cases/'+case_id+'/requests/'+draft['id']+'/simulate-send','POST')['request']['id']==draft['id']
        owner.call('/v1/local/cases/'+case_id+'/responses','POST',dict(response_type='guidance',body='Personal de muestra: consultar con Tesorería.',citation_keys=['PA57002563']))
        assert len(viewer.call('/v1/local/cases/'+case_id)['case']['responses'])==1
        editor.call('/v1/local/cases/'+case_id+'/responses','POST',dict(response_type='guidance',body='No autorizado.',citation_keys=[]),403)
        other.call('/v1/local/cases/'+case_id,expected=404)
        checks.append('UNCP synthetic case, TUPA citations, needs-confirmation status, CSRF/Origin, versioned consent, simulated-send idempotency, assigned staff response, RBAC and tenant isolation')
        students=owner.call('/v1/local/academic/demo-students')
        assert len(students)==2 and all('ficticio' in row['display_label'] for row in students)
        viewer.call('/v1/local/academic/demo-students',expected=403)
        editor.call('/v1/local/academic/guardian-links',expected=403)
        owner.call('/v1/local/academic/guardian-links','POST',dict(user_id=users['viewer']['id'],student_ref=students[0]['student_ref'],relationship_verified=False),400)
        owner.call('/v1/local/academic/guardian-links','POST',dict(user_id=users['viewer']['id'],student_ref=students[0]['student_ref'],relationship_verified=True),201)
        owner.call('/v1/local/academic/guardian-links','POST',dict(user_id=users['viewer']['id'],student_ref=students[0]['student_ref'],relationship_verified=True),201)
        other_viewer=other.call('/v1/local/users','POST',dict(username='family',display_name='Other Fixture Viewer',password=password,role='viewer'))
        owner.call('/v1/local/academic/guardian-links','POST',dict(user_id=other_viewer['id'],student_ref=students[0]['student_ref'],relationship_verified=True),404)
        resources=owner.call('/v1/local/resource-access/resources')['resources']
        assert len(resources)==2 and all('demostración' in row['display_label'] for row in resources)
        editor.call('/v1/local/resource-access/grants',expected=403)
        owner.call('/v1/local/resource-access/execute','POST',dict(resource_ref='demo-resource-001'),403)
        owner.call('/v1/local/resource-access/grants','POST',dict(user_id=users['viewer']['id'],resource_ref='demo-resource-001',authorization_verified=False),400)
        viewer.call('/v1/local/resource-access/execute','POST',dict(resource_ref='demo-resource-001'),403)
        viewer.call('/v1/local/resource-access/execute','POST',dict(resource_ref='demo-resource-001',user_id=users['editor']['id']),422,raw=True)
        viewer.call('/v1/local/resource-access/execute','POST',dict(resource_ref='demo-resource-002'),403)
        owner.call('/v1/local/users/'+users['viewer']['id'],'PATCH',dict(role='viewer',disabled=True))
        owner.call('/v1/local/resource-access/grants','POST',dict(user_id=users['viewer']['id'],resource_ref='demo-resource-001',authorization_verified=True),404)
        owner.call('/v1/local/users/'+users['viewer']['id'],'PATCH',dict(role='viewer',disabled=False))
        viewer.login('fixture-local-a','viewer',password)
        owner.call('/v1/local/resource-access/grants','POST',dict(user_id=other_viewer['id'],resource_ref='demo-resource-001',authorization_verified=True),404)
        owner.call('/v1/local/resource-access/grants','POST',dict(user_id=users['viewer']['id'],resource_ref='demo-resource-001',authorization_verified=True),201)
        owner.call('/v1/local/resource-access/grants','POST',dict(user_id=users['viewer']['id'],resource_ref='demo-resource-001',authorization_verified=True),201)
        accessible=viewer.call('/v1/local/resource-access/resources')['resources']
        assert [r['resource_ref'] for r in accessible]==['demo-resource-001']
        execution=viewer.call('/v1/local/resource-access/execute','POST',dict(resource_ref='demo-resource-001'))
        assert execution['simulated'] is True and 'ficticio' in execution['result']['summary']
        owner.call('/v1/local/resource-access/grants/'+users['viewer']['id']+'/demo-resource-001','DELETE',expected=204)
        viewer.call('/v1/local/resource-access/execute','POST',dict(resource_ref='demo-resource-001'),403)
        owner.call('/v1/local/resource-access/grants/'+users['viewer']['id']+'/demo-resource-001','DELETE',expected=404)
        events=owner.call('/v1/local/resource-access/audit')['events']
        assert any(event['allowed'] and event['adapter_invoked'] for event in events)
        assert any(not event['allowed'] and not event['adapter_invoked'] for event in events)
        assert all(set(event)=={'actor_id','capability','resource_digest','allowed','adapter_invoked','created_at'} for event in events)
        assert other.call('/v1/local/resource-access/audit')['events']==[]
        checks.append('resource-specific session authorization, same-tenant viewer grants, idempotency, forged identity rejection, deny-before-fixture, revocation and payload-free audit')
        channel=str(uuid.uuid4());meta_secret=secrets.token_hex(32);meta_verify=secrets.token_hex(32);sender='999999'
        organization_id=keys[0]['organization']['id']
        env.update(FIXTURE_META_SECRET=meta_secret,FIXTURE_META_VERIFY=meta_verify,KUSI_CHANNELS=json.dumps({channel:{'organization_id':organization_id,'provider':'whatsapp','external_id':'123456','secret_env':'FIXTURE_META_SECRET','verify_token_env':'FIXTURE_META_VERIFY'}}))
        process.terminate();process.wait(timeout=10);start()
        owner.call('/v1/local/link-codes','POST',dict(channel_id=channel,user_id=other_viewer['id']),404)
        editor.call('/v1/local/link-codes','POST',dict(channel_id=channel,user_id=users['viewer']['id']),403)
        old_code=owner.call('/v1/local/link-codes','POST',dict(channel_id=channel,user_id=users['viewer']['id']))['code']
        current_code=owner.call('/v1/local/link-codes','POST',dict(channel_id=channel,user_id=users['viewer']['id']))['code']
        whatsapp_event(channel,meta_secret,'demo-link-old','999998','/vincular '+old_code)
        wait_for(lambda:sql("SELECT processed_at IS NOT NULL FROM channel_inbox WHERE event_id='demo-link-old'")== 't')
        assert sql(f"SELECT count(*) FROM local_channel_bindings WHERE organization_id='{organization_id}' AND channel_id='{channel}' AND sender='999998'")=='0'
        whatsapp_event(channel,meta_secret,'demo-link-current',sender,'/vincular '+current_code)
        wait_for(lambda:sql(f"SELECT count(*) FROM local_channel_bindings WHERE organization_id='{organization_id}' AND channel_id='{channel}' AND sender='{sender}' AND user_id='{users['viewer']['id']}'")=='1')
        query_id='demo-academic-query'
        whatsapp_event(channel,meta_secret,query_id,sender,'mis calificaciones')
        whatsapp_event(channel,meta_secret,query_id,sender,'mis calificaciones')
        conversation=wait_for(lambda:next((c for c in viewer.call('/v1/local/conversations') if not c['local']),None))
        messages=wait_for(lambda:(lambda value:value if any(m['direction']=='out' and 'Resultados académicos simulados' in m['content'] for m in value['messages']) else None)(viewer.call('/v1/local/conversations/'+conversation['id']+'/messages')))
        response=next(m for m in messages['messages'] if m['direction']=='out' and 'Resultados académicos simulados' in m['content'])
        assert response['metadata']['simulated'] is True and '16/20' in response['content'] and 'Estudiante ficticio A' in response['content']
        assert 'Estudiante ficticio B' not in response['content']
        assert sql(f"SELECT count(*) FROM channel_inbox WHERE channel_id='{channel}' AND event_id='{query_id}'")=='1'
        payment_query='demo-student-payment-list'
        whatsapp_event(channel,meta_secret,payment_query,sender,'mis pagos')
        listed=wait_for(lambda:(lambda value:value if any(m['direction']=='out' and 'Pagos del estudiante en la plataforma de demostración' in m['content'] for m in value['messages']) else None)(viewer.call('/v1/local/conversations/'+conversation['id']+'/messages')))
        payment_list_message=next(m for m in listed['messages'] if m['direction']=='out' and 'Pagos del estudiante en la plataforma de demostración' in m['content'])
        assert 'UNCP-EXP-004821' in payment_list_message['content']
        trace_id='demo-bureaucratic-trace'
        whatsapp_event(channel,meta_secret,trace_id,sender,'UNCP-EXP-004821')
        traced=wait_for(lambda:(lambda value:value if any(m['direction']=='out' and 'Pago confirmado en la plataforma demo' in m['content'] for m in value['messages']) else None)(viewer.call('/v1/local/conversations/'+conversation['id']+'/messages')))
        trace_message=next(m for m in traced['messages'] if m['direction']=='out' and 'Pago confirmado en la plataforma demo' in m['content'])
        assert 'interrumpido' in trace_message['content']
        assert trace_message['metadata']['simulation_kind']=='student_payment_platform_clone'
        image_id='demo-bureaucratic-image'
        whatsapp_image_event(channel,meta_secret,image_id,sender)
        received=wait_for(lambda:(lambda value:value if any(m['direction']=='out' and 'Recibí la foto en el webhook' in m['content'] for m in value['messages']) else None)(viewer.call('/v1/local/conversations/'+conversation['id']+'/messages')))
        image_reply=next(m for m in received['messages'] if m['direction']=='out' and 'Recibí la foto en el webhook' in m['content'])
        assert 'no descarga ni valida el contenido' in image_reply['content']
        assert sql(f"SELECT content FROM channel_inbox WHERE channel_id='{channel}' AND event_id='{image_id}'")=='image_attachment_received'
        assert 'fixture-media-private-id' not in sql(f"SELECT content FROM local_messages WHERE conversation_id='{conversation['id']}'")
        assert sql(f"SELECT count(*) FROM local_outbox WHERE channel_id='{channel}' AND recipient='{sender}' AND content LIKE 'Resultados académicos simulados%' AND status='queued'")=='1'
        owner.call('/v1/channels/'+channel+'/send','POST',{},403)
        whatsapp_event(channel,meta_secret,'demo-academic-unsupported',sender,'¿cuánto debo?')
        wait_for(lambda:sql(f"SELECT count(*) FROM local_outbox WHERE channel_id='{channel}' AND recipient='{sender}' AND content LIKE 'En la demo de pagos UNCP%' AND status='queued'")=='1')
        assert sql(f"SELECT count(*) FROM local_outbox WHERE channel_id='{channel}' AND recipient='{sender}' AND content LIKE '%16/20%' AND status='queued'")=='1'
        owner.call('/v1/local/academic/guardian-links/'+users['viewer']['id']+'/'+students[0]['student_ref'],'DELETE',expected=204)
        whatsapp_event(channel,meta_secret,'demo-academic-denied',sender,'mis calificaciones')
        messages=wait_for(lambda:(lambda value:value if any(m['direction']=='out' and 'no se consultó ningún registro' in m['content'] for m in value['messages']) else None)(viewer.call('/v1/local/conversations/'+conversation['id']+'/messages')))
        denied=next(m for m in messages['messages'] if m['direction']=='out' and 'no se consultó ningún registro' in m['content'])
        assert '16/20' not in denied['content']
        owner.call('/v1/local/users/'+users['viewer']['id'],'PATCH',dict(role='viewer',disabled=True))
        whatsapp_event(channel,meta_secret,'demo-academic-disabled',sender,'mis calificaciones')
        wait_for(lambda:sql(f"SELECT count(*) FROM local_outbox WHERE channel_id='{channel}' AND recipient='{sender}' AND content LIKE 'Vincula tu cuenta%' AND status='queued'")=='1')
        assert sql(f"SELECT count(*) FROM local_outbox WHERE channel_id='{channel}' AND recipient='{sender}' AND content LIKE '%16/20%' AND status='queued'")=='1'
        checks.append('fictional academic links, same-tenant viewer codes, signed WhatsApp payment history/code lookup, image receipt-only acknowledgment, student-level authorization, simulation label, and blocked outbound sends')
        viewer.call('/v1/local/conversations','POST',dict(title='forbidden'),401)
        editor.call('/v1/local/users','POST',{},403)
        owner.call('/v1/local/conversations','POST',dict(title='csrf'),403,{'x-csrf-token':''})
        owner.call('/v1/local/conversations','POST',dict(title='origin'),403,{'Origin':'https://untrusted.invalid'})
        owner.call('/v1/ai/generate','POST',{},403)
        # Session tokens must retain role restrictions when presented as bearer credentials.
        bearer=Client();bearer.call('/v1/creative/jobs/00000000-0000-0000-0000-000000000001/approve','POST',{},403,{'Authorization':'Bearer '+next(iter(editor.jar)).value,'x-kusi-local-admin':'forged'})
        stale=Client();stale.jar.set_cookie(http.cookiejar.Cookie(version=0,name='kusi_local_session',value='invalid-stale-session',port=None,port_specified=False,domain='127.0.0.1',domain_specified=False,domain_initial_dot=False,path='/',path_specified=True,secure=False,expires=None,discard=True,comment=None,comment_url=None,rest={}))
        stale.login('fixture-local-b','owner',password)
        checks.append('RBAC including bearer sessions, stale-cookie reauthentication, CSRF, origin validation and external-provider block')
        a=editor.call('/v1/local/assets?label=Producto%20ficticio&filename=product.png','POST',png())
        other.call('/v1/local/assets/'+a['id'],expected=404)
        other.call('/v1/local/assets/'+a['id']+'/content',expected=404)
        editor.call('/v1/local/assets?label=malformed&filename=bad.png','POST',b'not an image',400)
        checks.append('validated image upload, tenant-scoped content and malformed-file rejection')
        c=editor.call('/v1/local/conversations','POST',dict(title='Fixture product'))
        msg=dict(content='Crea un video',asset_id=a['id'],client_id=str(uuid.uuid4()))
        path='/v1/local/conversations/'+c['id']+'/messages'
        other.call(path,expected=404)
        editor.call(path,'POST',msg,202);editor.call(path,'POST',msg,202)
        editor.call(path,'POST',dict(msg,content='changed duplicate'),409)
        editor.call(path,'POST',dict(content='De este producto',asset_id=None,client_id=str(uuid.uuid4())),202)
        # Recover the pending inbox after a process stop, without creating a second logical job.
        process.terminate();process.wait(timeout=10);start()
        job=wait_for(lambda:next(iter(owner.call('/v1/creative/jobs')),None))
        assert job['state']=='awaiting_approval',job
        assert len(owner.call('/v1/creative/jobs'))==1
        other.call('/v1/creative/jobs/'+job['id'],expected=404)
        editor.call('/v1/creative/jobs/'+job['id']+'/approve','POST',dict(confirm=True,approval_digest=job['approval_digest'],max_cost_microusd=0),403)
        approver.call('/v1/creative/jobs/'+job['id']+'/approve','POST',dict(confirm=True,approval_digest=job['approval_digest'],max_cost_microusd=0))
        artifact=wait_for(lambda:next((x for x in owner.call('/v1/local/assets') if x['creative_job_id']==job['id']),None),40)
        data=owner.call('/v1/local/assets/'+artifact['id']+'/content',raw=True)
        preview=Path(tmp)/'result.mp4';preview.write_bytes(data)
        probe=json.loads(subprocess.check_output(['ffprobe','-v','error','-show_format','-of','json',str(preview)]))
        assert 2.5<float(probe['format']['duration'])<4,probe
        assert artifact['simulated'] and len(data)>1000
        checks.append('debounce, duplicate rejection, restart recovery, single job, approval and real archived MP4')
        editor.call(path,'POST',dict(content='Prepara para Instagram | Producto ficticio',asset_id=None,client_id=str(uuid.uuid4())),202)
        pub=wait_for(lambda:next(iter(owner.call('/v1/local/publications')),None))
        assert pub['asset_id']==artifact['id'] and pub['platform']=='instagram'
        editor.call('/v1/local/publications/'+pub['id']+'/approve','POST',dict(confirm=True,approval_digest=pub['approval_digest']),403)
        updated=dict(asset_id=artifact['id'],platform='instagram',caption='Texto revisado',privacy='private',scheduled_at=int(time.time())+5,idempotency_key=pub['idempotency_key'])
        revised=editor.call('/v1/local/publications/'+pub['id'],'PATCH',updated)
        approver.call('/v1/local/publications/'+pub['id']+'/approve','POST',dict(confirm=True,approval_digest=pub['approval_digest']),409)
        approver.call('/v1/local/publications/'+pub['id']+'/approve','POST',dict(confirm=True,approval_digest=revised['approval_digest']))
        result=wait_for(lambda:(lambda x:x if x['state']=='published_simulated' else None)(owner.call('/v1/local/publications/'+pub['id'])))
        assert result['receipt'].startswith('simulated:')
        editor.call(path,'POST',dict(content='Y para TikTok',asset_id=None,client_id=str(uuid.uuid4())),202)
        wait_for(lambda:any(x['platform']=='tiktok' and x['asset_id']==artifact['id'] for x in owner.call('/v1/local/publications')))
        checks.append('conversational follow-up, separate publication approval, stale digest rejection and scheduled simulated receipt')
        assert owner.call('/v1/local/activity')
        owner.call('/v1/local/retention/preview','POST',dict(days=90))
        owner.call('/v1/local/users/'+users['editor']['id'],'PATCH',dict(role='editor',disabled=True))
        editor.call('/local/auth/session',expected=401)
        owner.call('/local/auth/logout','POST',expected=204)
        owner.call('/v1/local/overview',expected=401)
        checks.append('audit, retention preview, user disable and logout revoke sessions')
        print(json.dumps({'status':'passed','checks':checks},ensure_ascii=False,indent=2))
    except Exception:
        log.flush();log.seek(0);print(log.read()[-5000:]);raise
    finally:
        if process and process.poll() is None:process.terminate();process.wait(timeout=15)
        log.close();sql('DROP SCHEMA '+schema+' CASCADE')
