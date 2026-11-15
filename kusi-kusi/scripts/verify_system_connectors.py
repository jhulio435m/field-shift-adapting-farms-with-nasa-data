"""Axum + PostgreSQL acceptance; fixture mode is loopback-only, live reads are explicit.

Creates its own disposable PostgreSQL container. Never loads .env or an existing DB.
Needs Docker; --runtime-image runs the compiled Rust API in that image with host networking.
"""
import argparse
import hashlib
import http.server
import json
import os
import re
from pathlib import Path
import secrets
import socket
import subprocess
import tempfile
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid


def private_json(path, value):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "w") as stream:
        json.dump(value, stream)


def port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


class Fixture(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def reply(self, value, status=200, cookies=()):
        payload = json.dumps(value).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        for cookie in cookies:
            self.send_header("Set-Cookie", cookie)
        self.end_headers()
        self.wfile.write(payload)

    def do_GET(self):
        self.server.calls += 1
        path = self.path
        state = self.server
        if state.kind == "wppconnect_health":
            return self.reply({"message": "OK"})
        if path == "/auth/get-csrf-token/":
            return self.reply({"csrf_token": state.csrf}, cookies=("csrftoken=" + state.csrf + "; Path=/",))
        if not any("=" + sid in self.headers.get("Cookie", "") for sid in state.sessions):
            return self.reply({}, 401)
        if path == "/api/method/frappe.auth.get_logged_user":
            return self.reply({"message": state.username})
        if path == "/api/users/me/":
            return self.reply({"email": state.username})
        if path == "/api/users/me/workspaces/":
            return self.reply([])
        self.reply({}, 404)

    def do_POST(self):
        state = self.server
        state.calls += 1
        body = self.rfile.read(int(self.headers.get("Content-Length", "0")))
        form = urllib.parse.parse_qs(body.decode())
        plane = state.kind == "plane_identity"
        if plane and self.headers.get("X-CSRFToken") != state.csrf:
            return self.reply({}, 403)
        if self.path in ("/api/method/login", "/auth/sign-in/"):
            username = form.get("email" if plane else "usr", [None])[0]
            password = form.get("password" if plane else "pwd", [None])[0]
            if username != state.username or password != state.password:
                return self.reply({}, 401)
            sid = secrets.token_hex(12)
            state.sessions.add(sid)
            return self.reply({}, 302 if plane else 200, (f"{'plane-session' if plane else 'sid'}={sid}; Path=/; HttpOnly",))
        if self.path in ("/api/method/logout", "/auth/sign-out/"):
            for sid in list(state.sessions):
                if "=" + sid in self.headers.get("Cookie", ""):
                    state.sessions.remove(sid)
            return self.reply({}, 302 if plane else 200)
        self.reply({}, 404)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/debug/kusi-server")
    parser.add_argument("--runtime-image")
    parser.add_argument("--rust-workspace", type=Path)
    parser.add_argument("--postgres-image", default="postgres:15.7-alpine")
    parser.add_argument("--live-credentials", type=Path)
    parser.add_argument("--allow-live-reads", action="store_true")
    parser.add_argument("--result", type=Path)
    args = parser.parse_args()
    if bool(args.live_credentials) != args.allow_live_reads:
        parser.error("live reads require both --live-credentials and --allow-live-reads")
    if args.rust_workspace and not args.runtime_image:
        parser.error("--rust-workspace requires --runtime-image")
    binary = Path(args.binary).resolve(strict=True)
    run_id = uuid.uuid4().hex[:12]
    db_name = "kusi-system-db-" + run_id
    api_name = "kusi-system-api-" + run_id
    db_port, api_port = port(), port()
    api_base = "http://127.0.0.1:" + str(api_port)
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    fixtures = []
    process = None
    checks = []
    rust_tests = None
    admin_key = secrets.token_hex(32)
    with tempfile.TemporaryDirectory(prefix="kusi-system-qa-") as directory:
        directory = Path(directory)
        db_password = secrets.token_urlsafe(30)
        pg_env = directory / "postgres.env"
        pg_env.write_text("POSTGRES_USER=kusi_qa\nPOSTGRES_DB=kusi_qa\nPOSTGRES_PASSWORD=" + db_password + "\n")
        pg_env.chmod(0o600)
        api_env = directory / "api.env"
        logs = open(directory / "api.log", "w+")
        profile_config = {}
        hosts = ["fixture.test"]

        def sql(statement):
            result = subprocess.run(["docker", "exec", db_name, "psql", "-X", "-At", "-p", str(db_port), "-v", "ON_ERROR_STOP=1", "-U", "kusi_qa", "-d", "kusi_qa", "-c", statement], capture_output=True, text=True)
            if result.returncode:
                raise RuntimeError("isolated test database command failed")
            return result.stdout.strip()

        def call(path, method="GET", body=None, key=None, admin=False, expected=200):
            headers = {}
            if key:
                headers["Authorization"] = "Bearer " + key
            if admin:
                headers["x-admin-key"] = admin_key
            if body is not None:
                headers["Content-Type"] = "application/json"
            request = urllib.request.Request(api_base + path, data=json.dumps(body).encode() if body is not None else None, headers=headers, method=method)
            try:
                with opener.open(request, timeout=35) as response:
                    status, data = response.status, response.read()
            except urllib.error.HTTPError as error:
                status, data = error.code, error.read()
            assert status == expected, (method, path, status, expected)
            if status >= 400:
                return None  # Axum's extractor rejections are plain text, not API JSON.
            return json.loads(data) if data else None

        def stop_api():
            nonlocal process
            if process is None:
                return
            if args.runtime_image:
                subprocess.run(["docker", "rm", "-f", api_name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            else:
                process.terminate()
            try:
                process.wait(timeout=15)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
            process = None

        def start_api():
            nonlocal process
            environment = {
                "DATABASE_URL": f"postgres://kusi_qa:{db_password}@127.0.0.1:{db_port}/kusi_qa",
                "KUSI_ADMIN_API_KEY": admin_key,
                "BIND_ADDRESS": f"127.0.0.1:{api_port}",
                "KUSI_LOCAL_MODE": "false",
                "KUSI_SYSTEM_PROFILES": json.dumps(profile_config, separators=(",", ":")),
                "KUSI_SYSTEM_ALLOWED_HOSTS": ",".join(hosts),
                "KUSI_SYSTEM_PRIVATE_HTTP": "true",
                "KUSI_CREATIVE_SIMULATION": "false",
                "KUSI_CREATIVE_WORKER": "false",
                "KUSI_OAUTH_REFRESH_WORKER": "false",
                "RUST_LOG": "warn",
            }
            for name in ["KUSI_HTTP_DESTINATIONS", "KUSI_CHANNELS", "KUSI_LLM_PROFILES", "KUSI_AUDIO_PROFILES", "KUSI_CREATIVE_RECIPES", "KUSI_OAUTH_PROFILES"]:
                environment[name] = "{}"
            api_env.write_text("".join(key + "=" + value + "\n" for key, value in environment.items()))
            api_env.chmod(0o600)
            if args.runtime_image:
                command = ["docker", "run", "--rm", "--name", api_name, "--network", "host", "--cpus=1", "--memory=512m", "--read-only", "--cap-drop=ALL", "--security-opt=no-new-privileges", "--user", f"{os.getuid()}:{os.getgid()}", "--env-file", str(api_env), "-v", f"{binary}:/runner:ro", "-v", f"{directory}:{directory}:ro", "-w", "/tmp", args.runtime_image, "/runner"]
                process = subprocess.Popen(command, stdout=logs, stderr=logs)
            else:
                process = subprocess.Popen([str(binary)], cwd=directory, env={"PATH": os.environ.get("PATH", ""), **environment}, stdout=logs, stderr=logs)
            deadline = time.monotonic() + 35
            while time.monotonic() < deadline:
                try:
                    call("/health/ready")
                    return
                except (urllib.error.URLError, TimeoutError):
                    if process.poll() is not None:
                        raise RuntimeError("isolated API failed to start; no private log printed")
                    time.sleep(0.15)
            raise RuntimeError("isolated API startup timed out")

        def capability(system, kind):
            return {"identifier": system + ".read", "version": "1", "description": "Synthetic read-only acceptance", "adapter": system, "required_permission": system + ".read", "enabled": True, "timeout_ms": 30000, "input_schema": {"type": "object", "additionalProperties": False}, "output_schema": {"type": "object", "required": ["kind", "reachable", "authenticated"], "properties": {"kind": {"const": kind}, "reachable": {"const": True}, "authenticated": {"const": kind != "wppconnect_health"}}}}

        def execute(system, key, body=None, version="1", expected=200, extra=None):
            request = {"identifier": system + ".read", "version": version, "input": {} if body is None else body}
            request.update(extra or {})
            return call("/v1/capabilities/execute", "POST", request, key=key, expected=expected)

        try:
            subprocess.run(["docker", "run", "-d", "--name", db_name, "--network", "host", "--cpus=1", "--memory=384m", "--env-file", str(pg_env), "--tmpfs", "/var/lib/postgresql/data:rw,size=512m", args.postgres_image, "postgres", "-c", "listen_addresses=127.0.0.1", "-c", "port=" + str(db_port)], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
            for _ in range(100):
                ready = subprocess.run(["docker", "exec", db_name, "pg_isready", "-p", str(db_port), "-U", "kusi_qa", "-d", "kusi_qa"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                if ready.returncode == 0:
                    break
                time.sleep(0.15)
            if args.rust_workspace:
                test_env = directory / "tests.env"
                test_env.write_text(f"TEST_DATABASE_URL=postgres://kusi_qa:{db_password}@127.0.0.1:{db_port}/kusi_qa\n")
                test_env.chmod(0o600)
                workspace = args.rust_workspace.resolve(strict=True)
                with open(directory / "rust-tests.log", "w+") as test_log:
                    command = ["docker", "run", "--rm", "--network", "host", "--cpus=2", "--memory=2g", "--cap-drop=ALL", "--security-opt=no-new-privileges", "--user", f"{os.getuid()}:{os.getgid()}", "--env-file", str(test_env), "-e", "CARGO_HOME=/work/.cargo", "-e", "CARGO_BUILD_JOBS=2", "-e", "CARGO_PROFILE_DEV_DEBUG=0", "-e", "CARGO_INCREMENTAL=0", "-v", f"{workspace}:/work", "-w", "/work", args.runtime_image, "cargo", "test", "--workspace", "--offline"]
                    result = subprocess.run(command, stdout=test_log, stderr=test_log)
                    assert result.returncode == 0, "Rust workspace tests failed; private log withheld"
                    test_log.seek(0)
                    rust_tests = sum(int(number) for number in re.findall(r"test result: ok\. (\d+) passed", test_log.read()))
                    assert rust_tests > 0
                checks.append("full Rust workspace tests with isolated PostgreSQL and loopback sockets")
            start_api()
            tenants = []
            for label in ["primary", "other"]:
                tenants.append(call("/v1/organizations", "POST", {"name": "Synthetic " + label, "slug": "qa-" + label + "-" + run_id}, admin=True, expected=201))
            key, other_key = (tenant["api_key"] for tenant in tenants)
            tenant_id = tenants[0]["organization"]["id"]
            kinds = {"crm": "frappe_identity", "erp": "frappe_identity", "plane": "plane_identity", "wppconnect": "wppconnect_health"}
            for system, kind in kinds.items():
                fixture = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Fixture)
                fixture.kind, fixture.calls = kind, 0
                fixture.username = system + "@example.invalid"
                fixture.password, fixture.csrf = secrets.token_urlsafe(24), secrets.token_hex(20)
                fixture.sessions = set()
                fixtures.append(fixture)
                threading.Thread(target=fixture.serve_forever, daemon=True).start()
                profile = {"kind": kind, "base_url": "http://127.0.0.1:" + str(fixture.server_port), "host": "fixture.test"}
                if kind != "wppconnect_health":
                    credential_path = directory / (system + ".json")
                    private_json(credential_path, {"username": fixture.username, "password": fixture.password})
                    profile["credential_file"] = str(credential_path)
                profile_config.setdefault(tenant_id, {})[system] = profile
                for tenant_key in [key, other_key]:
                    call("/v1/capabilities", "POST", capability(system, kind), key=tenant_key, admin=True, expected=204)
                execute(system, key, expected=403)
                for tenant_key in [key, other_key]:
                    call("/v1/capabilities/grants", "POST", {"permission": system + ".read"}, key=tenant_key, admin=True, expected=204)
            stop_api()
            start_api()
            for (system, kind), fixture in zip(kinds.items(), fixtures):
                before = fixture.calls
                execute(system, "invalid-key", expected=401)
                execute(system, other_key, expected=403)
                execute(system, key, {"url": "http://169.254.169.254"}, expected=400)
                execute(system, key, version="unknown", expected=403)
                execute(system, key, extra={"organization_id": tenant_id}, expected=422)
                assert fixture.calls == before, "denied execution contacted destination"
                result = execute(system, key)
                assert result["data"]["kind"] == kind and result["data"]["reachable"] is True
                assert result["source"] == "configured_http_connector"
                assert fixture.calls > before and not fixture.sessions
                cap = capability(system, kind)
                cap["enabled"] = False
                call("/v1/capabilities", "POST", cap, key=key, admin=True, expected=204)
                before = fixture.calls
                execute(system, key, expected=403)
                assert fixture.calls == before
                cap["enabled"] = True
                cap["input_schema"] = {}  # adapter still rejects caller arguments with a permissive registry
                call("/v1/capabilities", "POST", cap, key=key, admin=True, expected=204)
                execute(system, key, {"username": "Administrator"}, expected=400)
                assert fixture.calls == before
                cap["output_schema"] = {"const": "impossible"}
                call("/v1/capabilities", "POST", cap, key=key, admin=True, expected=204)
                execute(system, key, expected=400)
                assert fixture.calls > before and not fixture.sessions
                call("/v1/capabilities", "POST", capability(system, kind), key=key, admin=True, expected=204)
            checks.append("four runtime adapters via Axum; verified tenant key, PostgreSQL registry/grants and input/output contracts")
            before_calls = sum(fixture.calls for fixture in fixtures)
            sql("ALTER TABLE execution_audit RENAME TO execution_audit_temporarily_unavailable")
            try:
                execute("crm", key, expected=503)
                assert sum(fixture.calls for fixture in fixtures) == before_calls
            finally:
                sql("ALTER TABLE execution_audit_temporarily_unavailable RENAME TO execution_audit")
            checks.append("audit failure blocks destination; denied/invalid/disabled/wrong-tenant/unknown-version requests do not call adapters")
            audit_before = int(sql("SELECT count(*) FROM execution_audit"))
            grants_before = int(sql("SELECT count(*) FROM capability_grants"))
            stop_api()
            start_api()
            assert int(sql("SELECT count(*) FROM execution_audit")) == audit_before
            assert int(sql("SELECT count(*) FROM capability_grants")) == grants_before
            for system in kinds:
                execute(system, key)
            checks.append("PostgreSQL capabilities, grants and audit survive API restart; reads succeed again")
            live_results = []
            if args.live_credentials:
                credentials = json.loads(args.live_credentials.read_text())
                profiles = {}
                for system, container, host in [("crm", "frappe-crm-backend-1", "crm.soryntel.com"), ("erp", "coneimera-erp-backend-1", "erp.coneimera2027.com")]:
                    addresses = subprocess.check_output(["docker", "inspect", "--format", "{{range .NetworkSettings.Networks}}{{.IPAddress}} {{end}}", container], text=True).split()
                    assert len(addresses) == 1
                    profiles[system] = {"kind": "frappe_identity", "base_url": "http://" + addresses[0] + ":8000", "host": host}
                profiles["plane"] = {"kind": "plane_identity", "base_url": "http://127.0.0.1:8080", "host": "plane.soryntel.com"}
                profiles["wppconnect"] = {"kind": "wppconnect_health", "base_url": "http://127.0.0.1:21465", "host": "127.0.0.1"}
                for system in ["crm", "erp", "plane"]:
                    assert credentials[system]["email"] == "kusi-qa-20261108@example.invalid"
                    path = directory / ("live-" + system + ".json")
                    private_json(path, {"username": credentials[system]["email"], "password": credentials[system]["password"]})
                    profiles[system]["credential_file"] = str(path)
                profile_config = {tenant_id: profiles}
                hosts = [profile["host"] for profile in profiles.values()]
                stop_api()
                start_api()
                for system, kind in kinds.items():
                    result = execute(system, key)["data"]
                    assert result["kind"] == kind and result["reachable"] is True
                    live_results.append({"system": system, "data": result})
                checks.append("real CRM/ERPNext/Plane/WPPConnect via production Axum runtime; only synthetic accounts; no messages")
            # Invalidated keys must stop execution, independently of persisted grants.
            sql("UPDATE organization_api_keys SET revoked_at=now() WHERE key_hash='" + hashlib.sha256(key.encode()).hexdigest() + "'")
            execute("crm", key, expected=401)
            assert sql("SELECT count(*) FROM (SELECT event_id FROM execution_audit GROUP BY event_id HAVING count(*)<>2 OR count(*) FILTER (WHERE outcome='started')<>1) broken") == "0"
            outcomes = sql("SELECT outcome,count(*) FROM execution_audit GROUP BY outcome ORDER BY outcome")
            summary = {"status": "passed", "checks": checks, "live": live_results, "audit_rows": int(sql("SELECT count(*) FROM execution_audit")), "audit_outcomes": outcomes.splitlines(), "grants": grants_before, "messages_sent": 0, "rust_tests_passed": rust_tests}
            if args.result:
                private_json(args.result, summary)
            print(json.dumps(summary, indent=2))
        finally:
            stop_api()
            subprocess.run(["docker", "rm", "-f", db_name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            for fixture in fixtures:
                fixture.shutdown()
                fixture.server_close()
            logs.close()


if __name__ == "__main__":
    main()
