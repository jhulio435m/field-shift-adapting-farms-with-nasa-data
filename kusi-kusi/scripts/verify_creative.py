"""End-to-end creative lifecycle in an isolated schema; no external providers or paid requests."""
import json
import os
import secrets
import subprocess
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from pathlib import Path

env = dict(os.environ)
if Path('.env').exists():
    env.update(dict(line.split('=', 1) for line in Path('.env').read_text().splitlines() if '=' in line and not line.startswith('#')))
url = urllib.parse.urlsplit(env['DATABASE_URL'])
pg = dict(os.environ, PGHOST=url.hostname or '127.0.0.1', PGPORT=str(url.port or 5432), PGDATABASE=url.path.lstrip('/'), PGUSER=urllib.parse.unquote(url.username or ''), PGPASSWORD=urllib.parse.unquote(url.password or ''))
pg.pop('PGOPTIONS', None)
schema = 'creative_api_' + uuid.uuid4().hex

def sql(query):
    return subprocess.check_output(['psql', '-X', '-v', 'ON_ERROR_STOP=1', '-At', '-c', query], env=pg, stderr=subprocess.DEVNULL, text=True).strip()

sql('CREATE SCHEMA ' + schema)
query = dict(urllib.parse.parse_qsl(url.query))
query['options'] = '-csearch_path=' + schema
env['DATABASE_URL'] = urllib.parse.urlunsplit(url._replace(query=urllib.parse.urlencode(query)))
env['KUSI_ADMIN_API_KEY'] = secrets.token_hex(32)
for name in ['KUSI_CHANNELS', 'KUSI_LLM_PROFILES', 'KUSI_AUDIO_PROFILES', 'KUSI_CREATIVE_RECIPES', 'KUSI_HTTP_DESTINATIONS']:
    env[name] = '{}'
env['KUSI_CREATIVE_SIMULATION'] = 'true'
env['KUSI_CREATIVE_WORKER'] = 'false'
port = int(os.environ.get('KUSI_CREATIVE_TEST_PORT', '18084'))
env['BIND_ADDRESS'] = '127.0.0.1:' + str(port)
base = 'http://' + env['BIND_ADDRESS']
process = None
token = None

def start():
    global process
    process = subprocess.Popen(['target/debug/kusi-server'], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for _ in range(150):
        try:
            urllib.request.urlopen(base + '/health/ready', timeout=1).close()
            return
        except OSError:
            if process.poll() is not None:
                raise RuntimeError('fixture server startup failed')
            time.sleep(.1)
    raise RuntimeError('fixture server readiness timed out')

def stop():
    global process
    if process is not None:
        process.terminate()
        process.wait(timeout=10)
        process = None

def call(path, body=None, bearer=None):
    headers = {'x-admin-key': env['KUSI_ADMIN_API_KEY'], 'Content-Type': 'application/json'}
    if bearer or token:
        headers['Authorization'] = 'Bearer ' + (bearer or token)
    request = urllib.request.Request(base + path, data=json.dumps(body).encode() if body is not None else None, headers=headers)
    try:
        with urllib.request.urlopen(request, timeout=5) as response:
            raw = response.read()
            return response.status, json.loads(raw) if raw else {}
    except urllib.error.HTTPError as error:
        return error.code, {}

try:
    start()
    status, org = call('/v1/organizations', {'name': 'Creative API Fixture', 'slug': 'creative-api-fixture'})
    assert status == 201
    token = org['api_key']
    _, other = call('/v1/organizations', {'name': 'Other Fixture', 'slug': 'other-fixture'})
    tenant = org['organization']['id']
    stop()
    recipe = {'provider': 'simulated', 'model': 'fixture', 'key_env': None, 'input_schema': {'type': 'object', 'required': ['prompt'], 'additionalProperties': False, 'properties': {'prompt': {'type': 'string', 'maxLength': 200}}}, 'defaults': {}, 'estimated_cost_microusd': 0, 'enabled': True}
    env['KUSI_CREATIVE_RECIPES'] = json.dumps({tenant: {'product_video': recipe}})
    env['KUSI_CREATIVE_WORKER'] = 'true'
    start()
    assert call('/v1/creative/recipes')[1]['recipes'][0]['recipe'] == 'product_video'
    assert call('/v1/creative/budget')[1]['daily_limit_microusd'] == 0
    assert call('/v1/creative/budget', {'daily_limit_microusd': 100})[0] == 204
    assert call('/v1/creative/budget')[1]['daily_limit_microusd'] == 100
    assert call('/v1/creative/recipes', bearer=other['api_key'])[1]['recipes'] == []
    draft = {'recipe': 'product_video', 'input': {'prompt': 'fictional product'}, 'idempotency_key': str(uuid.uuid4())}
    status, job = call('/v1/creative/jobs', draft)
    assert status == 201 and job['state'] == 'awaiting_approval'
    path = '/v1/creative/jobs/' + job['id']
    assert call('/v1/creative/jobs', draft)[1]['id'] == job['id']
    assert call(path, bearer=other['api_key'])[0] == 404
    assert call('/v1/creative/jobs', bearer=other['api_key'])[1] == []
    altered = dict(draft, input={'prompt': 'changed'})
    assert call('/v1/creative/jobs', altered)[0] == 409
    assert call(path + '/approve', {'confirm': True, 'approval_digest': '0' * 64, 'max_cost_microusd': 0})[0] == 409
    assert call(path + '/approve', {'confirm': False, 'approval_digest': job['approval_digest'], 'max_cost_microusd': 0})[0] == 400
    time.sleep(2.2)
    assert call(path)[1]['state'] == 'awaiting_approval'
    # Persisted draft survives restart unchanged.
    stop(); start()
    assert call(path)[1]['approval_digest'] == job['approval_digest']
    approval = {'confirm': True, 'approval_digest': job['approval_digest'], 'max_cost_microusd': 0}
    assert call(path + '/approve', approval)[0] == 200
    for _ in range(60):
        status, current = call(path)
        if current['state'] == 'succeeded':
            break
        assert current['state'] in ['approved', 'submitting', 'queued', 'processing'], current['state']
        time.sleep(.25)
    assert current['state'] == 'succeeded' and current['result']['simulated'] is True
    assert call(path + '/approve', approval)[1]['remote_request_id'] == current['remote_request_id']
    assert call(path + '/cancel', {})[0] == 409
    draft['idempotency_key'] = str(uuid.uuid4())
    _, cancel = call('/v1/creative/jobs', draft)
    cancel_path = '/v1/creative/jobs/' + cancel['id']
    assert call(cancel_path + '/cancel', {})[1]['state'] == 'canceled'
    assert sql(f"SELECT count(*) FROM {schema}.creative_events WHERE job_id='{job['id']}' AND event='submitting'") == '1'
    assert sql(f"SELECT count(*) FROM {schema}.creative_events WHERE job_id='{job['id']}' AND event='succeeded'") == '1'
    print('PASS creative API: recipes, tenant isolation, immutable approval, restart persistence, deduplication, worker completion, cancellation and audit. No media generated or money spent.')
finally:
    stop()
    sql('DROP SCHEMA ' + schema + ' CASCADE')
