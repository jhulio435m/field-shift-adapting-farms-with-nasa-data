"""Preserve Kusi's existing source protocols while adding Field Shift locally."""

import json
from pathlib import Path
import stat
import subprocess
import sys

from app.kusi_bridge import main as field_shift_main

KUSI_BRIDGE = Path('/home/blink/kusi-kusi/scripts/adaptive_source_bridge.py')


def main() -> None:
    try:
        if len(sys.argv) != 2:
            raise ValueError
        path = Path(sys.argv[1])
        info = path.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_mode & 0o077 or info.st_size > 65536:
            raise ValueError
        config = json.loads(path.read_text())
        protocol = config.get('protocol')
        if protocol == 'field_shift_loopback':
            field_shift_main()
            return
        if protocol is None and {'user', 'host', 'container', 'site', 'key_file'} <= set(config):
            protocol = 'frappe_ssh'
        if protocol not in ('graphql_loopback', 'frappe_ssh'):
            raise ValueError
        body = sys.stdin.buffer.read(65537)
        if len(body) > 65536:
            raise ValueError
        result = subprocess.run(
            [sys.executable, str(KUSI_BRIDGE), str(path)], input=body,
            capture_output=True, timeout=32, check=True,
        )
        if len(result.stdout) > 2 * 1024 * 1024:
            raise ValueError
        response = json.loads(result.stdout)
    except Exception:
        response = {'error': 'unavailable'}
    print(json.dumps(response, ensure_ascii=False))


if __name__ == '__main__':
    main()
