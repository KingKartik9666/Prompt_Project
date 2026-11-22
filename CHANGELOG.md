# Changelog

## 2026-11-22
- Created repository structure (prompts/, code/, test-data/, docs/).
- Added NumPy statevector simulator for Bell, Deutsch-Jozsa and 2-qubit Grover (code/quantum_sim.py).
- Added explainer prompt versions v1 (plain), v2 (role, level, fixed sections), v3 (few-shot, JSON, temperature 0.2).
- Added final prompts P1 router, P2 explainer (with verifier feedback), P3 claim extractor, P4 practice generator.
- Added pipeline with one repair retry when the simulator finds a mismatch (code/workflow.py).
- Added 13-case test set (6 normal, 5 edge, 2 invalid) and scripts to run tests and compare versions.
- Added diagrams (docs/diagrams.md).
- Code review fixes before the first test run: the simulator now uses the router's parameters, never the ones the LLM echoes back; router defaults are applied in code (norm_params); claim keys and values like "|00>", "1/2" and "50%" are normalised; Deutsch-Jozsa checks only P(all zeros) and rejects ancilla-included keys; the test runner checks intent, language, injection flag and marked item, escapes "|" in tables, and labels API errors as Error rather than Fail; the version comparison saves results after each version.
- P2 final: passes intent (compare questions), states P(all zeros) for Deutsch-Jozsa, and uses a corrected Bell analogy (the old one described classical correlation). P3: records "never" outcomes as 0.0.
- Model changed to gemini-3.6-flash: gemini-2.5-flash is closed to new API keys, and gemini-3.7/3.8-flash returned 503 (overloaded) today.
- Gemini free tier hit per-minute rate limits (429) during the full test run; the partial Gemini run is kept in test-data/outputs/run_20261122_1700.jsonl.
- Testing moved to Claude (claude-opus-5-5, run as one fresh Claude Code subagent per LLM call through code/step_runner.py). The code still does template filling, parameter normalisation, the simulator check, the repair decision and logging. The released prototype keeps the Gemini API backend (code/workflow.py).
- Claude test run (run_claude_20261122_1712.jsonl): 13/13 tests met the automatic checks; the repair loop was never triggered. Explainer versions (versions_claude_20261122_1712.json): v1, v2 and v3 were each 8/8 verified correct; clean JSON went from 0/8 (v1, v2) to 8/8 (v3), and average length from 562 to 527 to 291 words.
- Manual review by independent graders (manual_review_claude_20261122.json): 7 Pass, 6 Partial, 0 Fail. Main weakness: P4 practice questions often copy the explanation's example and their hints nearly give the answer away. Level fit was rated "good" in only 2 of 8 outputs for each of v1, v2 and v3.
- Prototype notebook executed on Gemini with outputs saved; model changed to gemini-3.5-flash-lite because gemini-3.6-flash and gemini-3.5-flash reached the free-tier limit of 20 requests per day.
- First end-to-end run (Bell state question): routed, explained, verified and practice question generated; saved in test-data/outputs/smoke_bell.json.

- 17:10 Full 13-case run on gemini-3.5-flash-lite (run_20261122_1710.jsonl): automatic checks 12/13. Test 9 ("compare Grover and Deutsch-Jozsa") was routed to other_quantum, so it was not verified.
- 17:14 Explainer version comparison on gemini-3.5-flash-lite (versions_20261122_1714.json, 4 questions x 3 repeats).
  - Verified correct: v1 5/12, v2 12/12, v3 12/12.
  - Clean JSON: v1 0/12, v2 0/12, v3 12/12.
  - Average words: 615, 394, 178.
- 17:35 workflow.py, run_tests.py and compare_versions.py edited after the runs above: model fallback chain (GEMINI_FALLBACKS) and models_used().
- Rubric review of the Gemini run by Claude grader agents (review_gemini_20261122_1710.json, rubric in test-data/grading_rubric.md): 4 Pass, 5 Partial, 4 Fail.
  - Test 6: the practice answer key is wrong.
  - Test 3: an unnormalised intermediate state.
  - Test 9: misrouted.
  - Test 10 (Shor, unverifiable): wrong post-QFT state.
  - The simulator check passed on tests 3 and 6 because it only checks the explanation's final probabilities.
- Renamed the "manual review" wording to "LLM-assisted rubric review", because those graders were Claude agents, not people.
- Added code/test_repair.py: a control-flow test of the repair retry with scripted model answers (all 6 checks pass). The retry never fired in a real run.
- Project title set to the Milestone 1 title: AI-Based Interactive Quantum-Algorithm Learning Platform.

<!-- Add a dated line each time you run tests or change a prompt, with who did it. -->
