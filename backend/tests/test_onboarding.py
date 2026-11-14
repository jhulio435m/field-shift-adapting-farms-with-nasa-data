"""Verified number and parcel ownership through the actual HTTP routes."""

import hashlib
import hmac
import stat
import time
from unittest.mock import patch

from fastapi.testclient import TestClient
import pytest

from app.main import app


def sign(challenge: str, phone: str, when: int, event: str, key: str) -> str:
    message = f"{challenge}\n{phone}\n{when}\n{event}".encode()
    return hmac.new(key.encode(), message, hashlib.sha256).hexdigest()


def verified_client(phone: str, key: str) -> tuple[TestClient, str]:
    client = TestClient(app)
    started = client.post('/v1/onboarding/start', json={'name': 'Agricultora prueba', 'phone': phone})
    assert started.status_code == 200
    challenge = started.json()['challenge']
    when = int(time.time())
    proof = {'challenge': challenge, 'phone': phone, 'verified_at': when,
             'event_id': 'wa-test-1', 'signature': sign(challenge, phone, when, 'wa-test-1', key)}
    assert client.post('/v1/onboarding/confirm', json={**proof, 'signature': 'bad'}).status_code == 403
    confirmed = client.post('/v1/onboarding/confirm', json=proof)
    assert confirmed.status_code == 200
    assert client.post('/v1/onboarding/confirm', json=proof).status_code == 403
    code = confirmed.json()['claim_code']
    assert client.post('/v1/onboarding/claim', json={'code': code}).status_code == 200
    assert client.post('/v1/onboarding/claim', json={'code': code}).status_code == 403
    return client, client.get('/v1/me').json()['csrf']


def test_verified_profile_multiple_parcels_and_isolation(tmp_path, monkeypatch) -> None:
    key = 'synthetic-verifier-key-at-least-thirty-two-characters'
    monkeypatch.setenv('FIELD_SHIFT_DB', str(tmp_path / 'farmers.sqlite3'))
    monkeypatch.setenv('FIELD_SHIFT_KUSI_VERIFY_KEY', key)
    first, csrf = verified_client('+51911111111', key)
    parcel = {'name': 'Lote A', 'lat': -12.065, 'lon': -75.205,
              'current_crop': 'potato_continuous', 'irrigation': 'rainfed',
              'priority': 'soil_health', 'soil_texture': 'unknown', 'has_livestock': False}
    assert first.post('/v1/parcels', json=parcel).status_code == 403
    a = first.post('/v1/parcels', json=parcel, headers={'X-CSRF-Token': csrf})
    b = first.post('/v1/parcels', json={**parcel, 'name': 'Lote B', 'lat': -13.0},
                   headers={'X-CSRF-Token': csrf})
    assert a.status_code == b.status_code == 200
    assert len(first.get('/v1/me').json()['parcels']) == 2
    assert stat.S_IMODE((tmp_path / 'farmers.sqlite3').stat().st_mode) == 0o600
    returning, new_csrf = verified_client('+51911111111', key)
    assert len(returning.get('/v1/me').json()['parcels']) == 2
    assert first.get('/v1/me').status_code == 401
    second, second_csrf = verified_client('+51922222222', key)
    denied = second.post(f"/v1/parcels/{a.json()['id']}/analyze", json={},
                         headers={'X-CSRF-Token': second_csrf})
    assert denied.status_code == 404
    with patch('app.main.compare_parcel_rotations') as compare:
        compare.side_effect = RuntimeError('engine reached')
        # The route reaches the engine with the stored parcel, not user-supplied coordinates.
        with pytest.raises(RuntimeError, match='engine reached'):
            returning.post(f"/v1/parcels/{a.json()['id']}/analyze", json={'rain_delta_pct': -20},
                           headers={'X-CSRF-Token': new_csrf})
        assert compare.await_args.args[0].lat == parcel['lat']
        assert compare.await_args.args[0].rain_delta_pct == -20


def test_unconfigured_verification_stays_closed(tmp_path, monkeypatch) -> None:
    monkeypatch.setenv('FIELD_SHIFT_DB', str(tmp_path / 'farmers.sqlite3'))
    monkeypatch.delenv('FIELD_SHIFT_KUSI_VERIFY_KEY', raising=False)
    client = TestClient(app)
    assert client.post('/v1/onboarding/start', json={
        'name': 'Prueba', 'phone': '+51933333333'}).status_code == 503
