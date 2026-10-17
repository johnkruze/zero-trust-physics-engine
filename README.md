# Zero-Trust Physics: the engine

Zero-Trust Physics is a physics verifier for Super Intelligence (SI). SI agents ask it questions about real hardware, and every answer comes with a cryptographic proof, so any agent can run it again and get the same thing. A model stops at a mathematical or physical limit and names it, and the range it was compared on, and how it did there, is in its description.

This is the source of the engine that answers at

```
https://mcp.zerotrustphysics.com/mcp
```

That address is open to any agent, and nothing here is needed to use it. This repository is for building the same engine on your own machine and getting the same proofs.

## What 0.14.0 is

This is engine 0.14.0, the version the open address has served since 17 October 2026: 37 models, 24 field solvers and 18 tools.

- **The same bytes on every machine tested.** Every exponential, logarithm, power and trigonometric function an answer depends on is computed by the engine's own code, never by the machine's math library. Built from this source on a Mac (Apple M4 Pro), on Linux arm64, and on Linux amd64 with and without FMA, it gave the same answer, byte for byte, to all 380 worked questions, and the same 13 printed proofs. The open address answers byte for byte as a local build does.
- **Anyone can recompute a proof.** A proof is a SHA-256 hash that binds the question to the answer, byte for byte. `genesis_seal`, in this repository, confirms one with no engine, and so do the Python programs at https://github.com/johnkruze/zero-trust-physics .
- **Misses are reported as misses.** A model compared with measurements of its real thing says how it did, in one of three words: met, met in part or missed. Its description says whether the comparison was blind and, where it was not, what was seen and when. A model that missed is still served, with its miss stated.

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

Nine of the gateway's tests fail in this repository, because of what they check. Each holds a model served forecast first to the commit, in the company's own repository of each crate, that carries its forecast's protocol, and this repository is one commit without that history:
`a_model_calibrated_to_the_record_is_held_to_its_calibration`, and `<model>_meets_the_served_contract` for `blast_wave_tnt_air`, `culture_trichoderma_atp`, `generator_mmrtg_seebeck`, `grasp_lift_glass_4tu`, `momentum_jwst_dihedral`, `momentum_jwst_srp`, `plasma_sheath_ram_c2` and `stress_plate_hole_s235`.
Every other test passed on 17 October 2026 on an Apple M4 Pro: kernel 817, solvers 516, seal 201, gateway 1,156, Parquet converter 20. One gateway test times a run while it holds its permit, and on a heavily loaded machine it can fail and then pass when run again.

## Two things to know

- **Proofs and machines.** Build it on your machine and every model and every field solver gives the proof the open address gives. That was measured on the four machine kinds above. On another kind, compare one worked answer with the open address before relying on it.
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
