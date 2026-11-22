"""Tiny NumPy statevector simulator used by the P3 verifier.

Ground truth for three algorithms: Bell state, Deutsch-Jozsa (n input qubits),
and Grover search on 2 qubits. No Qiskit dependency, so it runs anywhere.
"""
from __future__ import annotations

import numpy as np

H = np.array([[1, 1], [1, -1]]) / np.sqrt(2)
X = np.array([[0, 1], [1, 0]])
I2 = np.eye(2)


def kron_all(mats):
    out = np.array([[1.0]])
    for m in mats:
        out = np.kron(out, m)
    return out


def basis_labels(n):
    return [format(i, f"0{n}b") for i in range(n_states(n))]


def n_states(n):
    return 2**n


def probabilities(state):
    p = np.abs(state) ** 2
    n = int(np.log2(len(state)))
    return {lab: float(round(v, 6)) for lab, v in zip(basis_labels(n), p) if v > 1e-9}


def bell():
    """H on qubit 0 then CNOT(0->1) starting from |00>."""
    state = np.zeros(4)
    state[0] = 1
    state = kron_all([H, I2]) @ state
    cnot = np.array([[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 0, 1], [0, 0, 1, 0]])
    return probabilities(cnot @ state)


def deutsch_jozsa(n, f_kind):
    """Probability of measuring all-zeros on the n input qubits.

    f_kind: 'constant0', 'constant1' or 'balanced' (parity of the input bits).
    All-zeros probability is 1 for constant f and 0 for balanced f.
    """
    N = 2**n
    amp = np.ones(N) / np.sqrt(N)
    if f_kind == "constant0":
        signs = np.ones(N)
    elif f_kind == "constant1":
        signs = -np.ones(N)
    elif f_kind == "balanced":
        signs = np.array([(-1) ** bin(x).count("1") for x in range(N)])
    else:
        raise ValueError(f_kind)
    amp = amp * signs
    hn = kron_all([H] * n)
    out = hn @ amp
    return probabilities(out)


def grover2(marked):
    """Grover on 2 qubits for marked item (e.g. '11'). One iteration is exact."""
    idx = int(marked, 2)
    state = np.ones(4) / 2
    oracle = np.eye(4)
    oracle[idx, idx] = -1
    s = np.ones(4) / 2
    diffusion = 2 * np.outer(s, s) - np.eye(4)
    state = diffusion @ (oracle @ state)
    return probabilities(state)


TRUTH = {
    "bell": lambda **kw: bell(),
    "deutsch_jozsa": lambda **kw: deutsch_jozsa(kw["n"], kw["f_kind"]),
    "grover2": lambda **kw: grover2(kw["marked"]),
}


def verify_claim(claim: dict, tol: float = 0.02) -> dict:
    """claim = {"algorithm": ..., "params": {...}, "probabilities": {"00": 0.5, ...}}.

    Returns {"ok": bool, "truth": {...}, "mismatches": [...]}.
    """
    algo = claim.get("algorithm")
    if algo not in TRUTH:
        return {"ok": False, "truth": None, "mismatches": [f"unsupported algorithm: {algo}"]}
    params = claim.get("params") or {}
    try:
        truth = TRUTH[algo](**params)
    except (KeyError, TypeError, ValueError) as e:
        return {"ok": False, "truth": None, "mismatches": [f"bad params {params}: {e!r}"]}

    mism = []
    claimed = {}
    raw = claim.get("probabilities") or {}
    if isinstance(raw, dict):
        for k, v in raw.items():
            p = _to_prob(v)
            if p is None:
                mism.append(f"{k}: unparseable value {v!r}")
            else:
                claimed[_norm_key(k)] = claimed.get(_norm_key(k), 0.0) + p
    keys = set(truth) | set(claimed)
    if not claimed and not mism:
        mism.append("no final probability stated")
    if algo == "deutsch_jozsa":
        # Many balanced functions exist; only P(all zeros) is fixed (1 constant, 0 balanced).
        n = params["n"]
        zeros = "0" * n
        truth = {zeros: truth.get(zeros, 0.0)}
        keys = {zeros} if claimed else set()
        bad = [k for k in claimed if len(k) != n]
        if bad:
            mism.append(f"keys must be {n}-bit input-register strings, got {bad}")
        s = sum(claimed.values())
        if s > 1 + tol or (set(claimed) - {zeros} and abs(s - 1) > tol):
            mism.append(f"claimed probabilities sum to {s:.3f}, expected 1")
    for k in keys:
        t, c = truth.get(k, 0.0), claimed.get(k, 0.0)
        if abs(t - c) > tol:
            mism.append(f"{k}: claimed {c}, simulated {t}")
    return {"ok": not mism, "truth": truth, "mismatches": mism}


def _norm_key(k) -> str:
    """'|01>', ' 01 ', '|0>|1>' -> '01'."""
    return "".join(ch for ch in str(k) if ch in "01")


def _to_prob(v):
    """0.5, '0.5', '1/2', '50%' -> 0.5; anything else -> None."""
    try:
        if isinstance(v, str):
            s = v.strip()
            if s.endswith("%"):
                return float(s[:-1]) / 100
            if "/" in s:
                a, b = s.split("/", 1)
                return float(a) / float(b)
            return float(s)
        return float(v)
    except (TypeError, ValueError, ZeroDivisionError):
        return None


def norm_params(algo: str, p) -> dict:
    """Apply the defaults the router prompt promises and coerce LLM-written values."""
    p = p if isinstance(p, dict) else {}
    if algo == "grover2":
        m = _norm_key(p.get("marked", "11"))
        return {"marked": m if m in {"00", "01", "10", "11"} else "11"}
    if algo == "deutsch_jozsa":
        try:
            n = min(3, max(1, int(p.get("n", 2))))
        except (TypeError, ValueError):
            n = 2
        fk = str(p.get("f_kind", "balanced")).lower().strip()
        fk = {"constant": "constant0"}.get(fk, fk)
        return {"n": n, "f_kind": fk if fk in {"constant0", "constant1", "balanced"} else "balanced"}
    return {}


if __name__ == "__main__":
    print("bell", bell())
    print("dj n=2 balanced", deutsch_jozsa(2, "balanced"))
    print("dj n=2 constant0", deutsch_jozsa(2, "constant0"))
    print("grover2 11", grover2("11"))
    print(verify_claim({"algorithm": "grover2", "params": {"marked": "01"}, "probabilities": {"01": 1.0}}))
    print(verify_claim({"algorithm": "grover2", "params": {"marked": "01"}, "probabilities": {"01": 0.5}}))
    # balanced f(x)=x0 is valid too: only P(00)=0 is checked
    print(verify_claim({"algorithm": "deutsch_jozsa", "params": {"n": 2, "f_kind": "balanced"},
                        "probabilities": {"10": 1.0}}))
    print(verify_claim({"algorithm": "deutsch_jozsa", "params": {"n": 2, "f_kind": "constant0"},
                        "probabilities": {"00": 0.5, "11": 0.5}}))
    # robustness: ket keys and fraction strings; ancilla-included keys rejected; empty claim fails
    print(verify_claim({"algorithm": "bell", "params": {}, "probabilities": {"|00>": "1/2", "|11>": "50%"}}))
    print(verify_claim({"algorithm": "deutsch_jozsa", "params": {"n": 2, "f_kind": "balanced"},
                        "probabilities": {"001": 1.0}}))
    print(verify_claim({"algorithm": "grover2", "params": {"marked": "11"}, "probabilities": {}}))
    print(norm_params("grover2", {"marked": "|10>"}), norm_params("deutsch_jozsa", {"n": "2", "f_kind": "constant"}),
          norm_params("grover2", None))
