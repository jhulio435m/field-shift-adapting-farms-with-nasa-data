"""End-to-end OIDC checks against a synthetic issuer bound to loopback."""
import http.cookiejar
import json
import os
import secrets
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from pathlib import Path

from oidc_test_issuer import MockOIDCIssuer

root = Path(__file__).resolve().parent.parent
os.chdir(root)
env = dict(os.environ)
if env.get('KUSI_TEST_NO_DOTENV') != 'true' and Path('.env').exists():
    env.update(dict(line.split('=', 1) for line in Path('.env').read_text().splitlines()
                    if '=' in line and not line.startswith('#')))
database_url = urllib.parse.urlsplit(env['DATABASE_URL'])
if database_url.hostname not in ('127.0.0.1', 'localhost', '::1'):
    raise SystemExit('OIDC acceptance requires a loopback development database')
pg = dict(os.environ, PGHOST=database_url.hostname or '127.0.0.1',
          PGPORT=str(database_url.port or 5432), PGDATABASE=database_url.path.lstrip('/'),
          PGUSER=urllib.parse.unquote(database_url.username or ''),
          PGPASSWORD=urllib.parse.unquote(database_url.password or ''))
pg.pop('PGOPTIONS', None)


def sql(query):
    return subprocess.check_output(['psql', '-X', '-At', '-v', 'ON_ERROR_STOP=1', '-c', query],
                                   env=pg, text=True, stderr=subprocess.PIPE).strip()


schema = 'admin_oidc_' + uuid.uuid4().hex
sql('CREATE SCHEMA ' + schema)
pg['PGOPTIONS'] = '-csearch_path=' + schema
query = dict(urllib.parse.parse_qsl(database_url.query))
query['options'] = '-csearch_path=' + schema
env['DATABASE_URL'] = urllib.parse.urlunsplit(database_url._replace(query=urllib.parse.urlencode(query)))
env.update(KUSI_LOCAL_MODE='false', KUSI_ADMIN_API_KEY=secrets.token_hex(32),
           KUSI_CREATIVE_WORKER='false', KUSI_CREATIVE_SIMULATION='false',
           KUSI_OAUTH_REFRESH_WORKER='false')
for name in ['KUSI_CHANNELS', 'KUSI_LLM_PROFILES', 'KUSI_AUDIO_PROFILES',
             'KUSI_CREATIVE_RECIPES', 'KUSI_HTTP_DESTINATIONS', 'KUSI_OAUTH_PROFILES', 'KUSI_SYSTEM_PROFILES']:
    env[name] = '{}'


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, _req, _fp, _code, _msg, _headers, _url):
        return None


class Client:
    def __init__(self, base):
        self.base = base
        self.jar = http.cookiejar.CookieJar()
        self.opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(self.jar), NoRedirect)

    def request(self, url, method='GET', headers=None, data=None):
        request = urllib.request.Request(url, data=data, method=method, headers=headers or {})
        try:
            response = self.opener.open(request, timeout=10)
            return response.status, response.headers, response.read()
        except urllib.error.HTTPError as error:
            return error.code, error.headers, error.read()

    def flow(self, expected_success):
        status, headers, _ = self.request(self.base + '/v1/admin/auth/oidc/start')
        assert status == 307, ('start', status)
        status, auth_headers, _ = self.request(headers['Location'])
        assert status == 302, ('authorize', status)
        callback = auth_headers['Location']
        if not expected_success and getattr(self, 'alter_next_state', False):
            parsed = urllib.parse.urlsplit(callback)
            pairs = urllib.parse.parse_qsl(parsed.query)
            pairs = [(key, value + '-altered' if key == 'state' else value) for key, value in pairs]
            callback = urllib.parse.urlunsplit(parsed._replace(query=urllib.parse.urlencode(pairs)))
            self.alter_next_state = False
        status, callback_headers, _ = self.request(callback)
        assert status == 307, ('callback should normalize to redirect', status)
        location = callback_headers.get('Location', '')
        assert location == '/admin' if expected_success else location == '/admin?error=login_failed', location
        return callback


with tempfile.TemporaryDirectory(prefix='kusi-oidc-check-') as tmp, MockOIDCIssuer() as issuer:
    port_socket = socket.socket()
    port_socket.bind(('127.0.0.1', 0))
    api_port = port_socket.getsockname()[1]
    port_socket.close()
    base = 'http://127.0.0.1:' + str(api_port)
    env.update(BIND_ADDRESS='127.0.0.1:' + str(api_port),
               KUSI_LOCAL_STORAGE=str(Path(tmp) / 'local-storage'),
               KUSI_LOCAL_ORIGIN=base, KUSI_ADMIN_PUBLIC_ORIGIN=base,
               KUSI_ADMIN_OIDC_ISSUER=issuer.url,
               KUSI_ADMIN_OIDC_CLIENT_ID='test-client',
               KUSI_ADMIN_OIDC_CLIENT_SECRET='test-client-secret',
               KUSI_ADMIN_OIDC_REDIRECT_URI=base + '/v1/admin/auth/oidc/callback',
               KUSI_ADMIN_OIDC_STATE_KEY=secrets.token_hex(32))
    log = open(Path(tmp) / 'server.log', 'w+')
    process = subprocess.Popen([str(root/'target/debug/kusi-server')], env=env, cwd=tmp, stdout=log, stderr=log)
    try:
        for _ in range(150):
            try:
                status, _, _ = Client(base).request(base + '/health/ready')
                if status == 200:
                    break
            except (urllib.error.URLError, TimeoutError):
                pass
            if process.poll() is not None:
                raise AssertionError('OIDC test server exited during startup')
            time.sleep(.2)
        else:
            raise AssertionError('OIDC test server did not become ready')

        admin = Client(base)
        mode_status, _, mode_body = admin.request(base + '/v1/auth/mode')
        assert mode_status == 200 and json.loads(mode_body) == {'mode': 'admin', 'oidc_enabled': True}
        status, _, body = admin.request(base + '/v1/organizations', 'POST',
            {'x-admin-key': env['KUSI_ADMIN_API_KEY'], 'Content-Type': 'application/json'},
            json.dumps({'name': 'OIDC fixture', 'slug': 'oidc-fixture'}).encode())
        assert status == 201, ('organization', status, body[:200])
        provision_env = dict(env, KUSI_ADMIN_PROVISION_ISSUER=issuer.url,
                             KUSI_ADMIN_PROVISION_SUBJECT='admin-subject',
                             KUSI_ADMIN_PROVISION_ORGANIZATION='oidc-fixture',
                             KUSI_ADMIN_PROVISION_ROLE='approver')
        result = subprocess.run([str(root/'target/debug/provision_admin_identity')], env=provision_env, cwd=tmp,
                                check=False, capture_output=True, text=True)
        assert result.returncode == 0, 'synthetic identity provisioning failed'

        browser = Client(base)
        callback = browser.flow(True)
        assert int(sql('SELECT count(*) FROM admin_oidc_attempts')) == 0, \
            'consumed transaction secrets remained in storage'
        status, _, body = browser.request(base + '/v1/admin/auth/session')
        session = json.loads(body)
        assert status == 200 and len(session['memberships']) == 1
        assert session['memberships'][0]['role'] == 'approver'
        assert session['memberships'][0]['organization'] == 'oidc-fixture'
        count = int(sql("SELECT count(*) FROM admin_sessions WHERE revoked_at IS NULL"))
        status, _, _ = browser.request(callback)
        assert status == 307
        assert int(sql("SELECT count(*) FROM admin_sessions WHERE revoked_at IS NULL")) == count

        for variant in ['unknown_subject', 'bad_signature', 'bad_issuer', 'bad_audience',
                        'expired_token', 'bad_nonce', 'bad_pkce', 'altered_state']:
            if variant == 'unknown_subject':
                issuer.next_subject = 'unprovisioned-subject'
            elif variant == 'bad_signature':
                issuer.bad_signature = True
            elif variant == 'bad_nonce':
                issuer.bad_nonce = True
            elif variant == 'bad_issuer':
                issuer.bad_issuer = True
            elif variant == 'bad_audience':
                issuer.bad_audience = True
            elif variant == 'expired_token':
                issuer.expired_token = True
            elif variant == 'bad_pkce':
                issuer.reject_pkce = True
            else:
                browser.alter_next_state = True
            browser.flow(False)
            assert int(sql("SELECT count(*) FROM admin_sessions WHERE revoked_at IS NULL")) == count, variant
        status, _, _ = browser.request(base + '/v1/admin/auth/session')
        assert status == 200, 'failed login changed the existing session'

        status, _, _ = browser.request(base + '/v1/admin/auth/logout', 'POST',
            {'Origin': 'http://evil.invalid', 'x-csrf-token': session['csrf_token']}, b'')
        assert status == 403, 'logout accepted a foreign Origin'
        status, _, _ = browser.request(base + '/v1/admin/auth/logout', 'POST',
            {'Origin': base, 'x-csrf-token': 'invalid'}, b'')
        assert status == 403, 'logout accepted an invalid CSRF token'
        status, _, _ = browser.request(base + '/v1/admin/auth/logout', 'POST',
            {'Origin': base, 'x-csrf-token': session['csrf_token']}, b'')
        assert status == 204
        status, _, _ = browser.request(base + '/v1/admin/auth/session')
        assert status == 401
        browser.flow(True)
        status, _, _ = browser.request(base + '/v1/admin/auth/session')
        assert status == 200
        sql("UPDATE admin_memberships SET disabled_at=now() WHERE subject='admin-subject'")
        status, _, _ = browser.request(base + '/v1/admin/auth/session')
        assert status == 401, 'disabled membership retained session access'
        sql("UPDATE admin_memberships SET disabled_at=NULL WHERE subject='admin-subject'")
        browser.flow(True)
        sql("UPDATE admin_sessions SET expires_at=now()-interval '1 second' WHERE revoked_at IS NULL")
        status, _, _ = browser.request(base + '/v1/admin/auth/session')
        assert status == 401, 'expired session remained valid'
        print(json.dumps({'status': 'passed', 'checks': [
            'loopback issuer discovery and RS256 signed authorization-code flow',
            'provisioned organization and role come from Kusi Kusi, not the identity token',
            'one-use transaction replay rejected without another session',
            'unknown subject and invalid signature, issuer, audience, expiry, nonce, PKCE and state rejected',
            'Origin/CSRF logout revokes the opaque admin session',
            'disabled membership and expired session lose access on the next request',
        ]}, ensure_ascii=False, indent=2))
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=15)
        log.close()
        sql('DROP SCHEMA ' + schema + ' CASCADE')
