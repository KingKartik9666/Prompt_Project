# Expansion area: error correction, fault tolerance and noise

Research subagent, 2026-11-25.

## How the numbers were checked

Every fits-now circuit was run through the **real** `qcore::circuit::run` and `marginal` in a scratch crate (`scratchpad/qecverify`). The project was not modified. The results are exact simulator output.

**Section mapping.** The agent used the labels "Error correction", "Noise" and "Fault tolerance". All of these map to the existing section `machines`, unless a new section is added to `sections.json`.

## Notation

- q0 is the most significant bit.
- `CX a→b`: control a, target b.
- `CCX(a,b→t)`: Toffoli with controls a and b, target t.
- `CRY`, `CRZ`, `CS`: the base gate with a control in `c`. qcore supports these.
- **"Undo"**: apply the inverse of the state preparation at the end. If correction worked, q0 reads 0 with P = 1. This is a deterministic check.

## Ranked candidates

### bit-flip-code: The 3-qubit bit-flip code (extends error-correction)

- **Level:** beginner. **Prereqs:** cnot, swap-and-toffoli, measurement, error-correction.
- **Tag:** fits-now, 5 qubits (q0–q2 data, q3–q4 ancillas).
- **Why it matters:** encoding is entanglement, a|000⟩+b|111⟩, not three copies of the state. The parity checks never reveal a or b.
- **Circuit:**
  1. Prepare and encode: `RY(π/2) q0; CX 0→1; CX 0→2`.
  2. Inject an error, e.g. `X q1`.
  3. Measure the syndrome: `CX 0→3; CX 1→3; CX 1→4; CX 2→4`.
  4. Correct (deferred): `X q4; CCX(3,4→0); X q4; CCX(3,4→1); X q3; CCX(3,4→2); X q3`.
  5. Decode: `CX 0→2; CX 0→1`.
  6. Undo: `RY(−π/2) q0`.
- **Expected:**
  - Every single X error: `{qubits:[0]} "0":1`.
  - Syndrome on [3,4]:

    | Error | Syndrome |
    |---|---|
    | none | `00` |
    | X q0 | `10` |
    | X q1 | `11` |
    | X q2 | `01` |

- **Variant without the undo step:** RY(2π/3) with an X error on q1.
  - q0: `0:0.25, 1:0.75`.
  - Full state: `00011:0.25, 10011:0.75`.
- **Failure demos (verified):**
  - **Z on q1:** the syndrome is `00` and q0 after undo reads `1`. The code misses the error.
  - **Two X errors** (q0 and q1, input |0⟩): the syndrome is `01`, and the result is a logical flip.
- **Source:** IBM Quantum Learning, Foundations of QEC: https://quantum.cloud.ibm.com/learning/en/courses/foundations-of-quantum-error-correction

### phase-flip-code: The 3-qubit phase-flip code

- **Level:** beginner. **Prereqs:** bit-flip-code (NEW), hadamard, measurement-bases.
- **Tag:** fits-now, 5 qubits.
- **Circuit:**
  1. Encode: `RY(π/3) q0; CX 0→1; CX 0→2; H q0,q1,q2`.
  2. Inject `Z q1`.
  3. Apply `H q0,q1,q2`.
  4. Run the same syndrome, correction and decode steps as bit-flip-code.
  5. Undo: `RY(−π/3)`.
- **Expected:** q0 reads `0:1`. The syndrome is `10` for a Z error on q0, `11` for q1 and `01` for q2.
- **Contrast:** an X error on q1 slips through. q0 reads `0:0.25, 1:0.75` and the syndrome is `00`.
- **Source:** IBM QEC course (above).

### error-discretization: Why continuous errors can be corrected

- **Level:** intermediate. **Prereqs:** bit-flip-code (NEW), rotation-gates, measurement.
- **Tag:** fits-now, 5 qubits.
- **Circuit:** the bit-flip code with the error `RX(π/2) q1` instead of a full X.
- **Expected:**
  - Syndrome: `00:0.5, 11:0.5`. After undo, q0 still reads `0:1`.
  - With `RX(π/3)` instead: syndrome `00:0.75, 11:0.25`, and q0 again reads `0:1`.
- **Sources:**
  - IBM course, "Discretization of errors".
  - Knill & Laflamme: https://arxiv.org/abs/quant-ph/9604034

### repetition-code-under-noise: Encoding helps only below a threshold

- **Level:** intermediate. **Prereqs:** bit-flip-code (NEW), noise-channels (NEW).
- **Tag:** fits-now, 6 qubits. Noise comes from environment qubits.
- **Circuit:**
  1. Encode: `CX 0→1; CX 0→2`.
  2. Set the noise: `RY(θ) q3,q4,q5`, so each environment qubit fires with p = sin²(θ/2).
  3. Apply the noise: `CX 3→0; CX 4→1; CX 5→2`.
  4. Decode and take the majority: `CX 0→1; CX 0→2; CCX(1,2→0)`.
- **Expected P(q0 = 1):**

  | θ | p | P(q0 = 1) |
  |---|---|---|
  | π/4 | 0.146447 | 0.058058 |
  | π/3 | 0.25 | 0.15625 |
  | π/2 | 0.5 | 0.5 |
  | 2π/3 | 0.75 | **0.84375 (worse than no encoding)** |
  | 0.643501 rad | 0.1 | 0.028 |

  These match the formula 3p² − 2p³.
- **Caveat for the page:** the crossover at p = 0.5 is a classical pseudo-threshold. It is **not** the fault-tolerance threshold.
- **Source:** Aharonov & Ben-Or: https://arxiv.org/abs/quant-ph/9906129

### stabilizer-measurement: Stabilizers and syndrome extraction with ancillas

- **Level:** intermediate. **Prereqs:** bell-states, pauli-gates, phase-kickback.
- **Tag:** fits-now, 4 qubits.
- **Circuit:**
  1. Bell pair: `H q0; CX 0→1`.
  2. Optional error.
  3. ZZ check onto q2: `CX 0→2; CX 1→2`.
  4. XX check onto q3: `H q3; CX 3→0; CX 3→1; H q3`.
- **Expected:**
  - No error: checks [2,3] read `00:1`, and the data [0,1] read `00:0.5, 11:0.5`.
  - X on q0: checks read `10`, and the data read `01:0.5, 10:0.5`.
  - Z on q0: checks read `01`.
- **Sources:**
  - Gottesman, PhD thesis (1997): https://arxiv.org/abs/quant-ph/9705052
  - IBM QEC course.

### error-detection-422: The [[4,2,2]] error-detecting code

- **Level:** intermediate. **Prereqs:** stabilizer-measurement (NEW), ghz-states.
- **Tag:** fits-now, 6 qubits.
- **Circuit:**
  1. Encode: `H q0; CX 0→1; CX 0→2; CX 0→3`.
  2. Inject an error.
  3. ZZZZ check onto q4.
  4. XXXX check onto q5: `H q5; CX 5→q0..q3; H q5`.
- **Expected on [4,5]:**

  | Error | Checks |
  |---|---|
  | none | `00` |
  | X q2 | `10` |
  | Z q2 | `01` |
  | Y q2 | `11` |

- **Blind spot:** an X error on both q1 and q2 gives `00`, because it is a logical operator. The data read `0110:0.5, 1001:0.5`.
- **Source:** Linke et al., Sci. Adv. 2017: https://arxiv.org/abs/1611.06946
  - The abstract does not name [[4,2,2]].
  - No original [[4,2,2]] reference has been verified.

### noise-channels: Noise channels as hidden "environment" qubits (extends decoherence)

- **Level:** intermediate. **Prereqs:** decoherence, cnot, rotation-gates, entanglement.
- **Tag:** fits-now with environment qubits. Showing ρ needs a density-matrix feature.
- **Circuits:**
  - Bit-flip channel, p = 0.25: `RY(π/3) q1; CX 1→0`. q0 reads `0:0.75, 1:0.25`.
  - Phase damping: `H q0; CRY(2π/3) 0→1; H q0`. q0 reads `0:0.75, 1:0.25`, so coherence is scaled by cos(π/3).
- **Sources:** unverified (standard Stinespring/Kraus material).

### t1-t2-relaxation: Amplitude damping and T1/T2

- **Level:** intermediate. **Prereqs:** noise-channels (NEW), bloch-sphere.
- **Tag:** fits-now for up to 7 steps.
- **One damping step, γ = 0.5:** `CRY(π/2) sys→env_k; CX env_k→sys`.
- **Expected:**
  - Start from X q0: P(q0 = 1) is 0.5, 0.25 and 0.125 after 1, 2 and 3 steps.
  - Start from |+⟩, do one step, then H: q0 reads `0:0.853553, 1:0.146447`, so coherence is scaled by √(1−γ).
- **Sources:** unverified (textbook). T2 ≤ 2T1 has not been checked against a source.

### steane-code: The Steane [[7,1,3]] code

- **Level:** advanced. **Prereqs:** stabilizer-measurement (NEW), phase-flip-code (NEW).
- **Tag:** fits-now (partial, 7–8 qubits). The full syndrome needs 13 qubits, or mid-circuit measurement with reset.
- **Encoder:** `H q0,q1,q3; CX 3→4,5,6; CX 1→2,5,6; CX 0→2,4,6`.
- **|0⟩_L:** 8 strings at 0.125 each: `0000000, 0001111, 0110011, 0111100, 1010101, 1011010, 1100110, 1101001`.
- **Transversal H:** H on all 7 qubits gives |+⟩_L, which is 16 strings at 0.0625 each.
- **One-ancilla check** `CX q0,q2,q4,q6 → q7`:
  - No error: q7 reads `0`.
  - X on q4: q7 reads `1`.
  - X on q1: q7 reads `0`.
- **Full syndrome:** apply the Hamming checks by hand to the measured string. An X on qubit 5 gives 101 = 5.
- **Sources:**
  - Steane 1996: https://arxiv.org/abs/quant-ph/9601029. The abstract does not mention the 7-qubit code. The PRL that does is unverified.
  - IBM course, "CSS codes".

### error-propagation-and-flags: How errors spread, and flag qubits

- **Level:** advanced. **Prereqs:** cnot, phase-kickback, error-detection-422 (NEW).
- **Tag:** fits-now, 2 and 7 qubits.
- **Error spread through a CNOT:**
  - `X q0; CX 0→1` gives `11:1`. An X error copies forward to the target.
  - `H q0,q1; Z q1; CX 0→1; H q0,q1` gives `11:1`. Without the Z it gives `00`. A Z error travels backwards to the control.
- **Flag demo:**
  - Gates: `H q4; CX 4→0; CX 4→5; CX 4→1; [X q4 fault]; CX 4→2; CX 4→5; CX 4→3; H q4; CX q0..q3→6`.
  - With the fault: the flag q5 reads `1`, while q4 and q6 read `0`. The data hold `0011:0.5, 1100:0.5`, a weight-2 logical error that only the flag catches.
- **Source:** Chao & Reichardt, PRL 121, 050502 (2018): https://arxiv.org/abs/1705.02329

### dynamical-decoupling: Spin echo and dynamical decoupling

- **Level:** intermediate. **Prereqs:** pauli-gates, rotation-gates, bloch-sphere.
- **Tag:** fits-now, 1–2 qubits.
- **Circuits:**
  - No echo: `H; RZ(π/2); H` gives `0.5/0.5`.
  - Echo: `H; RZ(π/4); X; RZ(π/4); X; H` gives `0:1`.
  - Unknown detuning set by an environment qubit: `H q0; H q1; CRZ(π/2) 1→0; [X q0]; CRZ(π/2) 1→0; [X q0]; H q0`.
    - With the X pair: `0:1`.
    - Without the X pair: `0.5/0.5`, and the qubit is entangled with the environment.
- **Source:** Viola, Knill & Lloyd, PRL 82, 2417 (1999): https://arxiv.org/abs/quant-ph/9809071

### magic-states: T gates by magic-state injection; distillation and cultivation

- **Level:** advanced. **Prereqs:** universal-gates, phase-gates, teleportation, transversal-gates (NEW).
- **Tag:** fits-now for injection on 2 qubits. Distillation needs more than 8 qubits and a stabilizer simulator.
- **Circuit:** `H q0; H q1; T q1; CX 1→0; CX 0→1; CS 0→1; H q1`.
- **Expected:** q1 reads `0:0.853553, 1:0.146447`, which matches applying H;T;H directly.
- **Sources:**
  - Bravyi & Kitaev, PRA 71, 022316 (2005): https://arxiv.org/abs/quant-ph/0403025
  - Gidney, Shutty & Jones, "Magic state cultivation" (2024 preprint): https://arxiv.org/abs/2409.17595. It reports, from simulation, logical error down to 2·10⁻⁹ at 10⁻³ noise.

### transversal-gates: Transversal gates and the Eastin–Knill theorem

- **Level:** advanced. **Prereqs:** steane-code (NEW), universal-gates.
- **Tag:** fits-now for transversal H on the Steane code. The theorem itself is concept-only.
- **Sources:**
  - Eastin & Knill, PRL 102, 110502 (2009): https://arxiv.org/abs/0811.4262
  - Google color code: https://arxiv.org/abs/2412.14256. Its transversal Clifford gates add 0.0027(3) logical error.

### five-qubit-code: The [[5,1,3]] "perfect" code

- **Level:** advanced. **Prereqs:** stabilizer-measurement (NEW), entanglement.
- **Tag:** fits-now (partial).
- **Why it matters:** 2⁴ = 16 = 1 + 3×5, so every syndrome pattern is used exactly once.
- **Demo A, ring graph state on 5 qubits:**
  - Circuit: `H all; CZ ring (0,1)(1,2)(2,3)(3,4)(4,0)`, then the error, then the inverse.
  - All 15 single-qubit Paulis give distinct patterns:
    - Z on qubit j gives a single 1 at position j.
    - X on qubit j gives 1s at both neighbours of j.
    - Y on qubit j gives 1s at j and both its neighbours.
  - Examples: Z q0 → `10000`, X q0 → `01001`, Y q2 → `01110`.
- **Demo B, 8 qubits, 3 of the 4 generators:**
  - No error gives `000`. Z q0 gives `101`.
  - X q0 gives `000`, a blind spot. This shows why the 4th ancilla, and so a 9th qubit, is needed.
- **Source:** Laflamme, Miquel, Paz & Zurek (1996): https://arxiv.org/abs/quant-ph/9602019

### threshold-theorem: The threshold theorem and concatenation

- **Level:** advanced. **Tag:** concept-only. The 3p² − 2p³ maths can go on the page.
- **Sources:**
  - Aharonov & Ben-Or (above). It gives no single threshold number.
  - IBM course.

### error-mitigation: Error mitigation (ZNE, PEC, readout) vs correction

- **Level:** intermediate. **Prereqs:** noise-channels (NEW), measurement, variational-algorithms.
- **Tag:** fits-now for ZNE with environment qubits.
- **Setup:** a bit-flip channel with p = 0.1 applied k times.
- **Expected P(q0 = 0):**

  | k | P(q0 = 0) | ⟨Z⟩ |
  |---|---|---|
  | 1 | 0.9 | 0.8 |
  | 3 | 0.756 | 0.512 |
  | 5 | 0.66384 | 0.32768 |

- **Extrapolation:**
  - Richardson (linear, k = 1 and 3) gives 0.944.
  - Quadratic (k = 1, 3 and 5) gives 0.98288.
  - The true value is 1 and the raw value is 0.8.
- **Sources:**
  - Temme, Bravyi & Gambetta, PRL 119, 180509 (2017): https://arxiv.org/abs/1612.02058
  - Kandala et al., Nature 567, 491 (2019): https://arxiv.org/abs/1805.04492

### decoders: Matching, union-find and neural (AlphaQubit) decoders

- **Level:** advanced. **Tag:** concept-only. A classical matching widget is possible.
- **Sources:**
  - Delfosse & Nickerson, Quantum 5, 595 (2021): https://arxiv.org/abs/1709.06218. Thresholds ≈ 9.9% with perfect syndromes and ≈ 2.6% with faulty ones.
  - AlphaQubit, Nature 635, 834 (2024): https://arxiv.org/abs/2310.05900
  - Google blog, 20 Nov 2024: https://blog.google/technology/google-deepmind/alphaqubit-quantum-error-correction/
    - About 6% fewer errors than tensor-network decoding, and about 30% fewer than correlated matching.
    - Still too slow for real time.

### below-threshold-milestones: Hardware QEC milestones 2023–2025 (concept-only, dated)

- **Google Willow** (arXiv 2408.13687; Nature 638, 920, 2025):
  - Distance-7 code on 101 qubits.
  - 0.143% ± 0.003% error per cycle.
  - Λ = 2.14 ± 0.02.
  - Logical qubit lasts 2.4 ± 0.3× longer than the best physical qubit.
  - Decoder latency 63 μs at d = 5. Cycle time 1.1 μs.
  - Blog, 9 Dec 2024: https://blog.google/technology/research/google-willow-quantum-chip/
- **Google color code** (arXiv 2412.14256; Nature 645, 614, 2025):
  - Λ₃/₅ = 1.56(4).
  - Magic-state injection fidelity above 99%, with post-selection.
- **Harvard/QuEra** (Bluvstein et al., Nature 2023): https://arxiv.org/abs/2312.03982
  - Up to 280 physical qubits.
  - 48 logical qubits, using error **detection**.
- **AWS Ocelot:** see bosonic-codes.

### qldpc-codes: qLDPC and bivariate bicycle ("gross") codes

- **Tag:** concept-only.
- **Bravyi et al., Nature 627, 778 (2024):** https://arxiv.org/abs/2308.07915
  - 0.8% threshold.
  - 12 logical qubits from 288 physical qubits at 0.1% error, held for nearly 1M cycles.
  - The surface code would need nearly 3000 physical qubits for the same.
- **IBM blog, 27 Mar 2024:** https://www.ibm.com/quantum/blog/nature-qldpc-error-correction
  - The code is [[144,12,12]].
- **IBM roadmap blog, 10 Jun 2025 (targets, not results):** https://www.ibm.com/quantum/blog/large-scale-ftqc
  - Starling (2029): 100M gates on 200 logical qubits.
  - Loon (2025), Kookaburra (2026), Cockatoo (2027).

### bosonic-codes: Cat qubits and GKP

- **Tag:** concept-only. A real demo needs a Fock-space simulator.
- **Sources:**
  - GKP, PRA 64, 012310 (2001): https://arxiv.org/abs/quant-ph/0008040
  - AWS Ocelot (Putterman et al.), Nature 638, 927 (2025): https://arxiv.org/abs/2409.13025
    - Repetition code on cat qubits up to d = 5.
    - Logical error per cycle 1.75(2)% at d = 3 and 1.65(3)% at d = 5.
    - The "up to 90%" overhead reduction claim is unverified.

### benchmarking: Randomized benchmarking and quantum volume

- **Tag:** needs noise channels, shot sampling and random Cliffords.
- **Sources:**
  - Magesan, Gambetta & Emerson, PRL 106, 180504 (2011): https://arxiv.org/abs/1009.3639
  - Cross et al., PRA 100, 032328 (2019): https://arxiv.org/abs/1811.12926

### shor-code: Shor's 9-qubit code

- **Tag:** needs more than 8 qubits. The bit-flip and phase-flip demos already cover its two layers.
- **Source:** IBM course, "9-qubit Shor code" lesson. Shor (1995) is unverified.

## Simulator features that unlock the most (this area)

1. **Mid-circuit measurement, reset and classical feed-forward.** Unlocks repeated syndrome rounds, the full [[5,1,3]] code (7 qubits) and the Steane code in 8 qubits, flags, and real injection.
2. **A stabilizer (Clifford tableau) simulator with Pauli-error sampling.** Unlocks surface codes at d = 3/5/7, threshold curves, decoder widgets, distillation and qLDPC toys.
3. **A statevector cap of 12–14 qubits.** Unlocks the Shor code and the full Steane syndrome.
4. **Seeded shot sampling and Pauli/depolarizing channels.** Unlocks randomized benchmarking, quantum volume, PEC and readout mitigation.
5. **A density-matrix mode.** Gives exact T1/T2 states.
6. **A Fock-space mode.** Unlocks cat and GKP codes. Lowest priority.

## Misconception blocks

- **"Encoding always helps."** At p = 0.75 the encoded error is 0.84375, worse than the unencoded 0.75.
- **"The p = 0.5 crossover is the threshold."** It is a pseudo-threshold. Real thresholds depend on the model: 9.9% or 2.6% for toric codes, 0.8% for bivariate bicycle codes.
- **"QEC copies the qubit."** It does not. The state is a|000⟩+b|111⟩.
- **"Measuring the syndrome destroys the data."** It does not.
- **"Continuous errors can't be corrected."** They are discretized by the syndrome measurement.
- **"The 3-qubit code protects against everything."** A Z error passes through it.
- **"Mitigation is correction."** It is not.
- **"Λ = 2 means errors halve per qubit."** Λ applies per +2 of code distance.
- **"All 'logical qubit' headlines mean the same thing."** Detection with post-selection is not real-time correction.
- **"Every gate can be made transversal."** The Eastin–Knill theorem forbids this.
- **"Neural decoders are faster."** AlphaQubit is more accurate, but not yet real-time.
- **"Shor's code is Shor's algorithm."** They are different things.
