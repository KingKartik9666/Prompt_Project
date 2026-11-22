# AI-Based Interactive Quantum-Algorithm Learning Platform (S18 / SIH26140)

A prompt-engineering workflow that answers a learner's question about a quantum algorithm with a step-by-step explanation at their level, **checks the explanation's numbers against a simulator**, and gives a practice question.

Problem pool ID: **S18**, AI-Based Interactive Quantum-Algorithm Learning Platform [SIH26140] (Egreen Quanta).
Users: B.Tech CS students who know linear algebra but are new to quantum computing.

**Prototype:** [`code/s18_workflow.ipynb`](code/s18_workflow.ipynb) (outputs saved) · Open in Colab: https://colab.research.google.com/github/KingKartik9666/Prompt_Project/blob/main/code/s18_workflow.ipynb

Team **Chica** (Team ID PE-B65-13), batch 76, faculty: Sonal Kukreja.

## Workflow
P1 router → P2 explainer → P3 claim extractor + NumPy simulator check (one repair retry) → P4 practice question.
Diagrams: [docs/diagrams.md](docs/diagrams.md). Final prompts: [prompts/final/](prompts/final/).

Verifiable algorithms: Bell state, Deutsch-Jozsa, Grover on 2 qubits. Others are explained but labelled unverified.

## Run it
```bash
pip install -r requirements.txt
# set GEMINI_API_KEY in your environment (see .env.example). Never commit it.
cd code
python quantum_sim.py              # simulator self-check, no API needed
python workflow.py "How do you create a Bell state?"
python run_tests.py                # 13 test cases -> test-data/outputs/
python compare_versions.py         # explainer v1 vs v2 vs v3 -> test-data/outputs/
python test_repair.py              # repair-retry control-flow test, no API needed
```
Or open the notebook in VS Code / Colab (Colab: add `GEMINI_API_KEY` in Secrets).

## Run with Docker (any PC: Windows, macOS, Linux)
Needs only [Docker Desktop](https://www.docker.com/products/docker-desktop/). The API key is never stored in the image; you pass it when you run a container.
```bash
git clone https://github.com/KingKartik9666/Prompt_Project.git
cd Prompt_Project
docker build -t s18-tutor .

# 1. No key needed: simulator self-check, repair-retry test, saved test table
docker run --rm s18-tutor

# 2. Live run of the full pipeline on one question (uses your free Gemini key from https://aistudio.google.com/apikey)
docker run --rm -e GEMINI_API_KEY=<your key> s18-tutor ask "How do you create a Bell state?" beginner

# 3. The prototype notebook in JupyterLab at http://localhost:8888
docker run --rm -p 8888:8888 -e GEMINI_API_KEY=<your key> s18-tutor notebook
#    or: docker compose up --build   (reads GEMINI_API_KEY from your shell or a local .env file)

# Other modes: tests (all 13 cases, about 40 requests) | versions (v1/v2/v3 comparison) | bash
```
Without a key, the notebook still opens with its saved outputs, and the simulator cells run.

## Which model produced which results
- **Released prototype (main model):** Gemini 3.5 Flash-Lite (`gemini-3.5-flash-lite`) through the Gemini API (`code/workflow.py`, notebook).
  - Full 13-case run: `test-data/outputs/run_20261122_1710.jsonl` and `test_table_20261122_1710.md`. The automatic checks passed on 12/13; test 9 was misrouted.
  - Explainer v1/v2/v3 comparison: `versions_20261122_1714.json`, 4 questions x 3 repeats.
  - Also: the executed notebook (live demos) and `smoke_bell.json` (gemini-3.6-flash).
- **Cross-check model:** Claude (`claude-opus-5-5`), with each LLM call answered by a fresh Claude Code subagent through `code/step_runner.py`. The temperature settings in the prompt files do not apply to these runs. Files: `run_claude_20261122_1712.jsonl`, `test_table_claude_*.md`, `versions_claude_*.json` (4 questions x 2 repeats).
- **Rubric review (LLM-assisted, not human):** Claude grader agents applied [`test-data/grading_rubric.md`](test-data/grading_rubric.md) to both runs.
  - `review_gemini_20261122_1710.json`: 4 Pass, 5 Partial, 4 Fail.
  - `manual_review_claude_20261122.json`: 7 Pass, 6 Partial, 0 Fail.
- **Repair retry:** it never fired in a real run, because every answer was verified on its first attempt. `code/test_repair.py` checks the code path with scripted model answers and no API (`test-data/outputs/test_repair_console.txt`).

## Repository layout
| Path | Contents |
|---|---|
| `prompts/v1`, `v2`, `v3` | Older explainer versions |
| `prompts/final` | P1–P4 final prompts (settings, system, user template) |
| `code/` | Simulator, pipeline, test runner, version comparison, notebook |
| `test-data/` | `test_cases.csv` and saved outputs |
| `docs/` | Diagrams and Milestone 2 PDF |
| `CHANGELOG.md` | Dated changes |

## Team
| Name | Enrollment | Role |
|---|---|---|
| Kartik Negi (leader) | S24CSEU1982 | Prototype, simulator, repository |
| Divey Grover | S24CSEU1989 | Prompt design P1–P4, iteration log |
| Twinkle Sahu | S24CSEU0090 | Test set, evaluation, supporting document |

## AI assistance
We used Claude Code (Claude Opus 5.5) heavily. It wrote most of the code, the prompt drafts, the test cases, the grading rubric, the diagrams, this README and the report drafts. The cross-check test run and the rubric grading were run through Claude Code subagents, as described above. The team chose the problem, the users and the scope, made the design and tool decisions, and is responsible for the submission.

## Responsible use
Explanations are learning aids; numerical claims are checked only for the three supported algorithms. No personal data is collected. Model: Gemini 3.5 Flash-Lite (`gemini-3.5-flash-lite`) via the `google-genai` Python SDK.
