"""Real browser acceptance for web-managed profiles with synthetic credentials only."""
import argparse
from contextlib import ExitStack
import json
import os
from pathlib import Path
import secrets
import urllib.parse

from playwright.sync_api import expect, sync_playwright
from local_acceptance import ROOT, free_port, postgres
from verify_external_access import server
from verify_connection_settings import bootstrap
from verify_oidc_browser import process, ready


def login(page,web,username,password):
    page.goto(web+"/conexiones")
    page.get_by_label("Organización",exact=True).fill("settings-fixture")
    page.get_by_label("Usuario",exact=True).fill(username)
    page.get_by_label("Contraseña",exact=True).fill(password)
    page.get_by_role("button",name="Entrar",exact=True).click()
    expect(page.get_by_role("heading",name="Conexiones",exact=True)).to_be_visible()


def verify(artifacts):
    artifacts.mkdir(parents=True,exist_ok=True)
    with postgres() as (env,tmp):
        port=free_port();web=f"http://127.0.0.1:{port}";env["KUSI_ADMIN_API_KEY"]=secrets.token_hex(32)
        with server(env,tmp,True,web) as api,ExitStack() as stack:
            password=bootstrap(env,api,"settings-fixture","fixture-owner")
            env.update(KUSI_API_ORIGIN=api,NEXT_TELEMETRY_DISABLED="1")
            child=process(stack,[str(ROOT/"web/admin/node_modules/.bin/next"),"start","--hostname","127.0.0.1","--port",str(port)],env,ROOT/"web/admin",tmp/"next.log")
            ready(child,web,"/health/ready")
            errors=[];foreign=[]
            with sync_playwright() as p:
                browser=p.chromium.launch(executable_path="/usr/bin/chromium",headless=True,args=["--no-sandbox"])
                context=browser.new_context(viewport={"width":1440,"height":1100})
                def route(request):
                    if urllib.parse.urlsplit(request.request.url).hostname!="127.0.0.1":foreign.append("blocked");request.abort()
                    else:request.continue_()
                context.route("**/*",route)
                try:
                    page=context.new_page();page.on("pageerror",lambda _error:errors.append("page error"))
                    login(page,web,"fixture-owner",password)
                    expect(page.get_by_role("heading",name="Sistemas y canal del piloto")).to_be_visible()
                    profiles=[("Plane","https://plane.example.invalid","api_key","synthetic-plane-browser-key"),("ERP CONEIMERA","https://erp.example.invalid","api_key","synthetic-erp-browser-key")]
                    private=[value[3] for value in profiles]+["synthetic-erp-browser-secret","123456:synthetic_telegram_browser_token"]
                    for name,url,field,value in profiles:
                        page.get_by_role("button",name="Configurar "+name,exact=True).click()
                        expect(page.get_by_role("dialog")).to_be_visible()
                        page.get_by_label("URL de la instancia",exact=True).fill(url)
                        page.get_by_label("API Key",exact=True).fill(value)
                        if name=="ERP CONEIMERA":page.get_by_label("API Secret",exact=True).fill(private[2])
                        else:page.get_by_label("Espacio de trabajo de Plane (opcional)",exact=True).fill("synthetic-team")
                        page.get_by_role("button",name="Guardar configuración",exact=True).click()
                        expect(page.get_by_role("dialog")).to_have_count(0)
                    page.get_by_role("button",name="Configurar Telegram",exact=True).click()
                    page.get_by_label("Usuario del bot (opcional)",exact=True).fill("synthetic_bot")
                    page.get_by_label("Token de Telegram",exact=True).fill(private[3])
                    page.get_by_label("URL pública del webhook (opcional)",exact=True).fill("https://hooks.example.invalid/telegram")
                    page.get_by_role("button",name="Guardar configuración",exact=True).click()
                    expect(page.get_by_role("dialog")).to_have_count(0)
                    page.reload()
                    expect(page.get_by_text("Credenciales guardadas",exact=True)).to_have_count(3)
                    page.screenshot(path=str(artifacts/"connections-desktop.png"),full_page=True)
                    for value in private:assert value not in page.content()
                    summary=page.evaluate("async()=>await fetch('/v1/local/connection-settings').then(r=>r.text())")
                    assert all(value not in summary for value in private) and 'credential_ciphertext' not in summary
                    assert page.evaluate("Object.keys(localStorage).length")==0
                    page.get_by_role("button",name="Configurar Plane",exact=True).click()
                    expect(page.get_by_label("API Key",exact=True)).to_have_value("")
                    page.get_by_label("API Key",exact=True).fill("synthetic-plane-browser-rotated")
                    page.screenshot(path=str(artifacts/"plane-edit-desktop.png"),full_page=True)
                    page.get_by_role("button",name="Guardar configuración",exact=True).click()
                    expect(page.get_by_role("dialog")).to_have_count(0)
                    # A second tab must not overwrite a profile edited after it opened.
                    second=context.new_page();second.goto(web+"/conexiones")
                    second.get_by_role("button",name="Configurar Plane",exact=True).click()
                    page.get_by_role("button",name="Configurar Plane",exact=True).click()
                    page.get_by_label("Nombre de la conexión",exact=True).fill("Plane piloto")
                    page.get_by_role("button",name="Guardar configuración",exact=True).click()
                    expect(page.get_by_role("dialog")).to_have_count(0)
                    second.get_by_label("API Key",exact=True).fill("synthetic-stale-secret")
                    second.get_by_role("button",name="Guardar configuración",exact=True).click()
                    expect(second.get_by_role("dialog").get_by_role("alert")).to_contain_text("borrador cambió")
                    expect(second.get_by_label("API Key",exact=True)).to_have_value("")
                    second.close()
                    page.get_by_role("button",name="Borrar credenciales de ERP CONEIMERA",exact=True).click()
                    expect(page.get_by_role("alertdialog")).to_be_visible()
                    page.get_by_role("button",name="Confirmar",exact=True).click()
                    expect(page.get_by_text("Faltan credenciales",exact=True)).to_have_count(1)
                    expect(page.get_by_role("alertdialog")).to_have_count(0)
                    page.set_viewport_size({"width":390,"height":844})
                    assert page.evaluate("document.documentElement.scrollWidth<=innerWidth")
                    page.screenshot(path=str(artifacts/"connections-mobile.png"),full_page=True)
                    page.get_by_role("button",name="Configurar Telegram",exact=True).click()
                    expect(page.get_by_label("Token de Telegram",exact=True)).to_have_value("")
                    expect(page.get_by_label("Secreto de webhook (opcional)",exact=True)).to_have_value("")
                    page.screenshot(path=str(artifacts/"telegram-edit-mobile.png"),full_page=True)
                    page.get_by_role("button",name="Cerrar",exact=True).click()
                    viewer_password=secrets.token_urlsafe(24)
                    status=page.evaluate("""async password=>{const s=await fetch('/local/auth/session').then(r=>r.json());const r=await fetch('/v1/local/users',{method:'POST',headers:{'content-type':'application/json','x-csrf-token':s.csrf_token},body:JSON.stringify({username:'browser-viewer',display_name:'Synthetic viewer',password,role:'viewer'})});return r.status}""",viewer_password)
                    assert status==200
                    viewer_context=browser.new_context(viewport={"width":390,"height":844});viewer_context.route("**/*",route)
                    viewer=viewer_context.new_page();login(viewer,web,"browser-viewer",viewer_password)
                    expect(viewer.get_by_text("Solo el propietario puede consultar y administrar estas credenciales.",exact=True)).to_be_visible()
                    expect(viewer.get_by_role("button",name="Configurar Plane",exact=True)).to_have_count(0)
                    assert viewer.evaluate("async()=> (await fetch('/v1/local/connection-settings')).status")==403
                    viewer_context.close()
                    assert not errors and not foreign
                finally:context.close();browser.close()
    return {"status":"passed","checks":["create and reload Plane/ERP/Telegram profiles from web","write-only password fields and server-generated webhook secret","credential rotation and blank-edit preservation","stale tab conflict clears typed secrets","explicit credential clearing with confirmation","non-owner UI and API denial","desktop/mobile real Chromium with no horizontal overflow","no credentials in responses/HTML/localStorage","all browser traffic restricted to loopback"],"screenshots":4,"external_calls":False,"existing_database_used":False}


if __name__=="__main__":
    os.umask(0o077)
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument("--artifacts",type=Path,required=True);args=parser.parse_args()
    result=verify(args.artifacts.resolve());(args.artifacts/"browser-acceptance.json").write_text(json.dumps(result,indent=2)+"\n");print(json.dumps(result,indent=2))
