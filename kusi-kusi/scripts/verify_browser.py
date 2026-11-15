"""Local browser acceptance journey. Requires a running local server and setup_local.py."""
import json, time
from pathlib import Path
from playwright.sync_api import sync_playwright, expect
from PIL import Image, ImageDraw, ImageFont
root=Path(__file__).resolve().parent.parent
credentials=json.loads((root/'.local-data/login.json').read_text())
qa=root/'.local-data/qa';qa.mkdir(parents=True,exist_ok=True)
fixture_label='Cacao · producto ficticio '+str(int(time.time()))
image=Image.new('RGB',(800,800),'#ebe8dd');draw=ImageDraw.Draw(image)
draw.ellipse((210,680,610,720),fill='#d4d0bf');draw.rounded_rectangle((235,135,565,690),30,fill='#e1bd83');draw.rectangle((248,150,552,175),fill='#a48253');draw.rectangle((265,330,535,535),fill='#285745')
font=ImageFont.truetype('/usr/share/fonts/TTF/DejaVuSans.ttf',30)
small=ImageFont.truetype('/usr/share/fonts/TTF/DejaVuSans.ttf',17)
draw.text((298,368),'CACAO',font=font,fill='white');draw.text((313,420),'DEMO',font=font,fill='white');draw.text((282,478),'PRODUCTO FICTICIO',font=small,fill='#e9e8d1');draw.text((248,745),'ARCHIVO DE PRUEBA LOCAL',font=small,fill='#696a5e');fixture=qa/'producto-ficticio.png';image.save(fixture)
with sync_playwright() as p:
    browser=p.chromium.launch(executable_path='/usr/bin/chromium',headless=True,args=['--no-sandbox'])
    page=browser.new_page(viewport={'width':1440,'height':1000},device_scale_factor=1)
    errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
    base='http://127.0.0.1:3000'
    page.goto(base);expect(page.get_by_role('heading',name='Iniciar sesión')).to_be_visible()
    page.screenshot(path=str(qa/'login-desktop.png'),full_page=True)
    page.get_by_label('Organización',exact=True).fill(credentials['organization']);page.get_by_label('Usuario',exact=True).fill(credentials['username']);page.get_by_label('Contraseña',exact=True).fill(credentials['password']);page.get_by_role('button',name='Entrar',exact=True).click()
    expect(page.get_by_role('heading',name='Conversaciones',exact=True)).to_be_visible()
    assert '/conversaciones' in page.url
    second=browser.new_page(viewport={'width':1280,'height':900})
    second.goto(base);second.get_by_label('Organización',exact=True).fill(credentials['organization']);second.get_by_label('Usuario',exact=True).fill(credentials['username']);second.get_by_label('Contraseña',exact=True).fill(credentials['password']);second.get_by_role('button',name='Entrar',exact=True).click()
    expect(second.get_by_role('heading',name='Conversaciones',exact=True)).to_be_visible()
    def nav(name):
        
        if page.viewport_size['width']<768:page.get_by_role('button',name='Abrir navegación',exact=True).click()
        page.get_by_role('link',name=name,exact=True).click();expect(page.get_by_role('heading',name=name,exact=True)).to_be_visible()
    nav('Sesiones')
    expect(page.get_by_role('button',name='Sesión actual',exact=True)).to_be_visible()
    expect(page.get_by_role('button',name='Revocar sesión',exact=True).first).to_be_visible()
    page.get_by_role('button',name='Revocar sesión',exact=True).first.click()
    page.get_by_role('button',name='Confirmar',exact=True).click()
    expect(page.get_by_role('heading',name='¿Revocar esta sesión?')).to_have_count(0)
    assert second.evaluate("async()=> (await fetch('/local/auth/session')).status")==401
    assert page.evaluate("async()=> (await fetch('/local/auth/session')).status")==200
    page.screenshot(path=str(qa/'sessions-desktop.png'),full_page=True)
    second.close()
    nav('Usuarios')
    existing_family=page.evaluate("async()=>{const r=await fetch('/v1/local/users');return (await r.json()).find(u=>u.username.startsWith('qa_family_')&&u.role==='viewer'&&!u.disabled)||null}")
    if existing_family:
        family_username=existing_family['username'];family_name=existing_family['display_name']
    else:
        family_username='qa_family_'+str(int(time.time()))
        family_name='Familia QA '+str(int(time.time()))
        page.get_by_role('button',name='Agregar usuario',exact=True).click()
        page.get_by_label('Nombre',exact=True).fill(family_name)
        page.get_by_label('Usuario',exact=True).fill(family_username)
        page.get_by_label('Contraseña inicial',exact=True).fill('Local-QA-password-2026!')
        page.locator('select[name="role"]').select_option('viewer')
        page.get_by_role('button',name='Crear usuario',exact=True).click()
        expect(page.get_by_text(family_username,exact=True)).to_be_visible(timeout=10000)
    nav('Archivos');page.get_by_label('Nombre del producto o pieza').fill(fixture_label);page.get_by_label('Imagen o video · hasta 10 MB').set_input_files(str(fixture));page.get_by_role('button',name='Guardar archivo').click();expect(page.get_by_role('heading',name=fixture_label)).to_be_visible(timeout=15000)
    nav('Conversaciones');page.get_by_role('button',name='Nueva conversación',exact=True).click();page.get_by_label('Adjuntar archivo').select_option(label=fixture_label);page.get_by_label('Tu solicitud').fill('Crea un video de este producto');page.get_by_role('button',name='Enviar',exact=True).click();expect(page.get_by_role('button',name='Revisar trabajo',exact=False).first).to_be_visible(timeout=20000)
    expect(page.get_by_text('Respuesta simulada · datos ficticios',exact=True).first).to_be_visible()
    page.screenshot(path=str(qa/'chat-desktop.png'),full_page=True)
    page.get_by_role('button',name='Revisar trabajo',exact=False).first.click();expect(page.get_by_text('Revisión del trabajo',exact=True)).to_be_visible();page.get_by_role('checkbox').check();page.get_by_role('button',name='Aprobar esta versión').click();expect(page.locator('.result-preview video')).to_be_visible(timeout=30000)
    page.screenshot(path=str(qa/'job-desktop.png'),full_page=True)
    nav('Conversaciones');page.get_by_label('Tu solicitud').fill('Prepara para Instagram | Cacao ficticio de demostración');page.get_by_role('button',name='Enviar',exact=True).click();expect(page.get_by_role('button',name='Revisar publicación',exact=False).first).to_be_visible(timeout=20000);page.get_by_role('button',name='Revisar publicación',exact=False).first.click();expect(page.get_by_text('Instagram · versión 1',exact=False)).to_be_visible();page.get_by_role('checkbox').check();page.get_by_role('button',name='Aprobar esta versión').click();expect(page.get_by_text('Comprobante local:',exact=False)).to_be_visible(timeout=15000)
    page.screenshot(path=str(qa/'publication-desktop.png'),full_page=True)
    for name in ['Usuarios','Conexiones','Integraciones','Operación','Organizaciones']:
        nav(name);page.wait_for_timeout(450);assert page.locator('[data-slot=alert]').count()==0,(name,page.locator('[data-slot=alert]').all_text_contents())
    nav('Vínculos académicos')
    expect(page.get_by_text('Estudiante ficticio A',exact=True).first).to_be_visible()
    expect(page.get_by_text('Estudiante ficticio B',exact=True).first).to_be_visible()
    expect(page.get_by_text('Datos ficticios',exact=True)).to_be_visible()
    expect(page.get_by_text('no se envían mensajes',exact=False)).to_be_visible()
    page.get_by_label('Cuenta familiar',exact=True).select_option(label=family_name+' · '+family_username)
    page.get_by_label('Confirmar validación manual del vínculo').check()
    page.get_by_role('button',name='Guardar vínculo de prueba',exact=True).click()
    expect(page.get_by_text(family_name,exact=True)).to_be_visible(timeout=10000)
    family_password='Local-QA-password-2026!'
    page.evaluate("async ({name,password})=>{const s=await fetch('/local/auth/session').then(r=>r.json());const u=await fetch('/v1/local/users').then(r=>r.json());const f=u.find(x=>x.username===name);const r=await fetch('/v1/local/users/'+f.id+'/password',{method:'POST',credentials:'same-origin',headers:{'content-type':'application/json','x-csrf-token':s.csrf_token},body:JSON.stringify({password})});if(!r.ok)throw new Error('family password reset failed')} ",{'name':family_username,'password':family_password})
    nav('Accesos a recursos')
    page.get_by_label('Cuenta lectora',exact=True).select_option(label=family_name+' · '+family_username)
    page.get_by_label('Recurso',exact=True).select_option('demo-resource-001')
    page.get_by_role('checkbox').check()
    page.get_by_role('button',name='Conceder permiso',exact=True).click()
    expect(page.get_by_text(family_name+' · Recurso de demostración A',exact=True)).to_be_visible(timeout=10000)
    page.screenshot(path=str(qa/'resource-access-desktop.png'),full_page=True)
    page.get_by_role('button',name='Cerrar sesión',exact=True).click()
    page.get_by_label('Organización',exact=True).fill(credentials['organization']);page.get_by_label('Usuario',exact=True).fill(family_username);page.get_by_label('Contraseña',exact=True).fill(family_password);page.get_by_role('button',name='Entrar',exact=True).click()
    nav('Sesiones')
    expect(page.get_by_role('button',name='Sesión actual',exact=True)).to_be_visible()
    assert page.get_by_role('button',name='Revocar sesión',exact=True).count()==0
    nav('Accesos a recursos')
    page.get_by_role('button',name='Leer demostración',exact=True).click()
    expect(page.get_by_text('Simulado',exact=True)).to_be_visible(timeout=10000)
    expect(page.get_by_text('Resumen ficticio para probar permisos por recurso.',exact=True)).to_be_visible()
    page.get_by_role('button',name='Cerrar sesión',exact=True).click()
    page.get_by_label('Organización',exact=True).fill(credentials['organization']);page.get_by_label('Usuario',exact=True).fill(credentials['username']);page.get_by_label('Contraseña',exact=True).fill(credentials['password']);page.get_by_role('button',name='Entrar',exact=True).click()
    nav('Accesos a recursos');page.get_by_role('button',name='Revocar',exact=True).click()
    expect(page.get_by_text('Aún no hay permisos concedidos.',exact=True)).to_be_visible(timeout=10000)
    nav('Operación');page.screenshot(path=str(qa/'operations-desktop.png'),full_page=True)
    page.set_viewport_size({'width':390,'height':844})
    for name,filename in [('Conversaciones','chat-mobile.png'),('Publicaciones','publication-mobile.png'),('Operación','operations-mobile.png'),('Vínculos académicos','academic-links-mobile.png'),('Accesos a recursos','resource-access-mobile.png'),('Sesiones','sessions-mobile.png')]:
        nav(name);page.wait_for_timeout(400);page.screenshot(path=str(qa/filename),full_page=True)
        assert page.evaluate('document.documentElement.scrollWidth <= innerWidth+1'),f'horizontal overflow {name}'
    assert not errors,errors
    browser.close()
print(json.dumps({'status':'passed','browser':'Chromium','checks':['login','session inventory and owner revocation of another session','current session retained and viewer sees own session only','validated upload','conversation','creative approval','MP4 preview','publication approval and simulated receipt','local owner grant and revocation','session-bound viewer resource read with simulated label','all administrative views','desktop and mobile without page overflow','no JavaScript errors'],'screenshots':str(qa)},ensure_ascii=False,indent=2))
