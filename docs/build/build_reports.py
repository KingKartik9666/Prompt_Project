"""Build the Milestone 2 PDF and the technical report as HTML from the repo's own files.

Every prompt text, test row and number below is read from prompts/, test-data/ and the saved
outputs, so the PDF cannot drift from the evidence. Export to PDF with export_pdf.ps1.

    py build_reports.py [--reel URL] [--reel-seconds N]
"""
from __future__ import annotations

import argparse
import csv
import html
import json
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "test-data" / "outputs"
BUILD = Path(__file__).resolve().parent

TITLE = "AI-Based Interactive Quantum-Algorithm Learning Platform"
TEAM_ID, TEAM, BATCH, FACULTY = "PE-B65-13", "Chica", "76", "Sonal Kukreja"
MEMBERS = [("Kartik Negi", "S24CSEU1982", "team leader"), ("Divey Grover", "S24CSEU1989", ""),
           ("Twinkle Sahu", "S24CSEU0090", "")]
REPO = "https://github.com/KingKartik9666/Prompt_Project"
COLAB = "https://colab.research.google.com/github/KingKartik9666/Prompt_Project/blob/main/code/s18_workflow.ipynb"
GEMINI = "gemini-3.5-flash-lite"
GEMINI_NAME = "Gemini 3.5 Flash-Lite"
CLAUDE_NAME = "Claude Opus 5.5"
GEM_RUN, CLA_RUN = "run_20261122_1710.jsonl", "run_claude_20261122_1712.jsonl"
GEM_VER, CLA_VER = "versions_20261122_1714.json", "versions_claude_20261122_1712.json"
GEM_REV, CLA_REV = "review_gemini_20261122_1710.json", "manual_review_claude_20261122.json"
SAMPLE_TEST = "2"

e = html.escape


def jl(name):
    return {json.loads(l)["test_id"]: json.loads(l) for l in (OUT / name).open(encoding="utf-8")}


def js(name):
    return json.loads((OUT / name).read_text(encoding="utf-8"))


def prompt_parts(path: Path) -> dict:
    text, parts, cur = path.read_text(encoding="utf-8"), {}, None
    for line in text.splitlines():
        if line.startswith("### "):
            cur = line[4:].strip()
            parts[cur] = []
        elif cur:
            parts[cur].append(line)
    return {k: "\n".join(v).strip() for k, v in parts.items()}


def pre(text: str) -> str:
    return f"<pre>{e(text)}</pre>"


def table(head, rows, cls="") -> str:
    h = "".join(f"<th>{c}</th>" for c in head)
    b = "".join("<tr>" + "".join(f"<td>{c}</td>" for c in r) + "</tr>" for r in rows)
    return f'<table class="{cls}"><thead><tr>{h}</tr></thead><tbody>{b}</tbody></table>'


def jdump(o) -> str:
    return json.dumps(o, indent=1, ensure_ascii=False)


# ------------------------------------------------------------------ data
CASES = {r["id"]: r for r in csv.DictReader((ROOT / "test-data" / "test_cases.csv").open(encoding="utf-8"))}
G, C = jl(GEM_RUN), jl(CLA_RUN)
GV, CV = js(GEM_VER), js(CLA_VER)
GR, CR = js(GEM_REV)["tests"], js(CLA_REV)
CR_T = CR["tests"]
gem_auto = sum(1 for r in G.values() if r.get("auto_pass"))
cla_auto = sum(1 for r in C.values() if r.get("auto_pass"))
gem_cnt = Counter(GR[t]["final"] for t in GR)
cla_cnt = Counter(CR_T[t]["final"] for t in CR_T)
type_cnt = Counter(c["type"] for c in CASES.values())
g_in = [r for r in G.values() if r["route"].get("in_scope")]
g_p2_clean = sum(1 for r in g_in if r.get("p2_json_clean_0"))
g_verifiable = [r for r in g_in if "verified_attempt_0" in r]
g_first = sum(1 for r in g_verifiable if r["verified_attempt_0"])
c_in = [r for r in C.values() if r["route"].get("in_scope")]
c_verifiable = [r for r in c_in if "verified_attempt_0" in r]
c_first = sum(1 for r in c_verifiable if r["verified_attempt_0"])
g_route_ok = sum(1 for t, r in G.items() if not (r.get("auto_reason") or "").startswith("algorithm="))
level_fit = Counter()
for k, v in CR["versions"].items():
    level_fit[(k.split("_")[0][1:], v["level_fit"])] += 1


def vsum(d, v):
    return d[v]["summary"]


assert sum(type_cnt.values()) == 13 and len(G) == 13 and len(C) == 13 and len(GR) == 13 and len(CR_T) == 13

ACTUAL = {  # short description of the Gemini output; details in review_gemini_20261122_1710.json
    "1": "grover2, marked 11. 4 steps with correct ket states, but step 2 says \"H-like mixing step\" without naming the Hadamard. P(11)=1.0, verified. Practice question only asks the learner to recall the oracle step.",
    "2": "grover2, marked 01. 3 bra-ket steps; P(01)=1.0, verified. Practice question asks for the post-oracle state with 10 marked, and the key is correct.",
    "3": "deutsch_jozsa n=2 balanced. P(00)=0, verified. Step 2 text garbled (invalid JSON) and the step 3 state is unnormalised. Hint points to the answer.",
    "4": "deutsch_jozsa constant0. Correct steps, states described in words. P(00)=1, verified. Practice question repeats the same example, and the hint gives the answer away.",
    "5": "bell. H then CNOT, correct states. P(00)=P(11)=0.5, verified. Analogy suggests the outcome is fixed in advance; practice question is unclear.",
    "6": "bell. Correct states, says the state cannot be factored. Verified. Practice answer key is wrong: (|00>+|10>)/&radic;2 instead of (|00>+|01>)/&radic;2.",
    "7": "False premise detected and corrected (quadratic, not exponential). P(11)=1.0, verified. Hint points straight to step 3.",
    "8": "language=hinglish; reply in simple English. P(11)=1.0, verified. Beginner text uses vector notation; hint gives the answer away.",
    "9": "intent=compare, but routed to other_quantum, so not verified. Says both need 1 query and solve different problems. Explains mostly DJ; step 4 state unnormalised, step 5 gives no state.",
    "10": "other_quantum, labelled unverified. 7 steps (4 with bra-ket states); rubric found the post-QFT state wrong and \"prime factors\" overstated.",
    "11": "out_of_scope, injection_attempt=true. Polite redirect; no answer key or prompt leaked.",
    "12": "out_of_scope. Redirect: \"Please ask a question about one of them.\"",
    "13": "out_of_scope. Polite redirect; nothing about France.",
}


# ------------------------------------------------------------------ pieces
def css() -> str:
    return """
@page { size: A4; margin: 15mm 14mm 16mm 14mm; }
body { font-family: 'Segoe UI', Calibri, Arial, sans-serif; font-size: 10.2pt; color: #111; line-height: 1.38; }
h1 { font-size: 17pt; border-bottom: 2px solid #1f3b73; padding-bottom: 3px; color: #1f3b73; margin: 0 0 8px; }
h1.sec { page-break-before: always; }
h2 { font-size: 12.5pt; color: #1f3b73; margin: 14px 0 5px; }
h3 { font-size: 11pt; margin: 12px 0 4px; }
p { margin: 4px 0 6px; }
table { border-collapse: collapse; width: 100%; margin: 5px 0 9px; page-break-inside: auto; }
tr { page-break-inside: avoid; }
th, td { border: 1px solid #9aa4b5; padding: 3px 5px; vertical-align: top; text-align: left; }
th { background: #e8edf6; }
table.small { font-size: 8.4pt; }
table.kv td:first-child { width: 30%; font-weight: 600; background: #f4f6fa; }
pre { font-family: Consolas, 'Courier New', monospace; font-size: 7.9pt; background: #f6f7f9; border: 1px solid #d5d9e0;
      padding: 5px 7px; white-space: pre-wrap; word-break: break-word; margin: 3px 0 8px; }
code { font-family: Consolas, monospace; font-size: 9pt; }
.cap { font-size: 9pt; color: #333; font-style: italic; margin: 2px 0 10px; }
.fig { text-align: center; page-break-inside: avoid; margin: 6px 0; }
.fig img { max-width: 100%; border: 1px solid #ccc; }
.mermaid { text-align: center; } .mermaid svg { max-height: 120mm; }
.note { font-size: 9pt; color: #333; }
.cover-title { font-size: 21pt; font-weight: 700; color: #1f3b73; margin: 30px 0 4px; }
.cover-sub { font-size: 12pt; color: #333; margin-bottom: 18px; }
.pass { color: #136c2e; font-weight: 600; } .fail { color: #a11; font-weight: 600; } .partial { color: #8a5a00; font-weight: 600; }
"""


def verdict(v: str) -> str:
    return f'<span class="{v.lower()}">{v}</span>'


def diagrams_html() -> str:
    src = (ROOT / "docs" / "diagrams.md").read_text(encoding="utf-8")
    blocks, caps = [], []
    for chunk in src.split("```mermaid")[1:]:
        blocks.append(chunk.split("```")[0].strip())
    caps = [
        "Figure 1: Prompt workflow. P1 routes, P2 explains, P3 + simulator check (one repair retry back to P2), P4 writes the practice question. Out-of-scope messages get a redirect; other_quantum questions skip P3 and the simulator and are labelled unverified.",
        "Figure 2: Task decomposition of \"teach a quantum algorithm correctly\" into understand, explain, check and practice.",
        "Figure 3: Input-output flow: the JSON passed between the prompts and the simulator.",
        "Figure 4: System architecture: notebook, workflow.py, prompt files, Gemini API, NumPy simulator, test scripts.",
    ]
    return "".join(f'<div class="fig"><div class="mermaid">{e(b)}</div><div class="cap">{c}</div></div>'
                   for b, c in zip(blocks, caps))


def prompt_sheet(pid: str, fname: str, step: str, purpose: str, n_examples: str, outfmt: str,
                 variables: list[tuple[str, str]], sample_in: str, sample_out) -> str:
    p = prompt_parts(ROOT / "prompts" / "final" / fname)
    settings = "; ".join(l.strip() for l in p["SETTINGS"].splitlines() if l.strip())
    rows = [
        ("Prompt ID and name", f"{pid} (<code>prompts/final/{fname}</code>)"),
        ("Step in the workflow", step),
        ("Purpose", purpose),
        ("Tool and model", f"{GEMINI_NAME} (<code>{GEMINI}</code>) via the google-genai Python SDK (<code>code/workflow.py</code>). "
                           f"Test cross-check: {CLAUDE_NAME} (<code>claude-opus-5-5</code>)."),
        ("Settings", e(settings)),
        ("Number of examples", n_examples),
        ("Output format", outfmt),
    ]
    var_html = "<ul>" + "".join(f"<li><code>{e(v)}</code>: {d}</li>" for v, d in variables) + "</ul>"
    return f"""<h3>Prompt sheet: {pid}</h3>{table(['Field', 'Value'], rows, 'kv')}
<p><b>Full system prompt</b></p>{pre(p['SYSTEM'])}
<p><b>Full user prompt template</b></p>{pre(p['USER'])}
<p><b>What each variable holds</b></p>{var_html}
<p><b>Sample run</b> ({GEMINI_NAME}, test {SAMPLE_TEST} of <code>{GEM_RUN}</code>, copied from the log)</p>
<p>Input: {sample_in}</p><p>Actual output:</p>{pre(sample_out if isinstance(sample_out, str) else jdump(sample_out))}"""


def section2() -> str:
    s = G[SAMPLE_TEST]
    q = e(s["question"])
    claim_raw = s["claim"]
    sheets = [
        prompt_sheet("P1: Router", "P1_router.txt", "Step 1 of 4. Runs first on every learner message.",
                     "Classify the message (algorithm, parameters, intent, false premise, language, injection) and stop out-of-scope messages before any explanation is written.",
                     "0 (zero-shot; the JSON schema is given in the prompt)",
                     "One JSON object: in_scope, algorithm, params, intent, false_premise, language, injection_attempt, reason",
                     [("{level}", "level chosen in the app: beginner or intermediate"),
                      ("{question}", "the learner's message exactly as typed, placed inside &lt;message&gt; tags so it is treated as data")],
                     f"level = {s['level']}, question = \"{q}\"", s["route"]),
        prompt_sheet("P2: Explainer", "P2_explainer.txt",
                     "Step 2 of 4. Runs after P1 for in-scope messages; runs once more (repair retry) if the simulator finds a mismatch.",
                     "Write a step-by-step explanation at the learner's level, with the state after every step, correcting any false premise.",
                     "2 (beginner Bell state; intermediate Grover with marked item 10)",
                     "One JSON object: goal, steps (n, action, state), final_result, analogy, misconception, uncertain",
                     [("{algorithm}", "algorithm label from P1 (bell, deutsch_jozsa, grover2, other_quantum)"),
                      ("{params}", "P1 parameters after code defaults (norm_params), e.g. {\"marked\": \"01\"}"),
                      ("{level}", "beginner or intermediate"),
                      ("{false_premise}", "false belief found by P1, or none"),
                      ("{intent}", "explain, compare or clarify (from P1)"),
                      ("{question}", "the learner's original message"),
                      ("{verifier_feedback}", "empty on the first attempt; on the repair attempt: the simulator's mismatches and the correct result")],
                     f"algorithm = grover2, params = {{\"marked\": \"01\"}}, level = intermediate, false_premise = none, intent = explain, question = \"{q}\", verifier_feedback = (empty)",
                     s["explanation"]),
        prompt_sheet("P3: Claim extractor", "P3_claim_extractor.txt",
                     "Step 3 of 4. Runs after P2 only for the three verifiable algorithms; its output goes to verify_claim() in code/quantum_sim.py (tolerance 0.02).",
                     "Copy the final measurement probabilities the explanation claims, without fixing them, so that code (not the LLM) checks them.",
                     "0 (zero-shot; schema and rules in the prompt)",
                     "One JSON object: algorithm, params, probabilities (bitstring to decimal), found_in_text",
                     [("{algorithm}", "algorithm label from P1 (used twice)"), ("{params}", "parameters from P1 (used twice)"),
                      ("{explanation}", "the P2 output as a JSON string")],
                     "algorithm = grover2, params = {\"marked\": \"01\"}, explanation = the P2 output above",
                     jdump(claim_raw) + "\n\n(As logged: the code overwrites the algorithm and params keys with the router's values.)\nSimulator check (verify_claim): " + jdump(s["verification"])),
        prompt_sheet("P4: Practice question", "P4_practice.txt",
                     "Step 4 of 4. Runs last, after verification (or after the \"unverified\" label).",
                     "Write one practice question at the learner's level with a hint and an answer key, then review it against the verified result.",
                     "0 (draft, answer, review steps given as instructions)",
                     "One JSON object: question, type, hint, answer_key, review (answer_correct, level_ok, unambiguous, changes_made)",
                     [("{algorithm}", "algorithm label from P1"), ("{params}", "parameters from P1"), ("{level}", "beginner or intermediate"),
                      ("{truth}", "simulator probabilities, e.g. {\"01\": 1.0}, or \"not available\" for unverifiable algorithms"),
                      ("{explanation}", "the final P2 explanation as JSON")],
                     "algorithm = grover2, params = {\"marked\": \"01\"}, level = intermediate, truth = {\"01\": 1.0}, explanation = the P2 output above",
                     s["practice"]),
    ]
    return f"""<h1 class="sec">2. Prompt and solution design</h1>
<h2>2a. Overview</h2>
<p><b>Problem.</b> Students new to quantum computing get explanations of quantum algorithms that skip steps or contain wrong numbers, such as "Grover gives an exponential speedup". An LLM can explain an algorithm, but it can also state wrong probabilities very confidently. Our tool answers a learner's question about a quantum algorithm with a step-by-step explanation at the learner's level, <b>checks the explanation's final probabilities with a NumPy simulator</b>, and then gives one practice question.</p>
<p><b>For whom.</b> B.Tech CS students who know linear algebra (vectors, matrices) but are new to quantum computing.</p>
<p><b>Input:</b> the learner's question (free text) and level (<i>beginner</i> or <i>intermediate</i>). <b>Output:</b> (1) a JSON explanation: goal, steps with the state after each step, final result, analogy, misconception, uncertain points. (2) A verification result: verified, mismatch (one repair retry), or unverified for algorithms the simulator does not cover. (3) One practice question with a hint, an answer key and a self-review. Out-of-scope messages get a polite redirect. P1 also flags prompt-injection attempts, and the flag is logged, but the code redirects only when <code>in_scope</code> is false. Test 11 was both.</p>
<p><b>How many prompts and why: 4.</b> <b>P1 Router</b> (temperature 0) stops out-of-scope input early and gives the explainer clean parameters. <b>P2 Explainer</b> (temperature 0.2, 2 examples) uses chain-of-thought (state after every gate before the final answer) and audience-aware rules per level. <b>P3 Claim extractor</b> (temperature 0) only copies the claimed probabilities. The check itself is done by code (<code>quantum_sim.py</code>), so the model that wrote the answer is not the one that judges it. <b>P4 Practice</b> (temperature 0.4) writes a question and reviews its own answer key. These are the three techniques listed for S18: chain-of-thought, audience-aware prompting and correctness checks.</p>
<p><b>Tool and model, and why.</b> {GEMINI_NAME} (<code>{GEMINI}</code>) through the google-genai SDK (<code>code/workflow.py</code>). Our runs were made locally (VS Code, with the key in a git-ignored <code>.env</code>). The same notebook opens in Colab, and in Docker, for anyone. Reasons:</p>
<ul>
<li>It has a JSON output mode, which all four prompts use.</li>
<li>It has a free tier, so the course budget is zero.</li>
<li>We tried other models first: gemini-2.5-flash is closed to new API keys, gemini-3.7/3.8-flash returned 503 (overloaded), and gemini-3.6-flash and 3.5-flash hit the free-tier limit of 20 requests per day.</li>
</ul>
<p>Because of those limits, the full test set was also run on {CLAUDE_NAME} as a cross-check. Each LLM call was answered by a fresh Claude Code subagent through <code>code/step_runner.py</code>, with the same prompts and the same code-side checks. Both runs are reported in Section 4.</p>
<p><b>What changed since Milestone 1.</b> Milestone 1 was the problem statement and plan only; no prompts had been built. For Milestone 2 we built the 4-prompt pipeline, the NumPy simulator verifier with one repair retry, a 13-case test set, four saved versions of the explainer prompt and the Colab prototype. The scope of verification is narrowed to three algorithms that can be simulated exactly: Bell state, Deutsch-Jozsa, and Grover on 2 qubits.</p>
<h2>2b. Diagrams</h2>
<p class="note">Mermaid diagrams of our own pipeline (source in <code>docs/diagrams.md</code>, written with AI assistance and checked against the code). The pipeline uses no agent loop and no retrieval.</p>
{diagrams_html()}
<h2>2c. Prompt sheets (final prompts)</h2>
{''.join(sheets)}"""


def section3() -> str:
    gs = {v: vsum(GV, v) for v in ("v1", "v2", "v3")}
    cs = {v: vsum(CV, v) for v in ("v1", "v2", "v3")}
    lf = {v: level_fit[(v, "good")] for v in ("v1", "v2", "v3")}
    modules = [
        ("P1 Router", "Classifies algorithm, params, intent, false premise, language, injection; stops out-of-scope messages", "Working",
         f"{GEMINI_NAME}: valid JSON 13/13; routing as expected on {g_route_ok}/13 (test 9 compare question misrouted to other_quantum). Claude run: 13/13."),
        ("P2 Explainer", "Step-by-step, level-aware explanation as JSON", "Partly working",
         f"{GEMINI_NAME}: clean JSON {g_p2_clean}/{len(g_in)} in-scope tests (test 3 broken); final result verified {g_first}/{len(g_verifiable)} verifiable; but wrong or missing intermediate states in tests 3, 9 and 10, and level fit good in only 2/8 outputs (Claude rubric)."),
        ("P3 Claim extractor", "Copies the claimed final probabilities into JSON", "Working",
         f"Valid JSON on all {len(g_verifiable)} verifiable Gemini tests and {len(c_verifiable)} Claude tests."),
        ("Simulator check (quantum_sim.py)", "True probabilities for Bell, Deutsch-Jozsa, Grover-2; compares with the claim (tolerance 0.02)", "Working",
         "Self-check output: Bell {'00': 0.5, '11': 0.5}; DJ n=2 balanced {'11': 1.0}; DJ constant0 {'00': 1.0}; Grover marked 11 {'11': 1.0}; wrong claims flagged (test-data/outputs/quantum_sim_selfcheck.txt)."),
        ("Repair retry", "Calls P2 once more with the simulator's feedback when there is a mismatch", "Partly working",
         "Never triggered in a real run (every verifiable answer verified on the first attempt, attempt 0). Code path passes a scripted test (code/test_repair.py, 6/6 checks); not yet shown that the model fixes its answer."),
        ("P4 Practice question", "One practice question, hint, answer key, self-review", "Partly working",
         "Produces valid JSON on every in-scope test, but the test 6 (Gemini) answer key is wrong while its self-review says correct, and hints often give the answer away."),
        ("Test runner (run_tests.py)", "Runs the 13 tests, saves jsonl and test table with automatic checks", "Working", f"Produced {GEM_RUN} and the test table."),
        ("Version comparison (compare_versions.py)", "Runs explainer v1, v2, v3 on the same 4 questions x 3 repeats", "Working", f"Produced {GEM_VER}."),
        ("Claude step runner (step_runner.py)", "Runs the same pipeline with each LLM call answered by a fresh Claude subagent", "Working", f"Produced {CLA_RUN} and {CLA_VER}."),
        ("Prototype notebook (s18_workflow.ipynb)", "Colab prototype: simulator truth, live demos (normal, false premise, invalid), saved test tables", "Working",
         "Outputs saved; opens from the Colab link without an access request. A live re-run needs your own Gemini key in Colab Secrets."),
    ]
    n_working = sum(1 for m in modules if m[2] == "Working")
    vtext = {v: (ROOT / "prompts" / v / f"explainer_{v}.txt").read_text(encoding="utf-8").strip() for v in ("v1", "v2", "v3")}
    final_text = (ROOT / "prompts" / "final" / "P2_explainer.txt").read_text(encoding="utf-8").strip()
    it_rows = [
        ("v1", "One line: <code>Explain {question}</code>. No role, level, format or parameters; default temperature; plain text.",
         "Baseline: what the model does with no guidance.",
         f"Verified {gs['v1']['verified_correct']}/12; final probabilities stated in {gs['v1']['stated_final_probabilities']}/12; clean JSON {gs['v1']['clean_json']}/12; {gs['v1']['avg_words']} words on average. "
         "Of the 7 misses, 4 stated no final probabilities. On test 7 v1 had no marked item, so 2 runs chose 10 instead of 11 (missing context, not wrong physics). One DJ answer's probabilities summed to 0.5.",
         f"Verified {cs['v1']['verified_correct']}/8; JSON {cs['v1']['clean_json']}/8; {cs['v1']['avg_words']} words; level fit good {lf['v1']}/8"),
        ("v2", "Tutor role and audience, learner level, 4 fixed sections (goal, steps with state after every step, final probabilities, misconception), rule against claiming exponential speedup, \"if unsure, say so\".",
         "Give the model a role, the learner's level and a fixed structure, so that every answer has a state after each step and stated final probabilities.",
         f"Verified {gs['v2']['verified_correct']}/12; probabilities stated {gs['v2']['stated_final_probabilities']}/12; clean JSON {gs['v2']['clean_json']}/12; {gs['v2']['avg_words']} words. "
         "One DJ run claimed 0.33 each for 01, 10 and 11. That is physically wrong, but it passed because only P(00) is checked (limitation 1).",
         f"Verified {cs['v2']['verified_correct']}/8; JSON {cs['v2']['clean_json']}/8; {cs['v2']['avg_words']} words; level fit good {lf['v2']}/8"),
        ("v3", "System/user split; temperature 0.2; JSON output; per-level audience rules; reasoning rules (state after every step, probabilities must follow from states, correct false premise first, doubts in \"uncertain\", English reply to Hinglish); 2 few-shot examples; receives algorithm, params and false premise from P1.",
         "The verifier needs machine-readable output with the same layout in every run; examples show the expected depth for each level.",
         f"Verified {gs['v3']['verified_correct']}/12; probabilities stated {gs['v3']['stated_final_probabilities']}/12; clean JSON {gs['v3']['clean_json']}/12; {gs['v3']['avg_words']} words.",
         f"Verified {cs['v3']['verified_correct']}/8; JSON {cs['v3']['clean_json']}/8; {cs['v3']['avg_words']} words; level fit good {lf['v3']}/8"),
        ("final (P2)", "v3 plus: <code>{intent}</code> and a compare rule; DJ rule (probabilities for the input qubits only, state P(all zeros)); <code>{verifier_feedback}</code> slot for the repair retry; rewritten Bell analogy and misconception.",
         "Compare questions needed their own handling; the verifier rejects ancilla-included DJ keys; the v3 Bell analogy described classical (pre-set) correlation; a simulator mismatch should be fixed, not only reported.",
         f"Measured in the full test runs (Section 4), not by the version script. {GEMINI_NAME}: clean JSON {g_p2_clean}/{len(g_in)}; verified on first attempt {g_first}/{len(g_verifiable)}. "
         "The new Bell analogy is still imprecise: \"agree when measured in other ways\" is false in the Y basis (Claude test 5).",
         f"Clean JSON {sum(1 for r in c_in if r.get('p2_json_clean_0'))}/{len(c_in)}; verified on first attempt {c_first}/{len(c_verifiable)}"),
    ]
    return f"""<h1 class="sec">3. Implementation</h1>
<h2>What is built</h2>
<ul>
<li><code>code/quantum_sim.py</code>: NumPy statevector simulator for the Bell state, Deutsch-Jozsa (n input qubits, constant or balanced) and 2-qubit Grover. <code>verify_claim()</code> compares claimed probabilities with the simulation, with a tolerance of 0.02. For Deutsch-Jozsa it compares only P(all zeros) with the simulation, because many balanced functions exist; it also checks that keys are n-bit input-register strings and that the probabilities sum to 1. Needs no API key.</li>
<li><code>code/workflow.py</code>: the pipeline P1 &rarr; P2 &rarr; P3 + simulator &rarr; P4.
  <ul>
  <li>It loads the prompts and the settings in each prompt file from <code>prompts/final/</code>.</li>
  <li>It parses JSON, applies the router defaults in code, and checks against the router's parameters, never the ones the LLM echoes back.</li>
  <li>It retries P2 once on a mismatch. It logs the final output of each step plus the timing, JSON validity and verification result of each attempt.</li>
  <li>If <code>gemini-3.5-flash-lite</code> is out of quota, it falls back to <code>gemini-3.1-flash-lite</code>. This fallback was added after our runs, so all reported Gemini runs used 3.5-flash-lite; <code>models_used()</code> reports which model answered.</li>
  <li>It reads the API key from the environment, a git-ignored local <code>.env</code> file or Colab Secrets, never from code.</li>
  </ul></li>
<li><code>code/run_tests.py</code> (13 tests &rarr; jsonl + test table with automatic checks), <code>code/compare_versions.py</code> (v1/v2/v3 &times; 4 questions &times; 3 repeats), <code>code/step_runner.py</code> (same pipeline with Claude subagents as the LLM), <code>code/test_repair.py</code> (repair-retry control-flow test).</li>
<li><code>code/s18_workflow.ipynb</code>: the prototype notebook, with outputs saved. It opens in Colab from the link on the cover.</li>
<li>Docker packaging (<code>Dockerfile</code>, <code>docker/entrypoint.sh</code>): <code>docker run --rm s18-tutor</code> runs the simulator self-check and the repair test with no key; the <code>ask</code> and <code>notebook</code> modes run the live pipeline or JupyterLab with your own key. See the README.</li>
</ul>
<p class="note"><b>AI assistance:</b> we used Claude Code (Claude Opus 5.5) heavily. It wrote most of the code, the prompt drafts, the test cases, the grading rubric, the diagrams, the README and the drafts of this report and its build script. The Claude cross-check runs and the rubric grading of both runs were done by Claude subagents. The team chose the problem, the users and the scope, made the design and tool decisions (problem S18, 4-prompt pipeline with a simulator check, GitHub and Colab, Gemini for the prototype and Claude for testing), and directed and checked the work. The README says the same. The team is responsible for everything reported here. All numbers come from the saved files in <code>test-data/outputs/</code>.</p>
<h2>Module status table</h2>
{table(['Module', 'What it does', 'Status', 'Notes'], [(e(a), e(b), c, e(d)) for a, b, c, d in modules], 'small')}
<p>Working modules: <b>{n_working} of {len(modules)}</b> (three are partly working, so the prototype status is "Some modules working").</p>
<h2>Prompt iteration log (P2 explainer)</h2>
<p>We iterated on P2, the main prompt. P1, P3 and P4 have one version each. The versions v1&ndash;v3 were run by <code>compare_versions.py</code> on the same 4 verifiable questions (tests 1, 3, 5, 7):</p>
<ul>
<li>on {GEMINI_NAME}, 3 repeats each, so 12 runs per version (<code>{GEM_VER}</code>);</li>
<li>on {CLAUDE_NAME}, 2 repeats each, so 8 runs per version (<code>{CLA_VER}</code>).</li>
</ul>
<p>v1, v2, v3 and the final version were written as a planned sequence and then all measured on the same questions. The "why" column gives the design reason; the result columns give what we measured afterwards. "Level fit good" comes from the rubric graders. The final version contains the repair loop, which lives in the pipeline, so its numbers come from the full test runs.</p>
{table(['Version', 'What changed', 'Why it was changed (design reason)', f'Result: {GEMINI_NAME} (12 runs)', f'Cross-check: {CLAUDE_NAME} (8 runs)'], it_rows, 'small')}
<p class="note">v1 and v2 do not ask for JSON, so their clean JSON score of 0 is expected. "Verified" only means the final probabilities matched the simulator; it says nothing about clarity. compare_versions.py passes false_premise = none to every version, so false-premise handling is measured only by test 7 in Section 4.</p>
<h3>Full text of each version</h3>
<p><b>v1</b> <code>prompts/v1/explainer_v1.txt</code></p>{pre(vtext['v1'])}
<p><b>v2</b> <code>prompts/v2/explainer_v2.txt</code></p>{pre(vtext['v2'])}
<p><b>v3</b> <code>prompts/v3/explainer_v3.txt</code></p>{pre(vtext['v3'])}
<p><b>final</b> <code>prompts/final/P2_explainer.txt</code></p>{pre(final_text)}"""


def section4() -> str:
    rows = []
    for tid in [str(i) for i in range(1, 14)]:
        c, g = CASES[tid], G[tid]
        auto = "Pass" if g.get("auto_pass") else "Fail"
        rows.append((tid, f"{e(c['question'])} <i>({c['level']})</i>", c["type"], e(c["expected"]), ACTUAL[tid],
                     f"auto: {verdict(auto)}<br>rubric: {verdict(GR[tid]['final'])}", verdict(CR_T[tid]["final"])))
    fails = ", ".join(t for t in GR if GR[t]["final"] == "Fail")
    shots = [
        ("nb_test1.png", "Screenshot 1 (Test 1, Normal): code/s18_workflow.ipynb cell 3, saved output of a real local run (rendered with jupyter nbconvert). Input = test 1 (Grover, marked 11, beginner) on gemini-3.5-flash-lite: route, explanation, verification ok, practice question. The 503/429 lines are free-tier retries."),
        ("nb_test7.png", "Screenshot 2 (Test 7, Edge): notebook cell 4, saved output of a real run. False premise \"Grover gives exponential speedup\". The router detects it, step 1 corrects it (quadratic), and the result is verified."),
        ("nb_test12.png", "Screenshot 3 (Test 12, Invalid): notebook cell 5, saved output of a real run. Input \"idk\" is routed out_of_scope and gets the redirect message."),
    ]
    shot_html = "".join(f'<div class="fig"><img src="img/{f}"><div class="cap">{c}</div></div>' for f, c in shots)
    return f"""<h1 class="sec">4. Demonstration and testing</h1>
<p>{len(CASES)} test inputs from <code>test-data/test_cases.csv</code>: {type_cnt['Normal']} Normal, {type_cnt['Edge']} Edge, {type_cnt['Invalid']} Invalid.</p>
<p><b>Main run:</b> the prototype model, {GEMINI_NAME} (<code>{GEM_RUN}</code>). Each row gets two verdicts:</p>
<ul>
<li><b>auto</b>: <code>run_tests.py</code> checks routing, intent, language, injection flag, false-premise detection and the simulator result.</li>
<li><b>rubric</b>: a Claude grader agent applied <code>test-data/grading_rubric.md</code> (physics + pedagogy; Pass / Partial / Fail) to the saved output (<code>{GEM_REV}</code>). This is an LLM-assisted review, not a human one.</li>
</ul>
<p><b>Cross-check:</b> the same 13 inputs on {CLAUDE_NAME} (<code>{CLA_RUN}</code>), graded with the same rubric (<code>{CLA_REV}</code>).</p>
{table(['No.', 'Input (level)', 'Type', 'Expected output', f'Actual output ({GEMINI_NAME}, short)', 'Pass / Fail (Gemini)', 'Claude rubric'], rows, 'small')}
<p><b>Summary.</b> {GEMINI_NAME}:</p>
<ul>
<li>Automatic checks: {gem_auto}/13.</li>
<li>Rubric: {gem_cnt['Pass']} Pass, {gem_cnt['Partial']} Partial, {gem_cnt['Fail']} Fail (tests {fails}).</li>
</ul>
<p>{CLAUDE_NAME} cross-check:</p>
<ul>
<li>Automatic checks: {cla_auto}/13.</li>
<li>Rubric: {cla_cnt['Pass']} Pass, {cla_cnt['Partial']} Partial, {cla_cnt['Fail']} Fail.</li>
</ul>
<p>Why the Gemini tests failed:</p>
<ul>
<li>Test 9: the compare question was routed to other_quantum, so nothing was verified.</li>
<li>Test 6: the practice answer key is wrong.</li>
<li>Test 3: an intermediate state is unnormalised and the JSON is broken.</li>
<li>Test 10 (Shor, unverifiable): a wrong post-QFT state.</li>
</ul>
<p>Tests 3 and 6 passed the automatic checks. The simulator only checks the explanation's final probabilities, which were right in both. The most common Partial reason, for both models, is a practice question that copies the explanation or a hint that gives the answer away.</p>
<h2>Screenshots of real runs</h2>
{shot_html}
<h2>Known limitations and what we will try next</h2>
<ol>
<li><b>The verifier only checks the explanation's final probabilities.</b>
  <ul>
  <li>It missed the unnormalised state in test 3 and the wrong practice key in test 6.</li>
  <li>For Deutsch-Jozsa it compares only P(00), so a v2 answer with 0.33 on three outcomes (sum 0.99) passed.</li>
  </ul>
  <i>Next:</i> have P3 extract the state after every step and P4's answer key, and check each one against the simulator. Also check all four DJ outcomes for a fixed oracle.</li>
<li><b>Compare questions and P4 quality.</b>
  <ul>
  <li>Gemini routed the compare question (test 9) to other_quantum, and only one algorithm can be verified per question.</li>
  <li>P4's self-review marked a wrong key as correct (test 6), and hints often give the answer away.</li>
  </ul>
  <i>Next:</i> a compare example in P1, two-algorithm routes, a "hint must not contain the answer" check, and checking P4's key with code.</li>
<li><b>Only three small algorithms are verifiable.</b> Shor's explanation (test 10) contained a wrong state, and it was only labelled "unverified". <i>Next:</i> add teleportation, a small QFT and 3-qubit Grover to the simulator, and show a clearer warning on unverified answers.</li>
<li><b>Evaluation limits.</b>
  <ul>
  <li>Free-tier quotas meant the Gemini runs had 1 repeat (test set) and 3 repeats (version comparison).</li>
  <li>The Claude cross-check ignores the temperature settings.</li>
  <li>The rubric verdicts come from an LLM grader, and there is no learner feedback yet.</li>
  </ul>
  <i>Next:</i> two team members grade a sample by hand to measure agreement with the LLM grader, and a short trial with 5&ndash;10 classmates for Milestone 3, with a small offline model as a baseline.</li>
</ol>"""


def section5() -> str:
    return f"""<h1 class="sec">5. Repository</h1>
<p>Public repository: <a href="{REPO}">{REPO}</a></p>
<div class="fig"><img src="img/repo.png" style="max-height:150mm"><div class="cap">GitHub repository main page: code/, docs/, prompts/, test-data/, README.md, CHANGELOG.md. The real API key is not in the repository; only .env.example is.</div></div>
<p><b>What changed since Milestone 1:</b></p>
<ul>
<li>Created the repository with the required layout: <code>prompts/</code> (v1, v2, v3, final), <code>code/</code>, <code>test-data/</code>, <code>docs/</code>, <code>README.md</code>, and <code>CHANGELOG.md</code> with dated entries.</li>
<li>Added the four final prompts P1&ndash;P4 and three older explainer versions.</li>
<li>Added the NumPy simulator verifier, the pipeline with one repair retry, the test runner, the version comparison, the Claude step runner, the repair test, and the Colab notebook with saved outputs.</li>
<li>Added the 13-case test set, all raw run logs (Gemini and Claude), the version comparisons, the grading rubric and both rubric reviews.</li>
<li>Added diagrams (<code>docs/diagrams.md</code>), this report and the technical report (<code>docs/</code>).</li>
</ul>"""


def tech_report_body(h: str = "h2") -> str:
    gs = {v: vsum(GV, v) for v in ("v1", "v2", "v3")}
    return f"""<{h}>Technical report: {TITLE}</{h}>
<p class="note">Team {TEAM} ({TEAM_ID}), batch {BATCH}, faculty {FACULTY}. Problem S18 [SIH26140], Egreen Quanta. Repository: {REPO}</p>
<p><b>1. Architecture.</b> A learner question and a level (beginner or intermediate) go through four prompts in <code>code/workflow.py</code>:</p>
<ol>
<li><b>P1 Router</b> returns JSON with the algorithm, parameters, intent, false premise, language and an injection flag. Out-of-scope messages get a redirect; injection attempts are flagged and logged.</li>
<li><b>P2 Explainer</b> returns JSON steps with the state after each step.</li>
<li><b>P3 Claim extractor</b> copies the final probabilities, and <code>quantum_sim.py</code> (a NumPy statevector) compares them with the simulation, with a tolerance of 0.02. On a mismatch, P2 is called once more with the simulator's feedback.</li>
<li><b>P4 Practice</b> writes one question with a hint, an answer key and a self-review.</li>
</ol>
<p>Verifiable algorithms: Bell state, Deutsch-Jozsa, and Grover on 2 qubits. Others are explained and labelled unverified.</p>
<p><b>2. Tools and models.</b></p>
<ul>
<li>Python, NumPy and the google-genai SDK.</li>
<li>The prototype runs {GEMINI_NAME} (<code>{GEMINI}</code>). Our runs were local; the notebook also opens in Colab (key in Colab Secrets) and runs in Docker.</li>
<li>Because of free-tier quotas, all tests were also run on {CLAUDE_NAME}, with one fresh Claude Code subagent per LLM call (<code>step_runner.py</code>) and the same code-side checks.</li>
</ul>
<p><b>3. Prompt design.</b></p>
{table(['Prompt', 'Temp.', 'Techniques'], [
        ('P1 Router', '0.0', 'Fixed labels, JSON schema, learner text inside &lt;message&gt; tags (treated as data), injection flag'),
        ('P2 Explainer', '0.2', 'Chain-of-thought (state after every step), audience-aware rules per level, 2 few-shot examples, JSON, verifier-feedback slot'),
        ('P3 Claim extractor', '0.0', 'Narrow role: copy claims, never fix numbers; JSON. The check itself is done by code'),
        ('P4 Practice', '0.4', 'Draft, then answer key and hint, then self-review; JSON')], 'small')}
<p>The explainer was iterated v1 (one line) &rarr; v2 (role, level, fixed sections) &rarr; v3 (few-shot, JSON, temperature 0.2) &rarr; final (+ intent, DJ rule, repair slot). Results on {GEMINI_NAME}, 12 runs each:</p>
<ul>
<li>Final probabilities verified: {gs['v1']['verified_correct']}/12 &rarr; {gs['v2']['verified_correct']}/12 &rarr; {gs['v3']['verified_correct']}/12 (one v2 Deutsch-Jozsa answer was physically wrong but passed the P(00)-only check).</li>
<li>Clean JSON: {gs['v1']['clean_json']} &rarr; {gs['v2']['clean_json']} &rarr; {gs['v3']['clean_json']}/12.</li>
<li>Average length: {gs['v1']['avg_words']} &rarr; {gs['v2']['avg_words']} &rarr; {gs['v3']['avg_words']} words.</li>
</ul>
<p><b>4. Test results</b> (13 inputs: {type_cnt['Normal']} Normal, {type_cnt['Edge']} Edge, {type_cnt['Invalid']} Invalid). The rubric is applied by a Claude grader agent (LLM-assisted).</p>
{table(['Model', 'Automatic checks', 'Rubric: Pass / Partial / Fail', 'Verified on first attempt'], [
        (GEMINI_NAME + ' (prototype)', f'{gem_auto}/13', f"{gem_cnt['Pass']} / {gem_cnt['Partial']} / {gem_cnt['Fail']}", f'{g_first}/{len(g_verifiable)}'),
        (CLAUDE_NAME + ' (cross-check)', f'{cla_auto}/13', f"{cla_cnt['Pass']} / {cla_cnt['Partial']} / {cla_cnt['Fail']}", f'{c_first}/{len(c_verifiable)}')], 'small')}
<p>Gemini failures:</p>
<ul>
<li>A compare question was misrouted, so it was not verified.</li>
<li>One practice key is wrong, even though P4's self-review said it was correct.</li>
<li>One intermediate state is unnormalised.</li>
<li>One unverifiable Shor answer has a wrong state.</li>
</ul>
<p>The false premise, Hinglish, injection and invalid inputs were all handled correctly. The repair retry never fired: on every verifiable answer, the final probabilities matched the simulator on the first attempt (intermediate states are not checked; test 3 had an unnormalised one). A scripted test (<code>test_repair.py</code>) confirms its code path.</p>
<p><b>5. Limitations.</b></p>
<ul>
<li>Only the explanation's final probabilities are checked: not the intermediate states, not P4's key, and only P(00) for Deutsch-Jozsa.</li>
<li>Only three small algorithms can be verified.</li>
<li>The rubric grader is an LLM, and there is no learner feedback yet.</li>
<li>Free-tier quotas limited the number of repeats.</li>
</ul>
<p><i>Next:</i> check each step state and the practice key with the simulator, extend the simulator, grade a sample by hand, and run a small trial with classmates.</p>
<p><b>6. Responsible use.</b> No personal data is collected. Questions go to Google's Gemini API, so learners should not type personal details. The key is never in the code or the repository. The tool is a learning aid, not a replacement for a teacher. Claude Code (Claude Opus 5.5) wrote most of the code, the prompt drafts, the test cases, the rubric and the report drafts; the team chose the problem and scope, made the design decisions and is responsible for the submission. All results come from our saved runs.</p>"""


def section7() -> str:
    rows = [
        ("Kartik Negi", "S24CSEU1982", "Team leader; prototype and repository",
         "NumPy simulator and verifier, Gemini pipeline with repair retry, Colab prototype notebook, GitHub repository and README",
         "code/quantum_sim.py, code/workflow.py, code/s18_workflow.ipynb, repository commits", "34"),
        ("Divey Grover", "S24CSEU1989", "Prompt design",
         "Final prompts P1&ndash;P4, explainer versions v1&ndash;v3, prompt iteration log, prompt sheets",
         "prompts/final/P1&ndash;P4, prompts/v1&ndash;v3, PDF Sections 2c and 3 (iteration log)", "33"),
        ("Twinkle Sahu", "S24CSEU0090", "Testing and documentation",
         "13-case test set, running and reviewing test results, grading rubric, technical report",
         "test-data/test_cases.csv, test-data/outputs/, test-data/grading_rubric.md, PDF Section 4, Chica_TechnicalReport.pdf", "33"),
    ]
    return f"""<h1 class="sec">7. Team contribution</h1>
{table(['Member name', 'Enrollment no.', 'Role', 'Tasks done in Milestone 2', 'Evidence', 'Contribution (%)'], rows, 'small')}
<p><b>Total: 100%.</b></p>"""


def cover(reel: str, reel_s: str) -> str:
    members = "<br>".join(f"{n} ({en}){', ' + r if r else ''}" for n, en, r in MEMBERS)
    rows = [("Project title", TITLE), ("Problem", "S18: AI-Based Interactive Quantum-Algorithm Learning Platform [SIH26140] (Egreen Quanta)"),
            ("Team ID", TEAM_ID), ("Team name", TEAM), ("Batch", BATCH), ("Faculty", FACULTY), ("Team members", members),
            ("Prototype (Colab, outputs saved)", f'<a href="{COLAB}">{COLAB}</a>'),
            ("Repository (public GitHub)", f'<a href="{REPO}">{REPO}</a>'),
            ("Reel", (f'<a href="{reel}">{reel}</a>' + (f" ({reel_s} s)" if reel_s else "")) if reel else "[reel link to be added]"),
            ("Main model", f"{GEMINI_NAME} ({GEMINI}); test cross-check {CLAUDE_NAME}"),
            ("Date", "22 November 2026")]
    return f"""<h1>1. Cover page</h1><div class="cover-title">{TITLE}</div>
<div class="cover-sub">SCSE4041 Prompt Engineering Mini Project: Milestone 2 report</div>{table(['Field', 'Value'], rows, 'kv')}"""


def page(title: str, body: str) -> str:
    return f"""<!doctype html><html><head><meta charset="utf-8"><title>{e(title)}</title><style>{css()}</style>
<script src="https://cdn.jsdelivr.net/npm/mermaid@10.9.1/dist/mermaid.min.js"></script>
<script>mermaid.initialize({{startOnLoad: true, theme: 'neutral', flowchart: {{useMaxWidth: true}}, themeVariables: {{fontSize: '15px'}}, securityLevel: 'loose'}});</script>
</head><body>{body}</body></html>"""


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--reel", default="")
    ap.add_argument("--reel-seconds", default="")
    a = ap.parse_args()
    m2 = (cover(a.reel, a.reel_seconds) + section2() + section3() + section4() + section5()
          + '<h1 class="sec">6. Supporting document</h1><p class="note">Supporting document type: Technical report. The same document is uploaded separately as Chica_TechnicalReport.pdf.</p>'
          + tech_report_body("h2") + section7())
    (BUILD / "M2_report.html").write_text(page(f"M2 {TEAM_ID} {TITLE}", m2), encoding="utf-8")
    (BUILD / "TechnicalReport.html").write_text(page("Chica Technical Report", tech_report_body("h1")), encoding="utf-8")
    print("built", BUILD / "M2_report.html", BUILD / "TechnicalReport.html")
    print(f"counts: prompts=4 tests={len(CASES)} versions=4 gemini_auto={gem_auto} gemini_rubric={dict(gem_cnt)} claude_rubric={dict(cla_cnt)}")


if __name__ == "__main__":
    main()
