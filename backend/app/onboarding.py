"""Local farmer profiles and parcels, activated by a signed Kusi Kusi proof.

The Kusi service must verify possession of the WhatsApp sender before issuing
the proof. Field Shift never treats a typed phone number as verified identity.
"""

from __future__ import annotations

import hashlib
import hmac
import os
from pathlib import Path
import re
import secrets
import sqlite3
import time
from contextlib import contextmanager
from uuid import uuid4

from fastapi import HTTPException

PHONE = re.compile(r"^\+[1-9][0-9]{7,14}$")


def database_path() -> Path:
    return Path(os.environ.get("FIELD_SHIFT_DB", Path(__file__).parents[1] / ".local-data" / "farmers.sqlite3"))


@contextmanager
def connect():
    path = database_path()
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    db = sqlite3.connect(path)
    path.chmod(0o600)
    try:
        db.row_factory = sqlite3.Row
        db.executescript("""
        CREATE TABLE IF NOT EXISTS farmers (
            id TEXT PRIMARY KEY, name TEXT NOT NULL, phone_digest TEXT NOT NULL,
            phone_last4 TEXT NOT NULL, verified_at INTEGER, session_digest TEXT,
            csrf TEXT, session_expires INTEGER, challenge TEXT UNIQUE, challenge_expires INTEGER,
            claim_digest TEXT, claim_expires INTEGER
        );
        CREATE TABLE IF NOT EXISTS parcels (
            id TEXT PRIMARY KEY, farmer_id TEXT NOT NULL REFERENCES farmers(id),
            name TEXT NOT NULL, lat REAL NOT NULL, lon REAL NOT NULL,
            current_crop TEXT NOT NULL, irrigation TEXT NOT NULL,
            priority TEXT NOT NULL, soil_texture TEXT NOT NULL,
            has_livestock INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS parcels_owner ON parcels(farmer_id);
    """)
        db.execute("PRAGMA foreign_keys=ON")
        yield db
        db.commit()
    except Exception:
        db.rollback()
        raise
    finally:
        db.close()


def verifier_key() -> bytes:
    value = os.environ.get("FIELD_SHIFT_KUSI_VERIFY_KEY", "")
    if len(value) < 32:
        raise HTTPException(status_code=503, detail="Verificación Kusi Kusi no configurada")
    return value.encode()


def phone_digest(phone: str) -> str:
    return hmac.new(verifier_key(), phone.encode(), hashlib.sha256).hexdigest()


def start(name: str, phone: str) -> dict:
    if not 2 <= len(name.strip()) <= 100 or not PHONE.fullmatch(phone):
        raise HTTPException(status_code=422, detail="Nombre o número inválido; usa formato internacional, por ejemplo +51999999999")
    digest = phone_digest(phone)
    now = int(time.time())
    challenge = secrets.token_urlsafe(24)
    with connect() as db:
        db.execute("BEGIN IMMEDIATE")
        db.execute("DELETE FROM farmers WHERE verified_at IS NULL AND challenge_expires < ?", (now,))
        existing = db.execute("SELECT id FROM farmers WHERE phone_digest=? AND verified_at IS NOT NULL",
                              (digest,)).fetchone()
        if existing:
            db.execute("UPDATE farmers SET challenge=?, challenge_expires=? WHERE id=?",
                       (challenge, now + 600, existing["id"]))
            return {"challenge": challenge, "expires_in_seconds": 600, "status": "pending"}
        pending = db.execute("SELECT COUNT(*) FROM farmers WHERE phone_digest=? AND verified_at IS NULL",
                             (digest,)).fetchone()[0]
        if pending >= 3:
            raise HTTPException(status_code=429, detail="Demasiados intentos; espera diez minutos")
        db.execute("""INSERT INTO farmers(id,name,phone_digest,phone_last4,challenge,challenge_expires)
                      VALUES(?,?,?,?,?,?)""", (str(uuid4()), name.strip(), digest, phone[-4:], challenge, now + 600))
    return {"challenge": challenge, "expires_in_seconds": 600, "status": "pending"}


def confirm(challenge: str, phone: str, verified_at: int, event_id: str, signature: str) -> str:
    key = verifier_key()
    now = int(time.time())
    if (not PHONE.fullmatch(phone) or not 1 <= len(event_id) <= 120
            or not isinstance(verified_at, int) or abs(now - verified_at) > 300):
        raise HTTPException(status_code=403, detail="Prueba de canal inválida")
    message = f"{challenge}\n{phone}\n{verified_at}\n{event_id}".encode()
    expected = hmac.new(key, message, hashlib.sha256).hexdigest()
    if not hmac.compare_digest(expected, signature):
        raise HTTPException(status_code=403, detail="Prueba de canal inválida")
    claim_code = secrets.token_urlsafe(24)
    with connect() as db:
        db.execute("BEGIN IMMEDIATE")
        row = db.execute("SELECT * FROM farmers WHERE challenge=?", (challenge,)).fetchone()
        if (row is None or row["challenge_expires"] < now
                or row["phone_digest"] != phone_digest(phone)):
            raise HTTPException(status_code=403, detail="Desafío vencido o ya utilizado")
        db.execute("""UPDATE farmers SET verified_at=?, claim_digest=?, claim_expires=?,
                      challenge=NULL, challenge_expires=NULL WHERE id=?""",
                   (now, hashlib.sha256(claim_code.encode()).hexdigest(), now + 600, row["id"]))
    return claim_code


def claim(code: str) -> tuple[str, str]:
    if not 20 <= len(code) <= 100:
        raise HTTPException(status_code=403, detail="Código inválido")
    now = int(time.time())
    token = secrets.token_urlsafe(32)
    csrf = secrets.token_urlsafe(24)
    with connect() as db:
        db.execute("BEGIN IMMEDIATE")
        row = db.execute("SELECT id FROM farmers WHERE claim_digest=? AND claim_expires>=?",
                         (hashlib.sha256(code.encode()).hexdigest(), now)).fetchone()
        if row is None:
            raise HTTPException(status_code=403, detail="Código vencido o ya utilizado")
        db.execute("""UPDATE farmers SET session_digest=?, csrf=?, session_expires=?, claim_digest=NULL,
                      claim_expires=NULL WHERE id=?""",
                   (hashlib.sha256(token.encode()).hexdigest(), csrf, now + 86400, row["id"]))
    return token, csrf


def farmer(token: str | None) -> sqlite3.Row:
    if not token:
        raise HTTPException(status_code=401, detail="Valida tu número para continuar")
    digest = hashlib.sha256(token.encode()).hexdigest()
    with connect() as db:
        row = db.execute("SELECT * FROM farmers WHERE session_digest=? AND verified_at IS NOT NULL AND session_expires>=?",
                         (digest, int(time.time()))).fetchone()
    if row is None:
        raise HTTPException(status_code=401, detail="Sesión no válida")
    return row


def require_csrf(row: sqlite3.Row, value: str | None) -> None:
    if not value or not hmac.compare_digest(row["csrf"], value):
        raise HTTPException(status_code=403, detail="Protección de sesión inválida")


def list_parcels(farmer_id: str) -> list[dict]:
    with connect() as db:
        return [dict(row) for row in db.execute(
            "SELECT id,name,lat,lon,current_crop,irrigation,priority,soil_texture,has_livestock "
            "FROM parcels WHERE farmer_id=? ORDER BY rowid", (farmer_id,))]


def add_parcel(farmer_id: str, data: dict) -> dict:
    if not 1 <= len(data["name"].strip()) <= 80:
        raise HTTPException(status_code=422, detail="Ponle un nombre a la parcela")
    with connect() as db:
        db.execute("BEGIN IMMEDIATE")
        count = db.execute("SELECT COUNT(*) FROM parcels WHERE farmer_id=?", (farmer_id,)).fetchone()[0]
        if count >= 8:
            raise HTTPException(status_code=422, detail="Máximo 8 parcelas por cuenta")
        parcel_id = str(uuid4())
        db.execute("""INSERT INTO parcels(id,farmer_id,name,lat,lon,current_crop,irrigation,
                      priority,soil_texture,has_livestock) VALUES(?,?,?,?,?,?,?,?,?,?)""",
                   (parcel_id, farmer_id, data["name"].strip(), data["lat"], data["lon"],
                    data["current_crop"], data["irrigation"], data["priority"],
                    data["soil_texture"], int(data["has_livestock"])))
    return {"id": parcel_id, **{**data, "name": data["name"].strip()}}


def get_parcel(farmer_id: str, parcel_id: str) -> dict:
    with connect() as db:
        row = db.execute("SELECT * FROM parcels WHERE farmer_id=? AND id=?", (farmer_id, parcel_id)).fetchone()
    if row is None:
        raise HTTPException(status_code=404, detail="Parcela no encontrada")
    return dict(row)
