"""Existing Next acceptance against disposable local API/users/PostgreSQL."""
import argparse
from contextlib import ExitStack
import json
import os
from pathlib import Path
import secrets
import subprocess

from local_acceptance import ROOT, free_port, postgres
from verify_external_access import call, server
from verify_oidc_browser import process, ready


def verify():
    with postgres() as (env, temporary):
        web_port = free_port()
        web = f"http://127.0.0.1:{web_port}"
        env["KUSI_ADMIN_API_KEY"] = secrets.token_hex(32)
        with server(env, temporary, local=True, origin=web) as api, ExitStack() as stack:
            admin = {"x-admin-key":env["KUSI_ADMIN_API_KEY"]}
            created = call(api,"/v1/organizations",{"name":"Synthetic Next fixture","slug":"next-fixture"},admin,201)
            headers = {"Authorization":"Bearer " + created["api_key"],**admin}
            password = secrets.token_urlsafe(24)
            call(api,"/v1/local/bootstrap",{"username":"synthetic-owner","display_name":"Synthetic owner","password":password},headers)
            credentials = temporary / "login.json"
            credentials.write_text(json.dumps({"organization":"next-fixture","username":"synthetic-owner","password":password}))
            credentials.chmod(0o600)
            env.update(KUSI_API_ORIGIN=api,NEXT_TELEMETRY_DISABLED="1",KUSI_TEST_BASE=web,KUSI_TEST_LOGIN_FILE=str(credentials))
            child=process(stack,[str(ROOT/"web/admin/node_modules/.bin/next"),"start","--hostname","127.0.0.1","--port",str(web_port)],env,ROOT/"web/admin",temporary/"next.log")
            ready(child,web,"/health/ready")
            subprocess.run(["python3",str(ROOT/"scripts/verify_next.py")],env=env,cwd=temporary,check=True,timeout=90)
            assert call(web,"/v1/auth/mode") == {"mode":"local","oidc_enabled":False}
            call(web,"/v1/admin/auth/oidc/start",expected=404)
            return {"status":"passed","checks":["existing verify_next.py cookie/Origin/CSRF/session/proxy checks","OIDC disabled in local mode","fixed upstream with disposable API and users"],"existing_database_used":False,"external_calls":False}


if __name__ == "__main__":
    os.umask(0o077)
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--result",type=Path,required=True)
    args=parser.parse_args()
    result=verify()
    args.result.write_text(json.dumps(result,indent=2)+"\n")
    print(json.dumps(result,indent=2))
