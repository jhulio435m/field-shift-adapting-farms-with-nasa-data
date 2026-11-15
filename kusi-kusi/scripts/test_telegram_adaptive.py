from contextlib import closing
import base64
import hashlib
import hmac
import json
import os
from pathlib import Path
import sqlite3
import tempfile
import unittest
from unittest.mock import patch
import telegram_adaptive as adaptive


class GenericEvents(unittest.TestCase):
    def test_local_mode_blocks_before_configuration_or_subprocess(self):
        with patch.dict(os.environ, {'KUSI_LOCAL_MODE':'true'}), patch.object(adaptive.subprocess,'run') as run:
            self.assertEqual(adaptive.invoke({'message':'synthetic'})['status'],'blocked')
            run.assert_not_called()

    def test_required_acl_and_selected_database_environment_reach_binary(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary)
            config=root/'runtime.json'
            config.write_text(json.dumps({'binding_hash':hashlib.sha256(b'fixture-chat').hexdigest(),'llm':{'api_key_env':None},'access':{'database_url_env':'KUSI_TEST_DB','resources':{}}}))
            from types import SimpleNamespace
            with patch.object(adaptive,'ROOT',root), patch.object(adaptive,'CONFIG',config), patch.object(adaptive,'bound_chat',return_value='fixture-chat'), patch.dict(os.environ,{'KUSI_LOCAL_MODE':'false','KUSI_TEST_DB':'synthetic-db','KUSI_OTHER_PRIVATE':'must-not-forward'},clear=True), patch.object(adaptive.subprocess,'run',return_value=SimpleNamespace(returncode=0,stdout='{"status":"completed"}')) as run:
                self.assertEqual(adaptive.invoke({'message':'synthetic'})['status'],'completed')
                env=run.call_args.kwargs['env']
                self.assertEqual(env['KUSI_ADAPTIVE_REQUIRE_ACL'],'true')
                self.assertEqual(env['KUSI_TEST_DB'],'synthetic-db')
                self.assertNotIn('KUSI_OTHER_PRIVATE',env)

    def test_bound_chat_closes_sqlite_connection(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);ledger=root/'.local-data/telegram-staging';ledger.mkdir(parents=True)
            with closing(sqlite3.connect(ledger/'ledger.sqlite')) as db, db:
                db.execute('CREATE TABLE binding(id INTEGER PRIMARY KEY,chat TEXT)')
                db.execute('INSERT INTO binding VALUES(1,?)',('fixture-chat',))
            connections=[]
            connect=sqlite3.connect
            def track(*args,**kwargs):
                connection=connect(*args,**kwargs);connections.append(connection);return connection
            with patch.object(adaptive,'ROOT',root),patch.object(adaptive.sqlite3,'connect',side_effect=track):
                self.assertEqual(adaptive.bound_chat(),'fixture-chat')
            for connection in connections:
                with self.assertRaises(sqlite3.ProgrammingError):connection.execute('SELECT 1')

    def test_signature_scope_deduplication_and_destination_revocation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            ledger = root / '.local-data/telegram-staging'
            ledger.mkdir(parents=True)
            with closing(sqlite3.connect(ledger / 'ledger.sqlite')) as db, db:
                db.execute('CREATE TABLE binding(id INTEGER PRIMARY KEY,chat TEXT)')
                db.execute('INSERT INTO binding VALUES(1,?)', ('fixture-chat',))
            secret = root / 'secret'
            secret.write_text('fixture-secret')
            source = root / 'source.json'
            source.write_text(json.dumps({'event_secret_file':str(secret)}))
            config = root / 'config.json'
            config.write_text(json.dumps({'binding_hash':hashlib.sha256(b'fixture-chat').hexdigest(),'sources':[{'id':'alpha','config_file':str(source)}]}))
            raw = json.dumps({'resource':'alpha::unseen-resource','kind':'created','event_id':'test-1','values':{},'test':True}).encode()
            signature = base64.b64encode(hmac.new(b'fixture-secret',raw,hashlib.sha256).digest()).decode()
            sent=[]
            def send(chat,text):
                sent.append((chat,text))
                return {'message_id':1}
            with patch.object(adaptive,'ROOT',root), patch.object(adaptive,'CONFIG',config), patch.object(adaptive,'invoke',side_effect=[{'status':'notification','text':'Prueba'},{'status':'ignored'}]) as invoke:
                self.assertEqual(adaptive.event(raw,'wrong','alpha',send),401)
                self.assertEqual(adaptive.event(raw,signature,'other',send),404)
                malformed=json.dumps({'resource':123,'event_id':'x','kind':'created','values':{}}).encode()
                malformed_signature=base64.b64encode(hmac.new(b'fixture-secret',malformed,hashlib.sha256).digest()).decode()
                self.assertEqual(adaptive.event(malformed,malformed_signature,'alpha',send),400)
                self.assertEqual(invoke.call_count,0)
                self.assertEqual(adaptive.event(raw,signature,'alpha',send),200)
                self.assertEqual(adaptive.event(raw,signature,'alpha',send),200)
                self.assertEqual(len(sent),1)
                with closing(sqlite3.connect(ledger / 'ledger.sqlite')) as db, db:
                    self.assertEqual(db.execute('SELECT status FROM adaptive_deliveries').fetchone()[0],'accepted')
                    db.execute('DELETE FROM binding')
                self.assertEqual(adaptive.event(raw,signature,'alpha',send),403)
                self.assertEqual(len(sent),1)
            with closing(sqlite3.connect(ledger / 'ledger.sqlite')) as db, db:
                db.execute('INSERT INTO binding VALUES(1,?)', ('fixture-chat',))
            def failed_send(chat,text):
                raise RuntimeError('synthetic transport failure')
            with patch.object(adaptive,'ROOT',root), patch.object(adaptive,'CONFIG',config), patch.object(adaptive,'invoke',return_value={'status':'notification','text':'Prueba'}):
                self.assertEqual(adaptive.event(raw,signature,'alpha',failed_send),503)
                with closing(sqlite3.connect(ledger / 'ledger.sqlite')) as db, db:
                    self.assertEqual(db.execute('SELECT status FROM adaptive_deliveries').fetchone()[0],'unknown')



if __name__ == '__main__':
    unittest.main()
