"""Protocol dispatch for the existing adaptive runtime; no business/resource dispatch."""
import json
from pathlib import Path
import subprocess
import sys
from adaptive_graphql_bridge import private_json

def main():
    try:
        path=Path(sys.argv[1]);config=private_json(path)
        protocol=config.get('protocol')
        if protocol is None and {'user','host','container','site','key_file'} <= set(config):protocol='frappe_ssh'
        scripts={'graphql_loopback':'adaptive_graphql_bridge.py','frappe_ssh':'adaptive_frappe_bridge.py'}
        if protocol not in scripts:raise ValueError('unregistered protocol')
        body=sys.stdin.buffer.read(65537)
        if len(body)>65536:raise ValueError('request limit')
        result=subprocess.run([sys.executable,str(Path(__file__).with_name(scripts[protocol])),str(path)],input=body,capture_output=True,timeout=32)
        if result.returncode or len(result.stdout)>2*1024*1024:raise ValueError('bridge failed')
        print(json.dumps(json.loads(result.stdout)))
    except Exception:
        print(json.dumps({'error':'unavailable'}))
if __name__=='__main__':main()
