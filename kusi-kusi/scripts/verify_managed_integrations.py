"""010 acceptance against disposable PostgreSQL and the compiled server. No real credentials."""
import argparse
import json
import os
from pathlib import Path
import secrets
import uuid
import subprocess
from http.server import BaseHTTPRequestHandler, HTTPServer
import threading
import time
from local_acceptance import ROOT
from local_acceptance import postgres, sql
from verify_external_access import server
from verify_connection_settings import Client, bootstrap


def check_managed_assistant(env, tmp, org, principal):
    requests=[]
    class Planner(BaseHTTPRequestHandler):
        def log_message(self, *_args): pass
        def do_POST(self):
            body=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
            prompt=json.loads(body['messages'][-1]['content']);requests.append('synthetic')
            decision={'type':'query','plan':{'version':prompt['catalog']['version'],'queries':[{'resource':'alpha::items','filters':[],'analyses':[{'op':'count'}]}]}}
            payload=json.dumps({'choices':[{'finish_reason':'stop','message':{'content':json.dumps(decision)}}]}).encode()
            self.send_response(200);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(payload)));self.end_headers();self.wfile.write(payload)
    model=HTTPServer(('127.0.0.1',0),Planner);thread=threading.Thread(target=model.serve_forever,daemon=True);thread.start()
    try:
        source=tmp/'managed-source.json'
        source.write_text(json.dumps({'catalog':{'source':'Synthetic Plane fixture','version':'ignored','resources':[{'id':'items','label':'Synthetic records','fields':[{'id':'amount','label':'Amount','kind':'number','options':[]}],'events':[]}]},'rows':{'items':[{'amount':1},{'amount':2},{'amount':3}]}}));source.chmod(0o600)
        config={'organization':org,'principal':principal,'read':True,'subscribe':False,'sources':[{'id':'alpha','label':'Synthetic Plane','kind':'manifest','config_file':str(source)}],'state_directory':str(tmp/'managed-state'),'bridge_script':str(ROOT/'scripts/adaptive_frappe_bridge.py'),'binding_hash':'synthetic','llm':{'provider':'openai_compatible','model':'synthetic','api_key_env':None,'endpoint':f'http://127.0.0.1:{model.server_port}/v1/chat/completions','max_output_tokens':1000,'timeout_ms':5000,'tools_supported':False,'anthropic_version':None,'routing_providers':None},'access':{'database_url_env':'DATABASE_URL','resources':{'alpha::items':{'adapter':'plane','capability':'plane.resource.read','version':'1','resource_ref':'fixture-a'}}},'managed_fixture':{'kind':'plane','storage_directory':str(tmp/'storage')}}
        path=tmp/'managed-runtime.json';path.write_text(json.dumps(config));path.chmod(0o600)
        settings=dict(env,KUSI_ADAPTIVE_STAGING_ACK='authorized-environment',KUSI_ADAPTIVE_REQUIRE_ACL='true',KUSI_LOCAL_MODE='false')
        def invoke():
            run=subprocess.run([str(ROOT/'target/debug/adaptive_assistant'),str(path)],input=json.dumps({'message':'count','request_id':'managed-count'}),text=True,capture_output=True,env=settings,cwd=tmp,timeout=30,check=True)
            return json.loads(run.stdout)
        result=invoke();assert result['status']=='completed' and result['evidence'][0]['results'][0]['value']==3
        sql(env,"UPDATE managed_connection_verifications SET status='stale' WHERE organization_id='"+org+"'")
        result=invoke();assert result['status']=='blocked' and len(requests)==1
    finally:
        model.shutdown();model.server_close();thread.join(timeout=2)


def verify():
    with postgres() as (env, tmp):
        subprocess.run(['cargo','test','-p','kusi-infrastructure','--test','managed_integrations','--','--ignored'],env=env,check=True,timeout=180)
        env['KUSI_ADMIN_API_KEY'] = secrets.token_hex(32)
        with server(env, tmp, True) as base:
            password = bootstrap(env, base, 'managed-fixture', 'fixture-owner')
            other_password = bootstrap(env, base, 'other-fixture', 'other-owner')
            client = Client(base); client.login('managed-fixture', 'fixture-owner', password)
            other = Client(base); other.login('other-fixture', 'other-owner', other_password)
            manifests = client.call('/v1/local/connector-manifests')
            assert len(manifests) == 3
            for kind in ['plane', 'frappe', 'telegram']:
                path = '/v1/local/connection-settings/' + kind
                body = {'label': 'Synthetic ' + kind, 'credentials': {'bot_token': '123456:synthetic_telegram_token_010'} if kind == 'telegram' else {'api_key': 'synthetic-key', **({'api_secret': 'synthetic-secret'} if kind == 'frappe' else {})}}
                if kind != 'telegram': body['base_url'] = 'https://synthetic.example.invalid'
                profile = client.call(path, 'PUT', body)
                result = client.call(path + '/verify', 'POST', {'expected_revision': 1, 'mode': 'fixture'})
                assert result['mode'] == 'fixture' and result['status'] == 'verified'
                assert not result['runtime_active']
                client.call(path + '/verify', 'POST', {'expected_revision': 1, 'mode': 'provider'}, 403)
                client.call(path + '/enable', 'POST', {'expected_revision': 1, 'verification_id': result['id']}, 403)
                other.call(path + '/status', expected=404)
                client.call(path + '/disable', 'POST', {'expected_revision': 1})
                client.call(path + '/verify', 'POST', {'expected_revision': 1, 'mode': 'fixture', 'tenant_id': 'fake'}, 422)
                client.call(path + '/verify', 'POST', {'expected_revision': 1, 'mode': 'fixture'}, 403, {'x-csrf-token': ''})
                body['expected_revision'] = 1
                client.call(path, 'PUT', body)
                status = client.call(path + '/status')
                assert status['status'] == 'not_verified' and status['profile_revision'] == 2
                client.call(path + '/verify', 'POST', {'expected_revision': 1, 'mode': 'fixture'}, 409)
                client.call(path + '/credentials', 'DELETE', {'expected_revision': 2})
                result = client.call(path + '/verify', 'POST', {'expected_revision': 3, 'mode': 'fixture'})
                assert result['status'] == 'failed' and result['error_code'] == 'missing_credentials'
            assert client.call('/v1/local/connection-settings')[0]['runtime_active'] is False
            # A current, verified fixture profile is consumed through the real resource ACL gate.
            path = '/v1/local/connection-settings/plane'
            client.call(path, 'PUT', {'label':'Plane synthetic','base_url':'https://synthetic.example.invalid','expected_revision':3,'credentials':{'api_key':'synthetic-key'}})
            client.call(path+'/verify','POST',{'expected_revision':4,'mode':'fixture'})
            query = {'kind':'plane','version':'1','operation':'plane.resource.read','input':{'resource_ref':'fixture-a'},'expected_revision':4}
            client.call('/v1/local/managed-queries','POST',query,403)
            org = sql(env,"SELECT organization_id FROM local_users WHERE username='fixture-owner'")
            actor = sql(env,"SELECT id FROM local_users WHERE username='fixture-owner'")
            principal = sql(env,f"SELECT key_hash FROM local_sessions WHERE user_id='{actor}' ORDER BY created_at DESC LIMIT 1")
            sql(env,f"INSERT INTO capabilities(organization_id,identifier,version,description,adapter,required_permission,enabled,timeout_ms) VALUES('{org}','plane.resource.read','1','Synthetic','plane','managed.read',true,1000)")
            sql(env,f"INSERT INTO capability_grants VALUES('{org}','{principal}','managed.read')")
            # A capability grant alone must not bypass mandatory identity/resource grants.
            client.call('/v1/local/managed-queries','POST',query,403)
            sql(env,f"INSERT INTO external_resource_policies(organization_id,adapter) VALUES('{org}','plane')")
            sql(env,f"INSERT INTO external_identity_links(organization_id,principal_hash,adapter,external_subject,expires_at) VALUES('{org}','{principal}','plane','synthetic-subject',now()+interval '1 hour')")
            sql(env,"INSERT INTO external_resource_grants(link_id,capability,version,resource_ref,expires_at) SELECT id,'plane.resource.read','1','fixture-a',now()+interval '30 minutes' FROM external_identity_links WHERE adapter='plane'")
            read = client.call('/v1/local/managed-queries','POST',query)
            assert read['mode']=='fixture' and not read['provider_active'] and read['result']['evidence']
            client.call('/v1/local/managed-queries','POST',{**query,'input':{'resource_ref':'fixture-a','sql':'unsafe'}},400)
            check_managed_assistant(env,tmp,org,principal)
            client.call(path+'/verify','POST',{'expected_revision':4,'mode':'fixture'})
            for provider in ['frappe','telegram']:
                operation='telegram.context.read' if provider=='telegram' else 'frappe.resource.read'
                profile_path='/v1/local/connection-settings/'+provider
                body={'label':'Synthetic '+provider,'expected_revision':3,'credentials':{'bot_token':'123456:synthetic_telegram_token_010'} if provider=='telegram' else {'api_key':'synthetic-key','api_secret':'synthetic-secret'}}
                if provider=='frappe':body['base_url']='https://synthetic.example.invalid'
                client.call(profile_path,'PUT',body);client.call(profile_path+'/verify','POST',{'expected_revision':4,'mode':'fixture'})
                sql(env,f"INSERT INTO capabilities(organization_id,identifier,version,description,adapter,required_permission,enabled,timeout_ms) VALUES('{org}','{operation}','1','Synthetic','{provider}','managed.read',true,1000)")
                sql(env,f"INSERT INTO external_resource_policies(organization_id,adapter) VALUES('{org}','{provider}')")
                sql(env,f"INSERT INTO external_identity_links(organization_id,principal_hash,adapter,external_subject,expires_at) VALUES('{org}','{principal}','{provider}','synthetic-subject',now()+interval '1 hour')")
                sql(env,f"INSERT INTO external_resource_grants(link_id,capability,version,resource_ref,expires_at) SELECT id,'{operation}','1','fixture-a',now()+interval '30 minutes' FROM external_identity_links WHERE adapter='{provider}'")
                request={**query,'kind':provider,'operation':operation}
                assert client.call('/v1/local/managed-queries','POST',request)['mode']=='fixture'
                sql(env,f"UPDATE external_resource_grants SET revoked_at=now() WHERE capability='{operation}'")
                client.call('/v1/local/managed-queries','POST',request,403)
            sql(env,"UPDATE external_resource_grants SET revoked_at=now()")
            client.call('/v1/local/managed-queries','POST',query,403)
            # Version-bound revoke and durable outbox cancellation/recovery, without body readback.
            channel, binding, conversation, execution = [str(uuid.uuid4()) for _ in range(4)]
            sql(env,f"INSERT INTO local_channel_bindings(organization_id,channel_id,sender,user_id,id) VALUES('{org}','{channel}','synthetic-chat','{actor}','{binding}')")
            rows=client.call('/v1/local/channel-operations/telegram')['bindings'];assert len(rows)==1 and 'sender' not in rows[0]
            other.call(f'/v1/local/channel-operations/telegram/bindings/{binding}/revoke','POST',{'expected_version':1},404)
            client.call(f'/v1/local/channel-operations/telegram/bindings/{binding}/revoke','POST',{'expected_version':2},409)
            client.call(f'/v1/local/channel-operations/telegram/bindings/{binding}/revoke','POST',{'expected_version':1},204)
            client.call(f'/v1/local/channel-operations/telegram/bindings/{binding}/revoke','POST',{'expected_version':1},204)
            # Exercise the compiled worker with duplicate/out-of-order synthetic inbox events.
            for event in ['synthetic-102','synthetic-101','synthetic-102']:
                sql(env,f"INSERT INTO channel_inbox(organization_id,channel_id,event_id,sender,kind,content) VALUES('{org}','{channel}','{event}','synthetic-chat','text','estado') ON CONFLICT(channel_id,event_id) DO NOTHING")
            for _ in range(80):
                if sql(env,f"SELECT count(*) FROM channel_inbox WHERE channel_id='{channel}' AND processed_at IS NOT NULL")=='2':break
                time.sleep(.15)
            else:raise AssertionError('compiled worker did not finish synthetic events')
            assert sql(env,f"SELECT count(*) FROM local_outbox WHERE channel_id='{channel}'")=='1'
            assert sql(env,f"SELECT bool_and(c.user_id IS NULL) FROM local_outbox o JOIN local_conversations c ON c.id=o.conversation_id WHERE o.channel_id='{channel}'")=='t'
            sql(env,f"UPDATE local_channel_bindings SET revoked_at=NULL,expires_at=now()-interval '1 second' WHERE id='{binding}'")
            sql(env,f"INSERT INTO channel_inbox(organization_id,channel_id,event_id,sender,kind,content) VALUES('{org}','{channel}','synthetic-expired','synthetic-chat','text','estado')")
            for _ in range(80):
                if sql(env,f"SELECT count(*) FROM channel_inbox WHERE channel_id='{channel}' AND processed_at IS NOT NULL")=='3':break
                time.sleep(.15)
            else:raise AssertionError('compiled worker did not finish expired binding case')
            assert sql(env,f"SELECT bool_and(c.user_id IS NULL) FROM local_outbox o JOIN local_conversations c ON c.id=o.conversation_id WHERE o.channel_id='{channel}'")=='t'
            client.call(f'/v1/local/channel-operations/telegram/bindings/{binding}/revoke','POST',{'expected_version':2},204)
            sql(env,f"INSERT INTO local_conversations(id,organization_id,user_id,channel_id,sender,title) VALUES('{conversation}','{org}','{actor}','{conversation}','synthetic-chat','Synthetic')")
            sql(env,f"INSERT INTO local_outbox(id,organization_id,conversation_id,channel_id,recipient,content) VALUES('{execution}','{org}','{conversation}','{conversation}','synthetic-chat','private-synthetic-message')")
            execution_path='/v1/local/managed-executions/'+execution
            current=client.call(execution_path);assert current['state']=='pending' and len(current['timeline'])==1
            assert 'private-synthetic-message' not in json.dumps(current)
            other.call(execution_path,expected=404)
            client.call(execution_path+'/resume','POST',{'expected_version':1,'proposal_digest':'0'*64},409)
            client.call(execution_path+'/resume','POST',{'expected_version':1,'proposal_digest':current['proposal_digest']})
            cancelled=client.call(execution_path+'/cancel','POST',{'expected_version':1});assert cancelled['state']=='cancelled' and len(cancelled['timeline'])==2
            sql(env,f"UPDATE local_outbox SET status='awaiting_review' WHERE id='{execution}'")
            proposal=client.call(execution_path)
            preview=client.call(execution_path+'/proposal');assert preview['content']=='private-synthetic-message' and preview['proposal_digest']==proposal['proposal_digest']
            other.call(execution_path+'/proposal',expected=404)
            client.call(execution_path+'/review','POST',{'expected_version':proposal['version'],'proposal_digest':proposal['proposal_digest'],'decision':'approve'})
            sql(env,f"UPDATE local_outbox SET content='changed-synthetic-message' WHERE id='{execution}'")
            edited=client.call(execution_path);assert edited['state']=='awaiting_review' and edited['proposal_digest']!=proposal['proposal_digest']
            client.call(execution_path+'/review','POST',{'expected_version':proposal['version'],'proposal_digest':proposal['proposal_digest'],'decision':'approve'},409)
            sql(env,f"UPDATE local_outbox SET status='sending' WHERE id='{execution}'")
            running=client.call(execution_path)
            requested=client.call(execution_path+'/cancel','POST',{'expected_version':running['version']});assert requested['state']=='cancel_requested'
            sql(env,f"UPDATE local_outbox SET status='unknown' WHERE id='{execution}'")
            uncertain=client.call(execution_path)
            client.call(execution_path+'/resume','POST',{'expected_version':uncertain['version'],'proposal_digest':uncertain['proposal_digest']},409)
            assert sql(env,f"SELECT count(*) FROM local_outbox WHERE id='{execution}'")=='1'
            evaluation=client.call('/v1/local/integration-evaluations','POST',{'suite_version':'1'})
            assert evaluation['result']['passed']==24 and not evaluation['result']['quality_evaluated']
            assert client.call('/v1/local/integration-evaluations/'+evaluation['id'])['total']==24
            other.call('/v1/local/integration-evaluations/'+evaluation['id'],expected=404)
            # Memory is opt-in, actor-scoped, and invalidated durably when its source grant is revoked.
            viewer_password=secrets.token_urlsafe(24)
            viewer_user=client.call('/v1/local/users','POST',{'username':'fixture-viewer','display_name':'Synthetic viewer','role':'viewer','password':viewer_password})
            viewer=Client(base);viewer.login('managed-fixture','fixture-viewer',viewer_password)
            viewer.call('/v1/local/connector-manifests',expected=403)
            viewer.call('/v1/local/integration-evaluations','POST',{'suite_version':'1'},403)
            pref='/v1/local/authorized-memories/preferences'
            assert not viewer.call(pref)['consent']
            viewer.call(pref,'PUT',{'consent':True,'ttl_seconds':604801},400)
            viewer.call(pref,'PUT',{'consent':True,'ttl_seconds':600},204)
            grant={'user_id':viewer_user['id'],'resource_ref':'demo-resource-001','authorization_verified':True}
            client.call('/v1/local/resource-access/grants','POST',grant,201)
            before=sql(env,'SELECT count(*) FROM local_resource_grants')
            decision=client.call('/v1/local/policy-simulations','POST',{'operation':'demo.resource.read','resource_reference':'demo-resource-001','subject_user_id':viewer_user['id']})
            assert decision['allowed'] and before==sql(env,'SELECT count(*) FROM local_resource_grants')
            viewer.call('/v1/local/resource-access/execute','POST',{'resource_ref':'demo-resource-001'})
            memories=viewer.call('/v1/local/authorized-memories');assert len(memories)==1
            memory_id=memories[0]['id'];other.call('/v1/local/authorized-memories/'+memory_id,'DELETE',expected=404)
            raw=sql(env,f"SELECT ciphertext FROM authorized_memories WHERE id='{memory_id}'")
            assert raw and 'Resumen' not in raw and 'demo-resource' not in raw
            # Actual retrieval rechecks authorization; delete invalidates even after re-grant.
            viewer.call('/v1/local/resource-access/execute','POST',{'resource_ref':'demo-resource-001'})
            client.call('/v1/local/resource-access/grants/'+viewer_user['id']+'/demo-resource-001','DELETE',expected=204)
            assert viewer.call('/v1/local/authorized-memories')==[]
            assert sql(env,f"SELECT ciphertext IS NULL FROM authorized_memories WHERE id='{memory_id}'")=='t'
            viewer.call('/v1/local/resource-access/execute','POST',{'resource_ref':'demo-resource-001'},403)
            client.call('/v1/local/resource-access/grants','POST',grant,201)
            assert viewer.call('/v1/local/authorized-memories')==[]
            viewer.call('/v1/local/authorized-memories/'+memory_id,'DELETE',expected=204)
            grant_b={**grant,'resource_ref':'demo-resource-002'}
            client.call('/v1/local/resource-access/grants','POST',grant_b,201)
            viewer.call('/v1/local/resource-access/execute','POST',{'resource_ref':'demo-resource-002'})
            memory_b=viewer.call('/v1/local/authorized-memories')[0]['id']
            sql(env,f"UPDATE authorized_memories SET expires_at=now()-interval '1 second' WHERE id='{memory_b}'")
            assert viewer.call('/v1/local/authorized-memories')==[]
            viewer.call('/v1/local/resource-access/execute','POST',{'resource_ref':'demo-resource-002'})
            assert viewer.call('/v1/local/authorized-memories')==[]
            sql(env,f"UPDATE authorized_memories SET source_version='old-fixture-version',expires_at=now()+interval '1 minute' WHERE id='{memory_b}'")
            viewer.call('/v1/local/resource-access/execute','POST',{'resource_ref':'demo-resource-002'})
            assert sql(env,f"SELECT ciphertext IS NULL FROM authorized_memories WHERE id='{memory_b}'")=='t'
            # Verify state survives restarting the actual compiled server below.
        with server(env,tmp,True) as base:
            client=Client(base);client.login('managed-fixture','fixture-owner',password)
            current=client.call(execution_path);assert current['state']=='uncertain'
            assert client.call('/v1/local/integration-evaluations/'+evaluation['id'])['passed']==24
            assert client.call('/v1/local/connection-settings/plane/status')['profile_revision']==4
            assert client.call('/v1/local/channel-operations/telegram')['bindings'][0]['revoked']

    return {'status': 'passed', 'external_calls': False, 'checks': ['three manifests', 'fixture revision lifecycle', 'local provider/activation block', 'rotation and clear invalidate verification', 'tenant isolation', 'CSRF and unknown field rejection', 'typed fixture read through 007 identity/resource ACL', 'revocation blocks fixture evidence', 'binding revoke is versioned and tenant scoped', 'durable outbox timeline, CAS cancel and uncertain no-retry', '24 deterministic evaluation cases persisted', 'memory opt-in, encrypted, actor-scoped and revoked source erased', 'read-only permission simulation', 'real compiled-server restart preserves state', 'compiled adaptive assistant consumes saved fixture profile, ACL and revision, with stale receipt denied before loopback planner', 'typed synthetic reads and revocation for Plane, Frappe and Telegram', 'expired and changed-source memory never contributes to recovery', 'compiled inbox worker deduplicates reordered events and refuses revoked or expired binding context']}


if __name__ == '__main__':
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--result', type=Path)
    args = parser.parse_args()
    result = verify()
    if args.result: args.result.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))
