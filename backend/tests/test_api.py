"""Automated tests for Field Shift NASA Geo-Agro API."""

from fastapi.testclient import TestClient

from app.main import app

client = TestClient(app)


def test_health() -> None:
    resp = client.get("/health")
    assert resp.status_code == 200
    assert resp.json()["status"] == "ok"


def test_diagnose_parcel_includes_terrain_et_and_moisture() -> None:
    resp = client.post(
        "/v1/parcel/diagnose",
        json={
            "lat": -12.065,
            "lon": -75.205,
            "current_crop": "maize_continuous",
            "soil_texture": "loam",
        },
    )
    assert resp.status_code == 200
    data = resp.json()
    assert len(data["monthly"]) == 12
    assert "terrain" in data
    assert "slope_pct" in data["terrain"]
    assert data["annual_et_mm"] > 0
    assert 0.0 <= data["avg_soil_moisture_top"] <= 1.0
    assert data["farmer_summary"]
    assert data["agronomist_summary"]
    assert "no reemplaza" in data["disclaimer"].lower()


def test_compare_rotations_returns_at_least_three_strategies() -> None:
    resp = client.post(
        "/v1/rotations/compare",
        json={
            "lat": -12.065,
            "lon": -75.205,
            "current_crop": "potato_continuous",
            "irrigation": "rainfed",
            "priority": "soil_health",
            "soil_texture": "sandy",
            "has_livestock": True,
        },
    )
    assert resp.status_code == 200
    data = resp.json()
    strategies = data["strategies"]
    assert len(strategies) >= 3
    assert any(s["is_current"] for s in strategies)
    for s in strategies:
        assert s["why_suggested_farmer"]
        assert s["why_suggested_agronomist"]
        assert len(s["evidence"]) >= 1
        assert len(s["benefits"]) >= 1
        assert len(s["information_gaps"]) >= 1
        assert len(s["local_review_needed"]) >= 1
        assert len(s["monthly_calendar"]) == 12


def test_scenario_simulation_adjusts_climate() -> None:
    base = client.post(
        "/v1/scenarios/simulate",
        json={"lat": 38.5, "lon": -98.0, "rain_delta_pct": 0.0, "temp_delta_c": 0.0},
    ).json()
    dry_hot = client.post(
        "/v1/scenarios/simulate",
        json={"lat": 38.5, "lon": -98.0, "rain_delta_pct": -30.0, "temp_delta_c": 2.0},
    ).json()

    assert dry_hot["diagnosis"]["annual_precip_mm"] < base["diagnosis"]["annual_precip_mm"]
    assert dry_hot["diagnosis"]["annual_temp_mean_c"] > base["diagnosis"]["annual_temp_mean_c"]
    assert dry_hot["diagnosis"]["annual_et_mm"] >= base["diagnosis"]["annual_et_mm"]


def test_kusi_capability_endpoint() -> None:
    resp = client.post(
        "/v1/kusi/capability",
        json={
            "lat": -12.065,
            "lon": -75.205,
            "current_crop": "maize_continuous",
            "irrigation": "rainfed",
            "priority": "water",
            "audience": "farmer",
        },
    )
    assert resp.status_code == 200
    data = resp.json()
    assert data["strategies_count"] >= 3
    assert "summary_text" not in data
    assert data["raw_comparison"]["strategies"]


def test_weekly_alerts_and_telegram_webhook() -> None:
    alerts_resp = client.post("/v1/alerts/check", json={"lat": -12.065, "lon": -75.205})
    assert alerts_resp.status_code == 200
    alerts = alerts_resp.json()
    assert alerts["alert_level"] in ("ok", "rain", "frost", "storm")
    assert alerts["farmer_alert_message"]

    tg_resp = client.post(
        "/v1/telegram/webhook",
        json={
            "message": {
                "chat": {"id": 123456},
                "text": "Mi parcela esta en -12.065, -75.205",
            }
        },
    )
    assert tg_resp.status_code == 410
