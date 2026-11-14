"""Global multi-continent validation tests against live NASA POWER, DEM/SRTM, and 7-day forecast."""

import pytest
from fastapi.testclient import TestClient

from app.main import app

client = TestClient(app)

GLOBAL_FARMS = [
    ("Peru (Andes - Mantaro)", -12.065, -75.205, "potato_continuous", "rainfed", "soil_health"),
    ("USA (Great Plains - Kansas)", 38.500, -98.000, "wheat_continuous", "rainfed", "water"),
    ("Spain (Andalucia)", 37.389, -5.984, "wheat_continuous", "limited", "water"),
    ("Kenya (Rift Valley)", -0.283, 36.067, "maize_continuous", "rainfed", "pest_disruption"),
    ("India (Punjab)", 30.901, 75.857, "rice_continuous", "irrigated", "water"),
    ("Australia (Darling Downs)", -27.560, 151.954, "wheat_continuous", "rainfed", "forage_income"),
]


@pytest.mark.parametrize("region,lat,lon,crop,irrigation,priority", GLOBAL_FARMS)
def test_global_agricultural_regions(
    region: str,
    lat: float,
    lon: float,
    crop: str,
    irrigation: str,
    priority: str,
) -> None:
    resp = client.post(
        "/v1/rotations/compare",
        json={
            "lat": lat,
            "lon": lon,
            "current_crop": crop,
            "irrigation": irrigation,
            "priority": priority,
        },
    )
    assert resp.status_code == 200
    data = resp.json()
    diag = data["diagnosis"]
    strategies = data["strategies"]

    assert diag["live_nasa_data"] is True, f"NASA POWER fallback used for {region}"
    assert diag["terrain"]["live_terrain_data"] is True, f"Terrain fallback used for {region}"
    assert diag["weekly_alerts"]["live_forecast"] is True, f"Forecast fallback used for {region}"
    assert len(strategies) >= 3

    best = next(s for s in strategies if not s["is_current"])
    if "Mantaro" in region:
        all_names = " ".join(s["name"] + " " + " ".join(s["sequence"]) for s in strategies)
        assert "Quinua" in all_names and "Habas" in all_names
        assert "Sorgo" not in all_names

    print(
        f"\n[{region}] Alt={diag['terrain']['elevation_m']}m "
        f"Slope={diag['terrain']['slope_pct']}%({diag['terrain']['erosion_risk']}) | "
        f"P={diag['annual_precip_mm']}mm ET={diag['annual_et_mm']}mm T={diag['annual_temp_mean_c']}C | "
        f"Alert7d={diag['weekly_alerts']['alert_level']} | "
        f"Top={best['name']} ({best['fit_score']}/100)"
    )
