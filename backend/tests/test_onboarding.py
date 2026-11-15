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


def test_telegram_only_demo_mode_and_parcel_metadata(tmp_path, monkeypatch) -> None:
    key = 'synthetic-verifier-key-at-least-thirty-two-characters'
    monkeypatch.setenv('FIELD_SHIFT_DB', str(tmp_path / 'farmers.sqlite3'))
    monkeypatch.setenv('FIELD_SHIFT_KUSI_VERIFY_KEY', key)
    monkeypatch.setenv('FIELD_SHIFT_DEMO_MODE', '1')
    client = TestClient(app)

    started = client.post('/v1/onboarding/start', json={
        'name': 'Productora Telegram',
        'phone': '',
        'preferred_channel': 'telegram',
        'telegram_handle': '@agro_mantaro',
        'bot_audience': 'agronomist',
        'alert_frequency': 'weekly',
    })
    assert started.status_code == 200
    body = started.json()
    assert body['demo_mode'] is True
    assert client.post('/v1/onboarding/claim', json={'code': body['claim_code']}).status_code == 200

    me = client.get('/v1/me').json()
    assert me['bot_config']['preferred_channel'] == 'telegram'
    assert me['bot_config']['telegram_handle'] == '@agro_mantaro'
    assert me['bot_config']['bot_audience'] == 'agronomist'

    saved = client.post('/v1/parcels', json={
        'name': 'Lote con hectareas',
        'lat': -12.065,
        'lon': -75.205,
        'current_crop': 'potato_continuous',
        'irrigation': 'rainfed',
        'priority': 'soil_health',
        'soil_texture': 'loam',
        'has_livestock': False,
        'area_ha': 2.5,
        'planting_month': 10,
    }, headers={'X-CSRF-Token': me['csrf']})
    assert saved.status_code == 200
    assert saved.json()['area_ha'] == 2.5
    assert saved.json()['planting_month'] == 10


def test_demo_onboarding_seeds_parcels_and_supports_crud_and_logout(tmp_path, monkeypatch) -> None:
    monkeypatch.setenv('FIELD_SHIFT_DB', str(tmp_path / 'farmers.sqlite3'))
    monkeypatch.delenv('FIELD_SHIFT_KUSI_VERIFY_KEY', raising=False)
    client = TestClient(app)

    demo_resp = client.post('/v1/onboarding/demo', json={'seed_parcels': True, 'bot_language': 'es'})
    assert demo_resp.status_code == 200
    assert demo_resp.json()['demo_mode'] is True
    csrf = demo_resp.json()['csrf']

    me_resp = client.get('/v1/me')
    assert me_resp.status_code == 200
    parcels = me_resp.json()['parcels']
    assert len(parcels) == 2

    target_id = parcels[0]['id']
    update_resp = client.put(
        f'/v1/parcels/{target_id}',
        json={
            'name': 'Lote Mantaro Actualizado',
            'lat': -12.07,
            'lon': -75.21,
            'current_crop': 'maize_continuous',
            'irrigation': 'limited',
            'priority': 'water',
            'soil_texture': 'loam',
            'has_livestock': False,
            'area_ha': 3.2,
            'planting_month': 9,
        },
        headers={'X-CSRF-Token': csrf},
    )
    assert update_resp.status_code == 200
    assert update_resp.json()['name'] == 'Lote Mantaro Actualizado'
    assert update_resp.json()['current_crop'] == 'maize_continuous'

    delete_resp = client.delete(f'/v1/parcels/{target_id}', headers={'X-CSRF-Token': csrf})
    assert delete_resp.status_code == 200
    assert delete_resp.json() == {'status': 'deleted', 'id': target_id}
    assert len(client.get('/v1/me').json()['parcels']) == 1

    logout_resp = client.post('/v1/onboarding/logout')
    assert logout_resp.status_code == 200
    assert client.get('/v1/me').status_code == 401
