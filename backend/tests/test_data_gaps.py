"""Regression checks for missing data and freezing temperatures."""

import asyncio
from unittest.mock import patch

from fastapi.testclient import TestClient

from app.forecast_alerts import fetch_weekly_alerts
from app.main import app
from app.schemas import TerrainInfo


def test_no_nasa_data_returns_503() -> None:
    terrain = TerrainInfo(elevation_m=None, slope_pct=None, erosion_risk="unknown",
                          terrain_source="Sin datos", live_terrain_data=False)
    with patch("app.main.fetch_parcel_environment", return_value=({}, False, "NASA POWER no disponible", terrain)):
        response = TestClient(app).post("/v1/rotations/compare", json={"lat": -12.065, "lon": -75.205})
    assert response.status_code == 503
    assert "datos sinteticos" in response.json()["detail"]


def test_zero_degrees_is_frost() -> None:
    class Response:
        def raise_for_status(self):
            pass

        def json(self):
            return {"daily": {"time": ["2026-11-14"], "weather_code": [0],
                              "temperature_2m_max": [12], "temperature_2m_min": [0],
                              "precipitation_sum": [0], "precipitation_probability_max": [0]}}

    class Client:
        async def __aenter__(self):
            return self

        async def __aexit__(self, *_):
            pass

        async def get(self, *_args, **_kwargs):
            return Response()

    with patch("app.forecast_alerts.httpx.AsyncClient", return_value=Client()):
        result = asyncio.run(fetch_weekly_alerts(-12.065, -75.205))
    assert result.frost_expected
    assert result.days[0].temp_min_c == 0.0
