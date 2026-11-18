# Zero-Trust Physics: the engine

Zero-Trust Physics is a deterministic physics runtime with a proof on every answer. Agents ask it questions about real hardware, and every answer comes with a cryptographic proof, so any agent can run it again and get the same thing. Past what a model was compared with, it keeps going with the same equations and flags every step. It stops only at a physical limit, or where its equations, constants or tables no longer hold, and it names the reason. The range it was compared on, and how it did there, is in its description.

This is the Apache-2.0 reference source, frozen at tag `v0.15.2`, of the Rust physics and proof-verification engine serving:

```
https://mcp.zerotrustphysics.com/mcp
```

The open MCP endpoint (`$0`, no key) serves **`v0.15.3`** across **`61` models** (`36` time-stepping cards + `25` field solvers, `18` MCP tools, **`46` validated against laboratory or flight measurement**, SHA-384 seal rule v2, ML-DSA-87 receipt signatures with operator `key_id` `ca06027a972c81c8630fed9f4ea2d4e91596b9434a88a03b7eb325b24e40645b7090cb705927745e73ee8b272eb1be38` at `https://mcp.zerotrustphysics.com/answer-key`).

### Commercial RLVR Domain Packs, Custom Suites & Engine Licensing

Built for RL environment vendors and post-training labs (`Harbor`, `HUD`, `Inspect`, and native `MCP`; `--network none` offline static binaries; continuous graded proximity rewards alongside binary pass/fail; **`0 of 2,481`** adversarial cheat attempts scored above `0` across all five packs):

- **[Self-Serve Domain Packs ($9,500 per domain · $39,500 All-Five Bundle)](https://zerotrustphysics.com/offers/fast-start/):** 30 withheld calibrated multi-step tasks (`34.4%–37.5%` frontier tool `pass@1` medians) + 100,000 sealed SFT runs + static `macos-arm64`, `linux-amd64`, and `linux-arm64` runtime & verifier per domain (instant card download or net-15 invoice).
- **Free 5-task evaluation samples on Hugging Face (`sh sample/verify_sample.sh` -> `SAMPLE HOLDS`):**
  - [Group 1 · Batteries, Thermal Runaway & Grid Frequency](https://huggingface.co/datasets/zero-trust-physics/zero-trust-physics-sample-group-1)
  - [Group 2 · Orbital, Space & Atmospheric Entry](https://huggingface.co/datasets/zero-trust-physics/zero-trust-physics-sample-group-2)
  - [Group 3 · Fluids, Cryogenics, Hydraulics & Contact](https://huggingface.co/datasets/zero-trust-physics/zero-trust-physics-sample-group-3)
  - [Group 4 · Materials, Structures & RF](https://huggingface.co/datasets/zero-trust-physics/zero-trust-physics-sample-group-4)
  - [Group 5 · Nuclear & Fusion](https://huggingface.co/datasets/zero-trust-physics/zero-trust-physics-sample-group-5)
- **[Custom & Exclusive Environments (including AI Datacenters `chip -> cooling -> grid`) & Engine Licensing](https://zerotrustphysics.com/offers/):** Bespoke coupled multi-physics suites, exclusive withheld draws, and internal runtime & verifier licensing (read-only NDA source audit available).

## Architecture & guarantees

- **Bit-identical IEEE 754 execution across architectures.** Every exponential, logarithm, power, and trigonometric function an answer depends on is computed in pure Rust rather than the host system math library, giving byte-identical outputs across macOS `arm64`, Linux `arm64`, and Linux `x86-64`.
- **SHA-384 proofs and ML-DSA-87 receipt signatures (`zero-trust-physics/seal/v2`).** Every step, trajectory, solve, chained computation, and dataset carries a deterministic SHA-384 seal (`genesis_seal`, or offline Python verifiers at https://github.com/johnkruze/zero-trust-physics) and optional post-quantum `ML-DSA-87` signature over the receipt (`key_id`, issue time, expiry, and unique nonce). Every flagged step reports the limit, reached value, extent past the limit, and fractional exceedance.
- **Validation tiers:** Every model declares its evidence backing in its description: `compared with measurement` (`46` of `61` models), `held to a reference calculation` (`3`), and `analytical reference models` (`12`), along with exact operating bounds and largest residual gap.

Five crates, in Rust:

| Crate | What it holds |
|---|---|
| `genesis_kernel` | Time-stepping physical models (cards), each with inputs, operating bounds, and named refusals. |
| `genesis_solvers` | Steady and boundary-value field solvers. |
| `genesis_seal` | Deterministic SHA-384 seal construction, recomputation verification, and ML-DSA-87 signing/verification. |
| `genesis_seal_parquet` | Dataset and sealed trajectory conversion to Parquet. |
| `protocol_mcp_gateway` | Model Context Protocol (MCP) server over `stdio` and HTTP. |

## Build it

Rust 1.88 or later. The first build fetches its dependencies from crates.io, and nothing after that needs the network.

```sh
cargo build --release --locked
```

## Ask it a question

`target/release/mcp_stdio` speaks the Model Context Protocol over standard input and output. This asks whether a battery pack holds at 71.44 amps for ten minutes:

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

Ask again at 71.45 amps and the model stops at the pack's 36.0 V cutoff with `UndervoltageCutoff`.

With no signing key, answers carry their seals and receipt without a signature. To sign receipts with your own key:

```sh
target/release/answer_key new my_answer.key      # a new seed, readable by you alone; prints its public key
target/release/answer_key public my_answer.key   # the public key again: {"alg":"ML-DSA-87","key_id":...,"public_key":...}
ZTP_SIGNING_KEY_FILE=my_answer.key target/release/mcp_stdio
```

Only answers from the open endpoint carry the operator `key_id` shown above. Register the binary in an MCP client:

```json
{
  "mcpServers": {
    "zero-trust-physics": { "command": "/path/to/target/release/mcp_stdio" }
  }
}
```

`target/release/hosted_door` serves the same engine over HTTP and publishes its public key at `/answer-key` (accepts `--signing-key <file>`).

## Verify a proof with no engine

Save a run's JSON response to `answer.json`, then verify it offline:

```sh
target/release/genesis_seal verify-run answer.json
```

To verify the receipt signature as well, pass the saved `public_key` from `https://mcp.zerotrustphysics.com/answer-key`:

```sh
target/release/genesis_seal verify-run answer.json --public-key answer_key.pub --nonce-ledger seen.txt
```

Run `genesis_seal` alone for subcommands covering chains, datasets, individual rows, and standalone signatures. Pure-Python offline verifiers live at https://github.com/johnkruze/zero-trust-physics .

## Run the tests

```sh
cargo test --release --locked -p genesis_kernel
cargo test --release --locked -p genesis_solvers
cargo test --release --locked -p genesis_seal
cargo test --release --locked --no-fail-fast -p protocol_mcp_gateway
cd genesis_seal_parquet && cargo build --release --locked && cargo test --release --locked
```

Tests that compare against private verification fixtures print `skipped: … needs the saved runs` and return cleanly. The test suite uses Unix tools (`ps`, `mkfifo`).

## Credit

`NOTICE` names third-party code and published reference data credited by this source. Cite the monograph, version 2.0: https://doi.org/10.5281/zenodo.23517223 , or call the `cite` MCP tool.

## Links

- Website & commercial offers: https://zerotrustphysics.com · https://zerotrustphysics.com/offers/ · https://zerotrustphysics.com/llms.txt
- Self-serve RLVR Domain Packs ($9,500) & All-Five Bundle ($39,500): https://zerotrustphysics.com/offers/fast-start/
- Hugging Face organization (5 free sample packs, benchmarks & sealed datasets): https://huggingface.co/zero-trust-physics
- Real-hardware model index: https://zerotrustphysics.com/systems/
- Paper (version 2.0): https://doi.org/10.5281/zenodo.23517223
- Standalone Python SHA-384 & ML-DSA-87 proof verifiers: https://github.com/johnkruze/zero-trust-physics
- John Kruze, CEO: kruze@zerotrustphysics.com · [WhatsApp +1 928-264-7883](https://wa.me/19282647883)

Released under Apache-2.0.

---

Protocol Company.
