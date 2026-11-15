"""Exercise tenant-scoped inference and multipart transcription with local simulated providers."""
import json
import os
import subprocess
import threading
import time
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

class Provider(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_POST(self):
        body = self.rfile.read(int(self.headers['Content-Length']))
        if self.path == '/v1/chat/completions':
            value = json.loads(body)
            assert value['model'] == 'fixture-chat'
            assert value['messages'][1]['content'] == 'fictional input'
            result = {'choices': [{'finish_reason': 'stop', 'message': {'content': 'fixture response'}}]}
        elif self.path == '/v1/audio/transcriptions':
            assert 'multipart/form-data' in self.headers['Content-Type']
            for field in [b'name="file"', b'filename="voice.ogg"', b'fictional-audio-bytes', b'fixture-whisper', b'name="language"', b'es']:
                assert field in body
            result = {'text': 'transcripcion ficticia'}
        else:
            self.send_error(404)
            return
        encoded = json.dumps(result).encode()
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)

env = dict(os.environ)
env.update(dict(line.split('=', 1) for line in Path('.env').read_text().splitlines() if '=' in line and not line.startswith('#')))
headers = {'Authorization': 'Bearer ' + env['DEMO_ORGANIZATION_API_KEY'], 'x-admin-key': env['KUSI_ADMIN_API_KEY']}
bootstrap = os.environ.get('KUSI_API_URL', 'http://127.0.0.1:8080')
with urllib.request.urlopen(urllib.request.Request(bootstrap + '/v1/organizations/current', headers=headers), timeout=5) as response:
    tenant = json.load(response)['id']
provider = ThreadingHTTPServer(('127.0.0.1', 0), Provider)
thread = threading.Thread(target=provider.serve_forever, daemon=True)
thread.start()
endpoint = 'http://127.0.0.1:' + str(provider.server_port)
env['KUSI_LLM_PROFILES'] = json.dumps({tenant: {'local': {'provider': 'openai_compatible', 'model': 'fixture-chat', 'endpoint': endpoint + '/v1/chat/completions', 'max_output_tokens': 100, 'timeout_ms': 5000, 'tools_supported': False}}})
env['KUSI_AUDIO_PROFILES'] = json.dumps({tenant: {'audio': {'provider': 'local', 'model': 'fixture-whisper', 'endpoint': endpoint + '/v1/audio/transcriptions', 'timeout_ms': 5000}}})
env['BIND_ADDRESS'] = '127.0.0.1:18083'
env['KUSI_CHANNELS'] = '{}'
process = subprocess.Popen(['target/debug/kusi-server'], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
base = 'http://127.0.0.1:18083'

def call(path, body, content_type='application/json'):
    request = urllib.request.Request(base + path, data=body, headers={**headers, 'Content-Type': content_type})
    try:
        with urllib.request.urlopen(request, timeout=10) as response:
            return response.status, json.load(response)
    except urllib.error.HTTPError as error:
        return error.code, {}

try:
    for _ in range(100):
        try:
            urllib.request.urlopen(base + '/health/ready', timeout=1).close()
            break
        except OSError:
            if process.poll() is not None:
                raise RuntimeError('fixture API failed startup')
            time.sleep(.1)
    body = {'profile': 'local', 'request': {'instructions': 'trusted instructions', 'input': 'fictional input'}}
    status, result = call('/v1/ai/generate', json.dumps(body).encode())
    assert status == 200 and result['text'] == 'fixture response', (status, result)
    body['profile'] = 'not-authorized'
    assert call('/v1/ai/generate', json.dumps(body).encode())[0] == 403
    status, result = call('/v1/ai/transcribe?profile=audio&filename=voice.ogg&language=es', b'fictional-audio-bytes', 'application/octet-stream')
    assert status == 200 and result['text'] == 'transcripcion ficticia', (status, result)
    assert call('/v1/ai/transcribe?profile=audio&filename=voice.ogg&language=spanish', b'fictional-audio-bytes', 'application/octet-stream')[0] == 400
    print('PASS tenant profile routing, local chat HTTP, real multipart transcription transport, unauthorized profile and invalid language rejection')
finally:
    process.terminate()
    process.wait(timeout=10)
    provider.shutdown()
    provider.server_close()
