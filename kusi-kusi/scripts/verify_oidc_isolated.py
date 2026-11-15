"""Run the complete OIDC protocol suite against a fresh PostgreSQL, without .env."""
import argparse
import json
from pathlib import Path
import subprocess

from local_acceptance import ROOT, postgres


def verify():
    with postgres() as (env, temporary):
        subprocess.run(["python3",str(ROOT / "scripts/verify_oidc.py")],env=env,cwd=temporary,check=True,timeout=120)
    return {"status":"passed","checks":["existing verify_oidc.py complete positive and negative protocol suite"],"external_calls":False,"existing_database_used":False}


if __name__ == "__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--result",type=Path,required=True)
    args=parser.parse_args()
    result=verify()
    args.result.write_text(json.dumps(result,indent=2)+"\n")
    print(json.dumps(result,indent=2))
