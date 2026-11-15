"""Private disposable loopback infrastructure for acceptance, never reads .env."""
from contextlib import contextmanager
import getpass
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import urllib.parse

ROOT = Path(__file__).resolve().parent.parent


def clean_environment():
    return {key: value for key, value in os.environ.items()
            if not key.startswith(("KUSI_", "PG", "GROQ_", "META_", "OPENAI_"))
            and key not in {"DATABASE_URL", "TEST_DATABASE_URL", "BIND_ADDRESS", "RUST_LOG"}}


def free_port():
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return listener.getsockname()[1]


@contextmanager
def postgres():
    env = clean_environment()
    with tempfile.TemporaryDirectory(prefix="kusi-local-acceptance-") as directory:
        root = Path(directory)
        data, socket_dir = root / "postgres", root / "socket"
        socket_dir.mkdir(mode=0o700)
        port = free_port()
        with (root / "init.log").open("w") as log:
            subprocess.run(["initdb", "-D", str(data), "--auth=trust", "--no-locale", "--encoding=UTF8"],
                           env=env, stdout=log, stderr=log, check=True, timeout=30)
        try:
            subprocess.run(["pg_ctl", "-D", str(data), "-l", str(root / "postgres.log"),
                            "-o", f"-h 127.0.0.1 -p {port} -k {socket_dir}", "-w", "start"],
                           env=env, check=True, stdout=subprocess.DEVNULL, timeout=30)
            database_url = f"postgres://{urllib.parse.quote(getpass.getuser(), safe='')}@127.0.0.1:{port}/postgres"
            env.update(DATABASE_URL=database_url, TEST_DATABASE_URL=database_url,
                       KUSI_TEST_NO_DOTENV="true")
            yield env, root
        finally:
            if (data / "postmaster.pid").exists():
                subprocess.run(["pg_ctl", "-D", str(data), "-m", "fast", "-w", "stop"],
                               env=env, check=True, stdout=subprocess.DEVNULL, timeout=30)


def sql(env, statement):
    url = urllib.parse.urlsplit(env["DATABASE_URL"])
    if url.hostname != "127.0.0.1":
        raise RuntimeError("disposable loopback database required")
    pg = dict(env, PGHOST=url.hostname, PGPORT=str(url.port), PGDATABASE=url.path.lstrip("/"),
              PGUSER=urllib.parse.unquote(url.username or ""))
    return subprocess.check_output(["psql", "-X", "-At", "-v", "ON_ERROR_STOP=1", "-c", statement],
                                   env=pg, text=True, stderr=subprocess.PIPE).strip()
