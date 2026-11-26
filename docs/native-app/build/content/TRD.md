# Introduction

## Purpose

This Technical Requirements Document (TRD) states what the Quantum Explainer native Windows application must do, how it is built, and how each requirement is verified. It describes the application as it exists on 25 November 2026 (version 0.1.0, source in `k:\Prompt\app`). Each requirement carries a status, so that implemented behaviour, partial behaviour and planned work are kept apart.

## Scope

Quantum Explainer teaches three quantum algorithms (the Bell state, Deutsch–Jozsa and Grover search on two qubits). It explains them step by step with a large language model (LLM) of the learner's choice, and checks every final probability in the explanation with a simulator built into the application before the learner reads it. It also contains the Atlas, a growing map of quantum-computing topics whose interactive circuits are checked by the same simulator, and a Lab that runs fully offline.

In scope: the native window and its web view, the user interface, the four-stage AI pipeline, the simulator and verifier, the multi-provider LLM layer, key storage, the Atlas content system, the build process and the tests. Out of scope: the companion website (planned for later), installers and code signing, and any platform other than Windows.

## Relationship to the Milestone 2 prototype

The Milestone 2 prototype is a Python notebook workflow (`code/workflow.py`, `code/quantum_sim.py`) that was evaluated on 22 November 2026. The native application is a from-scratch rebuild in Rust. It embeds the same four evaluated prompt files (`prompts/final/P1–P4`) at compile time, and its simulator and verifier are ported from the prototype; unit tests assert that both give the same ground truth and the same mismatch messages.

## Intended readers

Engineers continuing the work, faculty evaluators, and reviewers who need to trace a requirement to the code and to the test that checks it.

# Definitions and abbreviations

| Term | Meaning |
|---|---|
| Atlas | The in-app map of quantum-computing topics, with one deep page per topic |
| CSP | Content-Security-Policy, the HTTP header that limits what the page may load |
| Host | The native Rust process (crate `desktop`) that owns the window, the keys and all network calls |
| IPC | Inter-process messages between the web page and the host (`window.ipc.postMessage` and `window.__qe`) |
| LLM | Large language model, reached through a provider's HTTP API |
| P1–P4 | The four prompts: router, explainer, claim extractor, practice |
| qcore | The shared simulator crate, compiled into both the host and the WebAssembly UI |
| Repair retry | One extra explainer call with the simulator's correct result when the first explanation is wrong |
| Verifiable | A question the simulator can check: Bell, Deutsch–Jozsa (n = 1–3) or Grover on 2 qubits |
| WebView2 | Microsoft's Chromium-based web engine built into Windows, used to draw the interface |
| WASM | WebAssembly, the compiled form of the Rust user interface |

# Stakeholders and users

| Stakeholder | Goal | Environment |
|---|---|---|
| Learner (B.Tech student, self-learner) | Understand an algorithm step by step and trust the numbers | Windows 11 PC, own LLM API key, internet for the Tutor |
| Evaluator (faculty) | Run the app, check claims, judge quality and honesty | Windows PC with WebView2 Runtime |
| Developer (team) | Extend topics, fix findings, rebuild | Rust toolchain described in chapter 10 |
| LLM provider | Receives requests made with the learner's key | Provider HTTPS APIs |

# Constraints and assumptions

1. **C-1 Platform.** The application runs on Windows only. It uses Windows Credential Manager, `MessageBoxW`, `ShellExecuteW` and WebView2 builder extensions.
2. **C-2 Runtime.** The Microsoft Edge WebView2 Runtime must be installed (it is included with Windows 11). Development used runtime 158 (Chromium 158).
3. **C-3 Toolchain.** The development PC has no MSVC Build Tools, so the app is built with Rust 1.100.0 on the `stable-x86_64-pc-windows-gnullvm` toolchain with LLVM-MinGW 23.1.6-20261117.
4. **C-4 Network.** The Tutor and key checks need internet access. The Lab and the Atlas work offline.
5. **C-5 Provider limits.** Providers apply their own quotas. During testing the Gemini free tier returned HTTP 503 "high demand" at times and enforced a small daily request limit.
6. **C-6 Learner's key.** The app ships with no key. Each learner supplies their own provider key.
7. **C-7 Verification scope.** Only the Bell state, Deutsch–Jozsa with n = 1–3 and 2-qubit Grover can be checked by the Tutor's verifier. Other questions are answered and labelled unverified.
8. **C-8 Newest stable technology.** The project owner asked for the newest stable release in each technology category and a native window rather than a browser tab.

# Functional requirements

Priority uses MoSCoW (Must, Should, Could). Status is Implemented, Partial or Planned. Verification names a unit test, the content gate, a dated manual check, or inspection of the code.

## Window and navigation

| ID | Requirement | Priority | Status | Verification |
|---|---|---|---|---|
| FR-WIN-1 | The app **shall** open as a single native window with no console and no browser tab | Must | Implemented | Inspection: `windows_subsystem = "windows"`, wry WebView in a tao window |
| FR-WIN-2 | The window **shall** be frameless with its own title bar, minimise, maximise, close and drag | Must | Implemented | Inspection of `ui/src/main.rs` title bar and host `win` commands |
| FR-WIN-3 | The window **shall** start at 1360 × 860 logical pixels, clamped to 90% of the primary monitor | Should | Implemented | Inspection of `desktop/src/main.rs` |
| FR-WIN-4 | The app **shall** offer Home, Atlas, Tutor and Lab screens and switch between them with a same-document View Transition | Must | Implemented | Screenshots 25 Nov; inspection of `bridge.rs` |
| FR-WIN-5 | Startup and runtime failures **shall** be shown in a native message box | Must | Implemented | Inspection: panic hook and `fatal()` |

## Home story

| ID | Requirement | Priority | Status | Verification |
|---|---|---|---|---|
| FR-HOME-1 | Home **shall** tell the product story as a scroll-driven sequence with a live Grover state shown in circle notation | Should | Implemented | Screenshots 01–02 (25 Nov) |
| FR-HOME-2 | Motion **shall** follow the scroll continuously, smoothed by a spring, without hijacking the scroll | Must | Implemented | Inspection of `home.rs` driver; manual review |
| FR-HOME-3 | Motion **shall** respect the reduced-motion setting | Must | Partial | Inspection; finding U3 notes clipped captions under reduced motion |

## Atlas and topic pages

| ID | Requirement | Priority | Status | Verification |
|---|---|---|---|---|
| FR-ATL-1 | The Atlas **shall** show all sections and topics as a map that grows outward from one root dot | Should | Implemented | Screenshot 03 |
| FR-ATL-2 | Search **shall** filter topics by title and text | Should | Implemented | Manual (headless run 25 Nov: no injected elements for hostile input) |
| FR-ATL-3 | Each topic page **shall** show prose, interactive circuits, misconceptions, a practice item and sources | Must | Implemented | Screenshots 04–05; content files |
| FR-ATL-4 | Every circuit outcome stated in a page **shall** match the simulator within 5 × 10⁻⁴ | Must | Implemented | `topic::tests::atlas_content_passes_the_gate` |
| FR-ATL-5 | Every link between topics **shall** point at an existing topic | Must | Implemented | Content gate; `gate_catches_a_wrong_claim_and_a_dead_link` |
| FR-ATL-6 | The Atlas **shall** cover the 40 topics planned in `atlas-spec.md` | Could | Partial | 3 of 40 pages exist; 3 of 6 sections show "coming soon" |

## Lab

| ID | Requirement | Priority | Status | Verification |
|---|---|---|---|---|
| FR-LAB-1 | The Lab **shall** run Bell, Deutsch–Jozsa (n = 1–3, any oracle kind) and Grover (any marked item) step by step without a network | Must | Implemented | Screenshots 06–07; `traces_are_normalised_and_consistent` |
| FR-LAB-2 | Each step **shall** show the circuit with the active gate, the ket, and every amplitude in circle notation (radius = magnitude, line = sign or phase) | Must | Implemented | Screenshot 07; inspection of `viz.rs` |
| FR-LAB-3 | Arrow keys **shall** step forward and back | Should | Implemented | Inspection of the global key handler |

## Tutor pipeline

| ID | Requirement | Priority | Status | Verification |
|---|---|---|---|---|
| FR-TUT-1 | The Tutor **shall** accept a question and a level (beginner or intermediate) | Must | Implemented | Inspection; 24 Nov live run |
| FR-TUT-2 | P1 **shall** route the question to an algorithm, parameters, intent, language, false premise and an injection flag | Must | Implemented | `embedded_prompts_parse`; 24 Nov live run |
| FR-TUT-3 | Out-of-scope questions **shall** stop after P1 with a polite redirect | Must | Implemented | Inspection of `pipeline.rs` |
| FR-TUT-4 | P2 **shall** explain the algorithm step by step at the chosen level | Must | Implemented | 24 Nov live run |
| FR-TUT-5 | P4 **shall** produce one practice question with a hint and an answer | Should | Implemented | 24 Nov live run |
| FR-TUT-6 | The UI **shall** show the four stages live, with retry notices | Must | Implemented | Inspection of `state.rs` event handling |
| FR-TUT-7 | The learner **shall** be able to cancel a running question | Could | Planned | No cancel exists today |

## Verification

| ID | Requirement | Priority | Status | Verification |
|---|---|---|---|---|
| FR-VER-1 | For verifiable questions, P3 **shall** extract the claimed final probabilities and qcore **shall** compare them with the simulation (absolute tolerance 0.02) | Must | Implemented | `verifier_matches_python_prototype` |
| FR-VER-2 | Ground truth **shall** come from the router's normalised parameters, never from P3's echo | Must | Implemented | Inspection of `pipeline.rs` |
| FR-VER-3 | On a mismatch, P2 **shall** be called once more with the simulator's result; there **shall** be no third attempt | Must | Implemented | Inspection |
| FR-VER-4 | The verdict **shall** read "Verified by the simulator", "Corrected, then verified", "The numbers do not match" or "Explained, not verified" | Must | Implemented | Inspection of `tutor.rs` |
| FR-VER-5 | The verifier **shall** reject non-finite, negative or out-of-range probabilities | Must | Planned | Findings V1–V3 open: NaN and out-of-range claims can pass |
| FR-VER-6 | For Deutsch–Jozsa only P(all zeros) **shall** be checked, because many balanced functions exist | Must | Implemented | `verifier_matches_python_prototype` |

## AI providers and keys

| ID | Requirement | Priority | Status | Verification |
|---|---|---|---|---|
| FR-KEY-1 | The app **shall** support 11 provider presets over three wire formats (Gemini, Anthropic Messages, OpenAI-compatible) | Must | Implemented | `every_detected_provider_exists_and_ids_are_unique` |
| FR-KEY-2 | Any user **shall** be able to paste their own key; the key **shall** be checked with the provider before it is saved | Must | Implemented | `save_key_provider_resolution`; 24 Nov fake-key rejection |
| FR-KEY-3 | Keys **shall** be stored in Windows Credential Manager (service `QuantumExplainer`, account `<provider>_api_key`) and never in a file | Must | Implemented | `stored_settings_hold_no_key` |
| FR-KEY-4 | The provider **shall** be guessed from the key prefix only where the prefix is unambiguous | Should | Implemented | `prefixes_route_only_when_unambiguous` |
| FR-KEY-5 | The learner **shall** be able to choose a model from the provider's live model list | Should | Implemented | Inspection of `list_models`; model-list tests |
| FR-KEY-6 | A "Get a key" link **shall** open the provider's fixed key page in the default browser | Should | Implemented | Inspection of `open_key_page` |
| FR-KEY-7 | The learner **shall** be able to remove a stored key | Must | Implemented | Inspection of `clear_key` |

## Error handling

| ID | Requirement | Priority | Status | Verification |
|---|---|---|---|---|
| FR-ERR-1 | HTTP 429, 500, 502, 503, 504 and 529 **shall** be retried after 10, 30 and 60 s, with a visible notice | Must | Implemented | Inspection; 24 Nov live 503 retry |
| FR-ERR-2 | DNS, TLS and no-route failures **shall** fail at once with a clear message | Must | Implemented | Inspection of `llm/mod.rs` |
| FR-ERR-3 | Refusals, content-filter stops, safety blocks and empty answers **shall** become errors, not empty explanations | Must | Implemented | `refusal_is_an_error_and_content_is_not_read`, `parse_errors_on_refusal_filter_and_empty`, `empty_answer_is_an_error` |
| FR-ERR-4 | A failed settings change **shall** be shown to the learner | Should | Partial | Finding U5: `settings_error` is sent but never displayed |

# Non-functional requirements

| ID | Requirement | Target | Status | Evidence |
|---|---|---|---|---|
| NFR-SIZE-1 | The release build is one self-contained executable | One file | Met | `QuantumExplainer.exe`, 6.8 MB (25 Nov) |
| NFR-SIZE-2 | The WebAssembly bundle is kept small | Target, not yet set | Measured only | `ui_bg.wasm` 3,900 KB; no wasm-opt step yet |
| NFR-BUILD-1 | A release build completes in one command | `.\build.ps1` | Met | 94 s on 25 Nov |
| NFR-PERF-1 | Startup time and memory use | Target, not yet measured | Unknown | Not measured |
| NFR-PERF-2 | Animations touch only transform and opacity where possible | Compositor-only | Met by design | Inspection of `glow.rs` and CSS |
| NFR-SEC-1 | No local server and no open port | 0 ports | Met | Custom `app://` protocol |
| NFR-SEC-2 | The key never reaches the web view | Last 4 characters only | Met | `main.rs` status event; `keys_never_leak_into_messages` |
| NFR-SEC-3 | The page may load only embedded files | Strict CSP | Partial | CSP still allows `'unsafe-inline'` scripts (finding H1) |
| NFR-PRIV-1 | Questions and answers are not stored by the app | No logs | Met | Only `settings.json` is written |
| NFR-A11Y-1 | Live regions, visible focus and reduced motion | WCAG-style | Partial | Findings U3, U4, U6 open |
| NFR-REL-1 | One question at a time; old events are ignored | Run id filter | Partial | Finding V7: ids restart after a page reload |
| NFR-MAINT-1 | Every verifier and content claim is covered by an automated test | Unit tests | Met | 36 passing tests |
| NFR-PORT-1 | Other operating systems | Not required | Not met | Windows-only APIs (C-1) |
| NFR-LEGAL-1 | Fonts and libraries are redistributable | Open licences | Met | OFL fonts; MIT/Apache crates |

# External interfaces

## IPC commands (page to host)

| Command | Fields | Host action | Runs on |
|---|---|---|---|
| `hello` | none | Returns the status event | IPC handler |
| `win` | `action`: min, max, drag, close | Window control | Event loop |
| `save_key` | `key`, optional `provider` | Route, format check, live check, store, make active | Worker thread |
| `open_key_page` | `provider` | Opens the preset's fixed key page | IPC handler |
| `clear_key` | `provider` | Deletes the Credential Manager entry | IPC handler |
| `set_provider` | `provider`, optional `model` | Writes `settings.json` | IPC handler |
| `list_models` | `provider` | Fetches the live model list | Worker thread |
| `ask` | `id`, `question`, `level` | Runs P1–P4 | Worker thread |

The host accepts a message only when the request comes from `app.localhost` or the `app` scheme. Invalid JSON and unknown commands are ignored.

## Host events (host to page)

| Event | Meaning |
|---|---|
| `status` | Active provider, key present, last-4 hint, key source, model, provider list, version |
| `key_error` | A key could not be routed, checked or saved |
| `models`, `models_error` | Live model list, or why it failed |
| `stage` | A stage (route, explain, verify, practice) became active, done, skipped or failed |
| `route`, `explanation`, `verification`, `repair`, `practice` | Stage results |
| `notice` | Retry or fallback message |
| `done`, `error` | End of a run |

Events are passed to `window.__qe` as a JSON string literal, so no model text is ever interpolated into script code.

## LLM provider presets

| Preset | Wire format | Default model |
|---|---|---|
| gemini | Gemini native | gemini-3.5-flash-lite (fallback gemini-3.1-flash-lite) |
| anthropic | Anthropic Messages | claude-opus-5-5 |
| openai | OpenAI-compatible | gpt-6.2-astra |
| deepseek | OpenAI-compatible | deepseek-flash |
| moonshot | OpenAI-compatible | kimi-k3.1-pro |
| qwen | OpenAI-compatible | qwen4-max |
| xai | OpenAI-compatible | grok-4.7 |
| mistral | OpenAI-compatible | mistral-small-latest |
| groq | OpenAI-compatible | llama-3.3-70b-versatile |
| together | OpenAI-compatible | MiniMaxAI/MiniMax-M3.1-Flash |
| openrouter | OpenAI-compatible | openai/gpt-5.2 |

Base URLs, key pages and default models are compile-time constants; the page can name a provider only by its id. Only the Gemini client sends the prompts' temperature and JSON response type.

## Atlas content schema

One JSON file per topic in `crates/ui/content/topics/<id>.json`, plus `sections.json`. A topic has an id, section, level, title, kicker, summary, prerequisite, next and related links, sources, and blocks of type text, math, circuit, misconception and practice. A circuit block lists gates (`I X Y Z H S SDG T TDG RX RY RZ P SWAP` and controlled forms) and an `expect` map of outcome probabilities, optionally over a subset of qubits.

# Data and storage

| Data | Location | Notes |
|---|---|---|
| API keys | Windows Credential Manager | One entry per provider; never written to a file |
| Active provider and models | `%LOCALAPPDATA%\QuantumExplainer\settings.json` | No key field; written through a temporary file and rename |
| WebView2 profile | `%LOCALAPPDATA%\QuantumExplainer` | Never next to the exe |
| Interface, fonts, Atlas content | Embedded in the exe | rust-embed and `include_str!` at build time |
| Questions and answers | Not stored | Sent only to the chosen provider |

Key lookup order per provider: Credential Manager, then the provider's environment variable, then a `.env` file.

# System architecture

```text
+-- QuantumExplainer.exe (one Windows GUI process) --------------------+
|  main thread: tao event loop, frameless window, Mica backdrop        |
|  wry WebView2  ->  app://localhost/  (served from embedded files)     |
|     page: Leptos UI compiled to WebAssembly                           |
|       screens: Home, Atlas, Topic, Tutor, Lab; Settings popover       |
|       qcore (wasm): Lab traces, Atlas circuits, Tutor visuals         |
|          |  window.ipc.postMessage(JSON)                              |
|          v                                                            |
|  host: handle_ipc -> worker threads                                   |
|     pipeline P1 -> P2 -> P3 + qcore::verify -> P4                     |
|     llm: Gemini | Anthropic | OpenAI-compatible (11 presets)          |
|     ureq 3 + native TLS (SChannel) ==HTTPS==> provider APIs           |
|          |  window.__qe(<JSON string>)                                |
|          v  back to the page                                          |
|  storage: Credential Manager, settings.json, WebView2 profile         |
+-----------------------------------------------------------------------+
```

| Crate | Responsibility | Key files |
|---|---|---|
| `qcore` | Simulators (real-amplitude traces; general complex n-qubit), claim verifier, Atlas model and content gate | `lib.rs`, `circuit.rs`, `topic.rs` |
| `desktop` | Window, web view, `app://` server, IPC, pipeline, providers, key storage | `main.rs`, `pipeline.rs`, `llm/*.rs`, `config.rs` |
| `ui` | Leptos interface compiled to WebAssembly | `home.rs`, `atlas.rs`, `tutor.rs`, `lab.rs`, `viz.rs`, `app.css` |

# Technology stack

| Technology | Version | Role |
|---|---|---|
| Rust, edition 2024 | 1.100.0 | All three crates |
| Toolchain | stable-x86_64-pc-windows-gnullvm, LLVM-MinGW 23.1.6 | Native and WASM builds |
| tao | 0.37.3 | Window and event loop (0.38 clashes with wry 0.57) |
| wry | 0.57.3 | WebView2 host, custom protocol, IPC |
| WebView2 Runtime | 158 (Chromium 158) | Renders the interface |
| window-vibrancy | 0.8.1 | Windows 11 Mica backdrop |
| Leptos (CSR) | 0.8.23 | Reactive interface compiled to WebAssembly |
| wasm-bindgen | 0.2.130 (crate and CLI pinned) | JavaScript glue |
| rust-embed | 8.14.0 | Embeds the interface in the exe |
| ureq + native-tls | 3.5.2 / 0.2.18 | Blocking HTTPS with the OS certificate store |
| keyring | 4.2.0 | Windows Credential Manager |
| windows-sys | 0.61.2 | Message boxes, opening the key page |
| serde / serde_json | 1.0.229 / 1.0.154 | JSON for IPC, prompts and content |
| Fonts | Oswald, Libre Franklin, Source Serif 4 | Typography, embedded |

The release profile uses `opt-level = 3`, fat LTO, one codegen unit, `panic = "abort"` and stripping; the `ui` package uses `opt-level = "z"` for a smaller WebAssembly file.

# Security requirements and threat model

## Assets

API keys, the learner's questions, the integrity of the "Verified" label, and the learner's PC.

## Threats and mitigations

| Threat | Mitigation in place | Residual risk |
|---|---|---|
| Key theft by the page or injected content | Key stays in the host; only the last 4 characters reach the page | Low |
| Page navigated to a hostile site | Navigation allow-list, new windows denied, drops blocked, IPC origin check | Low |
| Script injection from model output | No HTML sinks; text nodes only; events passed as string literals | Low |
| Wrong number labelled "Verified" | Simulator check with repair retry | Medium: NaN and out-of-range claims pass (V1–V3) |
| Key leaked through redirects or error text | Keys only in headers; error text scrubbed | Medium: redirect policy left at default (K1); truncation before scrubbing (K2) |
| Key sent to the wrong provider | Prefix detection | Medium: an explicit provider choice overrides a clear prefix (K3) |

## Findings status

Two attack rounds on 25 November 2026 and the 24 November review produced 33 distinct findings for today's code. One is fixed (the repair-feedback wording now matches the prototype). Thirty-two are still open; eleven of those were refuted by skeptic agents or never verified, and the code is unchanged. The open items that matter most are V1 (non-finite probabilities pass the verifier), K1 (key headers follow redirects), V7 (a page reload can mix events from two questions) and U1 (leaving a playing circuit panics the WebAssembly module). The three high-severity items from 24 November (navigation lock-down, result remount, WebView2 data folder) remain fixed.

# Testing and acceptance

## Test inventory

| Area | Tests | What they show |
|---|---|---|
| Verifier and Python parity (qcore) | 5 | Ground truth, mismatch wording and parameter defaults equal the prototype; traces are normalised |
| General circuit simulator (qcore) | 5 | Angles, Bell state, phase identities, controls, SWAP, Toffoli, teleportation |
| Atlas content gate (qcore) | 2 | All pages pass; a wrong claim and a dead link are caught |
| Key handling and routing (desktop) | 9 | Keys never leak into messages; prefix routing; settings hold no key |
| Provider wire formats (desktop) | 11 | Request shapes; refusals and empty answers become errors; model lists parse |
| Prompt handling (desktop) | 4 | Prompts parse; single-pass filling; Python-identical JSON |
| Live network (desktop, ignored) | 2 | One real router call each; run on demand |

Total: 36 passing and 2 ignored (`cargo test -p qcore -p desktop`, 25 November 2026). The `ui` crate has no automated tests.

## Acceptance criteria for a release

1. All unit tests pass, including the Atlas content gate.
2. `.\build.ps1` produces one executable that starts on a clean Windows 11 PC.
3. A verifiable question runs end to end with at least one provider and shows a verdict.
4. No open finding of high severity remains.

# Risks, open issues and pending decisions

| Item | Type | Detail |
|---|---|---|
| Verifier accepts non-finite values (V1–V3) | Open issue | Add finiteness and range checks and a total check for all algorithms |
| Redirect and scrubbing order (K1, K2) | Open issue | Disable cross-host redirects; scrub before truncating |
| Atlas coverage | Open issue | 3 of 40 planned topics written |
| No UI tests | Risk | UI behaviour is checked only manually and by headless runs |
| Native shell | Decision pending | Tauri 2 (same WebView2, tao and wry) recommended; owner undecided; Electron rejected |
| Companion website | Planned | Later, not now |
| Provider behaviour | Risk | Only Gemini has been run end to end; other providers do not receive temperature or JSON mode |

# Traceability matrix

| Requirement | Code | Test or check |
|---|---|---|
| FR-WIN-1 | `desktop/src/main.rs` | Inspection |
| FR-ATL-4, FR-ATL-5 | `qcore/src/topic.rs` | `atlas_content_passes_the_gate`, `gate_catches_a_wrong_claim_and_a_dead_link` |
| FR-LAB-1 | `qcore/src/lib.rs`, `ui/src/lab.rs` | `traces_are_normalised_and_consistent` |
| FR-VER-1, FR-VER-6 | `qcore/src/lib.rs` | `verifier_matches_python_prototype` |
| FR-VER-2, FR-VER-3 | `desktop/src/pipeline.rs` | Inspection |
| FR-TUT-2 | `desktop/src/prompts.rs` | `embedded_prompts_parse` |
| FR-KEY-1 | `desktop/src/llm/mod.rs` | `every_detected_provider_exists_and_ids_are_unique` |
| FR-KEY-2 | `desktop/src/main.rs` | `save_key_provider_resolution` |
| FR-KEY-3 | `desktop/src/config.rs` | `stored_settings_hold_no_key` |
| FR-KEY-4 | `desktop/src/llm/mod.rs` | `prefixes_route_only_when_unambiguous` |
| FR-ERR-3 | `desktop/src/llm/*.rs`, `gemini.rs` | Refusal and empty-answer tests |
| NFR-SEC-2 | `desktop/src/llm/mod.rs` | `keys_never_leak_into_messages` |
