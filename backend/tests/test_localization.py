"""Both languages describe the same numeric evidence and preserve uncertainty."""

from pathlib import Path
from unittest.mock import patch

from fastapi.testclient import TestClient

from app.correlation import HistoricalCorrelationSummary
from app.forecast_alerts import WeeklyAlertSummary
from app.main import app
from app.nasa_power import _synthetic_climatology
from app.schemas import TerrainInfo


def fixtures():
    terrain = TerrainInfo(elevation_m=3200, slope_pct=5.0, erosion_risk='moderate',
                          terrain_source='test DEM', live_terrain_data=True)
    alerts = WeeklyAlertSummary(live_forecast=True, source='test forecast', rain_expected=False,
                                storm_expected=False, frost_expected=False, alert_level='ok',
                                farmer_alert_message='Sin tormentas', days=[])
    history = HistoricalCorrelationSummary(
        live_historical=True, years_analyzed=20, temp_trend_c_per_decade=0.31,
        precip_change_pct_recent_5y=-5.2, rain_soil_moisture_corr=0.42,
        drought_years_last_20=3, sentinel2_clear_scenes_12m=5,
        latest_ndvi=0.38, latest_ndvi_date='2026-10-15',
        vegetation_status='cobertura_baja_o_estres',
        correlation_interpretation_farmer='La lluvia y la humedad varían juntas.',
        correlation_interpretation_agronomist='Correlación de Pearson r=0.42.',
    )
    return (_synthetic_climatology(-12.065), True, 'NASA POWER test fixture', terrain), alerts, history


def test_spanish_and_english_keep_evidence_and_different_text() -> None:
    environment, alerts, history = fixtures()
    client = TestClient(app)
    payload = {'lat': -12.065, 'lon': -75.205, 'current_crop': 'potato_continuous',
               'irrigation': 'rainfed', 'priority': 'soil_health', 'soil_texture': 'unknown'}
    with (patch('app.main.fetch_parcel_environment', return_value=environment),
          patch('app.main.fetch_weekly_alerts', return_value=alerts),
          patch('app.main.fetch_historical_correlation', return_value=history)):
        es = client.post('/v1/rotations/compare', json={**payload, 'language': 'es'})
        en = client.post('/v1/rotations/compare', json={**payload, 'language': 'en'})
        capability = client.post('/v1/kusi/capability', json={**payload, 'language': 'en'})
        diagnosis = client.post('/v1/parcel/diagnose', json={**payload, 'language': 'en'})
    assert all(response.status_code == 200 for response in (es, en, capability, diagnosis))
    spanish, english = es.json(), en.json()
    assert spanish['diagnosis']['annual_precip_mm'] == english['diagnosis']['annual_precip_mm']
    assert [s['fit_score'] for s in spanish['strategies']] == [s['fit_score'] for s in english['strategies']]
    assert english['diagnosis']['monthly'][0]['month_label'] == 'Jan'
    assert 'dry months' in english['diagnosis']['farmer_summary']
    assert 'correlation' in english['diagnosis']['historical_correlation']['correlation_interpretation_farmer'].lower()
    assert 'local agronomic review' in english['human_judgment_reminder']
    assert all('soil' in s['why_suggested_farmer'].lower() or 'crop' in s['why_suggested_farmer'].lower()
               for s in english['strategies'])
    assert capability.json()['raw_comparison']['strategies'][0]['name'] == english['strategies'][0]['name']
    assert diagnosis.json()['monthly'][0]['month_label'] == 'Jan'


def test_ui_has_language_control_and_evidence_panel() -> None:
    html = (Path(__file__).parents[1] / 'app/static/index.html').read_text()
    assert 'id="languageSelect"' in html
    assert 'id="workspaceLeft"' in html
    assert 'Sentinel-2 NDVI 10 m' in html
    assert 'historyOk ? `r = ' in html
