#!/usr/bin/env python3
import json
import os
from datetime import datetime, timezone
from pathlib import Path

MATRIX_ARTIFACT_ENV_VAR = "TYPEMILL_MATRIX_ARTIFACT_DIR"
DEFAULT_ARTIFACT_DIR = "perf-artifacts"
PERF_HISTORY_SCHEMA_VERSION = 1

ARTIFACT_DIR = Path(os.environ.get(MATRIX_ARTIFACT_ENV_VAR, DEFAULT_ARTIFACT_DIR))
HISTORY_DIR = Path(".perf-history")
HISTORY_FILE = HISTORY_DIR / "perf_history.json"


def load_json(path: Path):
    try:
        return json.loads(path.read_text())
    except Exception:
        return None


def main():
    HISTORY_DIR.mkdir(parents=True, exist_ok=True)
    history = load_json(HISTORY_FILE) or {
        "schema_version": PERF_HISTORY_SCHEMA_VERSION,
        "runs": [],
    }

    run_record = {
        "schema_version": PERF_HISTORY_SCHEMA_VERSION,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "artifacts": {},
    }

    for path in sorted(ARTIFACT_DIR.glob("*_matrix.json")):
        data = load_json(path)
        if not data:
            continue
        run_record["artifacts"][path.name] = {
            "schema_version": data.get("schema_version", PERF_HISTORY_SCHEMA_VERSION),
            "project": data.get("project"),
            "profile": data.get("profile"),
            "verify_every": data.get("verify_every"),
            "threshold_exceedances": data.get("threshold_exceedances", []),
            "run_timings": data.get("run_timings", {}),
            "results": data.get("results", []),
        }

    history["schema_version"] = PERF_HISTORY_SCHEMA_VERSION
    history.setdefault("runs", []).append(run_record)
    history["runs"] = history["runs"][-30:]

    HISTORY_FILE.write_text(json.dumps(history, indent=2))
    print(f"Updated {HISTORY_FILE} with {len(history['runs'])} runs")


if __name__ == "__main__":
    main()
