"""Local authenticated webhook and durable deduplication check. No provider calls."""
import hashlib
import hmac
import json
import os
import secrets
import subprocess
import time
import urllib.error
import urllib.request
import uuid
from pathlib import Path

env = dict(os.environ)
env.update(dict(line.split('=', 1) for line in Path('.env').read_text().splitlines() if '=' in line and not line.startswith('#')))
base = os.environ.get('KUSI_API_URL', 'http://127.0.0.1:8080')
request = urllib.request.Request(base + '/v1/organizations/current', headers={'Authorization': 'Bearer ' + env['DEMO_ORGANIZATION_API_KEY']})
with urllib.request.urlopen(request, timeout=5) as response:
    organization = json.load(response)['id']
tg, wa = str(uuid.uuid4()), str(uuid.uuid4())
env['FIXTURE_TG_SECRET'] = secrets.token_hex(24)
env['FIXTURE_META_SECRET'] = secrets.token_hex(24)
env['FIXTURE_META_VERIFY'] = secrets.token_hex(24)
env['KUSI_CHANNELS'] = json.dumps({
    tg: {'organization_id': organization, 'provider': 'telegram', 'external_id': 'fixture-bot', 'secret_env': 'FIXTURE_TG_SECRET', 'verify_token_env': None},
    wa: {'organization_id': organization, 'provider': 'whatsapp', 'external_id': '123456', 'secret_env': 'FIXTURE_META_SECRET', 'verify_token_env': 'FIXTURE_META_VERIFY'},
})
env['BIND_ADDRESS'] = '127.0.0.1:18082'
process = subprocess.Popen(['target/debug/kusi-server'], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
base = 'http://127.0.0.1:18082'

def post(channel, payload, headers):
    body = json.dumps(payload, separators=(',', ':')).encode()
    if headers == 'meta':
        headers = {'X-Hub-Signature-256': 'sha256=' + hmac.new(env['FIXTURE_META_SECRET'].encode(), body, hashlib.sha256).hexdigest()}
    request = urllib.request.Request(base + '/webhooks/' + channel, data=body, headers={'Content-Type': 'application/json', **headers})
    try:
        with urllib.request.urlopen(request, timeout=5) as response:
            return response.status
    except urllib.error.HTTPError as error:
        return error.code

try:
    for _ in range(100):
        try:
            urllib.request.urlopen(base + '/health/ready', timeout=1).close()
            break
        except OSError:
            if process.poll() is not None:
                raise RuntimeError('fixture API failed startup')
            time.sleep(.1)
    payload = {'update_id': 1, 'message': {'from': {'id': 42}, 'chat': {'id': 42, 'type': 'private'}, 'text': 'fictional fixture'}}
    assert post(tg, payload, {}) == 401
    headers = {'X-Telegram-Bot-Api-Secret-Token': env['FIXTURE_TG_SECRET']}
    assert post(tg, payload, headers) == 200
    assert post(tg, payload, headers) == 200
    payload = {'object': 'whatsapp_business_account', 'entry': [{'changes': [{'value': {'metadata': {'phone_number_id': '123456'}, 'messages': [{'id': 'fixture-wa-1', 'from': '999999', 'type': 'text', 'text': {'body': 'fictional fixture'}}]}}]}]}
    assert post(wa, payload, {}) == 401
    assert post(wa, payload, 'meta') == 200
    assert post(wa, payload, 'meta') == 200
    payload['entry'][0]['changes'][0]['value']['metadata']['phone_number_id'] = 'other-channel'
    assert post(wa, payload, 'meta') == 403
    url = base + '/webhooks/' + wa + '?hub.mode=subscribe&hub.verify_token=' + env['FIXTURE_META_VERIFY'] + '&hub.challenge=fixture-challenge'
    with urllib.request.urlopen(url, timeout=5) as response:
        assert response.read() == b'fixture-challenge'
    query = f"SELECT count(*) FROM channel_inbox WHERE channel_id IN ('{tg}', '{wa}');"
    count = subprocess.check_output(['docker', 'compose', 'exec', '-T', 'postgres', 'psql', '-U', 'kusi', '-d', 'kusi', '-tAc', query], text=True).strip()
    assert count == '2', count
    print('PASS: Telegram secret, Meta raw-body signature and challenge, wrong-channel rejection, durable deduplication (2 rows for 4 valid deliveries)')
finally:
    process.terminate()
    process.wait(timeout=10)
