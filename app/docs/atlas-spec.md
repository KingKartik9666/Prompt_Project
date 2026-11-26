# Atlas content spec

One JSON file per topic: `crates/ui/content/topics/<id>.json`, where the file name equals the topic `id`.
Sections are in `crates/ui/content/sections.json`.

The gate is `cargo test -p qcore`, which runs `topic::tests::atlas_content_passes_the_gate`. It checks that:
- every file parses;
- every circuit runs;
- every stated outcome matches the simulator (tolerance 5e-4);
- every link points at an existing topic.

A page with a failing check does not ship.

## Audience and voice

- **Readers:** B.Tech students and self-learners. The Atlas should be where a quantum-computing beginner goes and keeps going.
- **Language:** plain English. Define every term the first time it appears and use the same term after that.
- **Sentences:** short, one idea each, active voice.
- **Avoid:** hype, "simply", "obviously", and filler openings like "In this section".
- **Maths:** Unicode only (|0⟩, ⟨ψ|, √, π, ⊗, ψ, α, β). No TeX.
- **Length:** about 900–1600 words per page, from 4–8 text blocks plus the interactive parts.

## Accuracy rules (non-negotiable)

- Never invent facts, numbers, names, dates, quotations or citations.
- Only state an outcome as a circuit `expect` if the simulator can check it. All other numbers must be standard textbook facts you are certain of.
- **Sources** must exist, and you must be sure they exist:
  - Prefer chapter-level references to standard texts, e.g. "M. A. Nielsen and I. L. Chuang, Quantum Computation and Quantum Information (10th anniversary ed., Cambridge University Press, 2010), ch. 4".
  - Cite original papers only when you are certain of authors, title, journal, volume, page and year.
  - Do not give section or page numbers unless you checked them.
  - Fewer, certain sources beat many uncertain ones.
- Hardware and "state of the art" claims date quickly. Describe principles, not current records. If a date or number is needed, give the year it refers to.
- When a circuit uses deferred measurement (controlled gates standing in for measure-then-correct), say so in the caption.

## Conventions the simulator uses

- **Bit order:** qubit 0 is the leftmost (most significant) bit of a basis label. "10" means q0 = 1, q1 = 0.
- **Start state:** circuits start in |0…0⟩, unless `init` gives a bit string such as "01".
- **Gates** (case-insensitive): `I X Y Z H S SDG T TDG RX RY RZ P SWAP`.
  - Controlled forms put controls in `c` and targets in `t`: `{"g":"CX","c":[0],"t":[1]}`.
  - Other controlled gates: `CZ`, `CY`, `CP` (needs `theta`), `CH`, `CCX` (two controls), `CSWAP`.
  - Any gate also takes explicit controls: `{"g":"X","c":[0,1],"t":[2]}`.
- **Several targets:** a single-qubit gate with several targets applies to each one in the same step: `{"g":"H","t":[0,1,2]}`.
- **Angles:** radians, or text like `"pi/4"`, `"-3pi/4"`, `"2*pi/3"`. Definitions: RX(θ) = e^(−iθX/2), RY(θ) = e^(−iθY/2), RZ(θ) = e^(−iθZ/2), P(θ) = diag(1, e^(iθ)), S = diag(1, i), T = diag(1, e^(iπ/4)).
- **Size:** at most 8 qubits per circuit. Prefer 1–4 qubits so that the circle-notation state view stays readable.
- **Step notes:** every op may carry a `note`, shown while the reader steps through. Write one for each meaningful step.

## Schema

```json
{
  "id": "kebab-case, equals file name",
  "section": "foundations | many-qubits | gates | protocols | algorithms | machines",
  "title": "Title Case, short",
  "kicker": "One-line hook, ≤ 90 characters, a full sentence.",
  "summary": "2–3 plain sentences: what it is and why it matters.",
  "level": "beginner | intermediate | advanced",
  "prereqs": ["ids from the master list"],
  "next": ["ids"],
  "related": ["ids"],
  "keywords": ["search words"],
  "blocks": [
    {"type": "text", "heading": "optional", "body": "paragraphs separated by \n\n; *emphasis*"},
    {"type": "math", "expr": "|ψ⟩ = α|0⟩ + β|1⟩", "caption": "optional"},
    {"type": "circuit", "title": "…", "caption": "…",
     "circuit": {"qubits": 2, "init": "00", "ops": [{"g": "H", "t": [0], "note": "…"}]},
     "expect": {"qubits": [1], "probs": {"0": 0.5, "1": 0.5}}},
    {"type": "misconception", "myth": "…", "truth": "…"},
    {"type": "practice", "question": "…", "hint": "optional", "answer": "…"},
    {"type": "note", "title": "…", "body": "…"}
  ],
  "sources": [{"title": "…", "url": "optional, only if it was opened and checked"}]
}
```

`expect.probs` lists every outcome with non-zero probability, and the values sum to 1. Omit `expect.qubits` to mean all qubits in order.

**Required mix per page:**
- at least 3 text blocks;
- at least 1 misconception;
- at least 1 practice question with a worked answer;
- at least 1 circuit wherever the topic has a natural small circuit. Pure-concept pages (hardware, complexity, history) may have none, but should then carry a note block.

## Master topic list

Links (`prereqs`, `next`, `related`) may use only these ids.

| id | section | level | title | prereqs |
|---|---|---|---|---|
| bits-and-qubits | foundations | beginner | Bits and qubits | – |
| superposition | foundations | beginner | Superposition | bits-and-qubits |
| dirac-notation | foundations | beginner | Dirac notation | superposition |
| measurement | foundations | beginner | Measurement and the Born rule | superposition |
| phase | foundations | beginner | Phase and interference | superposition |
| bloch-sphere | foundations | intermediate | The Bloch sphere | phase |
| measurement-bases | foundations | intermediate | Measuring in other bases | measurement, phase |
| multi-qubit-states | many-qubits | beginner | Many qubits and tensor products | bits-and-qubits, dirac-notation |
| entanglement | many-qubits | beginner | Entanglement | multi-qubit-states, superposition |
| bell-states | many-qubits | intermediate | The four Bell states | entanglement |
| ghz-states | many-qubits | intermediate | GHZ states | bell-states |
| no-cloning | many-qubits | intermediate | The no-cloning theorem | superposition, multi-qubit-states |
| bell-inequalities | many-qubits | advanced | Bell's theorem and the CHSH game | entanglement, measurement-bases |
| reading-circuits | gates | beginner | Reading a circuit diagram | bits-and-qubits |
| pauli-gates | gates | beginner | Pauli gates X, Y and Z | superposition |
| hadamard | gates | beginner | The Hadamard gate | pauli-gates |
| phase-gates | gates | intermediate | Phase gates S and T | phase |
| rotation-gates | gates | intermediate | Rotation gates | bloch-sphere |
| cnot | gates | beginner | CNOT and controlled gates | multi-qubit-states |
| swap-and-toffoli | gates | intermediate | SWAP, Toffoli and Fredkin | cnot |
| phase-kickback | gates | intermediate | Phase kickback | cnot, phase |
| universal-gates | gates | advanced | Universal gate sets | rotation-gates, cnot |
| teleportation | protocols | intermediate | Quantum teleportation | entanglement, superposition |
| superdense-coding | protocols | intermediate | Superdense coding | bell-states |
| bb84 | protocols | intermediate | BB84 key distribution | measurement-bases, no-cloning |
| entanglement-swapping | protocols | advanced | Entanglement swapping | teleportation |
| deutsch-jozsa | algorithms | intermediate | Deutsch–Jozsa | phase-kickback, hadamard |
| bernstein-vazirani | algorithms | intermediate | Bernstein–Vazirani | deutsch-jozsa |
| simon | algorithms | advanced | Simon's algorithm | bernstein-vazirani |
| grover | algorithms | intermediate | Grover search | phase, hadamard |
| amplitude-amplification | algorithms | advanced | Amplitude amplification | grover |
| qft | algorithms | advanced | The quantum Fourier transform | phase-gates, swap-and-toffoli |
| phase-estimation | algorithms | advanced | Quantum phase estimation | qft, phase-kickback |
| shor | algorithms | advanced | Shor's algorithm | phase-estimation |
| variational-algorithms | algorithms | advanced | Variational algorithms (VQE and QAOA) | rotation-gates |
| decoherence | machines | intermediate | Noise and decoherence | measurement |
| error-correction | machines | intermediate | Quantum error correction | decoherence, cnot |
| surface-code | machines | advanced | Surface codes and logical qubits | error-correction |
| hardware | machines | beginner | How quantum computers are built | decoherence |
| complexity | machines | advanced | What quantum computers can solve (BQP) | grover, shor |
