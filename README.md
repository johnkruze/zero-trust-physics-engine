# Zero-Trust Physics: the engine

Zero-Trust Physics is a physics verifier for Super Intelligence (SI). SI agents ask it questions about real hardware, and every answer comes with a cryptographic proof, so any agent can run it again and get the same thing. Past what a model was compared with, it keeps going with the same equations and flags every step. It stops only at a physical limit, or where its equations, constants or tables no longer hold, and it names the reason. The range it was compared on, and how it did there, is in its description.

This is the source of the engine that answers at

```
https://mcp.zerotrustphysics.com/mcp
```

That address is open to any agent, and nothing here is needed to use it. This repository is for building the same engine on your own machine and getting the same proofs.

## What 0.14.3 is

This is engine 0.14.3, the version the open address has served since 18 October 2026: 40 models, 25 field solvers and 18 tools. Of the 380 worked answers 0.14.2 gives, 308 are the same bytes and 72 changed in their words only, every proof kept. The 23 other worked answers are the new models'.

- **`reactor_spert3_twin`**, the SPERT III E-core test reactor (1,392 UO2 rods) through a reactivity burst. Twelve numbers fitted on all 70 tests of report IDO-17281: at the median test, peak power within 6.0 percent, energy to the peak within 7.9 and time to the peak within 3.4. Predicted from fits that left each test out in turn: 6.2, 7.8 and 3.0. Its largest gap: from 19 MW of operating power, 3 tests, energy reads 81.8 percent low.
- **`reactor_spert3_ccore_twin`**, the SPERT III C-core (93 percent enriched plates), with subcooled boiling. Eight constants fitted on 111 transients of report IDO-16918; predicted with each tenth left out in turn, peak power within 12.6 percent RMS, energy to the peak within 19.6 and the hot-spot plate rise at peak power within 13.4. Its largest gap: at 230 psig with 5,000 to 20,000 gpm, peak power 21.2 percent RMS.
- **`reactor_msre_pump`**, the reactivity the control rods of the Molten-Salt Reactor Experiment supply while the fuel pump starts or coasts down. Two constants fitted on the start-up; on the 73 coast-down rod readings not in the fit, within 8.1 pcm RMS, largest 23.7. The steady circulation loss: 0.2120 % dk/k against the measured 0.212 ± 0.004.
- **`fuel_rod_triga_jsi`**, a field solver: the temperature of the instrumented fuel element 6753 of the Jožef Stefan Institute's TRIGA Mark II reactor through a pulse. Two constants fitted on 39 pulses; on 16 pulses not in the fit, the thermocouple's maximum rise within 9.2 percent (mean 5.5), its time after the power peak within 19.6 percent (mean 11.9), and its rise 10 s after the peak within 4.3 percent on the 8 records that run that long.
- **`solve` and `verify_solution` take `from`:** a sealed run of a model handed into a field solver under one chain proof. The coupling `triga_pulse_to_element_6753` hands a pulse of the `reactor_triga_jsi` model to `fuel_rod_triga_jsi`; chained on the same 16 pulses, the thermocouple's maximum rise reads 9.8 to 4.3 percent low.

And three things about every answer and every model:

- **The same bytes on every machine tested.** Every exponential, logarithm, power and trigonometric function an answer depends on is computed by the engine's own code, never by the machine's math library. For 0.14.3, the open address, built from this source on Linux x86-64, gave the same answer, byte for byte, to all 403 worked questions as a build on a Mac (Apple M4 Pro), and the same 13 printed proofs.
- **Anyone can recompute a proof.** A proof is a SHA-256 hash that binds the question to the answer, byte for byte. `genesis_seal`, in this repository, confirms one with no engine, and so do the Python programs at https://github.com/johnkruze/zero-trust-physics .
- **Every model says what stands behind it, numbers first.** One of three words: "compared with measurement" (50 of the 65 models: a real object), "held to a reference calculation" (3: a published calculation) or "company specimen" (12: an object the company specified, every constant stated). Each of the 50 describes itself as a spec sheet in six parts: what it computes; its range; what it was fitted on, or that nothing was fitted; what it was compared with, and within what; the gap, where it does worst; and what it refuses. A model replaced by a newer one names it in `superseded_by` and is still served: `reactor_spert3` and `reactor_spert3_fuel` name `reactor_spert3_twin`, and `reactor_spert3_ccore` names `reactor_spert3_ccore_twin`.

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
6bd7e54b83c71da84eda74f3314416d619bbbce112186aa548085cb5a4ba0d34
```

That is the proof the open address gives for the same question, and the one on the front page of https://zerotrustphysics.com . Ask again at 71.45 amps and the model stops and says why.

To hand the engine to your own agent, name the program in your MCP client:

```json
{
  "mcpServers": {
    "zero-trust-physics": { "command": "/path/to/target/release/mcp_stdio" }
  }
}
```

`target/release/hosted_door` serves the same engine over HTTP, as the open address does. Run it with no arguments and it says what it needs.

## Confirm a proof with no engine

Save the text of a run's answer to a file, then:

```sh
target/release/genesis_seal verify-run answer.json
```

Run `genesis_seal` alone and it lists what else it confirms: a chain of runs, a dataset, one row of a dataset. The same checks in Python, with nothing to install, are at https://github.com/johnkruze/zero-trust-physics .

## Run the tests

```sh
cargo test --release --locked -p genesis_kernel
cargo test --release --locked -p genesis_solvers
cargo test --release --locked -p genesis_seal
cargo test --release --locked --no-fail-fast -p protocol_mcp_gateway
cd genesis_seal_parquet && cargo build --release --locked && cargo test --release --locked
```

Some tests print one line, `skipped: … needs the saved runs`, and return: they compare with runs the company keeps, which are not in this repository. The tests use Unix tools such as `ps` and `mkfifo`.

One of the gateway's tests fails in this repository, because of what it checks: `generator_mmrtg_seebeck_meets_the_served_contract`. The MMRTG model's description states a prediction made before the data it predicts were published, and the test looks for the commit that recorded that prediction in the company's own repository of the gateway; this repository is one commit without that history.
Every other test passed on 18 October 2026 on an Apple M4 Pro: kernel 838, solvers 525, seal 201, gateway 1,192, Parquet converter 20. One gateway test times a run while it holds its permit, and on a heavily loaded machine it can fail and then pass when run again.

## Two things to know

- **Proofs and machines.** Build it on your machine and every model and every field solver gives the proof the open address gives. For 0.14.3 that was measured on the two machine kinds above, a Mac (Apple M4 Pro) and Linux x86-64. On another kind, compare one worked answer with the open address before relying on it.
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
