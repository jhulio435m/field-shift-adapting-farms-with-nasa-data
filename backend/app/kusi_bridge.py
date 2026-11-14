"""Read-only, parcel-bound source for Kusi Kusi's adaptive discover/read RPC.

The parcel and endpoint come from a private operator-owned file, never a chat
message or a model plan. No identity or messaging is handled here.
"""

from __future__ import annotations

import hashlib
import json
import math
from pathlib import Path
import stat
import sys
import urllib.error
import urllib.parse
import urllib.request

from pydantic import ValidationError

from .schemas import KusiCapabilityRequest, KusiCapabilityResponse


class Forbidden(Exception):
    pass


class Stale(Exception):
    pass


class Unavailable(Exception):
    pass


FIELDS = {
    "climate_months": [
        ("month", "Mes", "number"),
        ("precip_mm_month", "Lluvia mensual (mm)", "number"),
        ("evapotranspiration_mm_month", "Evapotranspiración mensual (mm)", "number"),
        ("water_balance_mm", "Balance hídrico mensual (mm)", "number"),
        ("temp_mean_c", "Temperatura media (°C)", "number"),
        ("soil_moisture_top", "Humedad superficial (fracción)", "number"),
        ("frost_risk", "Riesgo de helada", "category"),
        ("dry_stress", "Estrés hídrico", "category"),
    ],
    "rotation_options": [
        ("strategy", "Estrategia de rotación", "category"),
        ("fit_score", "Índice orientativo, no probabilidad", "number"),
        ("water_demand_level", "Demanda de agua", "category"),
        ("is_current", "Rotación actual", "category"),
    ],
}


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *_args, **_kwargs):
        raise Forbidden


def load_config(path: Path) -> dict:
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_mode & 0o077 or info.st_size > 65536:
        raise Forbidden
    config = json.loads(path.read_text())
    if set(config) != {"protocol", "enabled", "label", "endpoint", "parcel"}:
        raise Forbidden
    if config["protocol"] != "field_shift_loopback" or config["enabled"] is not True:
        raise Forbidden
    endpoint = urllib.parse.urlsplit(config["endpoint"])
    if (endpoint.scheme != "http" or endpoint.hostname != "127.0.0.1"
            or endpoint.username or endpoint.password or endpoint.query or endpoint.fragment
            or endpoint.path != "/v1/kusi/capability"):
        raise Forbidden
    if not isinstance(config["label"], str) or not 1 <= len(config["label"]) <= 120:
        raise Forbidden
    if not isinstance(config["parcel"], dict):
        raise Forbidden
    try:
        config["parcel"] = KusiCapabilityRequest.model_validate(config["parcel"]).model_dump()
    except ValidationError as exc:
        raise Forbidden from exc
    return config


def fetch(config: dict) -> KusiCapabilityResponse:
    body = json.dumps(config["parcel"]).encode()
    request = urllib.request.Request(
        config["endpoint"], body, {"Content-Type": "application/json"}, method="POST"
    )
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    try:
        with opener.open(request, timeout=25) as response:
            if response.status != 200:
                raise Unavailable
            raw = response.read(1024 * 1024 + 1)
        if len(raw) > 1024 * 1024:
            raise Unavailable
        report = KusiCapabilityResponse.model_validate_json(raw)
    except (urllib.error.URLError, TimeoutError, ValidationError, ValueError) as exc:
        raise Unavailable from exc
    if not report.live_nasa_data or not report.raw_comparison.diagnosis.live_nasa_data:
        raise Unavailable
    return report


def materialize(report: KusiCapabilityResponse) -> dict[str, list[dict]]:
    months = [
        {key: getattr(month, key) for key, _, _ in FIELDS["climate_months"]}
        for month in report.raw_comparison.diagnosis.monthly
    ]
    strategies = [
        {"strategy": item.name, "fit_score": item.fit_score,
         "water_demand_level": item.water_demand_level, "is_current": item.is_current}
        for item in report.raw_comparison.strategies
    ]
    return {"climate_months": months, "rotation_options": strategies}


def catalog(config: dict, rows: dict[str, list[dict]]) -> dict:
    resources = []
    for resource_id, fields in FIELDS.items():
        options = {
            key: sorted({row[key] for row in rows[resource_id]})
            for key, _, kind in fields if kind == "category"
        }
        resources.append({
            "id": resource_id,
            "label": "Clima mensual" if resource_id == "climate_months" else "Rotaciones orientativas",
            "fields": [
                {"id": key, "label": label, "kind": kind,
                 "options": options.get(key, [])}
                for key, label, kind in fields
            ],
            "events": [],
        })
    fingerprint = hashlib.sha256(json.dumps(
        {"parcel": config["parcel"], "resources": resources},
        sort_keys=True, ensure_ascii=False,
    ).encode()).hexdigest()
    return {"source": config["label"], "version": fingerprint, "resources": resources}


def execute(config: dict, request: dict) -> dict:
    if not isinstance(request, dict) or request.get("op") not in ("discover", "read"):
        raise Forbidden
    required = {"op"} if request["op"] == "discover" else {
        "op", "version", "resource", "fields", "filters", "limit"
    }
    if set(request) != required:
        raise Forbidden
    rows = materialize(fetch(config))
    current = catalog(config, rows)
    if request["op"] == "discover":
        return current
    if request["version"] != current["version"]:
        raise Stale
    resource = request["resource"]
    if resource not in rows:
        raise Forbidden
    allowed = {field["id"]: field for item in current["resources"]
               if item["id"] == resource for field in item["fields"]}
    selected = request["fields"]
    filters = request["filters"]
    limit = request["limit"]
    if (not isinstance(selected, list) or not 1 <= len(selected) <= 16
            or any(not isinstance(f, str) or f not in allowed for f in selected)
            or len(set(selected)) != len(selected)
            or type(limit) is not int or not 1 <= limit <= 10000
            or not isinstance(filters, list) or len(filters) > 8):
        raise Forbidden
    for item in filters:
        if (not isinstance(item, dict) or set(item) != {"field", "op", "value"}
                or not isinstance(item["field"], str) or not isinstance(item["op"], str)):
            raise Forbidden
        field = allowed.get(item["field"])
        value = item["value"]
        if not field or item["op"] not in ("eq", "ge", "le"):
            raise Forbidden
        if field["kind"] == "category":
            if item["op"] != "eq" or not any(
                type(value) is type(option) and value == option for option in field["options"]
            ):
                raise Forbidden
        elif type(value) not in (int, float) or not math.isfinite(value):
            raise Forbidden
    def matches(row: dict) -> bool:
        for item in filters:
            actual, value = row[item["field"]], item["value"]
            if item["op"] == "eq" and actual != value:
                return False
            if item["op"] == "ge" and actual < value:
                return False
            if item["op"] == "le" and actual > value:
                return False
        return True
    result = [{field: row[field] for field in selected} for row in rows[resource] if matches(row)]
    return {"rows": result[:limit], "truncated": len(result) > limit}


def main() -> None:
    try:
        if len(sys.argv) != 2:
            raise Forbidden
        raw = sys.stdin.buffer.read(65537)
        if len(raw) > 65536:
            raise Forbidden
        response = execute(load_config(Path(sys.argv[1])), json.loads(raw))
    except Stale:
        response = {"error": "stale"}
    except Forbidden:
        response = {"error": "forbidden"}
    except Exception:
        response = {"error": "unavailable"}
    print(json.dumps(response, ensure_ascii=False))


if __name__ == "__main__":
    main()
