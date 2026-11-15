"""OIDC desktop/mobile acceptance with real Chromium and disposable loopback services."""
import argparse
from contextlib import ExitStack
import json
import os
from pathlib import Path
import secrets
import subprocess
import time
import urllib.parse

from playwright.sync_api import expect, sync_playwright
from local_acceptance import ROOT, free_port, postgres, sql
from oidc_test_issuer import MockOIDCIssuer
from verify_external_access import call


def process(stack, command, env, cwd, log_path):
    log = stack.enter_context(log_path.open("w"))
    child = subprocess.Popen(command, env=env, cwd=cwd, stdout=log, stderr=log)
    def stop():
        if child.poll() is None:
            child.terminate()
            try:
                child.wait(timeout=10)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait(timeout=5)
    stack.callback(stop)
    return child


def ready(child, base, path):
    for _ in range(150):
        if child.poll() is not None:
            raise RuntimeError("disposable browser acceptance service exited")
        try:
            call(base, path)
            return
        except OSError:
            time.sleep(0.1)
    raise RuntimeError("disposable browser acceptance readiness timed out")


def verify(artifacts):
    artifacts.mkdir(parents=True, exist_ok=True)
    with postgres() as (env, temporary), MockOIDCIssuer() as issuer, ExitStack() as stack:
        api_port, web_port = free_port(), free_port()
        api, web = f"http://127.0.0.1:{api_port}", f"http://127.0.0.1:{web_port}"
        env.update(KUSI_ADMIN_API_KEY=secrets.token_hex(32), KUSI_LOCAL_MODE="false",
                   BIND_ADDRESS=f"127.0.0.1:{api_port}", KUSI_LOCAL_ORIGIN=web,
                   KUSI_LOCAL_STORAGE=str(temporary / "storage"),
                   KUSI_ADMIN_PUBLIC_ORIGIN=web, KUSI_ADMIN_OIDC_ISSUER=issuer.url,
                   KUSI_ADMIN_OIDC_CLIENT_ID="test-client", KUSI_ADMIN_OIDC_CLIENT_SECRET="test-client-secret",
                   KUSI_ADMIN_OIDC_REDIRECT_URI=web + "/v1/admin/auth/oidc/callback",
                   KUSI_ADMIN_OIDC_STATE_KEY=secrets.token_hex(32), KUSI_CREATIVE_WORKER="false",
                   KUSI_OAUTH_REFRESH_WORKER="false", KUSI_API_ORIGIN=api,
                   NEXT_TELEMETRY_DISABLED="1", RUST_LOG="error")
        for name in ["CHANNELS", "LLM_PROFILES", "AUDIO_PROFILES", "CREATIVE_RECIPES", "HTTP_DESTINATIONS", "OAUTH_PROFILES", "SYSTEM_PROFILES"]:
            env["KUSI_" + name] = "{}"
        child = process(stack, [str(ROOT / "target/debug/kusi-server")], env, temporary, temporary / "api.log")
        ready(child, api, "/health/ready")
        call(api, "/v1/organizations", {"name": "OIDC visual fixture", "slug": "oidc-visual-fixture"}, {"x-admin-key": env["KUSI_ADMIN_API_KEY"]}, 201)
        provision = dict(env, KUSI_ADMIN_PROVISION_ISSUER=issuer.url, KUSI_ADMIN_PROVISION_SUBJECT="admin-subject",
                         KUSI_ADMIN_PROVISION_ORGANIZATION="oidc-visual-fixture", KUSI_ADMIN_PROVISION_ROLE="approver")
        subprocess.run([str(ROOT / "target/debug/provision_admin_identity")], env=provision, cwd=temporary,
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, timeout=15)
        child = process(stack, [str(ROOT / "web/admin/node_modules/.bin/next"), "start", "--hostname", "127.0.0.1", "--port", str(web_port)],
                        env, ROOT / "web/admin", temporary / "next.log")
        ready(child, web, "/health/ready")
        errors, foreign = [], []
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(executable_path="/usr/bin/chromium", headless=True, args=["--no-sandbox"])
            try:
                for label, viewport in [("desktop", {"width": 1440, "height": 900}), ("mobile", {"width": 390, "height": 844})]:
                    context = browser.new_context(viewport=viewport)
                    def local_only(route):
                        if urllib.parse.urlsplit(route.request.url).hostname != "127.0.0.1":
                            foreign.append("foreign request blocked")
                            route.abort()
                        else:
                            route.continue_()
                    context.route("**/*", local_only)
                    page = context.new_page()
                    page.on("pageerror", lambda _error: errors.append("page error"))
                    page.goto(web + "/admin")
                    login = page.get_by_role("link", name="Continuar con el inicio corporativo")
                    expect(login).to_be_visible()
                    page.keyboard.press("Tab")
                    expect(login).to_be_focused()
                    page.screenshot(path=str(artifacts / f"oidc-login-{label}.png"), full_page=True)
                    issuer.bad_signature = True
                    login.click()
                    expect(page.get_by_role("alert").filter(has_text="No se pudo validar el acceso")).to_contain_text("No se pudo validar el acceso")
                    expect(login).to_be_visible()
                    page.screenshot(path=str(artifacts / f"oidc-error-{label}.png"), full_page=True)
                    login.click()
                    expect(page.get_by_text("Tus organizaciones", exact=True)).to_be_visible()
                    expect(page.get_by_text("oidc-visual-fixture", exact=True)).to_be_visible()
                    expect(page.get_by_text("Aprobador", exact=True)).to_be_visible()
                    cookies = [cookie for cookie in context.cookies() if cookie["name"] == "kusi_admin_session"]
                    assert len(cookies) == 1 and cookies[0]["httpOnly"]
                    assert "kusi_admin_session=" not in page.evaluate("document.cookie")
                    assert env["KUSI_ADMIN_API_KEY"] not in page.content()
                    assert page.evaluate("Object.keys(localStorage).length") == 0
                    assert page.evaluate("document.documentElement.scrollWidth<=innerWidth")
                    page.screenshot(path=str(artifacts / f"oidc-session-{label}.png"), full_page=True)
                    # A disabled membership must disappear on the next session check.
                    sql(env, "UPDATE admin_memberships SET disabled_at=now()")
                    page.reload()
                    expect(page.get_by_role("link", name="Continuar con el inicio corporativo")).to_be_visible()
                    expect(page.get_by_text("oidc-visual-fixture", exact=True)).to_have_count(0)
                    sql(env, "UPDATE admin_memberships SET disabled_at=NULL")
                    page.get_by_role("link", name="Continuar con el inicio corporativo").click()
                    expect(page.get_by_text("Tus organizaciones", exact=True)).to_be_visible()
                    page.get_by_role("button", name="Cerrar sesión", exact=True).click()
                    expect(page.get_by_role("link", name="Continuar con el inicio corporativo")).to_be_visible()
                    assert page.evaluate("async()=> (await fetch('/v1/admin/auth/session')).status") == 401
                    context.close()
            finally:
                browser.close()
        assert not errors and not foreign
        return {"status": "passed", "checks": ["desktop and mobile real Chromium", "keyboard focus", "invalid callback and retry", "corporate login and provisioned role", "HttpOnly cookie with no frontend admin key or localStorage", "membership revocation", "Origin/CSRF logout through fixed Next proxy", "no horizontal overflow or page errors", "browser requests restricted to loopback"], "screenshots": 6, "external_calls": False, "existing_database_used": False}


if __name__ == "__main__":
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, required=True)
    args = parser.parse_args()
    result = verify(args.artifacts.resolve())
    (args.artifacts / "acceptance.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))
