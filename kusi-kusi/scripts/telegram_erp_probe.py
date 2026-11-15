"""Fixed read-only ERP account probe for the explicitly paired staging chat."""
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def probe():
    credentials = json.loads((ROOT / '.local-data/qa/remote-test-users-20261108.json').read_text())['erp']
    if credentials['email'] != 'kusi-qa-20261108@example.invalid':
        raise RuntimeError('Synthetic ERP account required')
    code = 'EMAIL=' + repr(credentials['email']) + '\nPASSWORD=' + repr(credentials['password']) + '\n' + r'''
import urllib.request,urllib.error,urllib.parse,http.cookiejar,json
jar=http.cookiejar.CookieJar()
client=urllib.request.build_opener(urllib.request.ProxyHandler({}),urllib.request.HTTPCookieProcessor(jar))
def call(path,form=None):
 req=urllib.request.Request('http://127.0.0.1:8000'+path,data=urllib.parse.urlencode(form).encode() if form is not None else None,headers={'Host':'erp.coneimera2027.com'})
 try:
  with client.open(req,timeout=6) as r:return r.status,json.loads(r.read(65536))
 except urllib.error.HTTPError as e:return e.code,None
status,_=call('/api/method/login',{'usr':EMAIL,'pwd':PASSWORD})
assert status==200
try:
 status,value=call('/api/method/frappe.auth.get_logged_user')
 assert status==200 and value['message']==EMAIL
 status,_=call('/api/resource/User/Administrator')
 assert status==403
finally:
 status,_=call('/api/method/logout',{})
 assert status==200
print(json.dumps({'authenticated':True,'administrator_access':False}))
'''
    remote = 'import subprocess,sys\np=subprocess.run(["docker","exec","-i","coneimera-erp-backend-1","env/bin/python","-"],input=' + repr(code) + ',text=True,capture_output=True,timeout=22)\nif p.returncode:sys.exit(1)\nprint(p.stdout)\n'
    try:
        result = subprocess.run(['ssh','-F','/dev/null','-i','/home/blink/CMA/ssh-keys/ssh-key-2026-09-04.key','-o','BatchMode=yes','-o','ConnectTimeout=5','-o','StrictHostKeyChecking=yes','opc@132.226.244.23','python3 -'],input=remote,text=True,capture_output=True,timeout=30)
        if result.returncode:
            raise RuntimeError('ERP connection failed')
        value = json.loads(result.stdout)
        if value != {'authenticated':True,'administrator_access':False}:
            raise RuntimeError('ERP verification failed')
        return value
    except Exception:
        raise RuntimeError('ERP connection unavailable; private diagnostics withheld') from None
