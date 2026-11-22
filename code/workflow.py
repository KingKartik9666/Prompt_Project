"""S18 quantum tutor: P1 router -> P2 explainer -> P3 claim extractor + simulator check -> P4 practice.

Needs GEMINI_API_KEY in the environment (or Colab Secrets). Never hard-code the key.
Optional GEMINI_MODEL (default gemini-3.5-flash-lite; 2.5 models are closed to new keys; on 2026-11-22 3.7/3.8-flash were overloaded and 3.6/3.5-flash hit the 20-requests-per-day free-tier limit).
"""
from __future__ import annotations

import json
import os
import re
import time
from pathlib import Path

from quantum_sim import norm_params, verify_claim

ROOT = Path(__file__).resolve().parent.parent
PROMPTS = ROOT / "prompts"


def _load_dotenv(path: Path = ROOT / ".env") -> None:
    """Read KEY=VALUE lines from the git-ignored .env file without overriding real env vars."""
    if path.exists():
        for line in path.read_text(encoding="utf-8").splitlines():
            if "=" in line and not line.lstrip().startswith("#"):
                k, v = line.split("=", 1)
                os.environ.setdefault(k.strip(), v.strip().strip('"'))


_load_dotenv()
MODEL = os.environ.get("GEMINI_MODEL", "gemini-3.5-flash-lite")
# Fallback chain, highest tier first. GEMINI_MODEL is tried first; if it (or any model) is out of daily
# quota, not found, or keeps failing, the next one is used automatically. Override the tail with
# GEMINI_FALLBACKS="model-a,model-b".
FALLBACKS = [m.strip() for m in os.environ.get("GEMINI_FALLBACKS", "gemini-3.1-flash-lite").split(",") if m.strip()]
CHAIN = [MODEL] + [m for m in FALLBACKS if m != MODEL]
_dead: set[str] = set()      # models skipped for the rest of this process
_used: dict[str, int] = {}   # model -> number of successful calls (for honest reporting)
VERIFIABLE = {"bell", "deutsch_jozsa", "grover2"}

_client = None


def client():
    global _client
    if _client is None:
        from google import genai

        key = os.environ.get("GEMINI_API_KEY")
        if not key:
            raise RuntimeError("Set GEMINI_API_KEY in your environment (see .env.example).")
        _client = genai.Client(api_key=key)
    return _client


def load_prompt(path: Path) -> dict:
    """Parse a prompt file with ### SETTINGS / ### SYSTEM / ### USER sections."""
    text = path.read_text(encoding="utf-8")
    parts = re.split(r"^### (SETTINGS|SYSTEM|USER)\s*$", text, flags=re.M)
    sec = {parts[i]: parts[i + 1].strip() for i in range(1, len(parts), 2)}
    settings = {}
    for line in sec.get("SETTINGS", "").splitlines():
        if ":" in line:
            k, v = line.split(":", 1)
            settings[k.strip()] = v.strip()
    if not sec:  # plain v1-style file: whole text is the user prompt
        sec = {"USER": text.strip()}
    return {"settings": settings, "system": sec.get("SYSTEM"), "user": sec.get("USER", "")}


def fill(template: str, **vars) -> str:
    # Replace only known {name} placeholders so JSON braces in the prompt survive.
    for k, v in vars.items():
        if not isinstance(v, str):
            v = json.dumps(v)
        template = template.replace("{" + k + "}", v)
    return template


def call(prompt: dict, **vars) -> tuple[str, float]:
    from google.genai import types

    s = prompt["settings"]
    cfg = types.GenerateContentConfig(
        system_instruction=prompt["system"],
        temperature=float(s["temperature"]) if "temperature" in s else None,
        response_mime_type=s.get("response_mime_type"),
    )
    contents = fill(prompt["user"], **vars)
    last_err = None
    for model in CHAIN:
        if model in _dead:
            continue
        for wait in (10, 30, 60, None):  # retry transient errors on this model before falling back
            t0 = time.time()
            try:
                resp = client().models.generate_content(model=model, contents=contents, config=cfg)
                _used[model] = _used.get(model, 0) + 1
                return resp.text or "", round(time.time() - t0, 2)
            except Exception as e:
                msg = str(e)
                last_err = e
                low = msg.lower()
                # permanent for today / for this key: skip the model immediately, no waiting
                if ("429" in msg and ("perday" in low or "per day" in low)) or "404" in msg or "NOT_FOUND" in msg:
                    _dead.add(model)
                    print(f"{model} unavailable ({msg[:40].strip()}), switching to next model")
                    break
                transient = any(s in msg for s in ("429", "RESOURCE_EXHAUSTED", "503", "500", "UNAVAILABLE",
                                                     "disconnected", "timed out", "Timeout"))
                if not transient:
                    raise
                if wait is None:
                    print(f"{model} still failing after retries, switching to next model")
                    break
                print(f"transient API error on {model} ({msg[:60]}), retrying in {wait}s")
                time.sleep(wait)
    raise RuntimeError(f"All models in the fallback chain failed: {CHAIN}") from last_err


def models_used() -> str:
    """Models that actually answered in this process, e.g. 'gemini-3.5-flash-lite (40), gemini-3.1-flash-lite (12)'."""
    return ", ".join(f"{m} ({n})" for m, n in _used.items()) or MODEL


def parse_json(text: str):
    try:
        return json.loads(text), True
    except (json.JSONDecodeError, TypeError):
        m = re.search(r"\{.*\}", text or "", flags=re.S)
        if m:
            try:
                return json.loads(m.group(0)), False  # recovered, but raw output was not clean JSON
            except json.JSONDecodeError:
                pass
        return None, False


FINAL = {name: load_prompt(PROMPTS / "final" / f"{name}.txt")
         for name in ["P1_router", "P2_explainer", "P3_claim_extractor", "P4_practice"]}


def run(question: str, level: str = "beginner", repair: bool = True) -> dict:
    """Full pipeline. Returns a dict with every intermediate output for logging."""
    log = {"question": question, "level": level, "model": MODEL}

    raw, log["t_p1"] = call(FINAL["P1_router"], question=question, level=level)
    route, log["p1_json_clean"] = parse_json(raw)
    log["route"] = route
    if not route:
        log["final"] = "Router output was not valid JSON."
        return log
    if not route.get("in_scope"):
        log["final"] = ("I can help with quantum algorithms (Bell states, Deutsch-Jozsa, Grover). "
                        "Please ask a question about one of them.")
        return log

    algo = route.get("algorithm")
    params = norm_params(algo, route.get("params"))  # defaults applied in code, not only in the prompt
    log["params_used"] = params
    feedback = ""
    for attempt in range(2 if repair else 1):
        raw, t = call(FINAL["P2_explainer"], algorithm=algo, params=params, level=level,
                      false_premise=route.get("false_premise") or "none",
                      intent=route.get("intent") or "explain",
                      question=question, verifier_feedback=feedback)
        expl, clean = parse_json(raw)
        log[f"t_p2_{attempt}"], log[f"p2_json_clean_{attempt}"] = t, clean
        log["explanation"] = expl or raw

        if algo not in VERIFIABLE:
            log["verification"] = {"ok": None, "note": "not verifiable by simulator; shown as unverified"}
            break
        raw3, log[f"t_p3_{attempt}"] = call(FINAL["P3_claim_extractor"], algorithm=algo, params=params,
                                             explanation=json.dumps(expl) if expl else raw)
        claim, log[f"p3_json_clean_{attempt}"] = parse_json(raw3)
        # Ground truth always comes from the router's (normalised) params, never from P3's echo.
        claim = {**(claim if isinstance(claim, dict) else {}), "algorithm": algo, "params": params}
        log["claim"] = claim
        ver = verify_claim(claim)
        log["verification"] = ver
        log[f"verified_attempt_{attempt}"] = ver["ok"]
        if ver["ok"]:
            break
        feedback = ("Verifier feedback: a simulator found these errors in your previous answer: "
                    + "; ".join(ver["mismatches"]) + f". Correct result: {ver['truth']}. Fix the explanation.")

    truth = (log.get("verification") or {}).get("truth")
    raw4, log["t_p4"] = call(FINAL["P4_practice"], algorithm=algo, params=params, level=level,
                             truth=truth if truth else "not available", explanation=json.dumps(log["explanation"]))
    log["practice"], log["p4_json_clean"] = parse_json(raw4)
    log["models_used"] = models_used()
    return log


if __name__ == "__main__":
    import sys

    q = " ".join(sys.argv[1:]) or "How does Grover's algorithm find item 11 among 4 items?"
    print(json.dumps(run(q), indent=2, ensure_ascii=False))
