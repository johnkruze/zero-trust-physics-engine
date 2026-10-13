# Zero-Trust Physics: the engine

Zero-Trust Physics is a physics verifier for Super Intelligence (SI). SI agents ask it questions about real hardware, and every answer comes with a cryptographic proof, so any agent can run it again and get the same thing; when a question goes past what a model was tested for, the answer stops there and says why.

This is the source of the engine that answers at

```
https://mcp.zerotrustphysics.com/mcp
```

That address is open to any agent, and nothing here is needed to use it. This repository is for building the same engine on your own machine and getting the same proofs.

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
cargo test --release --locked -p protocol_mcp_gateway
cd genesis_seal_parquet && cargo build --release --locked && cargo test --release --locked
```

About seven minutes in all on a recent laptop, most of it the gateway's. Some tests print one line, `skipped: … needs the saved runs`, and return: they compare with runs the company keeps, which are not in this repository. The tests use Unix tools such as `ps` and `mkfifo`.

## Two things to know

- **Proofs and processors.** Build it on your machine and every model gives the proof the open address gives. A field solver's last digits, and so its proof, can differ between one kind of processor and another.
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
