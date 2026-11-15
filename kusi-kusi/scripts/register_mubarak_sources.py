"""Enroll seller-scoped connections into the EXISTING Kusi runtime; no provider calls.

Uses dedicated local Vendure accounts. Keeps planner, actor, channels and all
previous sources. Existing runtime is backed up before the additive change.
"""
import argparse
import datetime
import json
import shutil
from pathlib import Path
from adaptive_graphql_bridge import Bridge

ROOT=Path(__file__).resolve().parent.parent

def write(path,data):
    path.parent.mkdir(parents=True,exist_ok=True,mode=0o700)
    path.write_text(json.dumps(data,ensure_ascii=False,indent=2));path.chmod(0o600)

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--runtime',type=Path,default=ROOT/'.local-data/adaptive/runtime.json');args=parser.parse_args()
    from adaptive_graphql_bridge import private_json
    runtime=private_json(args.runtime)
    if runtime.get('access') or runtime.get('managed_fixture'):raise RuntimeError('This runtime has resource ACLs; enroll through its existing authorization flow instead')
    accounts=private_json(ROOT.parent/'Projects/mubarak-vendure/apps/mubarak/var/kusi-accounts.json')
    folder=ROOT/'.local-data/adaptive/mubarak';sources=[];summary=[]
    for account in accounts:
        credential=folder/(account['slug']+'-credential.json');write(credential,{'username':account['username'],'password':account['password']})
        config={'protocol':'graphql_loopback','enabled':True,'label':'Mubarak / '+account['name'],'endpoint':'http://127.0.0.1:3100/admin-api','channel_token':account['channelToken'],'credential_file':str(credential),'timezone':'America/Lima','resources':['products','productVariants','orders'],'categorical_fields':{'orders':['state']}}
        path=folder/(account['slug']+'-source.json');write(path,config)
        bridge=Bridge(config)
        try:catalog=bridge.discover()
        finally:bridge.close()
        if not catalog['resources']:raise RuntimeError('Connection has no authorized resources')
        sources.append({'id':'mubarak_'+account['slug'].replace('-','_'),'label':config['label'],'kind':'graphql_loopback','config_file':str(path)})
        summary.append({'seller':account['slug'],'resources':[r['id'] for r in catalog['resources']],'permissions':'read-only seller channel'})
    backup=ROOT/'.local-backups'/('adaptive-before-mubarak-'+datetime.datetime.now().strftime('%Y%m%d-%H%M%S')+'.json')
    backup.parent.mkdir(mode=0o700,parents=True,exist_ok=True);shutil.copyfile(args.runtime,backup);backup.chmod(0o600)
    ids={s['id'] for s in sources}
    # Preserve every pre-existing non-Mubarak connection and the current model/identity.
    runtime['sources']=[s for s in runtime['sources'] if s['id'] not in ids]+sources
    runtime['bridge_script']=str(ROOT/'scripts/adaptive_source_bridge.py')
    write(args.runtime,runtime)
    write(folder/'connections.json',{'status':'registered','runtime':str(args.runtime),'model_unchanged':True,'channels_unchanged':True,'connections':summary})
    print('Two seller-scoped sources added to the existing Kusi runtime. Model, identity, channels and previous sources preserved; no provider sends.')
if __name__=='__main__':main()
