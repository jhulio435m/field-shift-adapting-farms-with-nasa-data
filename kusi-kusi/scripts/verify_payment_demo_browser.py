"""Isolated desktop/mobile browser acceptance for the loopback payment demo."""
import json, os, secrets, subprocess, sys, tempfile, time, urllib.error, urllib.parse, urllib.request, uuid
from pathlib import Path

from playwright.sync_api import expect, sync_playwright

ROOT = Path(__file__).resolve().parent.parent
os.chdir(ROOT)
env = dict(os.environ)
if Path(".env").exists():
    env.update(
        dict(
            line.split("=", 1)
            for line in Path(".env").read_text().splitlines()
            if "=" in line and not line.startswith("#")
        )
    )
database_url = urllib.parse.urlsplit(env["DATABASE_URL"])
pg = dict(
    os.environ,
    PGHOST=database_url.hostname or "127.0.0.1",
    PGPORT=str(database_url.port or 5432),
    PGDATABASE=database_url.path.lstrip("/"),
    PGUSER=urllib.parse.unquote(database_url.username or ""),
    PGPASSWORD=urllib.parse.unquote(database_url.password or ""),
)
pg.pop("PGOPTIONS", None)
schema = "payment_browser_" + uuid.uuid4().hex
subprocess.run(
    ["psql", "-X", "-v", "ON_ERROR_STOP=1", "-c", f"CREATE SCHEMA {schema}"],
    env=pg,
    check=True,
    stdout=subprocess.DEVNULL,
)
query = dict(urllib.parse.parse_qsl(database_url.query))
query["options"] = f"-csearch_path={schema}"
env["DATABASE_URL"] = urllib.parse.urlunsplit(database_url._replace(query=urllib.parse.urlencode(query)))
api_origin = "http://127.0.0.1:18089"
web_origin = "http://127.0.0.1:18088"
env.update(
    KUSI_ADMIN_API_KEY=secrets.token_hex(32),
    KUSI_LOCAL_MODE="true",
    KUSI_CREATIVE_SIMULATION="true",
    KUSI_CREATIVE_WORKER="true",
    KUSI_OAUTH_REFRESH_WORKER="false",
    KUSI_LOCAL_ORIGIN=web_origin,
    KUSI_LOCAL_STORAGE="",
    KUSI_API_ORIGIN=api_origin,
    BIND_ADDRESS="127.0.0.1:18089",
    KUSI_CHANNELS="{}",
    KUSI_LLM_PROFILES="{}",
    KUSI_AUDIO_PROFILES="{}",
    KUSI_CREATIVE_RECIPES="{}",
    KUSI_HTTP_DESTINATIONS="{}",
    KUSI_OAUTH_PROFILES="{}",
    HOSTNAME="127.0.0.1",
    PORT="18088",
    NEXT_TELEMETRY_DISABLED="1",
)
api = web = None


def wait_for(url, process, timeout=40):
    deadline = time.time() + timeout
    while time.time() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"service exited before readiness: {url}")
        try:
            with urllib.request.urlopen(url, timeout=1) as response:
                if response.status < 500:
                    return
        except (urllib.error.URLError, TimeoutError):
            time.sleep(0.25)
    raise TimeoutError(f"service did not become ready: {url}")


try:
    with tempfile.TemporaryDirectory(prefix="kusi-payment-browser-") as temp:
        env["KUSI_LOCAL_STORAGE"] = str(Path(temp) / "storage")
        log = open(Path(temp) / "services.log", "w+")
        api = subprocess.Popen(["target/debug/kusi-server"], env=env, stdout=log, stderr=log)
        wait_for(api_origin + "/health/ready", api)
        web = subprocess.Popen(
            ["npm", "run", "start", "--prefix", "web/admin"], env=env, stdout=log, stderr=log
        )
        wait_for(web_origin + "/tramites", web)
        if "--serve" in sys.argv:
            print(
                json.dumps(
                    {
                        "status": "ready",
                        "url": web_origin + "/tramites",
                        "mode": "synthetic loopback demo",
                        "stop": "Ctrl+C removes the temporary database schema and files",
                    },
                    ensure_ascii=False,
                ),
                flush=True,
            )
            while api.poll() is None and web.poll() is None:
                time.sleep(1)
            raise RuntimeError("local demo service exited unexpectedly")
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(
                executable_path="/usr/bin/chromium", headless=True, args=["--no-sandbox"]
            )
            for label, viewport in (
                ("desktop", {"width": 1440, "height": 1000}),
                ("mobile", {"width": 390, "height": 844}),
            ):
                page = browser.new_page(viewport=viewport)
                errors = []
                page.on("pageerror", lambda error: errors.append(str(error)))
                def delay_history(route):
                    if route.request.method == "GET":
                        time.sleep(0.8)
                    route.continue_()

                page.route("**/v1/demo/payments", delay_history)
                page.goto(web_origin + "/tramites", wait_until="domcontentloaded")
                expect(page.get_by_role("heading", name="Consulta tus pagos")).to_be_visible()
                expect(page.get_by_text("Consultando la plataforma demo…")).to_be_visible()
                expect(page.get_by_text("UNCP-EXP-004821", exact=True)).to_have_count(0)
                expect(page.get_by_text("UNCP-EVAL-006173", exact=True)).to_have_count(0)
                page.get_by_text("Consultar por DNI", exact=True).click()
                page.get_by_label("Consultar historial por DNI de prueba").fill("ab12cd34")
                expect(page.get_by_label("Consultar historial por DNI de prueba")).to_have_value("1234")
                page.get_by_label("Consultar historial por DNI de prueba").fill("70000001")
                page.get_by_role("button", name="Consultar", exact=True).click()
                expect(page.get_by_text("DNI 70***001")).to_be_visible()
                page.get_by_label("Consultar historial por DNI de prueba").fill("70000002")
                page.get_by_role("button", name="Consultar", exact=True).click()
                expect(page.get_by_text("Este estudiante no tiene pagos registrados.", exact=True)).to_be_visible()
                page.get_by_label("Consultar historial por DNI de prueba").fill("70000003")
                page.get_by_role("button", name="Consultar", exact=True).click()
                expect(page.get_by_text("DNI no encontrado. Verifica los ocho dígitos e intenta nuevamente.", exact=True)).to_be_visible()
                page.get_by_role("button", name="Cerrar", exact=True).click()
                page.get_by_label("Código del pago o recibo").fill("UNCP-EXP-999999")
                page.get_by_role("button", name="Consultar pago").click()
                expect(page.get_by_text("No se encontró el recurso.", exact=True)).to_be_visible()
                page.get_by_label("Código del pago o recibo").fill("UNCP-EXP-004821")
                page.get_by_role("button", name="Consultar pago").click()
                expect(page.get_by_text("Pago encontrado.", exact=True)).to_be_visible()
                expect(page.get_by_text("Pago confirmado", exact=True).first).to_be_visible()
                page.get_by_role("button", name="Cerrar", exact=True).click()
                page.get_by_role("button", name="Subir foto", exact=True).click()
                page.get_by_label("O sube una foto del recibo").set_input_files({"name":"video.mp4","mimeType":"video/mp4","buffer":b"video"})
                expect(page.get_by_text("Solo se permiten fotos JPG, PNG o WebP de hasta 5 MB. No se permiten videos.", exact=True)).to_be_visible()
                page.get_by_label("O sube una foto del recibo").set_input_files({"name":"video.png","mimeType":"image/png","buffer":b"not-an-image"})
                expect(page.get_by_text("El archivo no es una foto válida. No se permiten videos; usa JPG, PNG o WebP.", exact=True)).to_be_visible()
                page.get_by_label("O sube una foto del recibo").set_input_files(
                    str(ROOT / "web/admin/public/demo/recibo-sintetico.png")
                )
                expect(page.get_by_text("Comprobante leído.")).to_be_visible(timeout=15000)
                expect(page.get_by_role("dialog")).to_be_visible()
                assert not errors, errors
                page.screenshot(path=f"/tmp/kusi-payments-{label}.png", full_page=True)
                page.keyboard.press("Escape")
                expect(page.get_by_role("dialog")).to_have_count(0)
                assert page.evaluate("document.documentElement.scrollHeight <= window.innerHeight + 2"), "landing should fit viewport"
                page.screenshot(path=f"/tmp/kusi-payments-home-{label}.png", full_page=True)
                page.close()
            browser.close()
        print(json.dumps({"status": "passed", "checks": ["no-login page and synthetic history", "fixed fake-DNI lookup", "payment code validation", "synthetic photo OCR", "desktop and mobile Chromium"]}, ensure_ascii=False))
        log.close()
finally:
    for process in (web, api):
        if process is not None and process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
    subprocess.run(
        ["psql", "-X", "-v", "ON_ERROR_STOP=1", "-c", f"DROP SCHEMA IF EXISTS {schema} CASCADE"],
        env=pg,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
