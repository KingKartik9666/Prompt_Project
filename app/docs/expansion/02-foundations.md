# Expansion area: foundations, maths tools and quantum information theory

Research subagent, 2026-11-25.

## How the numbers were checked

- Every probability here was **simulated**, not worked out by hand. The simulator was a JS port of qcore's gates and its q0-first bit order (`scratchpad/sim.js`).
- Even so, every page still has to pass `cargo test -p qcore` before it ships.
- A source counts as "verified" only if it was opened with WebFetch.

Exact values used below:
- (2+√2)/4 = 0.8536 and (2−√2)/4 = 0.1464
- (2+√3)/4 = 0.9330 and (2−√3)/4 = 0.0670

Notation:
- "over [q0]" means `expect.qubits: [0]`.
- CRY means RY with a control.

## Ranked candidates

### 1. `reduced-states`: Reduced states and density matrices
- **Placement:** many-qubits, intermediate.
- **Prereqs:** entanglement, bell-states, measurement-bases.
- **Tag:** fits-now as an illustration; the full page needs a density-matrix view.
- **Why it matters:** one qubit of an entangled pair has no pure state of its own. The partial trace gives its density matrix, and that is the key to telling a mixture from a superposition.
- **Circuits (2 qubits):**
  - Bell pair (H q0; CX q0→q1), over [q0]: 0: 0.5, 1: 0.5.
  - The same circuit plus H on q0, over [q0]: still 0: 0.5, 1: 0.5 (I/2 is unchanged by any unitary).
  - Contrast on 1 qubit: H; H gives 0: 1, because |+⟩ interferes.
- **Sources (verified):**
  - IBM Quantum Learning, Multiple systems and reduced states: https://quantum.cloud.ibm.com/learning/courses/general-formulation-of-quantum-information/density-matrices/multiple-systems
  - Preskill Ph219 notes, ch. 2: https://www.preskill.caltech.edu/ph219/

### 2. `mach-zehnder-interferometer`: Mach–Zehnder interferometer
- **Placement:** foundations, beginner.
- **Prereqs:** superposition, hadamard, phase-gates.
- **Tag:** fits-now, 1 qubit.
- **Why it matters:** this is the standard picture of interference, and it sets up the bomb tester, the eraser and phase estimation.
- **Circuit:** H; P(φ); H gives P(0) = cos²(φ/2).
  - φ = 0: 0: 1
  - φ = π/3: 0: 0.75, 1: 0.25
  - φ = π: 1: 1
- **Sources:** unverified; this is standard optics.

### 3. `bomb-tester`: The Elitzur–Vaidman bomb tester
- **Placement:** foundations, intermediate.
- **Prereqs:** mach-zehnder-interferometer (NEW), deferred-measurement (NEW), cnot.
- **Tag:** fits-now, 2 qubits. The bomb is a CNOT into a "detector" qubit.
- **Why it matters:** interference reveals the bomb is there even though the photon never touched it.
- **Circuits** (q0 = path, q1 = bomb):
  - With a bomb, H q0; CX q0→q1; H q0: 00, 01, 10, 11 at 0.25 each.
    - Outcome 10 is the interaction-free detection: the dark port fires and the bomb stays intact.
    - Outcomes with q1 = 1 (50% in total) mean the bomb went off.
  - With no bomb, H; H: 00: 1.
- **Sources (verified):**
  - Elitzur & Vaidman 1993: https://arxiv.org/abs/hep-th/9305002
  - Kwiat et al., PRL 83, 4725 (1999): https://arxiv.org/abs/quant-ph/9909083

### 4. `expectation-values`: Observables and expectation values
- **Placement:** foundations, beginner.
- **Prereqs:** measurement, measurement-bases, pauli-gates.
- **Tag:** fits-now, 1 qubit, using ⟨Z⟩ = P0 − P1. A built-in ⟨P⟩ readout would be better.
- **Why it matters:** VQE and real hardware (the Estimator) report results this way.
- **Circuits:**
  - RY(π/3) gives 0: 0.75, 1: 0.25, so ⟨Z⟩ = 0.5.
  - Adding H gives 0: (2+√3)/4, so ⟨X⟩ = √3/2.
- **Sources:** Preskill Ph219, ch. 2–3; IBM tomography lesson (see #5).

### 5. `state-tomography`: Quantum state tomography
- **Placement:** foundations, intermediate.
- **Prereqs:** bloch-sphere, measurement-bases, expectation-values (NEW).
- **Tag:** fits-now as 3 circuits on 1 qubit. A Bloch-vector readout would complete the page.
- **Circuits** (state prepared with H; T):
  - Z basis: 0.5 / 0.5
  - X basis (add H): 0: (2+√2)/4
  - Y basis (add SDG; H): 0: (2+√2)/4
  - Result: Bloch vector (√2/2, √2/2, 0).
- **Source (verified):** https://quantum.cloud.ibm.com/learning/courses/general-formulation-of-quantum-information/general-measurements/discrimination-and-tomography

### 6. `schmidt-decomposition`: Schmidt decomposition and entanglement entropy
- **Placement:** many-qubits, advanced.
- **Prereqs:** entanglement, reduced-states (NEW).
- **Tag:** fits-now for the coefficients. Entropy needs a readout, or can be stated in the text.
- **Circuits:**
  - RY(π/3) q0; CX q0→q1 gives 00: 0.75, 11: 0.25. The entropy S is 0.811 bits.
  - The product state RY(π/3) on both qubits gives 00: 0.5625, 01: 0.1875, 10: 0.1875, 11: 0.0625. The q1 marginal is the same as above, so you need the joint table to tell the two apart.
- **Source (verified):** https://quantum.cloud.ibm.com/learning/courses/general-formulation-of-quantum-information/purifications-and-fidelity/purifications
- **Note:** the "purification" topic can merge in here. Example: RY(π/3) q0; CX; H q1 gives 00: 0.375, 01: 0.375, 10: 0.125, 11: 0.125, with the same q0 marginal.

### 7. `swap-test`: The swap test and state overlap
- **Placement:** protocols, intermediate.
- **Prereqs:** swap-and-toffoli, hadamard.
- **Tag:** fits-now, 3 qubits.
- **Why it matters:** P(0) = (1 + |⟨ψ|φ⟩|²)/2. Quantum fingerprinting is built on it. The Hong–Ou–Mandel effect is the optical equivalent, so give it a Note block here.
- **Circuit** (ancilla q0, ψ on q1, φ on q2): H q0; CSWAP(q0; q1, q2); H q0. Results over [q0]:
  - φ = ψ = |0⟩: 0: 1
  - φ = |1⟩: 0.5 / 0.5
  - φ = |+⟩: 0: 0.75, 1: 0.25
  - φ = RY(π/3)|0⟩: 0: 0.875, 1: 0.125
- **Sources (verified):**
  - Buhrman, Cleve, Watrous, de Wolf 2001: https://arxiv.org/abs/quant-ph/0102001
  - Garcia-Escartin & Chamorro-Posada, PRA 87, 052330: https://arxiv.org/abs/1303.6814

### 8. `state-discrimination`: Telling two states apart (Helstrom bound)
- **Placement:** foundations, intermediate.
- **Prereqs:** measurement-bases, no-cloning.
- **Tag:** fits-now, 1 qubit.
- **Circuit:** apply RY(π/4), then measure.
  - |0⟩ in: 0: (2+√2)/4
  - |+⟩ in (H first): 1: (2+√2)/4
  - Success rate 0.8536 = ½ + ½·(1/√2), which is the optimum.
- **Source (verified):** same IBM page as #5.

### 9. `no-signalling`: Why entanglement cannot send messages
- **Placement:** many-qubits, beginner.
- **Prereqs:** entanglement, bell-states.
- **Tag:** fits-now, 2 qubits.
- **Why it matters:** it rebuts the "faster than light" myth. Brang et al. 2024 found this myth is common among learners.
- **Circuit:** Bell pair, then Bob applies nothing, H on q1, or RY(1.1) on q1. In every case q0 gives 0.5 / 0.5.
- **Sources:** the IBM reduced-states lesson. The theorem-level citation is unverified.

### 10. `quantum-eraser`: Which-path information and the quantum eraser
- **Placement:** foundations, intermediate.
- **Prereqs:** mach-zehnder-interferometer (NEW), entanglement.
- **Tag:** fits-now, 2 qubits.
- **Circuits:**
  - Marked path, H q0; CX q0→q1; H q0: q0 gives 0.5 / 0.5.
  - Erased, add H on q1: 00: 0.5, 11: 0.5. The fringe comes back only after sorting by q1.
- **Source (verified):** Kim, Yu, Kulik, Shih, Scully, PRL 84, 1 (2000): https://arxiv.org/abs/quant-ph/9903047

### 11. `uncertainty-and-commutators`: Commutators and uncertainty
- **Placement:** foundations, intermediate.
- **Prereqs:** pauli-gates, measurement-bases, expectation-values (NEW).
- **Tag:** fits-now (complementarity). Variances need ⟨P⟩.
- **Circuits:**
  - |0⟩ measured in Z: 0: 1
  - |0⟩ measured in X: 0.5 / 0.5
  - RY(π/3) state: Z gives 0.75 / 0.25; X gives (2+√3)/4 / (2−√3)/4
- **Source:** Preskill Ph219, ch. 2–3.

### 12. `w-states`: W states versus GHZ states
- **Placement:** many-qubits, intermediate.
- **Prereqs:** ghz-states, swap-and-toffoli, rotation-gates.
- **Tag:** fits-now, 3 qubits.
- **Preparation:**
  1. RY(1.9106) on q0, where 1.9106 = 2·arccos(1/√3)
  2. CRY(π/2) q0→q1
  3. CX q1→q0
  4. X on q0 and q1
  5. CCX(q0, q1 → q2)
  6. X on q0 and q1
  - Result: 001, 010, 100 at 1/3 each.
- **Robustness demo:** append CX q1→q2; H q1.
  - W: 001: 0.6667, 100: 0.1667, 110: 0.1667
  - GHZ: 000, 010, 100, 110 at 0.25 each
- **Source (verified):** Dür, Vidal, Cirac, PRA 62, 062314 (2000): https://arxiv.org/abs/quant-ph/0005115

### 13. `deferred-measurement`: Deferred measurement (enabling topic)
- **Placement:** gates, intermediate.
- **Prereqs:** measurement, cnot, reading-circuits.
- **Tag:** fits-now, 2 qubits.
- **Why it matters:** "measure, then act on the result" is the same as a controlled gate followed by measuring at the end. qcore uses this everywhere, so the page should come early.
- **Circuits:**
  - H q0; CX q0→q1; H q0: q0 gives 0.5 / 0.5.
  - Without the CX: 0: 1.
- **Sources:** Nielsen & Chuang, ch. 4 (chapter level); Preskill Ph219, ch. 5.

### 14. `quantum-zeno`: The quantum Zeno effect
- **Placement:** foundations, intermediate.
- **Prereqs:** deferred-measurement (NEW), rotation-gates.
- **Tag:** fits-now, N + 0 qubits up to 8. Ancillas hold the measurement records.
- **Circuit (N = 3):** RY(π/3) q0; CX q0→q1; RY(π/3) q0; CX q0→q2; RY(π/3) q0.
  - Full result: 000: 0.4219 (27/64), 001: 0.0469, 010: 0.0469, 011: 0.0469, 100: 0.1406, 101: 0.1406, 110: 0.0156, 111: 0.1406.
  - "Survival" means outcome 000.
  - Without the CX gates: 1: 1.
- **Survival probability for other N:** N = 2: 0.25, N = 4: 0.5308, N = 8: 0.7331.
- **Pitfall:** read the joint 000 outcome. The q0 marginal (0.5625) is not the survival probability.
- **Source:** Kwiat 1999 (verified, see #3).

### 15. `hadamard-test`: The Hadamard test
- **Placement:** algorithms, intermediate.
- **Prereqs:** phase-kickback, hadamard.
- **Tag:** fits-now, 2 qubits.
- **Why it matters:** P(0) = (1 + Re⟨ψ|U|ψ⟩)/2. It is the core of phase estimation and of overlap and Krylov methods.
- **Circuit:** X q1; H q0; CP(π/3) q0→q1; H q0.
  - Over [q0]: 0: 0.75, 1: 0.25.
  - Adding SDG on q0 before the last H gives 0: (2+√3)/4, which yields Im.
- **Source (verified):** NVIDIA CUDA-Q docs: https://nvidia.github.io/cuda-quantum/0.9.0/applications/python/hadamard_test.html

### 16. `quantum-channels`: Quantum channels, Kraus operators and Stinespring dilation
- **Placement:** foundations, advanced.
- **Prereqs:** reduced-states (NEW), decoherence.
- **Tag:** fits-now as a dilation on 2 qubits. Kraus matrices need a density-matrix view.
- **Circuits:**
  - Amplitude damping: X q0; CRY(θ) q0→q1; CX q1→q0, where γ = sin²(θ/2).
    - θ = 2π/3: over [q0], 0: 0.75, 1: 0.25.
  - Dephasing: H q0; RY(θ) q1; CZ(q0, q1); H q0.
    - θ = π/3: 00: 0.75, 11: 0.25.
- **Source (verified):** https://quantum.cloud.ibm.com/learning/courses/general-formulation-of-quantum-information/quantum-channels/representations-of-channels

### 17. `general-measurements`: POVMs and Naimark's theorem
- **Placement:** foundations, advanced.
- **Prereqs:** deferred-measurement (NEW), reduced-states (NEW).
- **Tag:** fits-now, 2 qubits (an unsharp measurement).
- **Circuit:** H q0; CRY(θ) q0→q1.
  - θ = π/2: 00: 0.5, 10: 0.25, 11: 0.25.
- **Disturbance:** add H q0 and read over [q0]. Weaker coupling keeps more coherence.
  - θ = π/3: 0: (2+√3)/4
  - θ = π/2: 0: (2+√2)/4
  - θ = π: 0: 0.5
- **Sources (verified):**
  - https://quantum.cloud.ibm.com/learning/courses/general-formulation-of-quantum-information/general-measurements/formulations-of-measurements
  - https://quantum.cloud.ibm.com/learning/courses/general-formulation-of-quantum-information/general-measurements/naimark-theorem

### 18. `measurement-based-qc`: Measurement-based QC and cluster states
- **Placement:** machines, advanced.
- **Prereqs:** deferred-measurement (NEW), teleportation, phase-gates.
- **Tag:** fits-now.
- **Circuits:**
  - One-bit step on 2 qubits: H q0; H q1; CZ(q0, q1); RZ(−π/3) q0; H q0; CX q0→q1.
    - Over [q1]: 0: 0.75, 1: 0.25.
  - Cluster state on 3 qubits: H on all; CZ(q0, q1); CZ(q1, q2); H on all.
    - 000, 010, 101, 111 at 0.25 each.
- **Source (verified):** Raussendorf & Briegel 2000: https://arxiv.org/abs/quant-ph/0010033

### 19. `stabilizer-formalism`: Stabilizers, the Clifford group and Gottesman–Knill
- **Placement:** gates, advanced.
- **Prereqs:** universal-gates, pauli-gates, phase-gates.
- **Tag:** fits-now, 1 qubit.
- **Circuits:**
  - HSH gives 0.5 / 0.5. Clifford circuits only ever give probabilities of 0 or 2⁻ᵏ.
  - HTH gives 0: (2+√2)/4, which is impossible for a Clifford circuit.
- **Sources (verified):**
  - Gottesman 1998: https://arxiv.org/abs/quant-ph/9807006
  - Aaronson & Gottesman, PRA 70, 052328 (2004): https://arxiv.org/abs/quant-ph/0406196

### 20. `hamiltonians-and-time-evolution`: Hamiltonians, time evolution, adiabatic theorem
- **Placement:** foundations, intermediate.
- **Prereqs:** rotation-gates, expectation-values (NEW).
- **Tag:** fits-now.
- **Circuits:**
  - Rabi: RX(π/3) gives 0.75 / 0.25.
  - ZZ coupling: H on both; CX; RZ(π/2) q1; CX; H on both gives 00: 0.5, 11: 0.5.
  - Digitized adiabatic run, from H = −X to H = −Z, 6 steps:
    - H, then RX(−1.6667) RZ(−0.3333) RX(−1.3333) RZ(−0.6667) RX(−1.0) RZ(−1.0) RX(−0.6667) RZ(−1.3333) RX(−0.3333)
    - Result: 0: 0.9998.
    - The sudden version (H alone) gives 0.5 / 0.5.
- **Sources (verified):**
  - Farhi et al. 2000: https://arxiv.org/abs/quant-ph/0001106
  - Albash & Lidar, RMP 90, 015002 (2018): https://arxiv.org/abs/1611.04471

### Shorter candidates

- **`random-access-codes`:** the Holevo bound via a 2→1 random access code. protocols, advanced; fits-now on 1 qubit.
  - Encode with RY(±π/4) or RY(±3π/4).
  - Decode with Z, or with H then Z.
  - Every case succeeds with probability (2+√2)/4.
- **`contextuality`:** the Mermin–Peres square. foundations, advanced; needs Pauli-product expectation values.
  - Partial demo on a Bell pair: XX parity +1, YY parity −1, ZZ parity +1.
  - Source (verified): Budroni et al., RMP 94, 045007 (2022): https://arxiv.org/abs/2102.13036
- **`linear-algebra-for-qubits`:** concept-only refresher. foundations, beginner.
- **`zx-calculus` and `tensor-networks`:** concept-only.
  - Source (verified): van de Wetering: https://arxiv.org/abs/2012.13966
  - Source (verified): Coecke & Duncan: https://arxiv.org/abs/0906.4725
  - Source (verified): Orús: https://arxiv.org/abs/1306.2164

## Simulator features that unlock the most (this area)

1. A reduced-state view for a chosen qubit: its 2×2 ρ, Bloch vector, purity and entropy.
2. Pauli-string expectation values.
3. Mid-circuit measurement with classically controlled gates.
4. A Bloch-vector readout.
5. A density-matrix backend with noise channels.
6. A custom unitary gate, or e^(−iHt) built from Pauli coefficients.

## Misconception blocks

- **"A mixed state is just a superposition we don't know."** |+⟩ interferes and I/2 does not.
- **"Same marginals means the same state."** See #6 and #12.
- **"Bob's operation can signal to Alice."** It cannot (#9).
- **"The eraser changes the past."** Fringes appear only after sorting by outcome (#10).
- **"Measurement is passive."** The bomb tester (#3) and the Zeno effect (#14) show it changes the outcome.
- **"Entanglement implies a speed-up."** Gottesman–Knill says otherwise (#19).
- **"A qubit stores 2 bits."** The Holevo bound says one.
- **"⟨Z⟩ is the most likely outcome."** It is an average.
- **"Uncertainty is just clumsy instruments."** It comes from non-commuting observables.
- **"The adiabatic theorem means 'slow enough' is easy."** The allowed speed depends on the energy gap, and Trotter steps add error.
