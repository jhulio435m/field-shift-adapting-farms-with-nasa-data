"""Loopback-only synthetic OIDC issuer used by verify_oidc.py."""
import base64
import hashlib
import json
import threading
import time
import urllib.parse
from http.server import BaseHTTPRequestHandler, HTTPServer

from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import padding, rsa


def b64url(value: bytes) -> str:
    return base64.urlsafe_b64encode(value).rstrip(b"=").decode("ascii")


class MockOIDCIssuer:
    def __init__(self):
        self.key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
        self.codes = {}
        self.next_subject = "admin-subject"
        self.bad_signature = False
        self.bad_nonce = False
        self.bad_issuer = False
        self.bad_audience = False
        self.expired_token = False
        self.reject_pkce = False
        self.httpd = None
        self.thread = None

    def __enter__(self):
        issuer = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, _format, *_args):
                return

            def send_json(self, value, status=200):
                body = json.dumps(value, separators=(",", ":")).encode()
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

            def do_GET(self):
                if self.path == "/.well-known/openid-configuration":
                    base = issuer.url
                    return self.send_json({
                        "issuer": base,
                        "authorization_endpoint": base + "/authorize",
                        "token_endpoint": base + "/token",
                        "jwks_uri": base + "/jwks",
                        "response_types_supported": ["code"],
                        "subject_types_supported": ["public"],
                        "id_token_signing_alg_values_supported": ["RS256"],
                        "token_endpoint_auth_methods_supported": ["client_secret_basic"],
                    })
                if self.path == "/jwks":
                    numbers = issuer.key.public_key().public_numbers()
                    return self.send_json({"keys": [{
                        "kty": "RSA", "use": "sig", "alg": "RS256", "kid": "test-key",
                        "n": b64url(numbers.n.to_bytes((numbers.n.bit_length() + 7) // 8, "big")),
                        "e": b64url(numbers.e.to_bytes((numbers.e.bit_length() + 7) // 8, "big")),
                    }]})
                if self.path.startswith("/authorize?"):
                    query = urllib.parse.parse_qs(urllib.parse.urlsplit(self.path).query)
                    state = query["state"][0]
                    nonce = query["nonce"][0]
                    redirect_uri = query["redirect_uri"][0]
                    challenge = query["code_challenge"][0]
                    code = b64url(hashlib.sha256((state + str(time.time_ns())).encode()).digest())
                    issuer.codes[code] = (nonce, redirect_uri, challenge, issuer.next_subject,
                                          issuer.bad_signature, issuer.bad_nonce,
                                          issuer.bad_issuer, issuer.bad_audience,
                                          issuer.expired_token, issuer.reject_pkce)
                    issuer.next_subject = "admin-subject"
                    issuer.bad_signature = False
                    issuer.bad_nonce = False
                    issuer.bad_issuer = False
                    issuer.bad_audience = False
                    issuer.expired_token = False
                    issuer.reject_pkce = False
                    callback = urllib.parse.urlsplit(redirect_uri)
                    params = urllib.parse.parse_qsl(callback.query)
                    params += [("code", code), ("state", state), ("iss", issuer.url)]
                    location = urllib.parse.urlunsplit((callback.scheme, callback.netloc,
                        callback.path, urllib.parse.urlencode(params), ""))
                    self.send_response(302)
                    self.send_header("Location", location)
                    self.send_header("Content-Length", "0")
                    self.end_headers()
                    return
                self.send_error(404)

            def do_POST(self):
                if self.path != "/token":
                    return self.send_error(404)
                form = urllib.parse.parse_qs(self.rfile.read(int(self.headers.get("Content-Length", "0"))).decode())
                code = form.get("code", [""])[0]
                transaction = issuer.codes.pop(code, None)
                verifier = form.get("code_verifier", [""])[0]
                expected = b64url(hashlib.sha256(verifier.encode()).digest())
                if (transaction is None or expected != transaction[2]
                        or form.get("redirect_uri", [""])[0] != transaction[1]
                        or self.headers.get("Authorization") != "Basic " + base64.b64encode(
                            b"test-client:test-client-secret").decode()):
                    return self.send_json({"error": "invalid_grant"}, 400)
                (nonce, _, _, subject, bad_signature, bad_nonce, bad_issuer,
                 bad_audience, expired_token, reject_pkce) = transaction
                if reject_pkce:
                    return self.send_json({"error": "invalid_grant"}, 400)
                now = int(time.time())
                header = b64url(json.dumps({"alg": "RS256", "kid": "test-key", "typ": "JWT"},
                                           separators=(",", ":")).encode())
                payload = b64url(json.dumps({
                    "iss": issuer.url + ("/wrong" if bad_issuer else ""), "sub": subject,
                    "aud": "different-client" if bad_audience else "test-client",
                    "exp": now - 30 if expired_token else now + 300, "iat": now,
                    "nonce": nonce + ("-wrong" if bad_nonce else ""),
                }, separators=(",", ":")).encode())
                signing_input = (header + "." + payload).encode()
                signing_key = (rsa.generate_private_key(public_exponent=65537, key_size=2048)
                               if bad_signature else issuer.key)
                signature = signing_key.sign(signing_input, padding.PKCS1v15(), hashes.SHA256())
                token = header + "." + payload + "." + b64url(signature)
                return self.send_json({"access_token": "synthetic-access-token", "token_type": "Bearer",
                                       "expires_in": 300, "id_token": token})

        self.httpd = HTTPServer(("127.0.0.1", 0), Handler)
        self.httpd.timeout = 0.2
        self.url = "http://127.0.0.1:" + str(self.httpd.server_port)
        self.thread = threading.Thread(target=self.httpd.serve_forever, daemon=True)
        self.thread.start()
        return self

    def __exit__(self, *_exc):
        if self.httpd:
            self.httpd.shutdown()
            self.httpd.server_close()
        if self.thread:
            self.thread.join(timeout=2)
