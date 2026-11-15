"""Protocol/security checks; no paid providers or message delivery."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from adaptive_graphql_bridge import Bridge, Denied, Stale, private_json

class ProtocolTests(unittest.TestCase):
    def test_private_files_and_loopback_only(self):
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)/'private.json';path.write_text('{}');path.chmod(0o644)
            with self.assertRaises(Denied):private_json(path)
            path.chmod(0o600);self.assertEqual(private_json(path),{})
            link=Path(temp)/'link.json';link.symlink_to(path)
            with self.assertRaises(Denied):private_json(link)
        for url in ['https://example.com/graphql','http://localhost:3100/graphql','http://169.254.169.254/graphql','http://127.0.0.1:3100/graphql?url=x','http://x@y/graphql']:
            with self.assertRaises(Denied):Bridge({'endpoint':url})

    def bridge(self):
        bridge=object.__new__(Bridge)
        bridge.shapes={'rows':{'root':'rows'}}
        fields=[{'id':'amount','kind':'number','options':[]},{'id':'status','kind':'category','options':['open','closed']}]
        catalog={'version':'v1','resources':[{'id':'rows','fields':fields}]}
        bridge.discover=lambda:catalog
        bridge.rows=lambda _:[{'amount':10,'status':'open'},{'amount':20,'status':'closed'}]
        return bridge

    def test_only_declared_fields_and_typed_filters_can_be_read(self):
        bridge=self.bridge()
        request={'op':'read','version':'v1','resource':'rows','fields':['amount'],'filters':[{'field':'status','op':'eq','value':'open'}],'limit':10}
        self.assertEqual(bridge.execute(request),{'rows':[{'amount':10}],'truncated':False})
        for change in [{'fields':['password']},{'resource':'other'},{'filters':[{'field':'amount','op':'eq','value':'10'}]},{'filters':[{'field':'status','op':'eq','value':'ungranted'}]},{'op':'write'}]:
            with self.assertRaises(Denied):bridge.execute({**request,**change})
        with self.assertRaises(Stale):bridge.execute({**request,'version':'old'})
        self.assertEqual(bridge.execute({**request,'filters':[],'limit':1})['truncated'],True)

if __name__=='__main__':unittest.main()
