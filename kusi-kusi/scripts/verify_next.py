"""Next runtime/proxy checks against the local workspace; no external providers."""
import http.cookiejar,json,os,urllib.request,urllib.error,urllib.parse
from pathlib import Path
c=json.loads(Path(os.environ.get('KUSI_TEST_LOGIN_FILE',str(Path(__file__).resolve().parent.parent/'.local-data/login.json'))).read_text())
base=os.environ.get('KUSI_TEST_BASE','http://127.0.0.1:3000')
if urllib.parse.urlsplit(base).hostname not in ('127.0.0.1','localhost','::1'):raise SystemExit('loopback acceptance required')
jar=http.cookiejar.CookieJar();client=urllib.request.build_opener(urllib.request.ProxyHandler({}),urllib.request.HTTPCookieProcessor(jar))
def call(path,method='GET',body=None,headers=None,expected=200):
    h={'Origin':base};h.update(headers or {})
    if body is not None:h['Content-Type']='application/json'
    request=urllib.request.Request(base+path,data=json.dumps(body).encode() if body is not None else None,headers=h,method=method)
    try:
        with client.open(request,timeout=10) as r:status=r.status;data=r.read();response_headers=r.headers
    except urllib.error.HTTPError as e:status=e.code;data=e.read();response_headers=e.headers
    assert status==expected,(path,status,expected)
    return data,response_headers
call('/v1/local/overview',expected=401)
call('/local/auth/login','POST',{k:c[k] for k in ('organization','username','password')},{'Origin':'https://untrusted.invalid'},403)
data,h=call('/local/auth/login','POST',{k:c[k] for k in ('organization','username','password')})
session=json.loads(data);assert 'password' not in data.decode() and 'api_key' not in data.decode()
assert 'HttpOnly' in h['Set-Cookie'] and 'SameSite=Strict' in h['Set-Cookie']
assert h.get('Cache-Control')=='no-store'
call('/v1/local/conversations','POST',{'title':'must not create'},expected=403)
call('/v1/local/conversations','POST',{'title':'must not create'},{'x-csrf-token':session['csrf_token'],'Origin':'https://untrusted.invalid'},403)
call('/v1/local/overview')
for path in ['/conversaciones','/archivos','/trabajos','/publicaciones','/usuarios','/sesiones','/conexiones','/integraciones','/operacion','/organizaciones','/trabajos?selected=00000000-0000-0000-0000-000000000001']:
    html,headers=call(path);assert b'<html lang="es"' in html
    assert headers.get('x-powered-by') is None
call('/seccion-inexistente',expected=404)
contract,headers=call('/openapi.yaml');assert b'/v1/local/publications' in contract and b'/v1/local/sessions' in contract
call('/local/auth/logout','POST',headers={'x-csrf-token':session['csrf_token']},expected=204)
call('/v1/local/overview',expected=401)
print(json.dumps({'status':'passed','checks':['HttpOnly/SameSite cookie forwarded','no-store API responses','same-origin session and CSRF','foreign origin rejected','all direct App Router URLs and deep links','unknown route 404','no x-powered-by','OpenAPI proxy','logout revocation']},indent=2))
