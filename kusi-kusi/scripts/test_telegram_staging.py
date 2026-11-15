import io
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import run_telegram_staging as staging


class WebhookTests(unittest.TestCase):
    def test_auth_pairing_isolation_and_durable_deduplication(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(staging, 'STATE', Path(directory)), patch.object(staging, 'api', return_value={'message_id': 1}) as send:
            def call(update, secret='fixture-secret'):
                handler = staging.Handler.__new__(staging.Handler)
                data = json.dumps(update).encode()
                handler.path = '/webhook'
                handler.headers = {'Content-Length': str(len(data)), 'X-Telegram-Bot-Api-Secret-Token': secret}
                handler.rfile = io.BytesIO(data)
                handler.server = SimpleNamespace(secret='fixture-secret', pair='fixture-pair', token='fictional')
                statuses = []
                handler.done = statuses.append
                handler.do_POST()
                return statuses[-1]
            def update(ident, chat, text):
                return {'update_id': ident, 'message': {'chat': {'id': chat, 'type': 'private'}, 'text': text}}
            self.assertEqual(call(update(1, 10, '/start fixture-pair'), 'wrong'), 401)
            self.assertEqual(call(update(2, 10, '/start')), 200)
            self.assertEqual(send.call_count, 1)
            self.assertIn('Falta vincular', send.call_args.args[2]['text'])
            send.reset_mock()
            self.assertEqual(call(update(3, 10, '/start fixture-pair')), 200)
            self.assertEqual(send.call_count, 1)
            self.assertEqual(call(update(3, 10, '/start fixture-pair')), 200)
            self.assertEqual(call(update(4, 11, '/estado')), 200)
            self.assertEqual(send.call_count, 1)
            self.assertEqual(call(update(5, 10, '/estado')), 200)
            self.assertEqual(send.call_count, 2)
            handler = staging.Handler.__new__(staging.Handler)
            handler.path = '/events/coneimera-inscripciones'
            handler.headers = {'Content-Length': '2'}
            handler.rfile = io.BytesIO(b'{}')
            statuses = []
            handler.done = statuses.append
            handler.do_POST()
            self.assertEqual(statuses[-1], 404)


if __name__ == '__main__':
    unittest.main()
