# Quantum Explainer (native app)

A Windows desktop app that explains the Bell state, Deutsch–Jozsa and 2-qubit Grover search step by step,
and checks every probability with a simulator before you read it. It is a Rust rebuild of the S18 Python
prototype in `../code/`. It uses the same four evaluated prompts (`../prompts/final/`, embedded at build time)
and gives the same simulator results (unit tests compare them).

## Stack

| Layer | Technology |
|---|---|
| Window | `tao` 0.37: frameless window with the Windows 11 Mica backdrop (`window-vibrancy`) |
| Renderer | `wry` 0.57 on WebView2 (the Chromium engine built into Windows). The UI is served from the exe through a private `app://` protocol, with no browser, local server or open port |
| Interface | Leptos 0.8 (Rust) compiled to WebAssembly, using current Chromium CSS: scroll-driven animations, View Transitions, `@starting-style`, anchor-positioned popover, `linear()` springs, `interpolate-size`, `oklch` |
| Simulator | `crates/qcore`: statevector simulator, step traces and claim verifier. The same code runs in the UI (Lab) and in the host (verification) |
| Gemini | `ureq` 3 through the Windows TLS stack. The key stays in the native process, read from an environment variable, Windows Credential Manager (set in the app) or a `.env` file |
| Toolchain | Rust 1.100, edition 2024, `x86_64-pc-windows-gnullvm` with LLVM-MinGW. The MSVC Build Tools are not needed |

## Screens

- **Home**: a scroll story. The headline holds a live amplitude reel, a pinned circuit zooms in, and claims sit on colour fields. Every effect is CSS scroll-driven, so scrolling stays native.
- **Tutor**: ask a question and watch the four stages (read → explain → check → practise). You get the verdict (claimed against simulated probabilities), a step deck (← / →), the simulated final state and a practice card.
- **Lab**: works offline, with no key. Pick Bell, Deutsch–Jozsa (1–3 qubits, any oracle) or Grover (any marked item), then step gate by gate. Bars swing below the axis and turn coral when a sign flips.

## Build

Needs Rust (rustup) with the `stable-x86_64-pc-windows-gnullvm` toolchain and the `wasm32-unknown-unknown`
target, plus LLVM-MinGW (`winget install MartinStorsjo.LLVM-MinGW.MSVCRT`). `wasm-bindgen` is built once into
`.tools/` (LLVM-MinGW on PATH): `cargo +stable-x86_64-pc-windows-gnullvm install wasm-bindgen-cli --version 0.2.130 --locked --root .tools`.
Its version must equal the `wasm-bindgen` crate version pinned in `crates/ui/Cargo.toml`.

```powershell
.\build.ps1          # release: target\release\QuantumExplainer.exe
.\build.ps1 -Dev     # debug build (WebView devtools enabled)
.\build.ps1 -Run     # build and start
cargo +stable-x86_64-pc-windows-gnullvm test -p qcore -p desktop   # unit tests
```

## Key handling

The Gemini key never reaches the web view. The interface only learns whether a key exists. A key entered in
Settings is stored in Windows Credential Manager under "QuantumExplainer".
