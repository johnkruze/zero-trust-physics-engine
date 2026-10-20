# Zero-Trust Physics: the engine

Zero-Trust Physics is a physics verifier for Super Intelligence (SI). SI agents ask it questions about real hardware, and every answer comes with a cryptographic proof, so any agent can run it again and get the same thing. Past what a model was compared with, it keeps going with the same equations and flags every step. It stops only at a physical limit, or where its equations, constants or tables no longer hold, and it names the reason. The range it was compared on, and how it did there, is in its description.

This is the source of the engine that answers at

```
https://mcp.zerotrustphysics.com/mcp
```

That address is open to any agent, and nothing here is needed to use it. This repository is for building the same engine on your own machine and getting the same proofs.

## What 0.15.0 is

This is engine 0.15.0, the version the open address has served since 19 October 2026: 36 models, 25 field solvers and 18 tools.

- **One model for each real object.** 36 models and 25 field solvers, where 0.14.3 served 40 and 25. Where a newer model of an object had been served beside the older one, the newer one now answers under the object's own ID, and the other IDs are retired: ten IDs in all, four of them served in 0.14.3 (`cell_2170_m80`, `reactor_spert3_fuel`, `reactor_spert3_twin`, `reactor_spert3_ccore_twin`). A request that names a retired ID is refused by name, `RetiredModel`, with the ID that serves its object.
- **Seal rule v2: SHA-384.** Every seal is SHA-384, written as 96 hex digits, and every answer that carries a receipt names its rule, `"seal_rule":"zero-trust-physics/seal/v2"`. The fields named for their hash are renamed to match (`receipt_sha384`, `sha384_seal`, `field_sha384` and the rest). A record of rule v1 (SHA-256, engine 0.14.3 and earlier) is refused by name, `SealRuleNotServed`. The texts hashed, their order and their separators are those of rule v1, so every seal of 0.15.0 differs from 0.14.3's.
- **Signed answers: ML-DSA-87.** Every answer that ends with a receipt (`compute_step`, `verify_step`, `run_trajectory`, `solve`, `run_dataset`'s head, `validate_series` and `design_run`) is signed as it leaves the open address: ML-DSA-87 over the receipt, an issue time, an expiry an hour later and a new nonce. The public key is 2,592 bytes and served at `https://mcp.zerotrustphysics.com/answer-key`. Its fingerprint, the `key_id` every signature names, is its SHA-384:

  ```
  ca06027a972c81c8630fed9f4ea2d4e91596b9434a88a03b7eb325b24e40645b7090cb705927745e73ee8b272eb1be38
  ```

  `verify_tape`, `verify_solution`, `verify_dataset` and `verify_receipt` take `public_key` and `seen_nonces`, and name a signature that does not hold: forged (`SignatureDoesNotVerify`), expired (`SignatureExpired`), of another key (`SignatureKeyMismatch`) or replayed (`SignatureNonceReplayed`). Signed ML-DSA-87 (RustCrypto `ml-dsa` 0.1.1); verified with `dilithium-py` 1.5.1 by the Python programs. A signature shows who issued an answer and when; the seals, and the answer computed again, show what was computed.
- **The Stardust → PICA chain.** The entry of the Stardust capsule (`capsule_stardust`) hands its nose conditions to the heat-shield solver (`ablator_slab`) through the coupling `stardust_entry_to_pica_slab`, under one chain proof. The entry: peak load 32.17 g against 32.89 g reconstructed, the drogue condition within 1.01 s of the radar record. The shield, fed by the entry: nose wear 4.65 mm, about a third more than measured, and the face within 156 K RMS of the airborne spectra. Alone, on 31 arc-jet tests outside the fit, the solver's wear has a mean of −2.6 percent and its hottest face is within 7 percent on 24 of 28; the face runs 4.1 percent hot on average.
- **Flags that say how far.** Past what a model was compared with it keeps going and flags every step, as before. Since 0.15.0 each flagged step also carries, for each flag, the limit, the value the step reached, the extent past the limit in the limit's unit, and that extent as a fraction of the limit. A run carries `exceedance_extremes`: for each limit it went past, how many steps, the first, and the step of the largest extent with its numbers. The receipt covers them, and `verify_tape` holds them to each frame's flags.

The models that now answer under the object's own ID, in their own words (`describe_card` and `describe_solver`):

- **`reactor_spert3`**, the SPERT III E-core test reactor (1,392 UO2 rods) through a reactivity burst. On each of the 70 tests left out of the fit in turn: at the median test, peak power within 6.8 percent, energy to the peak within 8.6, time to the peak within 4.1 and hot-spot clad rise within 9.2. Its largest gap: from 19 MW of operating power, 3 tests, energy reads 81.2 percent low.
- **`reactor_spert3_ccore`**, the SPERT III C-core (93 percent enriched plates), with subcooled boiling and void. With each tenth of the 111 transients left out of the fit in turn: peak power within 10.4 percent RMS, energy to the peak 17.2, the hot-spot plate rise at peak power 12.3. Its largest gap: at 5,000 to 20,000 gpm the plate maximum reads 24.6 percent RMS.
- **`cell_2170_model3`**, one Tesla Model 3 2170 lithium-ion cell (m80, m1000 or m448-N) from −20 to 40 °C, voltage and core temperature. With each cell left out of the fit in turn, its 234 runs: within 20.2 mV RMS. Its largest gap: the rated-current discharge at −20 °C, 68.7 mV RMS.
- **`grid_gb`**, the frequency of the Great Britain grid after an importing interconnector trips. With each of 86 trips left out of the fit in turn: depth of the fall within 0.224 RMS relative error, near 60 s within 0.076 Hz. Its largest gap: the 11 trips since November 2025, depth 0.359 RMS, 23 percent too deep on average.
- **`tokamak_jet_dte1`**, a heated JET pulse of the 1996 to 1997 D-T campaign. With each of 10 pulses left out in turn, medians: thermal energy within 16.0 percent of peak RMS, fast ions 5.3, fusion power 16.4. Its largest gap: pulse 42976 left out, fusion energy 4.03 MJ against 10.30.
- **`runaway_18650_lco`**, thermal runaway of one LiCoO2 18650 cell. With each record left out in turn: onsets within 16.2 K RMS on 10 records, peaks within 51.7 K RMS on 7. Its largest gap: Lu's cell peaked at 370 °C against 705.
- **`blast_wave_tnt_air`**, a field solver: the blast wave of a TNT charge in free air or on the ground. Against the surface-burst table kept out of the fit, impulse within 3.9 percent and duration within 8.0 from scaled distance 1.75 out; every arrival within 2.0 percent in free air and 5.3 on the ground. Its largest gap: near the fireball, at scaled distance 1.31, impulse 25 percent and duration 22 percent above the free-air table.

And three things about every answer and every model:

- **The same bytes on every machine tested.** Every exponential, logarithm, power and trigonometric function an answer depends on is computed by the engine's own code, never by the machine's math library. For 0.15.0, the open address, built from this source on Linux x86-64, gave the same answer, byte for byte with its signature taken out, to all 379 worked questions as a build on a Mac (Apple M4 Pro), and the same 13 printed proofs.
- **Anyone can recompute a proof.** A proof is a SHA-384 hash that binds the question to the answer, byte for byte. `genesis_seal`, in this repository, confirms one with no engine, and verifies the signature, and so do the Python programs at https://github.com/johnkruze/zero-trust-physics .
- **Every model says what stands behind it, numbers first.** One of three words: "compared with measurement" (46 of the 61 models: a real object), "held to a reference calculation" (3: a published calculation) or "company specimen" (12: an object the company specified, every constant stated). Each of the 46 states what it computes, what it was fitted on, what it was compared with and within what, and its gap, where it does worst.

## What is in this repository

Five crates, in Rust.

| Crate | What it holds |
|---|---|
| `genesis_kernel` | The models. Each is of one real thing, with its inputs, its range and every stop by name. The code calls a model a card. |
| `genesis_solvers` | The field solvers. |
| `genesis_seal` | The cryptographic proof: it makes one, and it confirms one with no engine. |
| `genesis_seal_parquet` | Converts a dataset and its proof to Parquet. |
| `protocol_mcp_gateway` | Serves the models and the solvers to agents over the Model Context Protocol. |

## Build it

Rust 1.88 or later. The first build fetches its dependencies from crates.io, and nothing after that needs the network.

```sh
cargo build --release --locked
```

## Ask it a question

`target/release/mcp_stdio` is the engine, speaking the Model Context Protocol over standard input and output. This asks it whether a battery pack holds at 71.44 amps for ten minutes:

```sh
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"me","version":"1"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"run_trajectory","arguments":{"domain":"energy","card":"battery_sag","initial":{"soc":1.0},"dt_s":0.1,"segments":[{"steps":6000,"load":{"current_a":71.44}}],"record_every":6000}}}' \
  | target/release/mcp_stdio
```

It holds, and the answer's `run_seal` is its proof:

```
d18acfdf04f5dd59a4baad927675e3b32c17ea57ee7867a20f3e7248634b93b3548210790ff6f935f67bd659662a3fea
```

That is the proof the open address gives for the same question. Ask again at 71.45 amps and the model stops at the pack's 36.0 V cutoff and says so, `UndervoltageCutoff`.

Run this way, with no key, the engine's answers carry their seals and receipt and no signature, and it says so once when it starts. To sign them as the open address does, make a key of your own and give it to the engine:

```sh
target/release/answer_key new my_answer.key      # a new seed, readable by you alone; prints its public key
target/release/answer_key public my_answer.key   # the public key again: {"alg":"ML-DSA-87","key_id":...,"public_key":...}
ZTP_SIGNING_KEY_FILE=my_answer.key target/release/mcp_stdio
```

Your signatures are your key's, and name its `key_id`. Only the open address's answers carry the key shown above.

To hand the engine to your own agent, name the program in your MCP client:

```json
{
  "mcpServers": {
    "zero-trust-physics": { "command": "/path/to/target/release/mcp_stdio" }
  }
}
```

`target/release/hosted_door` serves the same engine over HTTP, as the open address does, and serves its public key at `/answer-key`. Run it with no arguments and it says what it needs; it takes the key as `--signing-key <file>`.

## Confirm a proof with no engine

Save the text of a run's answer to a file, then:

```sh
target/release/genesis_seal verify-run answer.json
```

To verify the signature too, give it the public key, the base64 `public_key` of `https://mcp.zerotrustphysics.com/answer-key`, saved to a file:

```sh
target/release/genesis_seal verify-run answer.json --public-key answer_key.pub --nonce-ledger seen.txt
```

Run `genesis_seal` alone and it lists what else it confirms: a chain of runs, a dataset, one row of a dataset, a signature alone. The same verification in Python is at https://github.com/johnkruze/zero-trust-physics : the seals with Python's standard library alone, the signature with one package, `dilithium-py`.

## Run the tests

```sh
cargo test --release --locked -p genesis_kernel
cargo test --release --locked -p genesis_solvers
cargo test --release --locked -p genesis_seal
cargo test --release --locked --no-fail-fast -p protocol_mcp_gateway
cd genesis_seal_parquet && cargo build --release --locked && cargo test --release --locked
```

Some tests print one line, `skipped: … needs the saved runs`, and return: they compare with runs the company keeps, which are not in this repository. The tests use Unix tools such as `ps` and `mkfifo`.

Every test passed on 19 October 2026 on an Apple M4 Pro: kernel 901, solvers 545, seal 212, gateway 1,140, Parquet converter 23. One gateway test, `a_comparison_of_solves_opens_its_commitment_with_rows_in_the_place_of_steps`, starts the debug build of `genesis_seal` that the test itself builds; on a heavily loaded machine that program once did not start, and the test passed when run again (`--test validation_router`, 19 of 19 in 9 s).

## Two things to know

- **Proofs and machines.** Build it on your machine and every model and every field solver gives the proof the open address gives. For 0.15.0 that was measured on the two machine kinds above, a Mac (Apple M4 Pro) and Linux x86-64. On another kind, compare one worked answer with the open address before relying on it.
- **Paths that are not here.** Comments and some descriptions name files under `evidence/`, and `REFERENCE.md`, `AGENTS.md` and `HOSTING.md`. Those are the company's own records and are not part of this repository.

## Credit

`NOTICE` names the code and the numbers of other people that this source holds, and whose each is. To cite the work, cite the paper: https://doi.org/10.5281/zenodo.23305697 . An agent can ask the engine itself: the `cite` tool answers for the service and for each model.

## Where the rest is

- The site: https://zerotrustphysics.com
- A page for each real thing: https://zerotrustphysics.com/things/
- The paper: https://doi.org/10.5281/zenodo.23305697
- Programs that confirm a proof with no engine: https://github.com/johnkruze/zero-trust-physics
- John Kruze: kruze@zerotrustphysics.com · 928-264-7883

Released under Apache-2.0.

---

Written by Super Intelligence, on behalf of Protocol Company.
