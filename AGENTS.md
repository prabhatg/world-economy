# AGENTS.md

Guidance for AI coding agents (and humans) working in this repository. Read this first,
then the files it links to for the area you touch. If this file and a linked document
disagree, the linked document is the more specific source; fix the drift in the same
change.

## Project in one paragraph

**ManyWorlds** is an open-source, nonprofit simulation of human history from
~12,000 BCE to today, used as a policy laboratory for human welfare. History is the
calibration data; a data-assimilated "digital twin" of 2026 is the starting point for
testing ideas across thousands of simulated worlds. Ensemble runs are computed by
volunteers' devices (SETI@home-style). The engine is **Oikos** (Rust). Start with
[README.md](README.md) and [docs/01-vision.md](docs/01-vision.md).

## Repository map

| Path | What it is |
|---|---|
| [README.md](README.md) | Overview, decision summary, index of design docs |
| [docs/](docs/) | Design documents 01–14, [ADR log](docs/adr/README.md), [open questions](docs/open-questions.md) |
| [docs/index.html](docs/index.html) | Public website (GitHub Pages from `/docs` on `main`) |
| [engine/](engine/README.md) | Rust workspace: `oikos-core`, `oikos-math`, `oikos-grid`, `oikos-params`, `oikos-io`, `oikos-sim`, `oikos-cli` |
| [engine/params/](engine/params/m0-toy.toml) | Parameter registry: every parameter's unit, range, prior, status and sources |
| [engine/golden/](engine/golden/) | Golden work unit and its committed result (reproducibility oracle) |
| [engine/scripts/](engine/scripts/) | `check.sh` (verification gate), `bench.sh`, `compare-reports.sh`, git hooks |
| [design/](design/DESIGN-SYSTEM.md) | Design system, [tokens.json](design/tokens.json) (source of truth), [tokens.css](design/tokens.css) |
| [.claude/agents/design-reviewer.md](.claude/agents/design-reviewer.md) | Design review agent definition |
| [CREDITS.md](CREDITS.md) | Credit for every contributor type |

Key design documents by topic:

- Architecture and run modes: [docs/02-architecture.md](docs/02-architecture.md)
- Engine, determinism, numerics: [docs/03-engine.md](docs/03-engine.md)
- Domain mechanisms: [docs/04-domain-models.md](docs/04-domain-models.md)
- Calibration and data assimilation: [docs/05-calibration-assimilation.md](docs/05-calibration-assimilation.md)
- Policy lab: [docs/06-scenarios-and-policy.md](docs/06-scenarios-and-policy.md)
- Volunteer computing and work units: [docs/11-volunteer-computing.md](docs/11-volunteer-computing.md)
- Native clients: [docs/13-native-clients.md](docs/13-native-clients.md)
- **Engineering practices (TDD, gate, performance): [docs/14-engineering-practices.md](docs/14-engineering-practices.md)**
- Roadmap and current milestone: [docs/10-roadmap.md](docs/10-roadmap.md)

## Commands

All engine commands run from `engine/`. The Rust toolchain is pinned in
`engine/rust-toolchain.toml` (includes `wasm32-wasip1`, rustfmt, clippy).

```sh
cd engine
scripts/check.sh --quick       # inner loop: fmt, clippy -D warnings, release tests
scripts/check.sh               # full gate: + debug tests, wasm tests, golden runs, partition determinism
cargo test --release -p oikos-sim                 # one crate
cargo test --release --test determinism golden    # one test
cargo run --release -p oikos-cli -- run golden/m0-toy.workunit.json
cargo run --release -p oikos-cli -- bench --res 6 --years 20 --parts 4
scripts/bench.sh               # throughput + scaling sweep -> target/bench.jsonl
cargo run --release -p oikos-cli -- run examples/outputs.workunit.json -o target/example-result.json
cargo run --release -p oikos-cli -- params        # registry as JSON
```

The gate's output check runs zarr-python and pyarrow when available; point
`OIKOS_PYTHON` at a Python with `zarr`, `pyarrow` and `numpy` (e.g. a venv), otherwise
it is skipped with a notice.

WebAssembly tests need `wasmtime` (`cargo install wasmtime-cli --locked`); the gate
skips them with a notice if it is missing. `cargo test` does **not** rebuild the
`oikos` binary: run `cargo build --release -p oikos-cli` before benchmarking.

There is **no hosted CI**. The gate runs on our own machines
([ADR-0029](docs/adr/README.md)). Do not add GitHub Actions workflows.

## Non-negotiable rules

### 1. Test-driven development

Follow [docs/14-engineering-practices.md](docs/14-engineering-practices.md) §14.1.

- Red, green, refactor. Write the failing test first, see it fail, then implement.
- Every bug fix starts with a test that reproduces the bug.
- Use the right layer: unit / known-answer, property (`proptest`), determinism,
  conservation, golden, mechanism stylized-fact, performance.
- `scripts/check.sh` must pass before you commit. Report its result honestly; never
  claim a pass you did not run.

### 2. Determinism and the reproducibility contract

Results must be **bit-identical** for any partition count, thread count and target
(x86_64, aarch64, wasm32). Volunteer results are validated by hash. See
[docs/03-engine.md](docs/03-engine.md) §3.5.

- Conserved quantities (people, goods, money) are integers. Continuous dynamics use
  `f64` with only `+ - * / sqrt`.
- Call `oikos_math::{exp, ln, pow, ...}`, never `f64::exp()` and friends (platform
  libm differs). No `mul_add` unless explicit and uniform. NaN is a bug
  (`oikos_math::finite`).
- Every random draw comes from `Philox4x32` keyed by `(seed, Stream, entity, tick)`.
  No `rand`, no thread-local RNG, no time-based seeds. Never renumber an existing
  `Stream`; add new ones at the end.
- Stochastic decisions are `Probability::from_f64(p).hit(draw)` (integer compare).
- No iteration over `HashMap`/`HashSet` that affects simulation state; sort first or
  use ordered structures.
- Reductions must be order-independent (integer sums or sorted order).
- Entity ids come from the birth cell's counter (`EntityId::new(cell, counter)`),
  never from partition or thread.
- Grid geometry from `h3o` (areas, lat/lng, trig) must not be used inside a run.
- Outputs only read the world (observers get `&World`); recording must never change
  the state hash, and output bytes must be identical for any partition count.
- Every parameter comes from the registry; overrides are range-checked. Changing a
  registry default changes the golden hash.

### 3. Golden files change only on purpose

`engine/golden/m0-toy.json` is the oracle. A refactor or optimization that changes it
is a model change. Regenerate only when the change is intentional, and say why in the
commit message:

```sh
cargo run --release -p oikos-cli -- run golden/m0-toy.workunit.json -o golden/m0-toy.json
```

Changing the pinned `libm` version, RNG layout, checkpoint format or params schema
changes the golden hash.

### 4. Performance is a requirement

See [docs/14-engineering-practices.md](docs/14-engineering-practices.md) §14.3.

- Measure before and after with `oikos bench` (same `--res`, `--years`, `--parts`);
  include the numbers in the commit message for performance work.
- Hot paths: structure-of-arrays, no allocation per step in steady state, reuse
  partition buffers, no per-step thread spawning (use the rayon pool via
  `par_for_each`), no locks in the decide phase.
- Keep the wasm path working: rayon is native-only; `par_for_each` runs sequentially
  on wasm.

### 5. Design work goes through the design system and the design agent

Any user-facing change (website, viewer, apps, diagrams, reports) must:

1. Follow [design/DESIGN-SYSTEM.md](design/DESIGN-SYSTEM.md) and use tokens from
   [design/tokens.json](design/tokens.json) / [tokens.css](design/tokens.css); no ad hoc
   colors, fonts or spacing.
2. Respect the core principles: one message per region, guide the eye to one focal
   element, at most ~4 items per group, no harsh boxes (group by space and tone),
   diagrams first, uncertainty shown as ensembles, WCAG 2.2 AA, 48 px targets,
   light and dark themes, reduced motion.
3. Be reviewed by the **design-reviewer** agent
   ([.claude/agents/design-reviewer.md](.claude/agents/design-reviewer.md)) before it
   ships ([ADR-0032](docs/adr/README.md)). It renders the page at 390 px and 1440 px
   in light and dark with Playwright and returns prioritized fixes. Apply blockers and
   majors, or record why not.

The website is a single static file, [docs/index.html](docs/index.html), with tokens
inlined (mirroring `design/tokens.css`). No build step.

### 6. Decisions and documentation

- Architecture-level choices get an ADR row in [docs/adr/README.md](docs/adr/README.md)
  (next free number) and, if needed, a rationale section there.
- Update the design doc you changed behavior for, the roadmap status
  ([docs/10-roadmap.md](docs/10-roadmap.md)), and [docs/open-questions.md](docs/open-questions.md)
  when you resolve or raise a question.
- Every mechanism ships with: doc comment, citations, parameter priors, unit and
  property tests, and a stylized-fact test.

## Adding a mechanism

Mechanisms live in `engine/crates/oikos-sim/src/mechanisms.rs` (one module per domain
as they grow). See [docs/03-engine.md](docs/03-engine.md) §3.4.

1. Write the tests first: a unit or stylized-fact test of the behavior, and a world-level
   test in `engine/crates/oikos-sim/tests/` (see `tests/mechanisms.rs`).
2. Implement `Mechanism`: an honest `spec()` (every component read and written; `after`
   for anything it must follow) and `decide(sense, part, tile)`. Only touch agents in
   `tile`; read shared state only through `Sense`; create agents only via
   `PartitionView::spawn`.
3. Use a new `Stream` (appended at the end) for its random draws; count statistics in
   local variables and add them to `part.stats` once per tile (avoids false sharing).
4. Declare every parameter in the registry (`engine/params/m0-toy.toml`): group, unit,
   range (`kind`), default, prior, status and sources (`placeholder` needs a `note`;
   `literature`/`calibrated` need a source). Add the field to `Params` and to
   `Params::from_registry`; the registry/`Params` coverage test enforces the match.
   No magic numbers in mechanism code. Register the mechanism in
   `mechanisms::defaults` if it is part of the model.
5. Run the gate. Adding a default mechanism changes the golden hash: regenerate it
   deliberately and say why. Benchmark before and after.

## Code style

- Rust edition 2024, `rustfmt` (`engine/rustfmt.toml`: width 110, Max heuristics),
  `clippy -D warnings`, `unsafe_code = "forbid"` workspace-wide.
- Match the surrounding code: module-level `//!` docs explaining the why, sparse
  inline comments, small focused functions, descriptive names.
- Errors at boundaries are `Result<_, String>` with messages that say what to fix;
  inside the engine, invariant violations `assert!`/panic loudly.
- New dependencies: prefer pure-Rust crates that build for `wasm32-wasip1`; pin exact
  versions for anything that affects numerics.

## Git workflow

- Develop on a feature branch; `main` is updated only when the maintainer asks.
- Small, focused commits with clear messages (what and why; benchmark numbers for
  performance changes; reason for any golden change).
- Never force-push `main`. Never commit secrets, credentials or personal data.
- Commit and push finished work; the sandboxed environments are ephemeral.

## Safety, privacy, ethics

- Synthetic populations only; never model or target real private individuals.
- Results are decision support with uncertainty, not prescriptions; do not overclaim
  in code comments, docs or the website.
- Respect dataset licenses ([docs/09-data-sources.md](docs/09-data-sources.md)); store
  manifests and ingestion code, not restricted raw data.
- Give credit: cite sources for parameters and mechanisms; keep [CREDITS.md](CREDITS.md)
  current.

## Definition of done

- [ ] Failing test written first; now passes.
- [ ] `engine/scripts/check.sh` passes (or the non-engine equivalent was verified).
- [ ] Golden hash unchanged, or changed on purpose with the reason recorded.
- [ ] Benchmarks recorded for performance-relevant changes.
- [ ] User-facing changes reviewed by the design-reviewer agent.
- [ ] Docs, ADRs, roadmap and open questions updated.
- [ ] Committed and pushed.
