"""Run the full pipeline on every row of test-data/test_cases.csv.

Saves raw logs to test-data/outputs/run_<timestamp>.jsonl (written row by row) and a
Markdown test table (test-data/outputs/test_table_<timestamp>.md).

The automatic check covers routing, intent, language, injection flag, marked item and
simulator verification. It does NOT judge explanation quality (bra-ket use, clarity,
whether a comparison covers both algorithms). Read the jsonl and confirm or downgrade
each Pass by hand before copying the table into the PDF. API/infrastructure errors are
reported as "Error", never as a model "Fail".
"""
from __future__ import annotations

import csv
import json
import re
import time
from datetime import datetime
from pathlib import Path

from workflow import MODEL, ROOT, models_used, run

TESTS = ROOT / "test-data" / "test_cases.csv"
OUT = ROOT / "test-data" / "outputs"


def auto_check(row: dict, log: dict) -> tuple[bool, str]:
    route = log.get("route") or {}
    ver = log.get("verification") or {}
    reasons = []
    if str(route.get("in_scope")).lower() != row["exp_in_scope"]:
        reasons.append(f"in_scope={route.get('in_scope')}")
    if route.get("algorithm") not in row["exp_algorithm"].split("|"):
        reasons.append(f"algorithm={route.get('algorithm')}")
    if row["exp_verified"] == "true" and not ver.get("ok"):
        reasons.append("verifier failed: " + "; ".join(ver.get("mismatches") or [])[:120])
    if row["exp_verified"] == "none" and ver.get("ok") is True:
        reasons.append("claimed verification for unsupported case")
    if row["exp_false_premise"] == "true" and not route.get("false_premise"):
        reasons.append("false premise not detected")
    if row.get("exp_intent") and route.get("intent") != row["exp_intent"]:
        reasons.append(f"intent={route.get('intent')}")
    if row.get("exp_language") and route.get("language") != row["exp_language"]:
        reasons.append(f"language={route.get('language')}")
    if row.get("exp_injection") == "true" and route.get("injection_attempt") is not True:
        reasons.append("injection not flagged")
    m = re.search(r"P\((\d{2})\)=1\.0", row["expected"])
    if m and row["exp_algorithm"] == "grover2" and (log.get("params_used") or {}).get("marked") != m.group(1):
        reasons.append(f"marked={(log.get('params_used') or {}).get('marked')}")
    return (not reasons), "; ".join(reasons) or "all automatic checks met"


def short(log: dict) -> str:
    route = log.get("route") or {}
    if not route.get("in_scope"):
        return (f"Redirected (algo={route.get('algorithm')}, injection={route.get('injection_attempt')})")
    ver = log.get("verification") or {}
    probs = (log.get("claim") or {}).get("probabilities")
    repaired = " after 1 repair" if log.get("verified_attempt_0") is False and ver.get("ok") else ""
    extra = f"; intent={route.get('intent')}; lang={route.get('language')}"
    if route.get("false_premise"):
        extra += "; premise corrected"
    return f"algo={route.get('algorithm')} {log.get('params_used')}; claimed {probs}; verified={ver.get('ok')}{repaired}{extra}"


def esc(s) -> str:
    return str(s).replace("|", "\\|").replace("\n", " ").replace("\r", " ")


def main(ids: list[str] | None = None):
    OUT.mkdir(parents=True, exist_ok=True)
    stamp = datetime.now().strftime("%Y%m%d_%H%M")
    rows = [r for r in csv.DictReader(TESTS.open(encoding="utf-8")) if not ids or r["id"] in ids]
    table = [f"Model: {MODEL} · run {stamp} · automatic checks only (confirm by hand)", "",
             "| Test no. | Input (short) | Type | Expected output | Actual output (short) | Pass / Fail |",
             "|---|---|---|---|---|---|"]
    with (OUT / f"run_{stamp}.jsonl").open("w", encoding="utf-8") as f:
        for row in rows:
            try:
                log = run(row["question"], row["level"])
            except Exception as e:  # infrastructure error: record it, never call it a model failure
                log = {"error": repr(e)}
            ok, why = (None, log["error"]) if "error" in log else auto_check(row, log)
            log.update({"test_id": row["id"], "type": row["type"], "auto_pass": ok, "auto_reason": why})
            f.write(json.dumps(log, ensure_ascii=False) + "\n")
            f.flush()
            verdict = "Error" if ok is None else ("Pass" if ok else "Fail")
            actual = short(log) if ok is not None else log["error"][:80]
            table.append("| " + " | ".join(esc(c) for c in (row["id"], row["question"][:60], row["type"],
                                                         row["expected"], actual, verdict)) + " |")
            print(row["id"], verdict.upper(), why)
            time.sleep(1)  # stay under free-tier rate limits
    table[0] = f"Models that answered: {models_used()} · run {stamp} · automatic checks only (confirm by hand)"
    (OUT / f"test_table_{stamp}.md").write_text("\n".join(table) + "\n", encoding="utf-8")
    print("saved", OUT)


if __name__ == "__main__":
    import sys

    main(sys.argv[1:] or None)  # e.g. py run_tests.py 1 5 7 12  -> only those test ids
