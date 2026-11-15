"""Local development helper to simulate Kusi Kusi's signed WhatsApp confirmation."""

from __future__ import annotations

import argparse
import hashlib
import hmac
import json
import os
import time
import urllib.request
from pathlib import Path
from uuid import uuid4


def load_verify_key() -> str:
    key = os.environ.get("FIELD_SHIFT_KUSI_VERIFY_KEY", "").strip()
    if len(key) >= 32:
        return key

    env_file = Path(__file__).resolve().parents[1] / ".env"
    if env_file.is_file():
        for line in env_file.read_text().splitlines():
            line = line.strip()
            if line.startswith("FIELD_SHIFT_KUSI_VERIFY_KEY="):
                val = line.split("=", 1)[1].strip().strip('"').strip("'")
                if len(val) >= 32:
                    return val

    raise SystemExit("FIELD_SHIFT_KUSI_VERIFY_KEY (>= 32 chars) not found in env or ../.env")


def main() -> None:
    parser = argparse.ArgumentParser(description="Confirm a Field Shift onboarding challenge.")
    parser.add_argument("--challenge", required=True, help="Challenge returned by /v1/onboarding/start")
    parser.add_argument("--phone", required=True, help="E.164 phone number, e.g. +51999999999")
    parser.add_argument("--url", default="http://127.0.0.1:8000", help="Field Shift base URL")
    args = parser.parse_args()

    key = load_verify_key()
    verified_at = int(time.time())
    event_id = f"dev-{uuid4()}"
    message = f"{args.challenge}\n{args.phone}\n{verified_at}\n{event_id}".encode("utf-8")
    signature = hmac.new(key.encode("utf-8"), message, hashlib.sha256).hexdigest()

    payload = json.dumps(
        {
            "challenge": args.challenge,
            "phone": args.phone,
            "verified_at": verified_at,
            "event_id": event_id,
            "signature": signature,
        }
    ).encode("utf-8")

    req = urllib.request.Request(
        f"{args.url.rstrip('/')}/v1/onboarding/confirm",
        data=payload,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=10) as resp:
        data = json.loads(resp.read().decode("utf-8"))
        print(f"claim_code: {data['claim_code']}")
        print(f"expires_in_seconds: {data['expires_in_seconds']}")


if __name__ == "__main__":
    main()
