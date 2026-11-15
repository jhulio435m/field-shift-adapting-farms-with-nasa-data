"""Disposable PostgreSQL + loopback HTTP acceptance; never reads .env.

Runs the real HTTP boundary with an injected synthetic connector in Rust tests,
then checks the compiled server/CLI and the local external-call block. No real
connector destination, provider credential or existing database is configured.
"""
import argparse
from contextlib import contextmanager
import errno
import getpass
import hashlib
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parent.parent


def free_port():
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return listener.getsockname()[1]


def run(command, env, cwd=ROOT):
    subprocess.run(command, env=env, cwd=cwd, check=True, timeout=180)


def call(base, path, body=None, headers=None, expected=200):
    request = urllib.request.Request(
        base + path,
        data=json.dumps(body).encode() if body is not None else None,
        headers={"Content-Type": "application/json", **(headers or {})},
    )
    # Do not inherit HTTP proxies even for loopback acceptance.
    client = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    try:
        with client.open(request, timeout=5) as response:
            status, data = response.status, response.read()
    except urllib.error.HTTPError as error:
        status, data = error.code, error.read()
    if status != expected:
        raise RuntimeError(f"{path}: expected status {expected}, got {status}")
    return json.loads(data) if data else None


@contextmanager
def server(env, temporary, local, origin=None):
    port = free_port()
    base = f"http://127.0.0.1:{port}"
    settings = dict(
        env,
        BIND_ADDRESS=f"127.0.0.1:{port}",
        KUSI_LOCAL_MODE=str(local).lower(),
        KUSI_LOCAL_ORIGIN=origin or base,
        KUSI_LOCAL_STORAGE=str(temporary / "storage"),
        KUSI_CREATIVE_WORKER="false",
        KUSI_CREATIVE_SIMULATION="true",
        KUSI_OAUTH_REFRESH_WORKER="false",
        RUST_LOG="error",
    )
    for name in ["CHANNELS", "LLM_PROFILES", "AUDIO_PROFILES", "CREATIVE_RECIPES",
                 "HTTP_DESTINATIONS", "OAUTH_PROFILES", "SYSTEM_PROFILES"]:
        settings["KUSI_" + name] = "{}"
    with (temporary / "server.log").open("w") as log:
        # Empty temporary cwd ensures dotenv cannot load the workspace's .env.
        process = subprocess.Popen(
            [str(ROOT / "target/debug/kusi-server")], env=settings,
            cwd=temporary, stdout=log, stderr=log,
        )
        try:
            for _ in range(100):
                if process.poll() is not None:
                    raise RuntimeError("temporary server exited before readiness")
                try:
                    call(base, "/health/ready")
                    break
                except (OSError, RuntimeError):
                    time.sleep(0.1)
            else:
                raise RuntimeError("temporary server readiness timed out")
            yield base
        finally:
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)


def provision(env, temporary, organization, key, action, expected_success=True):
    path = temporary / "request.json"
    request = {
        "organization": organization,
        "principal_hash": hashlib.sha256(key.encode()).hexdigest(),
        "adapter": "system-a",
        "action": action,
    }
    # Set private permissions before writing any request data.
    with path.open("w") as handle:
        path.chmod(0o600)
        json.dump(request, handle)
    process = subprocess.run(
        [str(ROOT / "target/debug/provision_external_access")],
        env=dict(env, KUSI_EXTERNAL_ACCESS_REQUEST_FILE=str(path)),
        cwd=temporary, capture_output=True, text=True, timeout=15,
    )
    if (process.returncode == 0) != expected_success:
        raise RuntimeError("CLI provisioning returned an unexpected outcome")


def verify():
    # Do not copy provider settings, database credentials or PostgreSQL options.
    env = {key: value for key, value in os.environ.items()
           if not key.startswith(("KUSI_", "PG"))
           and key not in {"DATABASE_URL", "TEST_DATABASE_URL", "BIND_ADDRESS", "RUST_LOG"}}
    with tempfile.TemporaryDirectory(prefix="kusi-external-acceptance-") as directory:
        temporary = Path(directory)
        port = free_port()
        database = temporary / "postgres"
        socket_dir = temporary / "socket"
        socket_dir.mkdir(mode=0o700)
        with (temporary / "postgres-init.log").open("w") as log:
            subprocess.run(["initdb", "-D", str(database), "--auth=trust", "--no-locale", "--encoding=UTF8"],
                           env=env, stdout=log, stderr=log, check=True, timeout=30)
        started = False
        try:
            started = True
            subprocess.run(["pg_ctl", "-D", str(database), "-l", str(temporary / "postgres.log"),
                            "-o", f"-h 127.0.0.1 -p {port} -k {socket_dir}", "-w", "start"],
                           env=env, check=True, stdout=subprocess.DEVNULL, timeout=30)
            database_url = f"postgres://{getpass.getuser()}@127.0.0.1:{port}/postgres"
            env.update(DATABASE_URL=database_url, TEST_DATABASE_URL=database_url,
                       KUSI_ADMIN_API_KEY=secrets.token_hex(32))
            run(["cargo", "build", "-p", "kusi-server", "--bin", "kusi-server", "--bin", "provision_external_access"], env)
            run(["cargo", "test", "-p", "kusi-infrastructure", "--test", "external_access", "--", "--ignored", "--nocapture"], env)
            run(["cargo", "test", "-p", "kusi-server", "--bin", "kusi-server", "http_external_acl_acceptance", "--", "--ignored", "--nocapture"], env)
            with server(env, temporary, local=False) as base:
                admin = {"x-admin-key": env["KUSI_ADMIN_API_KEY"]}
                created = call(base, "/v1/organizations", {"name": "Synthetic CLI acceptance", "slug": "synthetic-cli"}, admin, 201)
                organization, key = created["organization"]["id"], created["api_key"]
                headers = {"Authorization": "Bearer " + key, **admin}
                call(base, "/v1/capabilities", {
                    "identifier": "record.read", "version": "1", "description": "Synthetic CLI test",
                    "adapter": "system-a", "required_permission": "record.read", "enabled": True,
                    "timeout_ms": 1000, "input_schema": {}, "output_schema": {},
                }, headers, 204)
                call(base, "/v1/capabilities/grants", {"permission": "record.read"}, headers, 204)
                for action in [
                    {"operation": "protect"},
                    {"operation": "link", "subject": "synthetic-cli-subject", "ttl_seconds": 600, "confirmed": True},
                    {"operation": "grant", "capability": "record.read", "version": "1", "resource_ref": "record-a", "ttl_seconds": 300, "confirmed": True},
                    {"operation": "revoke_grant", "capability": "record.read", "version": "1", "resource_ref": "record-a"},
                    {"operation": "revoke_link"},
                ]:
                    provision(env, temporary, organization, key, action)
                provision(env, temporary, organization, key,
                          {"operation": "link", "subject": "replacement", "ttl_seconds": 600, "confirmed": True}, expected_success=False)
                call(base, "/v1/capabilities/execute", {"identifier": "record.read", "version": "1", "input": {"resource_ref": "record-a"}}, headers, 403)
            with server(env, temporary, local=True) as base:
                call(base, "/v1/capabilities/execute", {"identifier": "record.read", "version": "1", "input": {"resource_ref": "record-a"}}, expected=403)
                call(base, "/v1/ai/generate", {}, expected=403)
            return {"status": "passed", "checks": [
                "fresh disposable PostgreSQL and migration 0018",
                "PostgreSQL exact ACL, expiry, revocation and system isolation",
                "real loopback HTTP boundary with PostgreSQL and synthetic connector",
                "HTTP credential, tenant, version, input and output validation",
                "global permission and unavailable-policy fail-closed checks",
                "compiled server and CLI provisioning/revocation without reactivation",
                "compiled local runtime blocks capability execution and remote AI",
            ], "external_calls": False, "existing_database_used": False,
                "connector": "synthetic test dependency; production transport unchanged"}
        finally:
            if started:
                subprocess.run(["pg_ctl", "-D", str(database), "-m", "fast", "-w", "stop"],
                               env=env, check=True, stdout=subprocess.DEVNULL, timeout=30)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--result", type=Path, help="Optional credential-free JSON evidence file")
    args = parser.parse_args(argv)
    exit_code = 0
    try:
        result = verify()
    except OSError as error:
        blocked = error.errno in {errno.EPERM, errno.EACCES}
        result = {
            "status": "blocked" if blocked else "failed",
            "reason": "operating_system_permission_denied" if blocked else "operating_system_error",
            "verification_complete": False,
        }
        exit_code = 2 if blocked else 1
    except (subprocess.SubprocessError, RuntimeError):
        # Exceptions may contain commands, response details or environment data.
        # Keep the report generic and never serialize their arguments or output.
        result = {
            "status": "failed",
            "reason": "acceptance_check_or_subprocess_failed",
            "verification_complete": False,
        }
        exit_code = 1
    if args.result:
        args.result.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
