# ManyWorlds viewer

Plays back an ensemble of simulated worlds (ADR-0063). The map shows one world's field
(people, farming, soil lost, obsidian) at one time; the charts show every world in the
ensemble, so one world is always seen against the many that fit the record.

The viewer reads what the engine recorded: each world's `fields.zarr` (run outputs,
docs/03 §3.8) and a `catalog.json` listing the worlds and the series the charts draw.

## Run it

```sh
# 1. Build the engine CLI and the M1 forcing packs (see AGENTS.md and data/README.md).
(cd engine && cargo build --release -p oikos-cli)
# 2. Run an ensemble: the calibrated defaults and the 8 best consistent worlds of round 8.
(cd calibration && PYTHONPATH=.:../data python -m oikos_cal ensemble runs/m1/round10/best-wave6.json --out ../viewer/data/m1)
# 3. Serve the viewer.
cd viewer && npm ci && npm run dev    # http://localhost:5173/
```

An M1 ensemble is about 13 MB per world, so `viewer/data/` is not committed. The dev
server serves it at `data/`; `?data=<url>/` points the page at another ensemble. To host
a build, put an ensemble directory next to `dist/index.html` as `data/m1/`.

## The volunteer page

`volunteer.html` is the browser client (ADR-0100, #22): it lends the tab to the
coordinator that serves it. Each world runs in a Web Worker on the engine the gate
checks (the wasm32-wasip1 `oikos` CLI, under a WASI shim), so results equal native
hosts' bit for bit; the page shows the simulated year, the world it last computed and
whether a second computer confirmed it.

```sh
(cd ../engine && cargo build --release --target wasm32-wasip1 -p oikos-cli && cargo build --release -p oikos-coord)
npm run build    # dist/volunteer.html and dist/oikos.wasm (the engine without its debug sections)
../engine/target/release/oikos-coord serve --dir /tmp/coord --client dist   # then open /client/volunteer.html
```

`npm run dev` serves the page at `/volunteer.html` with the engine from
`engine/target` and passes the coordinator's routes to `OIKOS_COORDINATOR`
(default `http://127.0.0.1:8650`).

## Develop

```sh
npm test            # unit tests (vitest): Zarr reader, fields, formatting, catalog, volunteer host
npm run typecheck
npm run build       # typecheck, then a static build in dist/
npm run smoke       # renders the dev server's ensemble at phone and desktop widths (Playwright),
                    # then the volunteer page end to end against a coordinator and a native host
```

| File | What it is |
|---|---|
| `src/zarr.ts` | Minimal Zarr v3 reader for engine outputs (uint64, gzip), lazy chunk cache |
| `src/catalog.ts` | The ensemble catalog written by `python -m oikos_cal ensemble` |
| `src/fields.ts` | The four map fields, their classes and labels |
| `src/cells.ts` | H3 cell polygons (h3-js) |
| `src/map.ts` | deck.gl map on the night surface; colors from design tokens |
| `src/chart.ts` | Ensemble charts (SVG) |
| `src/main.ts` | The page: state, playback, rendering |
| `src/volunteer/host.ts` | The browser host: lease, packs by id (BLAKE3-checked), run, archive, report |
| `src/volunteer/engine.ts` | Runs a unit on the wasm CLI under a WASI shim, files in memory |
| `src/volunteer/worker.ts` | The engine in a Web Worker |
| `src/volunteer/unit.ts`, `tar.ts` | A leased unit's checks; the run directory as a ustar archive |
| `src/volunteer/status.ts`, `main.ts` | What the volunteer page says; the page |
| `scripts/engine.mjs` | Copies the wasm engine without its custom sections for the build |

Every change to what people see follows [design/DESIGN-SYSTEM.md](../design/DESIGN-SYSTEM.md)
and is reviewed by the design-reviewer agent before it ships (ADR-0032).
