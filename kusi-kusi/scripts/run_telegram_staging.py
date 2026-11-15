#!/usr/bin/env python3
from contextlib import closing
"""Isolated single-chat staging: constrained language interpretation and ERP aggregates."""
import hashlib
import hmac
import http.server
import json
import os
from pathlib import Path
import secrets
import sqlite3
import urllib.error
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
STATE = ROOT / '.local-data/telegram-staging'


def config():
    values = {}
    for line in (ROOT / '.env').read_text().splitlines():
        if '=' in line and not line.lstrip().startswith('#'):
            k, v = line.split('=', 1)
            values[k.strip()] = v.strip().strip('\"').strip("'")
    tokens = [v for k, v in values.items() if k.startswith('KUSI_CHANNEL_TOKEN_') and v]
    if len(tokens) != 1 or len(values.get('TELEGRAM_WEBHOOK_SECRET', '')) < 16:
        raise RuntimeError('Configure exactly one Telegram token and webhook secret')
    return tokens[0], values['TELEGRAM_WEBHOOK_SECRET']


def api(token, method, payload):
    request = urllib.request.Request('https://api.telegram.org/bot' + token + '/' + method,
                                     data=json.dumps(payload).encode(), headers={'Content-Type': 'application/json'})
    try:
        with urllib.request.build_opener(urllib.request.ProxyHandler({})).open(request, timeout=15) as response:
            value = json.loads(response.read(65536))
        if not value.get('ok'):
            raise RuntimeError('Telegram rejected operation')
        return value['result']
    except Exception:
        raise RuntimeError('Telegram operation failed; credentials withheld') from None


def answer(text, request_id=None):
    command=text.strip().lower()
    if command=='/estado':
        return 'El chat está vinculado al entorno autorizado. Puedes conversar sobre sus datos; descubriré los recursos disponibles y comprobaré los permisos en cada consulta.'
    if command in ('/start','/help','/ayuda'):
        return 'Soy Kusi Kusi. Dime qué necesitas consultar, comparar, analizar o seguir en el entorno conectado. Si falta un dato o hay ambigüedad, te pediré una aclaración.'
    from telegram_adaptive import respond
    return respond(text,request_id)


class Handler(http.server.BaseHTTPRequestHandler):
    def setup(self):
        super().setup()
        self.connection.settimeout(10)

    def log_message(self, *args):
        pass

    def done(self, status):
        self.send_response(status)
        self.send_header('Content-Length', '0')
        self.end_headers()

    def do_GET(self):
        self.done(200 if self.path == '/health' else 404)

    def do_POST(self):
        if self.path.startswith('/adaptive/events/'):
            from telegram_adaptive import event
            try:
                length=int(self.headers.get('Content-Length','0'))
                if not 0<length<=65536:return self.done(413)
                source=self.path[len('/adaptive/events/'):]
                status=event(self.rfile.read(length),self.headers.get('X-Frappe-Webhook-Signature',''),source,lambda chat,text:api(self.server.token,'sendMessage',{'chat_id':chat,'text':text}))
                return self.done(status)
            except Exception:return self.done(503)
        if self.path != '/webhook':
            return self.done(404)
        if not hmac.compare_digest(self.headers.get('X-Telegram-Bot-Api-Secret-Token', '').encode(), self.server.secret.encode()):
            return self.done(401)
        try:
            length = int(self.headers.get('Content-Length', '0'))
            if not 0 < length <= 65536:
                return self.done(413)
            update = json.loads(self.rfile.read(length))
            ident = update['update_id']
            message = update.get('message', {})
            chat = message.get('chat', {})
            text = message.get('text', '')
            if not isinstance(ident, int) or not isinstance(text, str) or len(text) > 4096:
                return self.done(400)
            if chat.get('type') != 'private' or not isinstance(chat.get('id'), int):
                return self.done(200)
            chat_id = str(chat['id'])
            digest = hashlib.sha256(str(ident).encode()).hexdigest()
            with closing(sqlite3.connect(STATE / 'ledger.sqlite')) as db, db:
                db.execute('CREATE TABLE IF NOT EXISTS updates (digest TEXT PRIMARY KEY, status TEXT NOT NULL)')
                db.execute('CREATE TABLE IF NOT EXISTS binding (id INTEGER PRIMARY KEY CHECK(id=1), chat TEXT NOT NULL)')
                bound = db.execute('SELECT chat FROM binding WHERE id=1').fetchone()
                pairing = '/start ' + self.server.pair
                unpaired = bound is None and not hmac.compare_digest(text.encode(), pairing.encode())
                if bound is None and not unpaired:
                    db.execute('INSERT INTO binding VALUES (1, ?)', (chat_id,))
                    text = '/start'
                elif bound is not None and bound[0] != chat_id:
                    return self.done(200)
                changed = db.execute("INSERT OR IGNORE INTO updates VALUES (?, 'claimed')", (digest,)).rowcount
            if not changed:
                return self.done(200)
            try:
                result = api(self.server.token, 'sendMessage', {'chat_id': chat_id, 'text': ('Falta vincular tu chat. En tu computadora abre .local-data/telegram-staging/start-link.txt, copia la URL completa en el navegador y pulsa Iniciar en Telegram. Ese enlace contiene el código privado de vinculación.' if unpaired else answer(text,"telegram:"+str(ident)))})
                status = 'accepted' if result.get('message_id') else 'uncertain'
            except RuntimeError:
                status = 'uncertain'
            with closing(sqlite3.connect(STATE / 'ledger.sqlite')) as db, db:
                db.execute('UPDATE updates SET status=? WHERE digest=?', (status, digest))
            print('telegram staging: reply ' + status, flush=True)
            self.done(200)
        except (ValueError, KeyError, TypeError):
            self.done(400)
        except Exception:
            self.done(503)


def main():
    os.umask(0o077)
    STATE.mkdir(parents=True, exist_ok=True, mode=0o700)
    token, secret = config()
    pairing = STATE / 'pairing.txt'
    if not pairing.exists():
        pairing.write_text(secrets.token_urlsafe(24))
    server = http.server.HTTPServer(('127.0.0.1', 18197), Handler)
    server.timeout = 20
    server.token, server.secret, server.pair = token, secret, pairing.read_text().strip()
    print('Telegram staging listening on loopback 18197; private-chat pairing required', flush=True)
    server.serve_forever()


if __name__ == '__main__':
    main()
