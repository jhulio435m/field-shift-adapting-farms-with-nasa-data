"""Sentinel point readings use a recent bounded scene and reject invalid pixels."""

import asyncio
import datetime
from unittest.mock import patch

from app.correlation import _fetch_sentinel2_ndvi


class Response:
    def __init__(self, payload):
        self.payload = payload

    def raise_for_status(self):
        pass

    def json(self):
        return self.payload


class Client:
    def __init__(self, values):
        self.values = values
        self.search = None

    async def __aenter__(self):
        return self

    async def __aexit__(self, *_):
        pass

    async def post(self, _url, json):
        self.search = json
        return Response({'features': [{
            'id': 'synthetic-scene', 'properties': {'datetime': '2026-11-01T00:00:00Z'},
        }]})

    async def get(self, _url, params):
        assert params['assets'] == ['B04', 'B08']
        return Response({'values': self.values})


def test_latest_year_window_and_finite_ndvi() -> None:
    client = Client([20, 60])
    with patch('app.correlation.httpx.AsyncClient', return_value=client):
        count, ndvi, date, status = asyncio.run(_fetch_sentinel2_ndvi(-12.065, -75.205))
    assert (count, ndvi, date, status) == (1, 0.5, '2026-11-01', 'vegetacion_vigorosa')
    start, end = client.search['datetime'].split('/')
    today = datetime.datetime.now(datetime.timezone.utc).date()
    assert end.startswith(today.isoformat())
    assert start.startswith((today - datetime.timedelta(days=365)).isoformat())


def test_invalid_pixel_is_not_reported_as_ndvi() -> None:
    client = Client([float('nan'), 60])
    with patch('app.correlation.httpx.AsyncClient', return_value=client):
        count, ndvi, _date, status = asyncio.run(_fetch_sentinel2_ndvi(-12.065, -75.205))
    assert count == 1
    assert ndvi is None and status == 'sin_dato'
