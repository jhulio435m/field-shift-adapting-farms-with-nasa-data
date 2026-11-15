"""Isolated social OAuth API/security checks. No real social accounts or external requests."""
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
schema = 'oauth_api_' + uuid.uuid4().hex

def sql(query):
    return subprocess.check_output(['psql', '-X', '-v', 'ON_ERROR_STOP=1', '-At', '-c', query], env=pg, stderr=subprocess.DEVNULL, text=True).strip()

sql('CREATE SCHEMA ' + schema)
query = dict(urllib.parse.parse_qsl(url.query))
query['options'] = '-csearch_path=' + schema
env['DATABASE_URL'] = urllib.parse.urlunsplit(url._replace(query=urllib.parse.urlencode(query)))
env['KUSI_ADMIN_API_KEY'] = secrets.token_hex(32)
for name in ['KUSI_CHANNELS', 'KUSI_LLM_PROFILES', 'KUSI_AUDIO_PROFILES', 'KUSI_CREATIVE_RECIPES', 'KUSI_HTTP_DESTINATIONS', 'KUSI_OAUTH_PROFILES']:
    env[name] = '{}'
env['KUSI_CREATIVE_WORKER'] = 'false'
env['KUSI_OAUTH_SIMULATION'] = 'true'
env['KUSI_OAUTH_REFRESH_WORKER'] = 'false'
env['FIXTURE_OAUTH_KEY1'] = secrets.token_hex(32)
env['FIXTURE_OAUTH_KEY2'] = secrets.token_hex(32)
env['KUSI_OAUTH_KEYRING'] = json.dumps({'key1': 'FIXTURE_OAUTH_KEY1', 'key2': 'FIXTURE_OAUTH_KEY2'})
env['KUSI_OAUTH_ACTIVE_KEY'] = 'key1'
env['BIND_ADDRESS'] = '127.0.0.1:' + os.environ.get('KUSI_OAUTH_TEST_PORT', '18086')
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
                raise RuntimeError('OAuth fixture failed startup')
            time.sleep(.1)
    raise RuntimeError('OAuth fixture timed out')

def stop():
    global process
    if process is not None:
        process.terminate()
        process.wait(timeout=10)
        process = None

def call(path, body=None, bearer=None, cookie=None, authenticated=True):
    headers = {'Content-Type': 'application/json'}
    if authenticated:
        headers['x-admin-key'] = env['KUSI_ADMIN_API_KEY']
        if bearer or token:
            headers['Authorization'] = 'Bearer ' + (bearer or token)
    if cookie:
        headers['Cookie'] = cookie
    request = urllib.request.Request(base + path, data=json.dumps(body).encode() if body is not None else None, headers=headers)
    try:
        with urllib.request.urlopen(request, timeout=5) as response:
            raw = response.read()
            return response.status, json.loads(raw) if raw else {}, response.headers
    except urllib.error.HTTPError as error:
        return error.code, {}, error.headers

def initiate(bearer=None):
    status, data, headers = call('/v1/oauth/simulated/start', {}, bearer=bearer)
    assert status == 200
    cookie = headers['Set-Cookie']
    for flag in ['Secure', 'HttpOnly', 'SameSite=Lax', 'Max-Age=600']:
        assert flag in cookie
    assert headers['Cache-Control'] == 'no-store'
    params = urllib.parse.parse_qs(urllib.parse.urlsplit(data['authorization_url']).query)
    assert params['code_challenge_method'] == ['S256']
    return params['state'][0], cookie.split(';', 1)[0]

def callback(state, cookie=None, provider='simulated', code='fixture-code', error=None):
    params = {'state': state}
    params.update({'error': error} if error else {'code': code})
    return call('/v1/oauth/' + provider + '/callback?' + urllib.parse.urlencode(params), cookie=cookie, authenticated=False)

try:
    start()
    status, org, _ = call('/v1/organizations', {'name': 'OAuth Fixture', 'slug': 'oauth-fixture'})
    assert status == 201
    token = org['api_key']
    _, other, _ = call('/v1/organizations', {'name': 'Other OAuth Fixture', 'slug': 'other-oauth-fixture'})
    tenant = org['organization']['id']
    other_tenant = other['organization']['id']
    stop()
    profile = {'provider': 'simulated', 'client_id': 'fixture-app', 'client_secret_env': None, 'redirect_uri': 'https://fixture.example/v1/oauth/simulated/callback', 'scopes': ['fixture.publish'], 'graph_version': None, 'pkce': True}
    env['KUSI_OAUTH_PROFILES'] = json.dumps({tenant: {'simulated': profile}, other_tenant: {'simulated': profile}})
    start()
    assert call('/v1/oauth/simulated/start', {}, authenticated=False)[0] == 401
    state, cookie = initiate()
    assert callback(state)[0] == 403
    assert callback(state, '__Host-kusi_oauth=' + 'x' * 43)[0] == 403
    assert callback(state, cookie, provider='facebook')[0] == 403
    status, connection, headers = callback(state, cookie)
    assert status == 200 and connection['status'] == 'active'
    assert 'access_token' not in connection and 'refresh_token' not in connection
    assert headers['Cache-Control'] == 'no-store' and 'Max-Age=0' in headers['Set-Cookie']
    account = connection['id']
    assert callback(state, cookie)[0] == 403
    assert call('/v1/oauth/connections', bearer=other['api_key'])[1] == []
    assert call('/v1/oauth/connections/' + account + '/refresh', {}, bearer=other['api_key'])[0] == 404
    ciphertext = sql(f"SELECT tokens_ciphertext FROM {schema}.oauth_connections WHERE id='{account}'")
    assert ciphertext.startswith('key1:') and 'SIMULATED' not in ciphertext
    assert sql(f"SELECT count(*) FROM {schema}.oauth_sessions WHERE verifier_ciphertext IS NOT NULL") == '0'
    other_state, other_cookie = initiate(other['api_key'])
    assert callback(other_state, other_cookie)[0] == 409  # Same app/account cannot be claimed by another tenant.
    assert call('/v1/oauth/connections/' + account + '/refresh', {})[0] == 200
    updated = sql(f"SELECT tokens_ciphertext FROM {schema}.oauth_connections WHERE id='{account}'")
    assert updated != ciphertext and updated.startswith('key1:')
    # Restart and rotate the encryption key while retaining the decryption key.
    stop(); env['KUSI_OAUTH_ACTIVE_KEY'] = 'key2'; start()
    assert call('/v1/oauth/connections/' + account + '/refresh', {})[0] == 200
    assert sql(f"SELECT tokens_ciphertext LIKE 'key2:%' FROM {schema}.oauth_connections WHERE id='{account}'") == 't'
    sql(f"UPDATE {schema}.oauth_connections SET status='refreshing',lease_until=now()+interval '1 minute' WHERE id='{account}'")
    assert call('/v1/oauth/connections/' + account + '/refresh', {})[0] == 409
    sql(f"UPDATE {schema}.oauth_connections SET lease_until=now()-interval '1 second' WHERE id='{account}'")
    assert call('/v1/oauth/connections/' + account + '/refresh', {})[0] == 409
    assert call('/v1/oauth/connections')[1][0]['status'] == 'needs_reauth'
    state, cookie = initiate()
    assert callback(state, cookie)[1]['id'] == account
    assert call('/v1/oauth/connections/' + account + '/disconnect', {})[1]['status'] == 'disconnected'
    assert sql(f"SELECT tokens_ciphertext IS NULL FROM {schema}.oauth_connections WHERE id='{account}'") == 't'
    state, cookie = initiate()
    sql(f"UPDATE {schema}.oauth_sessions SET expires_at=now()-interval '1 second' WHERE consumed_at IS NULL")
    assert callback(state, cookie)[0] == 403
    state, cookie = initiate()
    assert callback(state, cookie, error='access_denied')[0] == 400
    assert callback(state, cookie)[0] == 403
    assert sql(f"SELECT count(*)>0 FROM {schema}.oauth_events WHERE event='refreshed'") == 't'
    print('PASS OAuth API: browser/state binding, replay/expiry/provider rejection, encrypted token storage, tenant isolation, account ownership, refresh, key rotation, interrupted-refresh recovery and disconnect. No external OAuth calls.')
finally:
    stop()
    sql('DROP SCHEMA ' + schema + ' CASCADE')
