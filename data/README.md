# Data pipelines

Dataset manifests in, content-addressed forcing packs out
([docs/03 §3.8a](../docs/03-engine.md), [docs/09](../docs/09-data-sources.md)). The
repository stores manifests and code, never raw data: pipelines fetch only what a
region needs into a cache outside the repository and aggregate it to H3 cells.

| Path | What it is |
|---|---|
| `regions/*.toml` | Regions: resolution and the rule selecting cells (M1: [m1-swasia](regions/m1-swasia.toml), ADR-0040) |
| `manifests/*.toml` | One per dataset: version, license (incl. whether derived data may be redistributed, and when that was checked), citation, access, coverage, variables and units |
| `oikos_data/` | Python package: `pack` (writer/reader, same format as `oikos-io::pack`), `region`, `geometry`, `manifest`, `raster` (cached COG overview reads), `terrain` |
| `packs.lock.json` | Id (BLAKE3) of every pack we build; rebuilds must match (like the engine's golden file) |
| `locks/<dataset>.json` | Fingerprint of every input tile a pack was built from, so changed upstream data is caught |
| `build/` | Built packs (not committed) |
| `tests/` | pytest; includes checks that the engine reads Python-built packs |

```sh
source engine/scripts/env.sh          # pinned venv (h3, blake3, numpy, pytest, ...)
cd data
python -m oikos_data build geometry m1-swasia     # -> build/m1-swasia.geometry.oikospack
python -m oikos_data build terrain m1-swasia      # fetches ~170 MB of DEM overviews once (~1 min)
python -m oikos_data check                        # rebuild every locked pack, compare ids
pytest tests
../engine/target/release/oikos pack build/m1-swasia.geometry.oikospack   # inspect
```

A build that differs from the lock fails; pass `--update-lock` only when the change is
intended, and say why in the commit. The engine gate (`engine/scripts/check.sh`) runs
the tests and the lock check; packs whose inputs are not in the cache are skipped with
a notice. The cache lives in `$OIKOS_DATA_CACHE` (default `~/.cache/manyworlds`).

| Pack | Fields | Source |
|---|---|---|
| `geometry` | `h3`, `lat`, `lng`, `area_km2` | H3 |
| `terrain` | `elev_mean`, `elev_min`, `elev_max`, `elev_sd` (m), `slope_mean` (degrees), `land_fraction` | Copernicus DEM GLO-90 at 1/4 resolution (~360 m): a modern surface model as a proxy; open sea (0 m) excluded, lakes count as land; slopes underestimate 90 m slopes |

Rules:

- A pack only contains data whose license allows redistributing derived data
  (`manifest.require_redistributable`); record attribution in [CREDITS.md](../CREDITS.md).
- Store values compactly and reproducibly: integers with `scale_factor`/`add_offset`
  rather than floats from platform math, so rebuilds on any host give the same bytes.
- Geometry comes from the geometry pack; runs never compute it.
