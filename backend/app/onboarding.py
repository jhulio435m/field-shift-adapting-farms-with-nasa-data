"""Local farmer profiles and parcels, activated by a signed Kusi Kusi proof.

The Kusi service must verify possession of the WhatsApp sender before issuing
the proof. Field Shift never treats a typed phone number as verified identity.
"""

from __future__ import annotations

import hashlib
import hmac
import json
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
            claim_digest TEXT, claim_expires INTEGER,
            preferred_channel TEXT NOT NULL DEFAULT 'whatsapp',
            telegram_handle TEXT NOT NULL DEFAULT '',
            bot_language TEXT NOT NULL DEFAULT 'es',
            bot_audience TEXT NOT NULL DEFAULT 'farmer',
            alert_frequency TEXT NOT NULL DEFAULT 'immediate',
            alerts_enabled INTEGER NOT NULL DEFAULT 1
        );
        CREATE TABLE IF NOT EXISTS parcels (
            id TEXT PRIMARY KEY, farmer_id TEXT NOT NULL REFERENCES farmers(id),
            name TEXT NOT NULL, lat REAL NOT NULL, lon REAL NOT NULL,
            current_crop TEXT NOT NULL, irrigation TEXT NOT NULL,
            priority TEXT NOT NULL, soil_texture TEXT NOT NULL,
            has_livestock INTEGER NOT NULL DEFAULT 0,
            area_ha REAL,
            planting_month INTEGER,
            polygon_json TEXT
        );
        CREATE INDEX IF NOT EXISTS parcels_owner ON parcels(farmer_id);
    """)
        existing_cols = {r[1] for r in db.execute("PRAGMA table_info(farmers)").fetchall()}
        migrations = [
            ("preferred_channel", "TEXT NOT NULL DEFAULT 'whatsapp'"),
            ("telegram_handle", "TEXT NOT NULL DEFAULT ''"),
            ("bot_language", "TEXT NOT NULL DEFAULT 'es'"),
            ("bot_audience", "TEXT NOT NULL DEFAULT 'farmer'"),
            ("alert_frequency", "TEXT NOT NULL DEFAULT 'immediate'"),
            ("alerts_enabled", "INTEGER NOT NULL DEFAULT 1"),
        ]
        for col_name, col_def in migrations:
            if col_name not in existing_cols:
                db.execute(f"ALTER TABLE farmers ADD COLUMN {col_name} {col_def}")
        parcel_cols = {r[1] for r in db.execute("PRAGMA table_info(parcels)").fetchall()}
        if "area_ha" not in parcel_cols:
            db.execute("ALTER TABLE parcels ADD COLUMN area_ha REAL")
        if "planting_month" not in parcel_cols:
            db.execute("ALTER TABLE parcels ADD COLUMN planting_month INTEGER")
        if "polygon_json" not in parcel_cols:
            db.execute("ALTER TABLE parcels ADD COLUMN polygon_json TEXT")
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


def _demo_verifier_key() -> bytes:
    value = os.environ.get("FIELD_SHIFT_KUSI_VERIFY_KEY", "")
    if len(value) >= 32:
        return value.encode()
    return b"field-shift-hackathon-demo-verifier-key-2026"


def phone_digest(phone: str) -> str:
    return hmac.new(verifier_key(), phone.encode(), hashlib.sha256).hexdigest()


def _normalize_bot_config(bot_config: dict | None) -> dict:
    cfg = bot_config or {}
    return {
        "preferred_channel": cfg.get("preferred_channel", "whatsapp"),
        "telegram_handle": str(cfg.get("telegram_handle", "")).strip(),
        "bot_language": cfg.get("bot_language", "es"),
        "bot_audience": cfg.get("bot_audience", "farmer"),
        "alert_frequency": cfg.get("alert_frequency", "immediate"),
        "alerts_enabled": int(bool(cfg.get("alerts_enabled", True))),
    }


def start(name: str, phone: str, bot_config: dict | None = None) -> dict:
    cfg = _normalize_bot_config(bot_config)
    if not 2 <= len(name.strip()) <= 100:
        raise HTTPException(status_code=422, detail="Nombre inválido")
    if cfg["preferred_channel"] == "telegram" and not phone:
        if len(cfg["telegram_handle"]) < 2:
            raise HTTPException(status_code=422, detail="Ingresa tu usuario o ID de Telegram")
        identity = f"tg:{cfg['telegram_handle'].lower()}"
        last4 = cfg["telegram_handle"][-4:]
    else:
        if not PHONE.fullmatch(phone):
            raise HTTPException(status_code=422, detail="Nombre o número inválido; usa formato internacional, por ejemplo +51999999999")
        identity = phone
        last4 = phone[-4:]
    digest = phone_digest(identity)
    now = int(time.time())
    challenge = secrets.token_urlsafe(24)
    with connect() as db:
        db.execute("BEGIN IMMEDIATE")
        db.execute("DELETE FROM farmers WHERE verified_at IS NULL AND challenge_expires < ?", (now,))
        existing = db.execute("SELECT id FROM farmers WHERE phone_digest=? AND verified_at IS NOT NULL",
                              (digest,)).fetchone()
        if existing:
            db.execute("""UPDATE farmers SET name=?, challenge=?, challenge_expires=?,
                          preferred_channel=?, telegram_handle=?, bot_language=?,
                          bot_audience=?, alert_frequency=?, alerts_enabled=? WHERE id=?""",
                       (name.strip(), challenge, now + 600,
                        cfg["preferred_channel"], cfg["telegram_handle"], cfg["bot_language"],
                        cfg["bot_audience"], cfg["alert_frequency"], cfg["alerts_enabled"],
                        existing["id"]))
            return {"challenge": challenge, "expires_in_seconds": 600, "status": "pending"}
        pending = db.execute("SELECT COUNT(*) FROM farmers WHERE phone_digest=? AND verified_at IS NULL",
                             (digest,)).fetchone()[0]
        if pending >= 3:
            raise HTTPException(status_code=429, detail="Demasiados intentos; espera diez minutos")
        db.execute("""INSERT INTO farmers(id,name,phone_digest,phone_last4,challenge,challenge_expires,
                      preferred_channel,telegram_handle,bot_language,bot_audience,alert_frequency,alerts_enabled)
                      VALUES(?,?,?,?,?,?,?,?,?,?,?,?)""",
                   (str(uuid4()), name.strip(), digest, last4, challenge, now + 600,
                    cfg["preferred_channel"], cfg["telegram_handle"], cfg["bot_language"],
                    cfg["bot_audience"], cfg["alert_frequency"], cfg["alerts_enabled"]))
    return {"challenge": challenge, "expires_in_seconds": 600, "status": "pending"}


def confirm(challenge: str, phone: str, verified_at: int, event_id: str, signature: str) -> str:
    key = verifier_key()
    now = int(time.time())
    if ((not PHONE.fullmatch(phone) and not phone.startswith("tg:"))
            or not 1 <= len(event_id) <= 120
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


def _normalize_polygon(poly: list[list[float]] | None) -> list[list[float]] | None:
    if not poly or len(poly) < 3:
        return None
    cleaned: list[list[float]] = []
    for pt in poly[:40]:
        if not isinstance(pt, (list, tuple)) or len(pt) < 2:
            continue
        try:
            plat = round(float(pt[0]), 6)
            plon = round(float(pt[1]), 6)
        except (TypeError, ValueError):
            continue
        if -90.0 <= plat <= 90.0 and -180.0 <= plon <= 180.0:
            cleaned.append([plat, plon])
    return cleaned if len(cleaned) >= 3 else None


def _serialize_polygon(poly: list[list[float]] | None) -> str | None:
    normalized = _normalize_polygon(poly)
    return json.dumps(normalized) if normalized is not None else None


def _row_to_parcel(row: sqlite3.Row) -> dict:
    item = dict(row)
    raw_poly = item.pop("polygon_json", None)
    try:
        item["polygon"] = json.loads(raw_poly) if raw_poly else None
    except (TypeError, ValueError):
        item["polygon"] = None
    return item


def list_parcels(farmer_id: str) -> list[dict]:
    with connect() as db:
        return [_row_to_parcel(row) for row in db.execute(
            "SELECT id,name,lat,lon,current_crop,irrigation,priority,soil_texture,has_livestock,area_ha,planting_month,polygon_json "
            "FROM parcels WHERE farmer_id=? ORDER BY rowid", (farmer_id,))]


def add_parcel(farmer_id: str, data: dict) -> dict:
    if not 1 <= len(data["name"].strip()) <= 80:
        raise HTTPException(status_code=422, detail="Ponle un nombre a la parcela")
    normalized_poly = _normalize_polygon(data.get("polygon"))
    poly_json = json.dumps(normalized_poly) if normalized_poly is not None else None
    with connect() as db:
        db.execute("BEGIN IMMEDIATE")
        count = db.execute("SELECT COUNT(*) FROM parcels WHERE farmer_id=?", (farmer_id,)).fetchone()[0]
        if count >= 8:
            raise HTTPException(status_code=422, detail="Máximo 8 parcelas por cuenta")
        parcel_id = str(uuid4())
        db.execute("""INSERT INTO parcels(id,farmer_id,name,lat,lon,current_crop,irrigation,
                      priority,soil_texture,has_livestock,area_ha,planting_month,polygon_json) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)""",
                   (parcel_id, farmer_id, data["name"].strip(), data["lat"], data["lon"],
                    data["current_crop"], data["irrigation"], data["priority"],
                    data["soil_texture"], int(data["has_livestock"]),
                    data.get("area_ha"), data.get("planting_month"), poly_json))
    return {"id": parcel_id, **{**data, "name": data["name"].strip(), "polygon": normalized_poly}}


def bot_config(row: sqlite3.Row) -> dict:
    return {
        "preferred_channel": row["preferred_channel"],
        "telegram_handle": row["telegram_handle"],
        "bot_language": row["bot_language"],
        "bot_audience": row["bot_audience"],
        "alert_frequency": row["alert_frequency"],
        "alerts_enabled": bool(row["alerts_enabled"]),
    }


def update_bot_config(farmer_id: str, data: dict) -> dict:
    cfg = _normalize_bot_config(data)
    with connect() as db:
        db.execute("""UPDATE farmers SET preferred_channel=?, telegram_handle=?, bot_language=?,
                      bot_audience=?, alert_frequency=?, alerts_enabled=? WHERE id=?""",
                   (cfg["preferred_channel"], cfg["telegram_handle"], cfg["bot_language"],
                    cfg["bot_audience"], cfg["alert_frequency"], cfg["alerts_enabled"], farmer_id))
    return {**cfg, "alerts_enabled": bool(cfg["alerts_enabled"])}


def get_parcel(farmer_id: str, parcel_id: str) -> dict:
    with connect() as db:
        row = db.execute("SELECT * FROM parcels WHERE farmer_id=? AND id=?", (farmer_id, parcel_id)).fetchone()
    if row is None:
        raise HTTPException(status_code=404, detail="Parcela no encontrada")
    return _row_to_parcel(row)


def update_parcel(farmer_id: str, parcel_id: str, data: dict) -> dict:
    if not 1 <= len(data["name"].strip()) <= 80:
        raise HTTPException(status_code=422, detail="Ponle un nombre a la parcela")
    normalized_poly = _normalize_polygon(data.get("polygon"))
    poly_json = json.dumps(normalized_poly) if normalized_poly is not None else None
    with connect() as db:
        db.execute("BEGIN IMMEDIATE")
        row = db.execute(
            "SELECT id FROM parcels WHERE id=? AND farmer_id=?",
            (parcel_id, farmer_id),
        ).fetchone()
        if row is None:
            raise HTTPException(status_code=404, detail="Parcela no encontrada")
        db.execute(
            """UPDATE parcels
               SET name=?, lat=?, lon=?, current_crop=?, irrigation=?, priority=?,
                   soil_texture=?, has_livestock=?, area_ha=?, planting_month=?, polygon_json=?
               WHERE id=? AND farmer_id=?""",
            (
                data["name"].strip(),
                data["lat"],
                data["lon"],
                data["current_crop"],
                data["irrigation"],
                data["priority"],
                data["soil_texture"],
                int(data["has_livestock"]),
                data.get("area_ha"),
                data.get("planting_month"),
                poly_json,
                parcel_id,
                farmer_id,
            ),
        )
    return {"id": parcel_id, **{**data, "name": data["name"].strip(), "polygon": normalized_poly}}


def delete_parcel(farmer_id: str, parcel_id: str) -> None:
    with connect() as db:
        db.execute("BEGIN IMMEDIATE")
        row = db.execute(
            "SELECT id FROM parcels WHERE id=? AND farmer_id=?",
            (parcel_id, farmer_id),
        ).fetchone()
        if row is None:
            raise HTTPException(status_code=404, detail="Parcela no encontrada")
        db.execute("DELETE FROM parcels WHERE id=? AND farmer_id=?", (parcel_id, farmer_id))


def start_demo(data: dict) -> tuple[str, str]:
    cfg = _normalize_bot_config(data)
    name = str(data.get("name") or "María Quispe").strip()
    if len(name) < 2:
        name = "María Quispe"

    phone = str(data.get("phone") or "").strip()
    if cfg["preferred_channel"] == "telegram" and len(cfg["telegram_handle"]) >= 2:
        identity = f"tg:{cfg['telegram_handle'].lower()}"
        last4 = cfg["telegram_handle"][-4:]
    elif PHONE.fullmatch(phone):
        identity = phone
        last4 = phone[-4:]
    elif len(cfg["telegram_handle"]) >= 2:
        identity = f"tg:{cfg['telegram_handle'].lower()}"
        last4 = cfg["telegram_handle"][-4:]
    else:
        identity = "+51987654321"
        last4 = "4321"

    digest = hmac.new(_demo_verifier_key(), identity.encode(), hashlib.sha256).hexdigest()
    now = int(time.time())
    token = secrets.token_urlsafe(32)
    csrf = secrets.token_urlsafe(24)
    token_digest = hashlib.sha256(token.encode()).hexdigest()

    with connect() as db:
        db.execute("BEGIN IMMEDIATE")
        existing = db.execute(
            "SELECT id FROM farmers WHERE phone_digest=? ORDER BY verified_at DESC LIMIT 1",
            (digest,),
        ).fetchone()
        if existing:
            farmer_id = existing["id"]
            db.execute(
                """UPDATE farmers
                   SET name=?, verified_at=?, session_digest=?, csrf=?, session_expires=?,
                       challenge=NULL, challenge_expires=NULL, claim_digest=NULL, claim_expires=NULL,
                       preferred_channel=?, telegram_handle=?, bot_language=?,
                       bot_audience=?, alert_frequency=?, alerts_enabled=?
                   WHERE id=?""",
                (
                    name,
                    now,
                    token_digest,
                    csrf,
                    now + 86400,
                    cfg["preferred_channel"],
                    cfg["telegram_handle"],
                    cfg["bot_language"],
                    cfg["bot_audience"],
                    cfg["alert_frequency"],
                    cfg["alerts_enabled"],
                    farmer_id,
                ),
            )
        else:
            farmer_id = str(uuid4())
            db.execute(
                """INSERT INTO farmers(
                       id, name, phone_digest, phone_last4, verified_at,
                       session_digest, csrf, session_expires,
                       preferred_channel, telegram_handle, bot_language,
                       bot_audience, alert_frequency, alerts_enabled
                   ) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?)""",
                (
                    farmer_id,
                    name,
                    digest,
                    last4,
                    now,
                    token_digest,
                    csrf,
                    now + 86400,
                    cfg["preferred_channel"],
                    cfg["telegram_handle"],
                    cfg["bot_language"],
                    cfg["bot_audience"],
                    cfg["alert_frequency"],
                    cfg["alerts_enabled"],
                ),
            )

        if data.get("seed_parcels", True):
            count = db.execute("SELECT COUNT(*) FROM parcels WHERE farmer_id=?", (farmer_id,)).fetchone()[0]
            if count == 0:
                mantaro_poly = _serialize_polygon([
                    [-12.0644, -75.2057],
                    [-12.0643, -75.2041],
                    [-12.0656, -75.2040],
                    [-12.0657, -75.2056],
                ])
                piura_poly = _serialize_polygon([
                    [-5.1937, -80.6338],
                    [-5.1936, -80.6318],
                    [-5.1953, -80.6317],
                    [-5.1954, -80.6337],
                ])
                demo_parcels = [
                    (
                        str(uuid4()),
                        farmer_id,
                        "Lote Mantaro — Papa",
                        -12.065,
                        -75.2049,
                        "potato_continuous",
                        "rainfed",
                        "soil_health",
                        "loam",
                        1,
                        2.5,
                        10,
                        mantaro_poly,
                    ),
                    (
                        str(uuid4()),
                        farmer_id,
                        "Valle Piura — Arroz",
                        -5.1945,
                        -80.6328,
                        "rice_continuous",
                        "irrigated",
                        "water",
                        "clay",
                        0,
                        4.0,
                        1,
                        piura_poly,
                    ),
                ]
                db.executemany(
                    """INSERT INTO parcels(
                           id, farmer_id, name, lat, lon, current_crop, irrigation,
                           priority, soil_texture, has_livestock, area_ha, planting_month, polygon_json
                       ) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)""",
                    demo_parcels,
                )

    return token, csrf


def logout(token: str | None) -> None:
    if not token:
        return
    digest = hashlib.sha256(token.encode()).hexdigest()
    with connect() as db:
        db.execute(
            "UPDATE farmers SET session_digest=NULL, csrf=NULL, session_expires=NULL WHERE session_digest=?",
            (digest,),
        )
