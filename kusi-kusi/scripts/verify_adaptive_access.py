"""Real compiled adaptive assistant + PostgreSQL ACL + registered synthetic channel.

The planner is a deterministic loopback HTTP fixture, not a paid LLM. Sends are
captured in memory. Uses no existing database, .env or staging configuration.
"""
import argparse
import base64
from contextlib import closing
import hashlib
import hmac
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os
from pathlib import Path
import sqlite3
import threading
from unittest.mock import patch

from local_acceptance import ROOT, postgres, sql
from verify_external_access import call, provision, server
import telegram_adaptive as channel


def write(path, value):
    with path.open("w") as file:
        path.chmod(0o600)
        json.dump(value, file)


def verify():
    with postgres() as (env, temporary), server(dict(env, KUSI_ADMIN_API_KEY="a" * 64), temporary, local=False) as api:
        created = call(api, "/v1/organizations", {"name":"Synthetic adaptive ACL", "slug":"synthetic-adaptive"}, {"x-admin-key":"a" * 64}, 201)
        organization, key = created["organization"]["id"], created["api_key"]
        headers = {"Authorization":"Bearer " + key, "x-admin-key":"a" * 64}
        call(api, "/v1/capabilities", {"identifier":"record.read", "version":"1", "description":"Synthetic whole dataset", "adapter":"system-a", "required_permission":"record.read", "enabled":True, "timeout_ms":1000, "input_schema":{}, "output_schema":{}}, headers, 204)
        call(api, "/v1/capabilities/grants", {"permission":"record.read"}, headers, 204)
        requests = []
        class Planner(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass
            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                prompt = json.loads(body["messages"][-1]["content"])
                requests.append(prompt["catalog"])
                assert [r["id"] for r in prompt["catalog"]["resources"]] == ["alpha::items"]
                if prompt["request"] == "subscribe":
                    decision = {"type":"subscribe", "resource":"alpha::items", "event":"created", "filters":[]}
                else:
                    decision = {"type":"query", "plan":{"version":prompt["catalog"]["version"], "queries":[{"resource":"alpha::items", "filters":[], "analyses":[{"op":"count"}]}]}}
                payload = json.dumps({"choices":[{"finish_reason":"stop", "message":{"content":json.dumps(decision)}}]}).encode()
                self.send_response(200);self.send_header("Content-Type","application/json");self.send_header("Content-Length",str(len(payload)));self.end_headers();self.wfile.write(payload)
        model = HTTPServer(("127.0.0.1",0),Planner)
        thread = threading.Thread(target=model.serve_forever,daemon=True);thread.start()
        try:
            ledger = temporary / ".local-data/telegram-staging";ledger.mkdir(parents=True)
            with closing(sqlite3.connect(ledger / "ledger.sqlite")) as db, db:
                db.execute("CREATE TABLE binding(id INTEGER PRIMARY KEY,chat TEXT)")
                db.execute("INSERT INTO binding VALUES(1,'synthetic-chat')")
            binary = temporary / "target/debug/adaptive_assistant";binary.parent.mkdir(parents=True);binary.symlink_to(ROOT / "target/debug/adaptive_assistant")
            fields = [{"id":"amount", "label":"Synthetic number", "kind":"number", "options":[]}]
            manifest = {"catalog":{"source":"fixture", "version":"ignored", "resources":[{"id":id,"label":id,"fields":fields,"events":["created"]} for id in ["items","hidden"]]},"rows":{"items":[{"amount":1},{"amount":2},{"amount":3}],"hidden":[{"amount":999}]},"enabled":True}
            source = temporary / "manifest.json";write(source,manifest)
            secret = temporary / "event-secret";secret.write_text("synthetic-event-secret");secret.chmod(0o600)
            config = {"organization":organization,"principal":hashlib.sha256(key.encode()).hexdigest(),"read":True,"subscribe":True,
                "sources":[{"id":"alpha","label":"Synthetic source","kind":"manifest","config_file":str(source),"event_secret_file":str(secret)}],
                "state_directory":str(temporary / "state"),"bridge_script":str(ROOT / "scripts/adaptive_frappe_bridge.py"),
                "binding_hash":hashlib.sha256(b"synthetic-chat").hexdigest(),
                "llm":{"provider":"openai_compatible","model":"synthetic","api_key_env":None,"endpoint":f"http://127.0.0.1:{model.server_port}/v1/chat/completions","max_output_tokens":1000,"timeout_ms":5000,"tools_supported":False,"anthropic_version":None,"routing_providers":None},
                "access":{"database_url_env":"DATABASE_URL","resources":{"alpha::items":{"adapter":"system-a","capability":"record.read","version":"1","resource_ref":"dataset-a"}}}}
            path = temporary / "runtime.json";write(path,config)
            with patch.object(channel,"ROOT",temporary),patch.object(channel,"CONFIG",path),patch.dict(os.environ,env,clear=True):
                # A missing policy is denied in the protected assistant, even
                # though unprotected service capabilities retain their behavior.
                assert channel.invoke({"message":"count","request_id":"missing"})["status"] == "blocked"
                assert not requests
                provision(env,temporary,organization,key,{"operation":"protect"})
                provision(env,temporary,organization,key,{"operation":"link","subject":"synthetic-person","ttl_seconds":600,"confirmed":True})
                assert channel.invoke({"message":"count","request_id":"no-grant"})["status"] == "blocked"
                assert not requests
                provision(env,temporary,organization,key,{"operation":"grant","capability":"record.read","version":"1","resource_ref":"dataset-a","ttl_seconds":300,"confirmed":True})
                reply = channel.invoke({"message":"count","request_id":"query"})
                assert reply["status"] == "completed" and reply["evidence"][0]["results"][0]["value"] == 3
                assert len(requests) == 1
                assert channel.invoke({"message":"subscribe","request_id":"subscribe"})["status"] == "completed"
                sent = []
                def event(id):
                    raw = json.dumps({"test":True,"resource":"alpha::items","event_id":id,"kind":"created","values":{"amount":4}}).encode()
                    signature = base64.b64encode(hmac.new(b"synthetic-event-secret",raw,hashlib.sha256).digest()).decode()
                    return channel.event(raw,signature,"alpha",lambda chat,text:(sent.append(chat) or {"message_id":1}))
                assert event("event-a") == 200 and len(sent) == 1
                assert event("event-a") == 200 and len(sent) == 1
                count = len(requests)
                with patch.dict(os.environ,{"KUSI_LOCAL_MODE":"true"}):
                    assert channel.invoke({"message":"count"})["status"] == "blocked"
                assert len(requests) == count
                assert channel.invoke({"message":"count","organization":organization})["status"] == "clarification"
                no_access = {k:v for k,v in config.items() if k!="access"};write(path,no_access)
                with patch.dict(os.environ,{"KUSI_ADAPTIVE_REQUIRE_ACL":"true"}):
                    assert channel.invoke({"message":"count"})["status"] == "blocked"
                write(path,config)
                sql(env,"UPDATE external_resource_grants SET created_at=now()-interval '2 hours',expires_at=now()-interval '1 hour'")
                assert channel.invoke({"message":"count","request_id":"expired"})["status"] == "blocked"
                sql(env,"UPDATE external_resource_grants SET expires_at=now()+interval '1 minute'")
                provision(env,temporary,organization,key,{"operation":"revoke_grant","capability":"record.read","version":"1","resource_ref":"dataset-a"})
                reply = channel.invoke({"message":"count","request_id":"query"})
                assert reply["status"] == "blocked" and not reply["evidence"]
                assert event("event-b") == 403 and len(sent) == 1
                assert len(requests) == count
                with closing(sqlite3.connect(ledger / "ledger.sqlite")) as db, db:
                    db.execute("UPDATE binding SET chat='different-chat'")
                assert channel.invoke({"message":"count"})["status"] == "blocked"
                assert len(requests) == count
            return {"status":"passed","checks":["compiled Rust assistant with real PostgreSQL ACL","registered channel binding and private configuration","missing policy/grant denied before loopback planner","catalog hides ungranted resource","typed aggregate over granted whole dataset","subscription and signed event through compiled application","duplicate event produces no second simulated send","expiry/revocation block receipt replay and later notifications","local mode and required ACL omission blocked","caller tenant rejected and changed channel binding denied"],"planner":"deterministic loopback fixture, no paid LLM","simulated_sends":len(sent),"external_calls":False,"existing_database_used":False}
        finally:
            model.shutdown();model.server_close();thread.join(timeout=2)


if __name__ == "__main__":
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--result",type=Path,required=True)
    args = parser.parse_args()
    result = verify()
    args.result.write_text(json.dumps(result,indent=2)+"\n")
    print(json.dumps(result,indent=2))
