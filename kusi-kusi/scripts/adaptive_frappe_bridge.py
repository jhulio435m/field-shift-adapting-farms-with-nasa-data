#!/usr/bin/env python3
"""Registered Frappe platform RPC. No business names, SQL, or executable user expressions."""
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys


def private_json(path):
    path=Path(path)
    fd=os.open(path,os.O_RDONLY|getattr(os,'O_NOFOLLOW',0))
    with os.fdopen(fd) as stream:
        info=os.fstat(stream.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_mode&0o077 or info.st_size>1048576:raise ValueError('private file required')
        return json.load(stream)


REMOTE=r'''
import os,json,hashlib
import frappe
os.chdir('/home/frappe/frappe-bench/sites')
frappe.init(site=CFG['site'],sites_path='.')
frappe.connect()
try:
 from frappe.utils.password import check_password
 username=CREDENTIALS['username']
 assert frappe.db.get_value('User',username,'enabled')
 assert check_password(username,CREDENTIALS['password'])==username
 roles=sorted(r.role for r in frappe.get_doc('User',username).roles if r.role not in ('All','Guest','Desk User'))
 candidates=set()
 for permission_type in ('DocPerm','Custom DocPerm'):
  candidates.update(frappe.get_all(permission_type,filters={'role':['in',roles],'read':1},pluck='parent'))
 user_permissions=frappe.get_all('User Permission',filters={'user':username},fields=['allow','for_value','applicable_for','apply_to_all_doctypes'],order_by='name asc')
 frappe.set_user(username)
 from frappe.model import get_permitted_fields
 resources=[]
 for name in sorted(candidates):
  if not frappe.has_permission(name,'read'):continue
  meta=frappe.get_meta(name)
  if meta.issingle or meta.istable or meta.get('is_virtual'):continue
  permitted=set(get_permitted_fields(name,user=username,permission_type='read'))
  fields=[]
  for field in meta.fields:
   if field.fieldname not in permitted or field.hidden or field.fieldtype not in ('Select','Check','Int','Float','Currency','Percent','Date','Datetime'):continue
   if field.fieldtype=='Select':
    options=[v for v in (field.options or '').split('\n') if v]
    if not options or len(options)>100 or any(len(v)>120 for v in options):continue
    kind='category'
   elif field.fieldtype=='Check':kind='category';options=[0,1]
   elif field.fieldtype in ('Date','Datetime'):kind='date';options=[]
   else:kind='number';options=[]
   fields.append({'id':field.fieldname,'label':field.label or field.fieldname,'kind':kind,'options':options})
  fields=[{'id':'creation','label':'Fecha de creación','kind':'date','options':[]},{'id':'modified','label':'Fecha de modificación','kind':'date','options':[]}]+fields
  if len(fields)>40:fields=fields[:40]
  resources.append({'id':name,'label':name,'fields':fields,'events':['created'] if CFG.get('allow_watch',False) else []})
 if len(resources)>80:raise ValueError('catalog limit')
 version=hashlib.sha256(frappe.as_json({'resources':resources,'roles':roles,'user_permissions':user_permissions}).encode()).hexdigest()
 catalog={'source':CFG['id'],'version':version,'resources':resources}
 if REQUEST['op']=='discover':result=catalog
 else:
  if REQUEST.get('version')!=version:
   print(json.dumps({'error':'stale'}));raise SystemExit(0)
  resource=next((r for r in resources if r['id']==REQUEST.get('resource')),None)
  if not resource:
   print(json.dumps({'error':'forbidden'}));raise SystemExit(0)
  allowed={f['id']:f for f in resource['fields']}
  if REQUEST['op']=='read':
   columns=REQUEST['fields'];limit=REQUEST['limit'];filters=[]
   if not isinstance(columns,list) or not 1<=len(columns)<=16 or any(f not in allowed for f in columns) or type(limit) is not int or not 1<=limit<=10000:raise ValueError('read limits')
   if len(REQUEST['filters'])>8:raise ValueError('filter limits')
   for value in REQUEST['filters']:
    if set(value)!={'field','op','value'} or value['field'] not in allowed:raise ValueError('filter field')
    field=allowed[value['field']];op=value['op'];data=value['value']
    if op not in ('eq','ge','le'):raise ValueError('filter op')
    if field['kind']=='category' and (op!='eq' or data not in field['options']):raise ValueError('category filter')
    if field['kind']=='number' and (type(data) not in (int,float)):raise ValueError('number filter')
    if field['kind']=='date':
     import datetime
     datetime.date.fromisoformat(data)
     # Calendar day bounds, including the complete end date for datetimes.
     if op=='eq':
      filters.extend([[value['field'],'>=',data+' 00:00:00'],[value['field'],'<=',data+' 23:59:59.999999']]);continue
     if op=='le':data+=' 23:59:59.999999'
     elif op=='ge':data+=' 00:00:00'
    filters.append([value['field'],{'eq':'=','ge':'>=','le':'<='}[op],data])
   rows=frappe.get_list(resource['id'],fields=columns,filters=filters,limit_page_length=limit+1,order_by='name asc')
   result={'rows':rows[:limit],'truncated':len(rows)>limit}
  elif REQUEST['op']=='authorize_event':
   ident=REQUEST['event_id']
   if not isinstance(ident,str) or not 1<=len(ident)<=200:raise ValueError('event identity')
   if REQUEST.get('test') is True and ident.startswith('KUSI-ADAPTIVE-TEST-') and CFG.get('allow_watch'):
    result={'authorized':True}
   else:result={'authorized':bool(frappe.get_list(resource['id'],filters={'name':ident},fields=['name'],limit_page_length=1))}
  elif REQUEST['op']=='watch':
   import re
   if not re.fullmatch(r'[a-f0-9]{64}',REQUEST.get('namespace','')):raise ValueError('watch namespace')
   if not CFG.get('allow_watch') or not CFG.get('callback_url') or not SECRET:raise ValueError('watch unavailable')
   # Delegate is explicit in the registered source config; source user's read scope was checked above.
   hook_name='Kusi Adaptive '+hashlib.sha256((REQUEST['namespace']+':'+CFG['id']+':'+resource['id']).encode()).hexdigest()[:24]
   pairs=[json.dumps(f['id'])+': {{ doc.get('+json.dumps(f['id'])+') | tojson }}' for f in resource['fields']]
   payload='{"resource":'+json.dumps(CFG['id']+'::'+resource['id'])+',"kind":"created","event_id":{{ doc.name | tojson }},"test":{% if doc.name.startswith("KUSI-ADAPTIVE-TEST-") %}true{% else %}false{% endif %},"values":{'+','.join(pairs)+'}}'
   frappe.set_user('Administrator')
   exists=frappe.db.exists('Webhook',hook_name)
   hook=frappe.get_doc('Webhook',hook_name) if exists else frappe.get_doc({'doctype':'Webhook','name':hook_name})
   if exists and hook.webhook_doctype!=resource['id']:raise ValueError('hook scope')
   hook.update({'webhook_doctype':resource['id'],'webhook_docevent':'after_insert','enabled':1,'request_url':CFG['callback_url'],'request_method':'POST','request_structure':'JSON','enable_security':1,'webhook_secret':SECRET,'timeout':30,'background_jobs_queue':'default','webhook_json':payload})
   hook.set('webhook_headers',[{'key':'Content-Type','value':'application/json'}])
   if exists:hook.save(ignore_permissions=True)
   else:hook.insert(ignore_permissions=True)
   frappe.db.commit()
   result={'watching':True}
  else:raise ValueError('operation')
 print(frappe.as_json(result))
except Exception:
 frappe.db.rollback();print(json.dumps({'error':'forbidden'}))
finally:frappe.destroy()
'''


def main():
    try:
        config=private_json(sys.argv[1]);credentials=private_json(config['credential_file'])
        request=json.loads(sys.stdin.buffer.read(65537))
        shapes={'discover':{'op'},'read':{'op','version','resource','fields','filters','limit'},'watch':{'op','version','resource','namespace'},'authorize_event':{'op','version','resource','event_id','test'}}
        if request.get('op') not in shapes or set(request)!=shapes[request['op']]:raise ValueError('request shape')
        for name in ('user','host','container','site','id'):
            if not re.fullmatch(r'[A-Za-z0-9_.-]{1,150}',config[name]) or config[name].startswith('-'):raise ValueError('source target')
        secret=''
        if request['op']=='watch':
            if not config.get('allow_watch'):raise ValueError('not delegated')
            from urllib.parse import urlsplit
            url=urlsplit(config.get('callback_url',''))
            if url.scheme!='https' or not url.hostname or url.username or url.password or url.query or url.fragment or url.port not in (None,443):raise ValueError('callback')
            secret=Path(config['event_secret_file']).read_text().strip()
        remote='CFG='+repr(config)+'\nCREDENTIALS='+repr(credentials)+'\nREQUEST='+repr(request)+'\nSECRET='+repr(secret)+'\n'+REMOTE
        driver='import subprocess,sys\np=subprocess.run(["docker","exec","-i",'+repr(config['container'])+',"env/bin/python","-"],input='+repr(remote)+',text=True,capture_output=True,timeout=27)\nif p.returncode:sys.exit(1)\nprint(p.stdout)\n'
        output=subprocess.run(['ssh','-F','/dev/null','-i',config['key_file'],'-o','BatchMode=yes','-o','ConnectTimeout=5','-o','StrictHostKeyChecking=yes',config['user']+'@'+config['host'],'python3 -'],input=driver,text=True,capture_output=True,timeout=32)
        if output.returncode or len(output.stdout)>2*1024*1024:raise ValueError('transport')
        print(json.dumps(json.loads(output.stdout)))
    except Exception:
        print(json.dumps({'error':'unavailable'}))


if __name__=='__main__':main()
