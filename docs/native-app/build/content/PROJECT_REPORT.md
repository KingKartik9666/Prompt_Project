# Abstract

Large language models explain quantum algorithms fluently, but they can state wrong probabilities with full confidence, and a beginner cannot tell. Our Milestone 2 prototype, a Python workflow, answered this by checking each explanation's numbers with a simulator. This report describes the next step: Quantum Explainer, a from-scratch native Windows application written in Rust. It keeps the four evaluated prompts and the verifier logic, and adds a cinematic interface, a growing topic map (the Atlas), an offline Lab, and support for eleven LLM providers with each learner's own key. The interface is Leptos compiled to WebAssembly and drawn by WebView2 inside a frameless native window. The simulator crate, qcore, runs both in the host, where it verifies explanations, and in the interface, where it animates circuits. On 25 November 2026 the release build was a single 6.8 MB executable, and 36 unit tests passed, including tests that prove parity with the Python prototype and a content gate that checks every outcome stated in the Atlas against the simulator. A live run on 24 November verified a Bell-state explanation end to end in 16.6 seconds. Two security attack rounds found 33 issues that still need work, the most important being that the verifier accepts non-finite probabilities. We conclude that the "checked, not trusted" approach carries over to a native application, and that the remaining work is hardening the verifier, finishing the Atlas, and adding interface tests.

# Introduction

## Problem

Students who know linear algebra but are new to quantum computing increasingly learn from AI chat assistants. These assistants write clear step-by-step explanations, yet the numbers in them, such as the probability of measuring |00⟩ after a Bell circuit, are not checked. A wrong probability presented confidently is worse than no answer, because the learner builds on it.

## Objectives

1. Rebuild the Milestone 2 tutor as a native Windows application with no browser tab and no console window.
2. Keep the evaluated behaviour: the same four prompts and the same verification rules as the prototype.
3. Check every final probability that the Tutor can check before the learner reads it, and say clearly when a number cannot be checked.
4. Let any learner use the provider and key of their choice, with the key kept out of the interface.
5. Add an offline Lab and a topic map (the Atlas) whose interactive circuits are checked by the same simulator.
6. Use the newest stable technology in each category, and a fluid, Apple-like presentation without scroll-hijacking.

## Scope

The application targets Windows 11. The Tutor verifies three algorithms: the Bell state, Deutsch–Jozsa with one to three input qubits, and Grover search on two qubits. Other questions are answered and labelled unverified. A companion website is planned for later and is not part of this report.

# Background

## Quantum concepts used in this report

A qubit's state is a combination α|0⟩ + β|1⟩ with |α|² + |β|² = 1; measuring it gives 0 with probability |α|² and 1 with probability |β|². Several qubits are described by one vector of amplitudes over all basis states, for example four amplitudes for two qubits.

- **Bell state.** A Hadamard gate on the first qubit followed by a CNOT produces (|00⟩ + |11⟩)/√2. Measuring gives 00 or 11, each with probability ½, and never 01 or 10.
- **Deutsch–Jozsa.** One query to an oracle decides whether a function is constant or balanced. The probability of measuring all zeros on the input register is 1 for a constant function and 0 for a balanced one.
- **Grover search on two qubits.** For four items, one oracle call and one diffusion step move all probability onto the marked item.

These are standard textbook results [1].

## The Milestone 2 prototype

The prototype runs four prompts in order. P1 routes the question, P2 explains it, P3 extracts the claimed final probabilities, and a NumPy simulator checks them, with one repair retry when they are wrong. P4 then writes a practice question. On 22 November 2026, with Gemini 3.5 Flash-Lite, it passed 12 of 13 automatic test checks; an LLM-assisted rubric gave 4 Pass, 5 Partial and 4 Fail. Across explainer versions, verified answers rose from 5/12 (v1) to 12/12 (v2 and v3), and clean JSON output from 0/12 to 12/12 (v3).

## Design research

Before building the interface we studied three references the project owner named. The racing driver site landonorris.com is built on Webflow with GSAP and ScrollTrigger, Lenis smooth scrolling, Three.js and Rive [2]. Apple's product pages use an in-house scroll-keyframe engine with sticky sections. Gaming-laptop pages such as ASUS ROG pin pre-rendered video with GSAP. Two platform facts shaped our choices: CSS scroll-driven animations and same-document View Transitions are now supported across major browser engines [3][4]. Usability research from Nielsen Norman Group found that scroll-jacking hurts task-focused users [5]. We therefore drive motion from the scroll position but never take control of scrolling.

# System design

## Architecture

The application is one Windows process. A frameless window from the tao library hosts a WebView2 view through the wry library. The interface is embedded in the executable and served over a private `app://` protocol, so there is no local web server and no open port. All network calls to LLM providers are made by the native host; the page's security policy blocks it from contacting anything else.

```text
+-- QuantumExplainer.exe ----------------------------------------------+
|  tao window (frameless, Mica)                                         |
|    wry + WebView2  ->  app://localhost/  (embedded interface)         |
|      Leptos UI in WebAssembly: Home, Atlas, Tutor, Lab                |
|      qcore in WebAssembly: Lab traces, Atlas circuits                 |
|        | ipc.postMessage                    ^ window.__qe(event)     |
|        v                                    |                         |
|  native host: IPC handler -> worker threads                           |
|    pipeline P1 -> P2 -> P3 + qcore verify -> P4                       |
|    11 provider presets over 3 wire formats, HTTPS via OS TLS          |
|    keys in Windows Credential Manager                                 |
+-----------------------------------------------------------------------+
```

The workspace has three crates. `qcore` holds the simulators, the verifier and the Atlas content model. `desktop` is the native host. `ui` is the interface, compiled to WebAssembly.

## Design decisions

| Decision | Alternatives considered | Why chosen |
|---|---|---|
| Native window with WebView2 | Browser web app; Electron | The owner asked for a native app with no browser; Electron was rejected by the owner |
| tao + wry directly | Tauri 2 (same engine), native Rust GUIs (egui, Slint, Iced) | Lowest overhead with the same engine; native GUIs cannot easily match web-grade motion. Tauri 2 remains under consideration |
| Leptos compiled to WebAssembly | TypeScript front end | Keeps the whole application in Rust and lets the simulator run in the page |
| LLVM-MinGW with the gnullvm toolchain | MSVC Build Tools | The owner chose the smaller LLVM-MinGW install; the plain GNU toolchain failed to link |
| Local wasm-bindgen build | Trunk | Downloading the Trunk binary was blocked; building wasm-bindgen from source worked |
| ureq with the OS TLS stack | rustls with a bundled C library | No C compiler was available at first; the OS certificate store is used |
| Keys in Credential Manager | Settings file | A key is never written to a file and never reaches the page |

# Implementation

## Native host

The host creates a 1360 × 860 window, clamped to 90% of the monitor, with its own title bar drawn by the page. It serves embedded files with a strict content-security policy. It allows navigation only to its own pages, denies new windows and blocks file drops. Messages from the page are accepted only from the app's own origin. Failures show a native message box, because the app has no console. WebView2 keeps its profile in `%LOCALAPPDATA%\QuantumExplainer`, so the app can run from a read-only folder.

## Interface and motion

The interface has four screens: Home, Atlas, Tutor and Lab. Screens change with a View Transition. The Home screen tells the product story as the learner scrolls. Its headline "Every qubit is proven" holds a live Grover state.

![The Home screen: the headline holds a live two-qubit state in circle notation.](figures/01-home-hero.png)

Qubit states are drawn in circle notation. The radius of each filled disc is the amplitude's magnitude, so its area is the probability. A line shows the sign or phase, and a negative amplitude turns coral. As the learner scrolls, Grover's oracle flips the marked item's sign, and diffusion then concentrates all probability on it.

![After diffusion, all probability sits on the marked item |01⟩.](figures/02-home-scroll-3.png)

Motion is tied to the scroll position and smoothed by a critically damped spring, so it is continuous rather than stepped. At the owner's request it is slow and has no motion blur. A soft background light follows the pointer, changing only a transform so the browser does not repaint the page. Headlines use the condensed Oswald typeface, with Libre Franklin and Source Serif 4 for text; all fonts are embedded.

![The "Checked, not trusted" field on the Home screen.](figures/02-home-scroll-6.png)

## Simulator and verifier

`qcore` contains two simulators. The first uses real amplitudes for the three verifiable circuits and records the state after every gate; it drives the Lab and the Tutor's verifier. Its ground truth and its mismatch messages are tested against the Python prototype. The second is a general simulator with complex amplitudes for up to eight qubits. It supports the gates I, X, Y, Z, H, S, S†, T, T†, RX, RY, RZ, P and SWAP, with any number of controls, and it powers the Atlas.

The verifier compares each claimed final probability with the simulation within an absolute tolerance of 0.02. For Deutsch–Jozsa it checks only the probability of all zeros, because many balanced functions exist.

## Lab

The Lab runs offline. The learner picks Bell, Deutsch–Jozsa (one to three qubits, any oracle kind) or Grover (any marked item) and steps through the circuit gate by gate, with buttons or the arrow keys.

![The Lab after Grover's oracle: the marked item's amplitude is negative (coral) while every probability is still ¼.](figures/07-lab-oracle.png)

## Tutor pipeline

The Tutor sends the question through the same four prompts as the prototype, embedded byte for byte. P1 routes it. Out-of-scope questions stop with a polite redirect. P2 explains at the chosen level, P3 extracts the claimed probabilities, and qcore checks them using the router's own parameters rather than P3's echo. On a mismatch, P2 is called once more with the correct result. P4 writes a practice question. The interface shows each stage as it runs, and retry notices such as "busy; retrying in 30s" appear in view.

![A real end-to-end run on 24 November 2026 (earlier visual style): the Bell-state explanation was verified by the simulator.](figures/10-tutor-live-run-24nov.png)

## AI providers and keys

The learner can choose any of eleven presets over three wire formats: Google Gemini's own API, Anthropic's Messages API, and the OpenAI-compatible API used by OpenAI, DeepSeek, Moonshot, Qwen, xAI, Mistral, Groq, Together and OpenRouter. A pasted key is checked with the provider before it is saved. Listing models costs nothing; Qwen has no documented model list, so its check costs one request. The key is stored in Windows Credential Manager. Only its last four characters ever reach the page, and error messages are scrubbed of it. The provider is guessed from the key's prefix only when the prefix is unambiguous.

![Settings: choose a provider, paste a key, and get a key from the provider's own page.](figures/09-settings.png)

## The Atlas

The Atlas is a map of quantum computing that grows outward from one dot, with a hub for each of six sections. Each topic has one deep page with prose, interactive circuits, misconceptions, a practice question and sources. Three pages exist today: Superposition, Entanglement and Quantum teleportation. Three of the six sections show "coming soon".

![The Atlas map.](figures/03-atlas-map.png)

Every outcome a page states is checked by the content gate, a unit test that runs each circuit and compares the result with the text within 5 × 10⁻⁴. It also checks that every link points at an existing topic.

![A topic page circuit, labelled as checked by the simulator, with live outcome chips.](figures/05-topic-scroll-2.png)

# Testing and evaluation

## Unit tests

On 25 November 2026, `cargo test -p qcore -p desktop` passed 36 tests and skipped 2 live network tests that are run on demand.

| Area | Tests | What they show |
|---|---|---|
| Verifier and Python parity | 5 | Same ground truth, mismatch text and defaults as the prototype |
| General simulator | 5 | Gates, controls, phases, teleportation |
| Atlas content gate | 2 | All pages pass; wrong claims and dead links are caught |
| Keys and provider routing | 9 | Keys never leak; prefix routing; no key in settings |
| Provider wire formats | 11 | Request shapes; refusals and empty answers become errors |
| Prompt handling | 4 | Prompts parse; Python-identical JSON |
| Live (ignored) | 2 | One real router call each |

## Live and manual evidence

| Date | Evidence |
|---|---|
| 24 Nov 2026 | Bell-state question with gemini-3.5-flash-lite: "Verified by the simulator", "Answered by gemini-3.5-flash-lite (4) in 16.6 s" |
| 24 Nov 2026 | A fake key was rejected: "Google rejected this key: API key not valid" |
| 24 Nov 2026 | HTTP 503 "high demand" from Gemini was retried with a visible notice |
| 25 Nov 2026 | Release build: one 6.8 MB executable in 94 s; interface bundle 3,900 KB of WebAssembly |
| 25 Nov 2026 | Headless run of the interface: hostile search input (scripts, emoji, 10,000 characters) injected no elements |

## Code review and security testing

A four-lens review on 24 November (parity, security, interface, build) found 20 confirmed issues. Its three high-severity issues are fixed and remain fixed: navigation lock-down, a result panel that reset on every event, and the WebView2 data folder. Two attack rounds on 25 November produced 33 distinct findings for today's code. One is fixed. Thirty-two are open, though eleven of them were refuted or never verified.

| Finding | Severity claimed | Status |
|---|---|---|
| V1: non-finite probabilities ("NaN", "inf") pass the verifier | High | Open |
| V2–V3: out-of-range values pass; no total check for Bell and Grover | High / medium | Open |
| K1: key headers can follow a redirect | High (rated low by skeptics) | Open |
| V7: a page reload can mix events from two questions | Medium | Open |
| U1: leaving a playing Atlas circuit panics the WebAssembly module | Medium | Open |
| P1: repair feedback wording differed from the prototype | Low | Fixed |

# Results and discussion

The rebuild meets its main goal. A learner gets a native, single-file application whose explanations of three algorithms are checked by a simulator before display, with the result stated plainly. The prompts and verifier are carried over unchanged and tested for parity, so the prototype's design is preserved. New in the native app: the Lab and Atlas, the multi-provider key flow, and a simulator that also runs in the interface, so the circles a learner watches come from the same code that judges the explanation.

The evaluation is narrower than the prototype's. We have one recorded live end-to-end run, with Gemini, and have not rerun the 13-question Milestone 2 suite on the native app. Providers other than Gemini do not receive the prompts' temperature and JSON mode, so the prototype's results cannot be assumed to hold for them.

# Limitations

- Windows only, and the WebView2 Runtime is required.
- The verifier accepts non-finite and out-of-range probabilities (findings V1–V3). An explanation could therefore be labelled verified when it should not be.
- The Atlas has 3 of 40 planned pages, and its gate compares probabilities only, so a phase error in a circuit is invisible to it.
- The interface has no automated tests.
- The WebAssembly bundle is 3,900 KB with no wasm-opt step.
- There is no way to cancel a running question, and provider free tiers can be slow or busy.
- Startup time and memory use have not been measured.

# Future work

1. Harden the verifier: finiteness and range checks, and a total check for every algorithm.
2. Fix the open security findings, starting with redirects and scrubbing order.
3. Write the remaining Atlas pages from the research notes in `app/docs/expansion`.
4. Add interface tests and rerun the 13-question suite on the native app with several providers.
5. Decide on Tauri 2 as the native shell, and build the companion website later.
6. Reduce the bundle size and add an installer and code signing.

# Conclusion

Quantum Explainer shows that "checked, not trusted" works outside a notebook. A native Rust application can keep an LLM's explanation honest by recomputing its numbers locally. It can present the result with a modern, fluid interface and still keep each learner's key safe. The core is tested and the parity with the evaluated prototype is proven. The next priority is the verifier hardening that the security review identified, because the value of the "Verified" label depends on it.

# References

1. M. A. Nielsen and I. L. Chuang, *Quantum Computation and Quantum Information*, 10th anniversary ed., Cambridge University Press, 2010.
1. Awwwards, "Lando Norris" (Site of the Day, 17 November 2025). https://www.awwwards.com/sites/lando-norris
1. web.dev, "Web Platform October 2026". https://web.dev/blog/web-platform-10-2026
1. web.dev, "Same-document view transitions are now Baseline Newly available". https://web.dev/blog/same-document-view-transitions-are-now-baseline-newly-available
1. Nielsen Norman Group, "Scrolljacking 101". https://www.nngroup.com/articles/scrolljacking-101/
1. E. R. Johnston, N. Harrigan and M. Gimeno-Segovia, *Programming Quantum Computers*, O'Reilly, 2019 (circle notation, as credited in the code).

# Appendix A: Build and run

Requirements: Rust with the `stable-x86_64-pc-windows-gnullvm` toolchain and the `wasm32-unknown-unknown` target, LLVM-MinGW (`winget install MartinStorsjo.LLVM-MinGW.MSVCRT`), and wasm-bindgen-cli 0.2.130 built into `app\.tools`.

```text
cd k:\Prompt\app
.\build.ps1          release build: target\release\QuantumExplainer.exe
.\build.ps1 -Dev     debug build with WebView devtools
.\build.ps1 -Run     build and start
cargo +stable-x86_64-pc-windows-gnullvm test -p qcore -p desktop
```

To use the Tutor, open Settings, choose a provider, paste your own API key and save. The Lab and the Atlas need no key.

# Appendix B: AI assistance disclosure

The native application's code, tests, research and these documents were produced with substantial help from an AI coding assistant, Claude Code, under the team's direction and review. Security testing and fact checking also used AI agents. All results reported here come from real builds, test runs and dated live runs; none were invented.
