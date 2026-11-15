#!/usr/bin/env python3
"""Explicit isolated staging launcher; never starts the local workspace or exposes its UI."""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[1]
config = {}
for line in (root / '.env').read_text().splitlines():
    if '=' in line and not line.lstrip().startswith('#'):
        key, value = line.split('=', 1)
        config[key.strip()] = value.strip().strip('\"').strip("'")
required = ('META_APP_SECRET', 'META_ACCESS_TOKEN', 'META_PHONE_NUMBER_ID', 'META_VERIFY_TOKEN', 'META_TEST_RECIPIENT', 'META_GRAPH_VERSION')
if any(not config.get(key) for key in required):
    raise SystemExit('Faltan variables META requeridas en .env; no se muestran sus valores.')
os.umask(0o077)
state = root / '.local-data' / 'whatsapp-staging'
state.mkdir(parents=True, exist_ok=True, mode=0o700)
env = {key: value for key, value in os.environ.items() if not key.startswith(('KUSI_', 'META_'))}
env.update({key: config[key] for key in required})
if config.get('GROQ_API_KEY'):
    env['GROQ_API_KEY'] = config['GROQ_API_KEY']
env['KUSI_WHATSAPP_STAGING_ACK'] = 'real-replies-to-test-number'
env['KUSI_WHATSAPP_STAGING_DATA'] = str(state)
subprocess.run(['cargo', 'run', '--bin', 'whatsapp_demo'], cwd=root, env=env, check=True)
