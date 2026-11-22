# Reel script (target about 130 s; must be 90-150 s)

Record the screen with OBS or Xbox Game Bar (Win+Alt+R). Use a teammate's voice-over or captions.
Do not show `.env`, Colab Secrets or any API key on screen.

| # | Time | Show on screen | Voice-over (read at a normal pace) |
|---|---|---|---|
| 1 Problem | 0:00-0:12 | README on GitHub (repo page) | "Students learning quantum computing get explanations that skip steps or state wrong numbers. Our tool explains a quantum algorithm step by step at the learner's level, checks the numbers with a simulator, and gives a practice question. It's for B.Tech CS students who know linear algebra." |
| 2 Prompt experiment | 0:12-0:50 | `docs/reel/before_after.html` in a browser: scroll the left column, then the right | "Before: version 1 was just 'Explain the question'. Gemini wrote 535 words of LaTeX, and our checker couldn't find the final probabilities, so the simulator check failed. Over 12 runs, v1 was verified only 5 times. After: version 3 adds a tutor role, level rules, two examples, JSON output and temperature 0.2. Now it's 133 words of clean JSON, verified by the simulator, and 12 out of 12 runs were verified." |
| 3 Working demo | 0:50-1:50 | VS Code terminal, a **live** run: `cd code` then `py workflow.py "How do you create a Bell state?"`. Scroll the output: route, explanation, `"verification": {"ok": true ...}`, practice. Then `py test_repair.py` (repair retry, no API). | "This is a real run on Gemini 3.5 Flash-Lite. P1 routes the question to the Bell state. P2 explains it with the state after every gate. P3 extracts the claimed probabilities, and our NumPy simulator confirms 0.5 and 0.5, so it's verified. P4 writes a practice question. If the numbers were wrong, the pipeline retries once with the simulator's feedback; this test shows that path." |
| 4 Progress and next steps | 1:50-2:10 | PDF Section 4 test table (Fail rows visible) | "We ran 13 tests, including edge cases. On Gemini, 4 passed, 5 were partial and 4 failed. For example, one wrong practice answer key and one misrouted compare question. For Milestone 3 we'll verify every intermediate state and the practice key with the simulator, add more algorithms, and test with real students." |

If the live run fails with a 429 or 503 error, wait a minute and retry, or run
`$env:GEMINI_MODEL="gemini-3.1-flash-lite"` first (PowerShell). Use a run that worked; do not edit the output.
Upload to Google Drive or unlisted YouTube with "Anyone with the link can view", and note the exact length in seconds.
