import contextlib
import io
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import adaptive_source_bridge as router

class DispatchTests(unittest.TestCase):
    def test_existing_frappe_and_new_graphql_protocols_keep_separate_bridges(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'source.json'
            for config,script in [({'user':'u','host':'h','container':'c','site':'s','key_file':'/tmp/key'},'adaptive_frappe_bridge.py'),({'protocol':'graphql_loopback'},'adaptive_graphql_bridge.py')]:
                path.write_text(json.dumps(config));path.chmod(0o600)
                stdin=SimpleNamespace(buffer=io.BytesIO(b'{"op":"discover"}'))
                with patch.object(router.sys,'argv',['bridge',str(path)]),patch.object(router.sys,'stdin',stdin),patch.object(router.subprocess,'run',return_value=SimpleNamespace(returncode=0,stdout=b'{}')) as call,contextlib.redirect_stdout(io.StringIO()) as output:
                    router.main()
                self.assertEqual(Path(call.call_args.args[0][1]).name,script)
                self.assertEqual(json.loads(output.getvalue()),{})
            path.write_text('{"protocol":"unknown"}')
            with patch.object(router.sys,'argv',['bridge',str(path)]),patch.object(router.subprocess,'run') as call,contextlib.redirect_stdout(io.StringIO()) as output:
                router.main()
            call.assert_not_called();self.assertEqual(json.loads(output.getvalue()),{'error':'unavailable'})
if __name__=='__main__':unittest.main()
