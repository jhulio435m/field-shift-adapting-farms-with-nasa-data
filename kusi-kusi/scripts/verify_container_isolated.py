"""Two-image acceptance on an isolated Docker network and disposable PostgreSQL."""
import argparse
import base64
import json
import os
from pathlib import Path
import re
import secrets
import subprocess
import tempfile
import time
import uuid

from local_acceptance import free_port


def run(command):
    result=subprocess.run(command,capture_output=True,text=True,timeout=90)
    if result.returncode:
        raise RuntimeError("isolated Docker acceptance operation failed; diagnostics withheld")
    return result.stdout.strip()


def container_fetch(container,base,path="/"):
    # Internal networks intentionally have no external gateway. Exercise the
    # published web runtime from its own namespace using its bundled Node.
    script="""fetch(process.argv[1],{redirect:'manual',signal:AbortSignal.timeout(2000)}).then(async r=>console.log(JSON.stringify({status:r.status,headers:Object.fromEntries(r.headers),body:Buffer.from(await r.arrayBuffer()).toString('base64')}))).catch(()=>process.exit(2))"""
    result=json.loads(run(["docker","exec",container,"node","-e",script,base+path]))
    return result['status'],result['headers'],base64.b64decode(result['body'])


def verify(api_image,web_image,require_managed=False):
    prefix="kusi-qa008-"+uuid.uuid4().hex[:10]
    network=prefix+"-net"
    names=[prefix+"-db",prefix+"-api",prefix+"-web"]
    api_port,web_port=free_port(),free_port()
    created=False
    with tempfile.TemporaryDirectory(prefix="kusi-container-acceptance-") as directory:
        try:
            run(["docker","network","create","--internal",network]);created=True
            run(["docker","run","--rm","--detach","--name",names[0],"--network",network,"--network-alias","postgres","--tmpfs","/var/lib/postgresql/data:rw","--env","POSTGRES_HOST_AUTH_METHOD=trust","postgres:17-alpine"])
            for _ in range(100):
                ready=subprocess.run(["docker","exec",names[0],"pg_isready","-U","postgres"],capture_output=True)
                if ready.returncode==0:break
                time.sleep(.2)
            else:raise RuntimeError("disposable container database readiness timed out")
            path=Path(directory)/"runtime.env"
            path.touch(mode=0o600)
            path.write_text("DATABASE_URL=postgres://postgres@postgres/postgres\nKUSI_ADMIN_API_KEY="+secrets.token_hex(32)+"\nBIND_ADDRESS=0.0.0.0:8080\nKUSI_CREATIVE_WORKER=false\nKUSI_OAUTH_REFRESH_WORKER=false\n")
            run(["docker","run","--rm","--detach","--name",names[1],"--network",network,"--network-alias","api","--publish",f"127.0.0.1:{api_port}:8080","--env-file",str(path),api_image])
            run(["docker","run","--rm","--detach","--name",names[2],"--network",network,"--publish",f"127.0.0.1:{web_port}:3000","--env","KUSI_API_ORIGIN=http://api:8080",web_image])
            web="http://127.0.0.1:3000"
            def request(path='/'):
                return container_fetch(names[2],web,path)
            for _ in range(150):
                try:
                    code,_,body=request("/health/ready")
                    if code==200 and json.loads(body)["status"]=="ready":break
                except (OSError,RuntimeError):pass
                time.sleep(.2)
            else:raise RuntimeError("web image readiness timed out")
            code,headers,_=request();assert code==307 and headers.get('location')=='/tramites'
            code,_,body=request('/tramites');assert code==200 and b'<html lang="es"' in body
            stylesheet=re.search(rb'href="([^\"]+\.css)"',body);assert stylesheet
            code,_,css=request(stylesheet[1].decode());assert code==200 and len(css)>1000
            code,headers,body=request("/v1/auth/mode");assert code==200 and json.loads(body)=={"mode":"admin","oidc_enabled":False}
            assert headers.get("cache-control")=="no-store"
            code,headers,_=request("/v1/admin/auth/session");assert code==401 and headers.get("cache-control")=="no-store"
            code,_,body=request("/openapi.yaml");assert code==200 and b'/v1/capabilities/execute' in body
            if require_managed:
                assert b'/v1/local/managed-executions/{id}/proposal' in body and b'ManagedFixtureQueryResult' in body
                code,_,_=request('/v1/local/connector-manifests');assert code==404
            filters=run(["docker","exec",names[1],"ffmpeg","-hide_banner","-filters"])
            encoders=run(["docker","exec",names[1],"ffmpeg","-hide_banner","-encoders"])
            assert "drawtext" in filters and "libx264" in encoders
            run(["docker","exec",names[1],"test","-f","/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"])
            for name in names[1:]:
                ports=json.loads(run(["docker","inspect",name]))[0]["NetworkSettings"]["Ports"]
                assert all(binding["HostIp"]=="127.0.0.1" for bindings in ports.values() for binding in (bindings or []))
            return {"status":"passed","checks":["fresh PostgreSQL container on internal network","Rust image migrations/readiness and FFmpeg/font","Next standalone image SSR/CSS","fixed upstream proxy and private no-store response","admin session unauthorized through image proxy","OpenAPI proxy","published ports only on loopback",*(["010 managed contract present and local routes disabled in admin mode"] if require_managed else [])],"images":{"api":run(["docker","image","inspect","--format","{{.Id}}",api_image]),"web":run(["docker","image","inspect","--format","{{.Id}}",web_image])},"existing_database_used":False,"application_external_egress":"blocked by internal Docker network"}
        finally:
            for name in reversed(names):
                subprocess.run(["docker","rm","--force",name],capture_output=True,timeout=30)
            if created:run(["docker","network","rm",network])


if __name__=="__main__":
    os.umask(0o077)
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--api-image",default="kusi-kusi-api:qa008")
    parser.add_argument("--web-image",default="kusi-kusi-web:qa008")
    parser.add_argument("--require-managed",action="store_true",help="Require the 010 contract and admin/local separation")
    parser.add_argument("--result",type=Path,required=True)
    args=parser.parse_args()
    result=verify(args.api_image,args.web_image,args.require_managed)
    args.result.write_text(json.dumps(result,indent=2)+"\n")
    print(json.dumps(result,indent=2))
