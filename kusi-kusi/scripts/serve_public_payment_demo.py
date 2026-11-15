"""Explicit public synthetic-demo gateway. Never forwards cookies or admin routes."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlsplit, unquote
from urllib.request import Request, urlopen, ProxyHandler, build_opener
from urllib.error import HTTPError

UPSTREAM = 'http://127.0.0.1:18088'
READ = {'/tramites', '/v1/demo/payments'}
WRITE = {'/v1/demo/payments/by-dni', '/v1/demo/payments/validate', '/v1/demo/payments/validate-receipt'}
client = build_opener(ProxyHandler({}))
class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass
    def do_GET(self):
        self.forward(False)
    def do_POST(self):
        self.forward(True)
    def forward(self, write):
        path = unquote(urlsplit(self.path).path)
        if path == '/' and not write:
            self.send_response(302); self.send_header('Location', '/tramites'); self.end_headers(); return
        allowed = path in WRITE if write else path in READ or (path.startswith('/_next/static/') and not path.endswith('.map'))
        if not allowed or '..' in path.split('/') or '\\' in path:
            self.send_error(404); return
        if write and self.headers.get('Origin') != 'https://' + self.headers.get('Host', ''):
            self.send_error(403); return
        try:
            size = int(self.headers.get('Content-Length', '0'))
            if size < 0 or size > 5 * 1024 * 1024:
                self.send_error(413); return
            self.connection.settimeout(20)
            body = self.rfile.read(size) if write else None
            headers = {'Origin': UPSTREAM, 'Content-Type': self.headers.get('Content-Type', 'application/json')}
            request = Request(UPSTREAM + self.path, data=body, headers=headers, method='POST' if write else 'GET')
            try:
                response = client.open(request, timeout=30)
            except HTTPError as error:
                response = error
            with response:
                payload = response.read(8 * 1024 * 1024)
                self.send_response(response.status)
                self.send_header('Content-Type', response.headers.get('Content-Type', 'application/octet-stream'))
                self.send_header('Content-Length', str(len(payload)))
                self.send_header('Cache-Control', 'no-store')
                self.send_header('X-Content-Type-Options', 'nosniff')
                self.end_headers(); self.wfile.write(payload)
        except (ValueError, OSError):
            self.send_error(502)
if __name__ == '__main__':
    print('Public synthetic payment gateway: 127.0.0.1:18196; admin routes blocked', flush=True)
    ThreadingHTTPServer(('127.0.0.1',18196), Handler).serve_forever()
