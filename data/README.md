# Data pipelines

Dataset manifests in, content-addressed forcing packs out
([docs/03 §3.8a](../docs/03-engine.md), [docs/09](../docs/09-data-sources.md)). The
repository stores manifests and code, never raw data: pipelines fetch only what a
region needs into a cache outside the repository and aggregate it to H3 cells.

| Path | What it is |
|---|---|
| `regions/*.toml` | Regions: resolution and the rule selecting cells (M1: [m1-swasia](regions/m1-swasia.toml), ADR-0040) |
| `manifests/*.toml` | One per dataset: version, license (incl. whether derived data may be redistributed, and when that was checked), citation, access, coverage, variables and units |
| `oikos_data/` | Python package: `pack` (writer/reader, same format as `oikos-io::pack`), `region`, `geometry`, `manifest` |
| `packs.lock.json` | Id (BLAKE3) of every pack we build; rebuilds must match (like the engine's golden file) |
| `build/` | Built packs (not committed) |
| `tests/` | pytest; includes checks that the engine reads Python-built packs |

```sh
source engine/scripts/env.sh          # pinned venv (h3, blake3, numpy, pytest, ...)
cd data
python -m oikos_data build geometry m1-swasia     # -> build/m1-swasia.geometry.oikospack
python -m oikos_data check                        # rebuild every locked pack, compare ids
pytest tests
../engine/target/release/oikos pack build/m1-swasia.geometry.oikospack   # inspect
```

A build that differs from the lock fails; pass `--update-lock` only when the change is
intended, and say why in the commit. The engine gate (`engine/scripts/check.sh`) runs
the tests and the lock check.

Rules:

- A pack only contains data whose license allows redistributing derived data
  (`manifest.require_redistributable`); record attribution in [CREDITS.md](../CREDITS.md).
- Store values compactly and reproducibly: integers with `scale_factor`/`add_offset`
  rather than floats from platform math, so rebuilds on any host give the same bytes.
- Geometry comes from the geometry pack; runs never compute it.
