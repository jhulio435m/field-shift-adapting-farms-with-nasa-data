"""Private, disposable API acceptance of managed connection settings; no provider calls."""
import argparse
import http.cookiejar
import json
import os
from pathlib import Path
import secrets
import urllib.error
import urllib.request

from local_acceptance import postgres, sql
from verify_external_access import server, call


class Client:
    def __init__(self, base):
        self.base=base;self.csrf=""
        self.opener=urllib.request.build_opener(urllib.request.ProxyHandler({}),urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
    def call(self,path,method="GET",body=None,expected=200,headers=None):
        h={"Origin":self.base,"x-csrf-token":self.csrf,**(headers or {})}
        if body is not None:h["Content-Type"]="application/json"
        request=urllib.request.Request(self.base+path,method=method,headers=h,data=json.dumps(body).encode() if body is not None else None)
        try:
            with self.opener.open(request,timeout=10) as response:status,data=response.status,response.read()
        except urllib.error.HTTPError as error:status,data=error.code,error.read()
        assert status==expected,f"unexpected status for {method} {path}: {status} != {expected}"
        return json.loads(data) if data else None
    def login(self,organization,username,password):
        result=self.call("/local/auth/login","POST",{"organization":organization,"username":username,"password":password})
        self.csrf=result["csrf_token"]


def bootstrap(env,base,slug,username):
    admin={"x-admin-key":env["KUSI_ADMIN_API_KEY"]}
    org=call(base,"/v1/organizations",{"name":"Synthetic "+slug,"slug":slug},admin,201)
    password=secrets.token_urlsafe(24)
    call(base,"/v1/local/bootstrap",{"username":username,"display_name":"Synthetic owner","password":password},{**admin,"Authorization":"Bearer "+org["api_key"]})
    return password


def verify():
    path="/v1/local/connection-settings"
    secret_values=["synthetic-plane-token-009","synthetic-erp-key-009","synthetic-erp-secret-009","123456:synthetic_telegram_token_009"]
    with postgres() as (env,temporary):
        env["KUSI_ADMIN_API_KEY"]=secrets.token_hex(32)
        with server(env,temporary,True) as base:
            password=bootstrap(env,base,"settings-fixture","fixture-owner")
            other_password=bootstrap(env,base,"other-fixture","other-owner")
            client=Client(base);client.call(path,expected=401);client.login("settings-fixture","fixture-owner",password)
            other=Client(base);other.login("other-fixture","other-owner",other_password)
            assert client.call(path)==[]
            inputs={
                "plane":{"label":"Plane","base_url":"https://plane.example.invalid","workspace":"synthetic-space","credentials":{"api_key":secret_values[0]}},
                "frappe":{"label":"ERP CONEIMERA","base_url":"https://erp.example.invalid","credentials":{"api_key":secret_values[1],"api_secret":secret_values[2]}},
                "telegram":{"label":"Telegram","bot_username":"synthetic_bot","webhook_url":"https://hooks.example.invalid/telegram","credentials":{"bot_token":secret_values[3]}},
            }
            for kind,body in inputs.items():
                client.call(path+"/"+kind,"PUT",body,403,{"x-csrf-token":""})
                client.call(path+"/"+kind,"PUT",body,403,{"Origin":"https://foreign.example.invalid"})
                bad={**body,"organization_id":"00000000-0000-4000-8000-000000000001"};client.call(path+"/"+kind,"PUT",bad,422)
                result=client.call(path+"/"+kind,"PUT",body)
                assert result["configured"] and not result["runtime_active"] and result["verification"]=="not_verified"
                assert result["revision"]==1
                assert all(value not in json.dumps(result) for value in secret_values)
            profiles=client.call(path);assert len(profiles)==3 and other.call(path)==[]
            assert all(value not in json.dumps(profiles) for value in secret_values)
            ciphertext=sql(env,"SELECT credential_ciphertext FROM managed_connection_settings WHERE kind='plane'")
            assert ciphertext and all(value not in ciphertext for value in secret_values)
            patch={**inputs["plane"],"expected_revision":1,"credentials":{},"label":"Plane editado"}
            result=client.call(path+"/plane","PUT",patch);assert result["revision"]==2 and result["configured"]
            client.call(path+"/plane","PUT",patch,409)
            rotated={**patch,"expected_revision":2,"credentials":{"api_key":"synthetic-plane-rotated-009"}}
            result=client.call(path+"/plane","PUT",rotated);assert result["revision"]==3
            current=sql(env,"SELECT credential_ciphertext FROM managed_connection_settings WHERE kind='plane'")
            assert current!=ciphertext and "synthetic-plane-rotated-009" not in current
            assert all(char.isalnum() or char==":" for char in current)
            sql(env,"UPDATE managed_connection_settings SET credential_ciphertext=credential_ciphertext||'00' WHERE kind='plane'")
            client.call(path+"/plane","PUT",{**patch,"expected_revision":3},503)
            assert sql(env,"SELECT revision FROM managed_connection_settings WHERE kind='plane'")=="3"
            sql(env,"UPDATE managed_connection_settings SET credential_ciphertext='"+current+"' WHERE kind='plane'")
            for role in ['viewer','editor','approver']:
                viewer_password=secrets.token_urlsafe(24)
                client.call("/v1/local/users","POST",{"username":"fixture-"+role,"display_name":"Synthetic "+role,"password":viewer_password,"role":role})
                viewer=Client(base);viewer.login("settings-fixture","fixture-"+role,viewer_password)
                viewer.call(path,expected=403);viewer.call(path+"/plane","PUT",patch,403)
                viewer.call(path+"/plane/credentials","DELETE",{"expected_revision":3},403)
            assert client.call(path+'/plane','PUT',{**patch,'expected_revision':'synthetic-secret-in-invalid-field'},422) is None
            client.call(path+"/frappe/credentials","DELETE",{"expected_revision":999},409)
            cleared=client.call(path+"/frappe/credentials","DELETE",{"expected_revision":1})
            assert cleared["revision"]==2 and not cleared["configured"] and cleared["credential_fields"]==[]
            assert sql(env,"SELECT credential_ciphertext IS NULL FROM managed_connection_settings WHERE kind='frappe'")=="t"
            client.call(path+"/telegram/activate","POST",{},404)
            client.call(path+"/telegram","PUT",{**inputs["telegram"],"expected_revision":1,"enabled":True},422)
            audit=client.call("/v1/local/activity")
            assert all(value not in json.dumps(audit) for value in secret_values+["synthetic-plane-rotated-009"])
            other.call(path+'/plane','PUT',inputs['plane'])
            assert len(other.call(path))==1 and len(client.call(path))==3
            assert sql(env,"SELECT count(DISTINCT credential_ciphertext) FROM managed_connection_settings WHERE kind='plane'")=="2"
            key=temporary/"storage/connection-vault.key"
            assert key.stat().st_mode & 0o077 == 0
            key_bytes=key.read_bytes()
        # Same private key and data must survive a real compiled-server restart.
        with server(env,temporary,True) as base:
            client=Client(base);client.login("settings-fixture","fixture-owner",password)
            profiles=client.call(path);assert len(profiles)==3 and key.read_bytes()==key_bytes
            plane=next(profile for profile in profiles if profile["kind"]=="plane")
            result=client.call(path+"/plane","PUT",{**patch,"expected_revision":plane["revision"]})
            assert result["configured"] and result["revision"]==4
        backup=key.with_suffix(".saved");key.rename(backup)
        try:
            failed=False
            try:
                with server(env,temporary,True):pass
            except RuntimeError:failed=True
            assert failed and not key.exists(),"missing master key was silently replaced"
        finally:backup.rename(key)
        key.chmod(0o644)
        try:
            failed=False
            try:
                with server(env,temporary,True):pass
            except RuntimeError:failed=True
            assert failed,"public master key permissions were accepted"
        finally:key.chmod(0o600)
    return {"status":"passed","checks":["three tenant-scoped profiles via authenticated API","CSRF/Origin/session/owner RBAC and caller tenant rejection","write-only encrypted credentials and payload-free audit","blank edit preserves existing secrets","optimistic revision conflicts and explicit credential rotation","tampered ciphertext fails closed without revision change","explicit clear deletes encrypted credentials","real restart preserves key and encrypted state","missing or publicly readable master key blocks startup","saving does not activate Telegram or provider calls"],"external_calls":False,"existing_database_used":False}


if __name__=="__main__":
    os.umask(0o077)
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--result",type=Path,required=True)
    args=parser.parse_args()
    result=verify();args.result.write_text(json.dumps(result,indent=2)+"\n");print(json.dumps(result,indent=2))
