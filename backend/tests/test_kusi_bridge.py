"""The adaptive source never takes parcel scope from a model request."""

import json
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import pytest

from app.kusi_bridge import Forbidden, Stale, execute, load_config


def config(tmp_path: Path) -> dict:
    path = tmp_path / 'parcel.json'
    path.write_text(json.dumps({
        'protocol': 'field_shift_loopback', 'enabled': True,
        'label': 'Parcela de prueba',
        'endpoint': 'http://127.0.0.1:8000/v1/kusi/capability',
        'parcel': {'lat': -12.065, 'lon': -75.205, 'current_crop': 'potato_continuous'},
    }))
    path.chmod(0o600)
    return load_config(path)


def report():
    months = [SimpleNamespace(month=i, precip_mm_month=40.0 + i,
                              evapotranspiration_mm_month=35.0, water_balance_mm=5.0 + i,
                              temp_mean_c=12.0, soil_moisture_top=0.4,
                              frost_risk=False, dry_stress=False) for i in range(1, 13)]
    strategies = [
        SimpleNamespace(name=name, fit_score=score, water_demand_level=demand,
                        is_current=current)
        for name, score, demand, current in [
            ('Papa continua', 40, 'high', True),
            ('Papa y habas', 75, 'medium', False),
            ('Papa y quinua', 80, 'low', False),
        ]
    ]
    return SimpleNamespace(raw_comparison=SimpleNamespace(
        diagnosis=SimpleNamespace(monthly=months), strategies=strategies))


def test_discover_read_and_typed_filters(tmp_path: Path) -> None:
    source = config(tmp_path)
    with patch('app.kusi_bridge.fetch', return_value=report()) as fetch:
        catalog = execute(source, {'op': 'discover'})
        assert [r['id'] for r in catalog['resources']] == ['climate_months', 'rotation_options']
        answer = execute(source, {
            'op': 'read', 'version': catalog['version'], 'resource': 'rotation_options',
            'fields': ['strategy', 'fit_score'],
            'filters': [{'field': 'water_demand_level', 'op': 'eq', 'value': 'low'}],
            'limit': 10,
        })
    assert answer == {'rows': [{'strategy': 'Papa y quinua', 'fit_score': 80}], 'truncated': False}
    assert fetch.call_count == 2
    assert source['parcel']['lat'] == -12.065


def test_scope_version_and_invalid_inputs_fail_closed(tmp_path: Path) -> None:
    source = config(tmp_path)
    with patch('app.kusi_bridge.fetch', return_value=report()):
        catalog = execute(source, {'op': 'discover'})
        base = {'op': 'read', 'version': catalog['version'], 'resource': 'climate_months',
                'fields': ['month'], 'filters': [], 'limit': 12}
        with pytest.raises(Stale):
            execute(source, {**base, 'version': 'old'})
        with pytest.raises(Forbidden):
            execute(source, {**base, 'lat': 0})
        with pytest.raises(Forbidden):
            execute(source, {**base, 'fields': ['password']})
        with pytest.raises(Forbidden):
            execute(source, {**base, 'limit': True})
        with pytest.raises(Forbidden):
            execute(source, {**base, 'filters': [
                {'field': 'frost_risk', 'op': 'eq', 'value': 1}]})


def test_private_file_and_loopback_only(tmp_path: Path) -> None:
    source = config(tmp_path)
    path = tmp_path / 'parcel.json'
    path.chmod(0o644)
    with pytest.raises(Forbidden):
        load_config(path)
    path.chmod(0o600)
    for endpoint in ('https://example.org/v1/kusi/capability',
                     'http://localhost:8000/v1/kusi/capability',
                     'http://127.0.0.1:8000/v1/rotations/compare'):
        modified = {**source, 'endpoint': endpoint}
        path.write_text(json.dumps(modified))
        with pytest.raises(Forbidden):
            load_config(path)
