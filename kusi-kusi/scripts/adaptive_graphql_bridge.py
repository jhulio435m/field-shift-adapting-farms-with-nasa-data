"""Generic local GraphQL discovery/read bridge; no business names, intents or model SQL.

Private configuration delegates paginated root queries, not canned questions.
Only numeric/date/enum/boolean fields are discovered automatically; explicitly
allowed strings become bounded categories. All business mutations are refused.
"""
import datetime
import hashlib
import json
import math
from pathlib import Path
import re
import stat
import sys
import time
import urllib.request
import urllib.parse
from zoneinfo import ZoneInfo

SENSITIVE = re.compile(r'password|secret|token|credential|email|phone|address|firstname|lastname|fullname|username',re.I)
NAME = re.compile(r'^[A-Za-z_][A-Za-z0-9_]*$')
MAX_ROWS = 10000
REF = 'kind name ofType { kind name ofType { kind name ofType { kind name } } }'
SCHEMA_QUERY = '{__schema{queryType{name} types{kind name enumValues{name} fields{name description args{name defaultValue type{'+REF+'}} type{'+REF+'}} inputFields{name type{'+REF+'}}}}}'

class Denied(Exception): pass
class Incomplete(Exception): pass
class Stale(Exception): pass

def private_json(path):
    path = Path(path)
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_mode & 0o077 or info.st_size > 65536:
        raise Denied()
    return json.loads(path.read_text())

def named(ref):
    while ref and ref.get('ofType'):
        ref = ref['ofType']
    return ref.get('name') if ref else None

def required(ref): return ref.get('kind') == 'NON_NULL'
def list_ref(ref):
    while ref and ref.get('kind') == 'NON_NULL': ref = ref.get('ofType')
    return bool(ref and ref.get('kind') == 'LIST')

def digest(value):
    return hashlib.sha256(json.dumps(value,sort_keys=True,separators=(',',':')).encode()).hexdigest()

class Bridge:
    def __init__(self, config):
        self.config = config
        endpoint = urllib.parse.urlsplit(config['endpoint'])
        if endpoint.scheme != 'http' or endpoint.hostname != '127.0.0.1' or endpoint.username or endpoint.password or endpoint.query or endpoint.fragment or not endpoint.path.startswith('/'):
            raise Denied()
        if config.get('enabled') is not True or not config.get('resources') or len(config['resources']) > 16:
            raise Denied()
        if any(not NAME.fullmatch(x) for x in config['resources']): raise Denied()
        self.deadline = time.monotonic()+27
        self.opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
        self.token = None
        self.scope = config['channel_token']
        self.zone = ZoneInfo(config.get('timezone','UTC'))
        credentials = private_json(config['credential_file'])
        result = self.call('mutation($u:String!,$p:String!){login(username:$u,password:$p){__typename ... on CurrentUser{channels{token}}}}', {'u':credentials['username'],'p':credentials['password']})
        user = result.get('login',{})
        if user.get('__typename') != 'CurrentUser' or not self.token or self.scope not in [c['token'] for c in user.get('channels',[])]:
            self.close(); raise Denied()
        schema = self.call(SCHEMA_QUERY)['__schema']
        self.types = {t['name']:t for t in schema['types']}
        self.roots = {f['name']:f for f in self.types[schema['queryType']['name']]['fields']}
        self.shapes = {}
        self.datasets = {}

    def call(self, query, variables=None):
        remaining = self.deadline-time.monotonic()
        if remaining <= 0: raise TimeoutError()
        request = urllib.request.Request(self.config['endpoint'], data=json.dumps({'query':query,'variables':variables or {}}).encode(), headers={'Content-Type':'application/json','vendure-token':self.scope,**({'Authorization':'Bearer '+self.token} if self.token else {})})
        with self.opener.open(request,timeout=min(5,remaining)) as response:
            body = response.read(2*1024*1024+1)
            self.token = response.headers.get('vendure-auth-token') or self.token
        if len(body)>2*1024*1024: raise Incomplete()
        result = json.loads(body)
        if result.get('errors'): raise Denied()
        return result['data']

    def close(self):
        if self.token:
            try: self.call('mutation{logout{success}}')
            except Exception: pass
            self.token = None

    def describe(self, name):
        root = self.roots.get(name)
        if not root: raise Denied()
        # A reusable protocol shape, not a per-resource resolver.
        target = self.types.get(named(root['type']),{})
        fields = {f['name']:f for f in target.get('fields') or []}
        item = fields.get('items')
        if not item or not list_ref(item['type']) or 'totalItems' not in fields: raise Denied()
        args = root['args']
        options = next((a for a in args if {f['name'] for f in self.types.get(named(a['type']),{}).get('inputFields') or []} >= {'skip','take'}),None)
        if not options or any(required(a['type']) and a is not options and a.get('defaultValue') is None for a in args): raise Denied()
        row_type = self.types.get(named(item['type']),{})
        categorical = set(self.config.get('categorical_fields',{}).get(name,[]))
        discovered = []
        for field in row_type.get('fields') or []:
            if field['args'] or list_ref(field['type']) or SENSITIVE.search(field['name']): continue
            scalar = named(field['type'])
            kind, values = None, []
            if scalar in ('Int','Float','Money'): kind='number'
            elif scalar in ('Date','DateTime'): kind='date'
            elif scalar == 'Boolean': kind='category';values=[False,True]
            elif self.types.get(scalar,{}).get('kind') == 'ENUM':
                kind='category';values=[x['name'] for x in self.types[scalar]['enumValues']]
            elif scalar == 'String' and field['name'] in categorical: kind='category'
            if kind:
                label=field['name']
                if scalar=='Money': label+=' (unidades menores de la moneda; no equivale a cobrado)'
                if kind=='date': label+=' ('+str(self.zone)+')'
                discovered.append({'id':field['name'],'label':label,'kind':kind,'options':values})
        if not discovered or len(discovered)>16: raise Incomplete()
        shape={'root':name,'arg':options['name'],'input':named(options['type']),'fields':discovered}
        self.shapes[name]=shape
        return shape

    def rows(self, shape):
        name=shape['root']
        if name in self.datasets:return self.datasets[name]
        fields=[f['id'] for f in shape['fields']]
        row_type=self.types[named(next(f for f in self.types[named(self.roots[name]['type'])]['fields'] if f['name']=='items')['type'])]
        if any(f['name']=='id' for f in row_type['fields']): fields.append('id')
        query='query($o:'+shape['input']+'){'+name+'('+shape['arg']+':$o){totalItems items{'+ ' '.join(fields)+'}}}'
        rows=[];total=None;seen=set()
        option_fields={f['name']:f for f in self.types[shape['input']]['inputFields']}
        while True:
            opts={'skip':len(rows),'take':100}
            if 'sort' in option_fields:
                sort_type=self.types.get(named(option_fields['sort']['type']),{})
                if any(f['name']=='id' for f in sort_type.get('inputFields') or []):opts['sort']={'id':'ASC'}
            page=self.call(query,{'o':opts})[name]
            if type(page['totalItems']) is not int or page['totalItems']>MAX_ROWS: raise Incomplete()
            if total is None: total=page['totalItems']
            if page['totalItems']!=total:raise Stale()
            if not page['items'] and len(rows)<total:raise Incomplete()
            for row in page['items']:
                if 'id' in row:
                    if row['id'] in seen:raise Stale()
                    seen.add(row['id']);row.pop('id')
                for field in shape['fields']:
                    value=row.get(field['id'])
                    if field['kind']=='date' and value:
                        date=datetime.datetime.fromisoformat(value.replace('Z','+00:00'))
                        row[field['id']]=date.astimezone(self.zone).date().isoformat()
                    elif field['kind']=='number' and value is not None and (type(value) not in (int,float) or not math.isfinite(value)):raise Denied()
                rows.append(row)
            if len(rows)>=total:break
        if len(rows)!=total:raise Incomplete()
        self.datasets[name]=rows
        return rows

    def discover(self):
        resources=[]
        for name in self.config['resources']:
            try:
                shape=self.describe(name)
                # A permission check against the current account, not schema visibility.
                rows=self.rows(shape)
            except Denied: continue
            fields=shape['fields']
            for field in fields:
                if field['kind']=='category' and not field['options']:
                    values={r[field['id']] for r in rows if r.get(field['id']) is not None}
                    if len(values)>80 or any(not isinstance(x,str) or len(x)>120 for x in values):raise Incomplete()
                    field['options']=sorted(values)
            resources.append({'id':name,'label':name,'fields':fields,'events':[]})
        catalog={'source':self.config.get('label','GraphQL local'),'resources':resources}
        catalog['version']=digest({'catalog':catalog,'scope':self.scope})
        return catalog

    def execute(self, request):
        if request.get('op') not in ('discover','read'):raise Denied()
        expected={'op'} if request['op']=='discover' else {'op','version','resource','fields','filters','limit'}
        if set(request)!=expected:raise Denied()
        catalog=self.discover()
        if request['op']=='discover':return catalog
        if request['version']!=catalog['version']:raise Stale()
        resource=next((r for r in catalog['resources'] if r['id']==request['resource']),None)
        if not resource:raise Denied()
        allowed={f['id']:f for f in resource['fields']}
        selected=request['fields']
        if not selected or len(selected)>16 or any(x not in allowed for x in selected) or len(set(selected))!=len(selected):raise Denied()
        limit=request['limit']
        if type(limit) is not int or not 1<=limit<=MAX_ROWS or len(request['filters'])>8:raise Denied()
        filters=request['filters']
        for item in filters:
            if set(item)!={'field','op','value'} or item['field'] not in allowed or item['op'] not in ('eq','ge','le'):raise Denied()
            field=allowed[item['field']];value=item['value']
            if field['kind']=='category' and (item['op']!='eq' or value not in field['options']):raise Denied()
            if field['kind']=='number' and (type(value) not in (int,float) or not math.isfinite(value)):raise Denied()
            if field['kind']=='date':datetime.date.fromisoformat(value)
        def match(row):
            for item in filters:
                a,b=row.get(item['field']),item['value']
                if a is None:return False
                if item['op']=='eq' and a!=b:return False
                if item['op']=='ge' and a<b:return False
                if item['op']=='le' and a>b:return False
            return True
        rows=[{f:row[f] for f in selected} for row in self.rows(self.shapes[resource['id']]) if match(row)]
        return {'rows':rows[:limit],'truncated':len(rows)>limit}

class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self,*_args,**_kwargs): raise Denied()

def main():
    bridge=None
    try:
        request=json.loads(sys.stdin.buffer.read(65537))
        bridge=Bridge(private_json(sys.argv[1]))
        result=bridge.execute(request)
    except Stale: result={'error':'stale'}
    except Denied: result={'error':'forbidden'}
    except Exception: result={'error':'unavailable'}
    finally:
        if bridge:bridge.close()
    print(json.dumps(result,ensure_ascii=False))

if __name__=='__main__':main()
