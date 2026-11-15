"""Platform metadata policy with a fake Frappe API, no SSH or external calls."""
import contextlib
import io
import json
from pathlib import Path
import tempfile
from types import ModuleType, SimpleNamespace as Obj
import unittest
from unittest.mock import Mock, patch
import adaptive_frappe_bridge as bridge


def field(name,kind,hidden=False,options=''):
    return Obj(fieldname=name,label=name,fieldtype=kind,hidden=hidden,options=options)


class MetadataPolicy(unittest.TestCase):
    def rpc(self,request,visible=True):
        frappe=ModuleType('frappe')
        frappe.init=Mock();frappe.connect=Mock();frappe.destroy=Mock()
        frappe.db=Obj(get_value=Mock(return_value=True),rollback=Mock())
        frappe.get_doc=Mock(return_value=Obj(roles=[Obj(role='Scoped'),Obj(role='All')]))
        frappe.get_all=Mock(side_effect=lambda kind,**kw: ['Unfamiliar resource'] if kind in ('DocPerm','Custom DocPerm') else [])
        frappe.set_user=Mock()
        frappe.has_permission=Mock(return_value=visible)
        fields=[field('classification','Select',options='Red\nBlue'),field('units','Int'),field('customer','Data'),field('password','Password'),field('attachment','Attach'),field('hidden','Int',True),field('restricted','Int')]
        frappe.get_meta=Mock(return_value=Obj(issingle=False,istable=False,get=lambda key:False,fields=fields))
        frappe.as_json=json.dumps
        frappe.get_list=Mock(return_value=[])
        model=ModuleType('frappe.model')
        model.get_permitted_fields=Mock(return_value=['classification','units','customer','password','attachment','hidden'])
        password=ModuleType('frappe.utils.password')
        password.check_password=Mock(return_value='fixture')
        modules={'frappe':frappe,'frappe.model':model,'frappe.utils.password':password}
        output=io.StringIO()
        with patch.dict('sys.modules',modules), patch('os.chdir'), contextlib.redirect_stdout(output):
            try:exec(bridge.REMOTE,{'CFG':{'site':'fixture.invalid','id':'source','allow_watch':False},'CREDENTIALS':{'username':'fixture','password':'synthetic'},'REQUEST':request,'SECRET':''})
            except SystemExit as error:
                if error.code:raise
        return json.loads(output.getvalue()),frappe

    def test_unknown_schema_excludes_private_hidden_and_unpermitted_fields(self):
        catalog,frappe=self.rpc({'op':'discover'})
        self.assertEqual({f['id'] for f in catalog['resources'][0]['fields']},{'creation','modified','classification','units'})
        self.assertEqual(catalog['resources'][0]['events'],[])
        frappe.get_list.assert_not_called()
        frappe.set_user.assert_called_with('fixture')

    def test_revoked_resource_disappears_without_reading_rows(self):
        catalog,frappe=self.rpc({'op':'discover'},visible=False)
        self.assertEqual(catalog['resources'],[])
        frappe.get_list.assert_not_called()

    def test_unpermitted_columns_and_stale_catalogue_never_read(self):
        catalog,_=self.rpc({'op':'discover'})
        for version,columns in [(catalog['version'],['password']),('stale',['units'])]:
            result,frappe=self.rpc({'op':'read','version':version,'resource':'Unfamiliar resource','fields':columns,'filters':[],'limit':10})
            self.assertIn(result['error'],('forbidden','stale'))
            frappe.get_list.assert_not_called()

    def test_credentials_require_private_regular_file(self):
        with tempfile.TemporaryDirectory() as temporary:
            path=Path(temporary)/'private.json'
            path.write_text('{"fixture":true}')
            path.chmod(0o600)
            self.assertEqual(bridge.private_json(path),{'fixture':True})
            link=Path(temporary)/'link.json'
            link.symlink_to(path)
            with self.assertRaises(OSError):bridge.private_json(link)
            path.chmod(0o644)
            with self.assertRaises(ValueError):bridge.private_json(path)


if __name__=='__main__':unittest.main()
