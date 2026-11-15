#!/usr/bin/env python3
"""Bounded live-planner acceptance against synthetic mutable environments, never business data.
Runs 26 query objectives plus subscriptions/cancellation and up to 6 clarification replies (max 35 model calls).
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import uuid

ROOT=Path(__file__).resolve().parents[1]


def write(path,value):
    fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_TRUNC,0o600)
    with os.fdopen(fd,'w') as stream:json.dump(value,stream)


def dataset(id,label,category,date,n,options):
    fields=[{'id':category,'label':'Modalidad' if category!='zone' else 'Zona','kind':'category','options':options},{'id':date,'label':'Fecha de creación','kind':'date','options':[]},{'id':'amount','label':'Importe' if id!='rsv' else 'Unidades','kind':'number','options':[]}]
    today=datetime.date.today()
    rows=[{category:options[i%2],date:str(today-datetime.timedelta(days=8-i//4)),'amount':10*(i+1) if id!='rsv' else i+1} for i in range(n)]
    return {'id':id,'label':label,'fields':fields,'events':['created']},rows


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--allow-provider',action='store_true',required=True)
    parser.add_argument('--result',type=Path,required=True)
    parser.add_argument('--tail-only',action='store_true',help='Run only the final projection objective and subscription lifecycle; never claims full-matrix coverage.')
    args=parser.parse_args()
    environment={k:v for k,v in os.environ.items() if not k.startswith(('KUSI_','GROQ_','META_'))}
    for line in (ROOT/'.env').read_text().splitlines():
        if line.startswith('GROQ_API_KEY='):environment['GROQ_API_KEY']=line.split('=',1)[1].strip().strip('\"').strip("'")
    environment['KUSI_ADAPTIVE_STAGING_ACK']='authorized-environment'
    live=json.loads((ROOT/'.local-data/adaptive/runtime.json').read_text())
    results=[];calls=0
    with tempfile.TemporaryDirectory(prefix='kusi-adaptive-acceptance-') as temporary:
        root=Path(temporary)
        a,arows=dataset('att','Asistencias','category','occurred',32,['General','Premium'])
        b,brows=dataset('ord','Pedidos','tier','booked',12,['Basic','Plus'])
        c,crows=dataset('rsv','Reservas','zone','opened',5,['North','South'])
        # Ensure the short second series has four complete days and no future rows.
        today=datetime.date.today()
        for i,row in enumerate(brows):row['booked']=str(today-datetime.timedelta(days=4-i//3))
        alpha={'catalog':{'source':'alpha','version':'ignored','resources':[a]},'rows':{'att':arows},'enabled':True}
        beta={'catalog':{'source':'beta','version':'ignored','resources':[b]},'rows':{'ord':brows},'enabled':True}
        write(root/'alpha.json',alpha);write(root/'beta.json',beta)
        config={**live,'organization':str(uuid.uuid4()),'principal':'synthetic-acceptance','state_directory':str(root/'state'),'sources':[{'id':'alpha','label':'Centro A','kind':'manifest','config_file':str(root/'alpha.json')},{'id':'beta','label':'Centro B','kind':'manifest','config_file':str(root/'beta.json')}]}
        write(root/'runtime.json',config)

        def run(payload,model=True):
            nonlocal calls
            if model:
                if calls:time.sleep(20)
                calls+=1
                assert calls<=35,'provider budget exceeded'
            if 'message' in payload:payload['request_id']=uuid.uuid4().hex
            process=subprocess.run([str(ROOT/'target/debug/adaptive_assistant'),str(root/'runtime.json')],input=json.dumps(payload),text=True,capture_output=True,env=environment,timeout=95)
            assert process.returncode==0,'assistant crashed; private output withheld'
            return json.loads(process.stdout)

        def check(label,text,op,value=None,field=None,groups=None):
            if args.tail_only and label!='H6':return
            reply=run({'message':text})
            if reply['status']=='clarification':
                clarifications={'count':'Quiero contar registros.','group':'Quiero contar registros por cada categoría.','sum':'Quiero sumar los valores numéricos.','mean':'Quiero la media aritmética de los valores numéricos.','forecast':'Quiero el total acumulado en esa fecha, proyectado a partir del ritmo de todo el histórico disponible, sin filtros.'}
                assert op in clarifications,(label,'unexpected clarification',reply['text'])
                results.append({'id':label+'.clarification','status':'clarified'})
                reply=run({'message':clarifications[op]})
            assert reply['status']=='completed',(label,reply['status'],reply.get('text'))
            if op=='reset':
                assert not reply['evidence'];results.append({'id':label,'status':'passed'});return
            items=[result for e in reply['evidence'] for result in e['results'] if result['op']==op and (field is None or result.get('field')==field)]
            assert items,(label,'missing analysis',op,reply)
            if value is not None:
                key='central' if op=='forecast' else 'difference' if op=='compare' else 'value'
                assert abs(items[0][key]-value)<1e-7,(label,items[0],value)
            if groups is not None:assert items[0]['groups']==groups,(label,items[0])
            results.append({'id':label,'status':'passed','resources':[e['resource'] for e in reply['evidence']]})
            print(json.dumps({'case':label,'status':'passed'}),flush=True)

        target=str(today+datetime.timedelta(days=46))
        check('C1.1','¿Cuántos registros de Asistencias tenemos en total?','count',32)
        check('C1.2','¿Y solo los Premium?','count',16)
        check('C1.3',f'Proyecta esos Premium al {target}, usando todo el histórico sin limitar fechas.','forecast',108)
        check('C2.1','Ahora resume todos los Pedidos, sin filtros.','count',12)
        check('C2.2','¿Y solo la modalidad Plus?','count',6)
        check('C2.3','Suma sus importes, manteniendo ese filtro.','sum',420,'amount')
        check('C3.1','Retomemos todas las Asistencias sin filtros. Cuenta sus registros.','count',32)
        check('C3.2','Agrúpalas por modalidad, sin filtros.','group',groups={'General':16,'Premium':16})
        start=today-datetime.timedelta(days=4);end=today-datetime.timedelta(days=1)
        prior=today-datetime.timedelta(days=8);prior_end=today-datetime.timedelta(days=5)
        check('C3.3',f'Compara cuántas Asistencias hubo entre {start} y {end} frente a {prior} y {prior_end}, ambos inclusive y sin filtros.','compare',0)
        check('C4.1','Cuenta los registros de todos los Pedidos, sin filtros.','count',12)
        check('C4.2','Ahora solo Basic.','count',6)
        check('C4.3','¿Cuál es su importe medio, manteniendo Basic?','mean',60,'amount')
        alpha['catalog']['resources'].append(c);alpha['rows']['rsv']=crows;write(root/'alpha.json',alpha)
        check('C5.1','Hay un recurso nuevo: Reservas. ¿Cuántas hay en total?','count',5)
        check('C5.2','¿Y solo en zona North?','count',3)
        check('C5.3','Suma sus unidades, manteniendo North.','sum',9,'amount')
        check('C6.1','Cuenta todas las Asistencias, sin filtros.','count',32)
        check('C6.2','Ahora solo General.','count',16)
        check('C6.3','Reinicia el contexto de nuestra conversación.','reset')
        check('Q19','Suma los importes de absolutamente todos los Pedidos, sin filtros.','sum',780,'amount')
        check('Q20','Obtén el promedio de importes de todas las Asistencias, sin filtros.','mean',165,'amount')
        check('H1','Indícame el número de elementos que reúne Asistencias sin recortes.','count',32)
        check('H2','Reparte todos los Pedidos entre sus modalidades y muestra cuántos hay en cada una.','group',groups={'Basic':6,'Plus':6})
        check('H3','Agrega los valores del importe de todo el conjunto de Pedidos.','sum',780,'amount')
        check('H4','Dime la media aritmética del importe de todo el conjunto de Asistencias, sin restricciones.','mean',165,'amount')
        check('H5','En Reservas, ¿qué cantidad de registros pertenece a North?','count',3)
        check('H6',f'Lleva el ritmo histórico total de Asistencias hasta {target} y estima el número acumulado, usando todas las fechas y categorías.','forecast',216)
        for resource in ['Asistencias','Pedidos']:
            r=run({'message':f'Avísame cuando se cree un registro nuevo en {resource}, sin filtros.'})
            assert r['status']=='completed',r
        state_dir=next((root/'state').iterdir())
        subscriptions=[json.loads(p.read_text())['data'] for p in state_dir.glob('subscription-*.json')]
        assert len(subscriptions)==2 and all(s['status']=='active' for s in subscriptions)
        event_results=[]
        for sub in subscriptions:
            event={'resource':sub['resource'],'kind':'created','event_id':'synthetic-event-1','values':{},'test':True}
            assert run({'event':event},False)['status']=='notification'
            for _ in range(2):assert run({'event':event},False)['status']=='ignored'
            event_results.append({'resource':sub['resource'],'deduplicated':True})
        r=run({'message':'Cancela el aviso de nuevos registros de Asistencias; conserva el de Pedidos.'})
        assert r['status']=='completed',r
        cancelled=[json.loads(p.read_text())['data'] for p in state_dir.glob('subscription-*.json')]
        assert any(s['status']=='cancelled' and s['resource']=='alpha::att' for s in cancelled)
        assert any(s['status']=='active' and s['resource']=='beta::ord' for s in cancelled)
        assert run({'event':{'resource':'alpha::att','kind':'created','event_id':'after-cancel','values':{}}},False)['status']=='ignored'
        alpha['enabled']=False;write(root/'alpha.json',alpha)
        environment.pop('GROQ_API_KEY',None)
        blocked=run({'event':{'resource':'alpha::att','kind':'created','event_id':'after-revoke','values':{}}},False)
        assert blocked['status']=='blocked',blocked
        audits=[json.loads(p.read_text())['data'] for p in state_dir.glob('audit-*.json')]
        assert len(audits)>=(4 if args.tail_only else 29),len(audits)
        report={'status':'passed','stage':'tail' if args.tail_only else 'full','query_objectives':1 if args.tail_only else 26,'query_turns':sum(1 for r in results if r['status']=='clarified')+(1 if args.tail_only else 26),'conversations_three_turns':0 if args.tail_only else 6,'provider_calls':calls,'resources':3,'environments':2,'third_resource_added_during_run':not args.tail_only,'checks':results,'events':event_results,'audits_recorded':len(audits),'cancellation_verified':True,'revocation_verified':True,'messages_sent':0,'synthetic_data':True,'planner':'real configured Groq profile'}
        write(args.result,report)
        print(json.dumps({k:v for k,v in report.items() if k not in ('checks','events')},indent=2))


if __name__=='__main__':main()
