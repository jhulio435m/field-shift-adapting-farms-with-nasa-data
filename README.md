# Field Shift: Adapting Farms with NASA Data

Decision support for farmers exploring climate-aware crop rotations. The app
combines NASA POWER observations, terrain information, recent Sentinel-2 point
NDVI, and a seven-day forecast with farmer priorities and local soil input.
It compares the current rotation with alternatives, explains evidence and gaps,
and keeps local agronomic review explicit. The interface and analysis are
available in Spanish and English.

This repository contains a prototype for the NASA Space Apps challenge of the
same name. It does **not** confirm planting decisions or replace field data.
If NASA POWER is unavailable, the API returns 503 instead of presenting
synthetic values as observations.

## Run locally

Requires Python 3.11+ and [`uv`](https://docs.astral.sh/uv/).

```sh
cd backend
uv sync --dev
uv run uvicorn app.main:app --host 127.0.0.1 --port 8000
```

Open `http://127.0.0.1:8000`. The API schema is at `/openapi.json`.

```sh
cd backend
uv run pytest -q
```

## Kusi Kusi and farmer registration

Field Shift includes a read-only adaptive source bridge and a parcel workflow.
Kusi Kusi remains responsible for verified WhatsApp identity and messaging.
The real channel callback, signed proof delivery, and source enrollment with
per-user access controls are **not yet connected**. Without a configured
`FIELD_SHIFT_KUSI_VERIFY_KEY`, registration fails closed. See
[the integration contract](specs/001-field-shift-mvp/kusi-integration.md) and
[remaining tasks](specs/001-field-shift-mvp/tasks.md).

Local farmer data are stored in `backend/.local-data/` and excluded from Git.
Do not put real phone numbers, secrets, or parcel records in this repository.
