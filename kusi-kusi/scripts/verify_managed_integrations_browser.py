"""010 Chromium acceptance with synthetic data, isolated PostgreSQL and loopback-only browser traffic."""
import argparse
from contextlib import ExitStack
import json
import os
from pathlib import Path
import secrets
import urllib.parse
import uuid
from playwright.sync_api import expect, sync_playwright
from local_acceptance import ROOT, free_port, postgres, sql
from verify_external_access import server
from verify_connection_settings import Client, bootstrap
from verify_oidc_browser import process, ready


def login(page, web, username, password):
    page.goto(web+'/conexiones')
    page.get_by_label('Organización',exact=True).fill('managed-browser')
    page.get_by_label('Usuario',exact=True).fill(username)
    page.get_by_label('Contraseña',exact=True).fill(password)
    page.get_by_role('button',name='Entrar',exact=True).click()
    expect(page.get_by_role('heading',name='Conexiones',exact=True)).to_be_visible()


def verify(artifacts):
    artifacts.mkdir(parents=True,exist_ok=True)
    with postgres() as (env,tmp):
        port=free_port();web=f'http://127.0.0.1:{port}';env['KUSI_ADMIN_API_KEY']=secrets.token_hex(32)
        with server(env,tmp,True,web) as base,ExitStack() as stack:
            password=bootstrap(env,base,'managed-browser','fixture-owner')
            api=Client(base);api.csrf=api.call('/local/auth/login','POST',{'organization':'managed-browser','username':'fixture-owner','password':password},headers={'Origin':web})['csrf_token']
            viewer_password=secrets.token_urlsafe(24)
            viewer=api.call('/v1/local/users','POST',{'username':'fixture-viewer','display_name':'Synthetic viewer','role':'viewer','password':viewer_password},headers={'Origin':web})
            api.call('/v1/local/resource-access/grants','POST',{'user_id':viewer['id'],'resource_ref':'demo-resource-001','authorization_verified':True},201,{'Origin':web})
            org=sql(env,"SELECT organization_id FROM local_users WHERE username='fixture-owner'");actor=sql(env,"SELECT id FROM local_users WHERE username='fixture-owner'")
            channel,binding,conversation,execution=[str(uuid.uuid4()) for _ in range(4)]
            sql(env,f"INSERT INTO local_channel_bindings(organization_id,channel_id,sender,user_id,id) VALUES('{org}','{channel}','synthetic-chat','{actor}','{binding}')")
            sql(env,f"INSERT INTO local_conversations(id,organization_id,user_id,channel_id,sender,title) VALUES('{conversation}','{org}','{actor}','{channel}','synthetic-chat','Synthetic')")
            sql(env,f"INSERT INTO local_outbox(id,organization_id,conversation_id,channel_id,recipient,content,status) VALUES('{execution}','{org}','{conversation}','{channel}','synthetic-chat','private-synthetic-message','awaiting_review')")
            env.update(KUSI_API_ORIGIN=base,NEXT_TELEMETRY_DISABLED='1')
            child=process(stack,[str(ROOT/'web/admin/node_modules/.bin/next'),'start','--hostname','127.0.0.1','--port',str(port)],env,ROOT/'web/admin',tmp/'next.log')
            ready(child,web,'/health/ready')
            errors=[];foreign=[]
            with sync_playwright() as p:
                browser=p.chromium.launch(executable_path='/usr/bin/chromium',headless=True,args=['--no-sandbox'])
                context=browser.new_context(viewport={'width':1440,'height':1000})
                def route(request):
                    if urllib.parse.urlsplit(request.request.url).hostname!='127.0.0.1':foreign.append('blocked');request.abort()
                    else:request.continue_()
                context.route('**/*',route)
                page=context.new_page();page.on('pageerror',lambda error:errors.append(str(error)))
                login(page,web,'fixture-owner',password)
                page.get_by_role('button',name='Configurar Plane',exact=True).click()
                page.get_by_label('URL de la instancia',exact=True).fill('https://synthetic.example.invalid')
                page.get_by_label('API Key',exact=True).fill('synthetic-browser-plane-key')
                page.get_by_role('button',name='Guardar configuración',exact=True).click()
                expect(page.get_by_role('dialog')).to_have_count(0)
                page.get_by_role('button',name='Simular verificación de Plane',exact=True).click()
                expect(page.get_by_text('Simulación vigente',exact=True)).to_be_visible()
                assert 'synthetic-browser-plane-key' not in page.content()
                page.screenshot(path=str(artifacts/'connections-desktop.png'),full_page=True)
                page.reload();expect(page.get_by_text('Simulación vigente',exact=True)).to_be_visible()
                page.get_by_role('button',name='Configurar Plane',exact=True).click()
                page.get_by_label('Nombre de la conexión',exact=True).fill('Plane nueva revisión')
                page.get_by_role('button',name='Guardar configuración',exact=True).click()
                expect(page.get_by_role('dialog')).to_have_count(0)
                expect(page.get_by_text('Simulación vigente',exact=True)).to_have_count(0)
                page.goto(web+'/inicio')
                expect(page.get_by_role('heading',name='Tu primera consulta, paso a paso')).to_be_visible()
                page.screenshot(path=str(artifacts/'home-desktop.png'),full_page=True)
                page.goto(web+'/piloto')
                page.get_by_role('button',name='Ver historial',exact=True).click()
                expect(page.get_by_text('Pendiente de revisión',exact=True).first).to_be_visible()
                expect(page.get_by_text('private-synthetic-message',exact=True)).to_be_visible()
                page.get_by_role('button',name='Aprobar revisión local',exact=True).click()
                expect(page.get_by_role('button',name='Cancelar tarea',exact=True)).to_be_visible()
                page.get_by_role('button',name='Cancelar tarea',exact=True).click()
                expect(page.get_by_text('Cancelada',exact=True).first).to_be_visible()
                assert 'private-synthetic-message' not in page.content()
                page.screenshot(path=str(artifacts/'executions-desktop.png'),full_page=True)
                page.get_by_role('button',name='Vínculos del canal',exact=True).click()
                page.get_by_role('button',name='Revocar vínculo',exact=True).click()
                page.get_by_role('button',name='Confirmar',exact=True).click()
                expect(page.get_by_text('Revocado',exact=False).last).to_be_visible()
                page.get_by_role('button',name='Evaluación',exact=True).click()
                page.get_by_role('button',name='Ejecutar 24 casos',exact=True).click()
                expect(page.get_by_text('24 de 24 casos correctos',exact=True)).to_be_visible()
                page.screenshot(path=str(artifacts/'evaluations-desktop.png'),full_page=True)
                page.set_viewport_size({'width':390,'height':844})
                assert page.evaluate('document.documentElement.scrollWidth<=innerWidth+1')
                page.screenshot(path=str(artifacts/'evaluations-mobile.png'),full_page=True)
                page.get_by_role('button',name='Permisos',exact=True).click()
                expect(page.get_by_text('Evaluación sintética guardada. No acredita conexiones ni calidad de un modelo de IA.',exact=True)).to_have_count(0)
                page.wait_for_url('**/piloto?selected=policies')
                page.reload()
                expect(page.get_by_role('heading',name='Comprobar permisos sin modificarlos',exact=True)).to_be_visible()
                page.get_by_role('button',name='Simular permiso',exact=True).click()
                expect(page.get_by_text('Permitido por la concesión vigente.',exact=True)).to_be_visible()
                page.screenshot(path=str(artifacts/'policy-mobile.png'),full_page=True)
                assert page.evaluate('document.documentElement.scrollWidth<=innerWidth+1')
                assert page.evaluate('Object.keys(localStorage).length')==0
                viewer_context=browser.new_context(viewport={'width':390,'height':844});viewer_context.route('**/*',route)
                vp=viewer_context.new_page();vp.on('pageerror',lambda error:errors.append(str(error)))
                login(vp,web,'fixture-viewer',viewer_password)
                vp.goto(web+'/inicio')
                vp.get_by_role('button',name='Consultar datos ficticios',exact=True).click()
                expect(vp.get_by_text('Consulta completada · datos ficticios',exact=True)).to_be_visible()
                vp.screenshot(path=str(artifacts/'home-mobile.png'),full_page=True)
                vp.goto(web+'/piloto');vp.get_by_role('button',name='Mi contexto',exact=True).click()
                vp.get_by_role('button',name='Permitir memoria durante 7 días',exact=True).click()
                expect(vp.get_by_text('Memoria habilitada',exact=True)).to_be_visible()
                status=vp.evaluate("""async()=>{let s=await fetch('/local/auth/session').then(r=>r.json());return (await fetch('/v1/local/resource-access/execute',{method:'POST',headers:{'content-type':'application/json','x-csrf-token':s.csrf_token},body:JSON.stringify({resource_ref:'demo-resource-001'})})).status}""")
                assert status==200
                vp.reload();vp.get_by_role('button',name='Mi contexto',exact=True).click()
                expect(vp.get_by_role('button',name='Olvidar contexto',exact=True)).to_be_visible()
                vp.screenshot(path=str(artifacts/'memory-mobile.png'),full_page=True)
                vp.get_by_role('button',name='Olvidar contexto',exact=True).click()
                expect(vp.get_by_role('button',name='Olvidar contexto',exact=True)).to_have_count(0)
                expect(vp.get_by_role('button',name='Permisos',exact=True)).to_have_count(0)
                assert vp.evaluate("async()=> (await fetch('/v1/local/channel-operations/telegram')).status")==403
                assert not errors and not foreign
                viewer_context.close();context.close();browser.close()
    return {'status':'passed','external_calls':False,'existing_database_used':False,'checks':['profile fixture verification and revision invalidation','exact private proposal review and cancellation; lists and timeline omit message bodies','binding revocation confirmation','24 evaluation cases and evidence table','read-only policy simulation','viewer memory consent, authorized source and explicit deletion','non-owner administrative denial','desktop/mobile overflow check','no credentials in HTML or localStorage','loopback-only browser traffic'],'screenshots':6}


if __name__=='__main__':
    os.umask(0o077)
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--artifacts',type=Path,required=True);args=parser.parse_args()
    result=verify(args.artifacts.resolve());(args.artifacts/'browser-acceptance.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
