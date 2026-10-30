# Data pipelines

Dataset manifests in, content-addressed forcing packs out
([docs/03 §3.8a](../docs/03-engine.md), [docs/09](../docs/09-data-sources.md)). The
repository stores manifests and code, never raw data: pipelines fetch only what a
region needs into a cache outside the repository and aggregate it to H3 cells.

| Path | What it is |
|---|---|
| `regions/*.toml` | Regions: resolution and the rule selecting cells (M1: [m1-swasia](regions/m1-swasia.toml), ADR-0040) |
| `manifests/*.toml` | One per dataset: version, license (incl. whether derived data may be redistributed, and when that was checked), citation, access, coverage, variables and units |
| `oikos_data/` | Python package: `pack` (writer/reader, same format as `oikos-io::pack`), `region`, `geometry`, `manifest`, `raster` (cached COG overview and strip-window reads), `terrain`, `climate`, `coastline`, `vector` (verified downloads, cached bbox reads), `hydrology`, `soils`, `radiocarbon`, `rachis`, `sites`, `obsidian` |
| `packs.lock.json` | Id (BLAKE3) of every pack and observation table we build; rebuilds must match (like the engine's golden file) |
| `locks/<dataset>.json` | Fingerprint of every input tile a pack was built from, so changed upstream data is caught |
| `build/` | Built packs (not committed) |
| `tests/` | pytest; includes checks that the engine reads Python-built packs |

```sh
source engine/scripts/env.sh          # pinned venv (h3, blake3, numpy, pytest, ...)
cd data
python -m oikos_data build geometry m1-swasia     # -> build/m1-swasia.geometry.oikospack
python -m oikos_data build terrain m1-swasia      # fetches ~170 MB of DEM overviews once (~1 min)
python -m oikos_data build climate m1-swasia      # reads ~21 GB of CHELSA rows once (~1 h), caches ~2 GB
python -m oikos_data build coastline m1-swasia    # reads ~1 GB of paleo-DEM rows once (~5 min)
python -m oikos_data build hydrology m1-swasia    # downloads RiverATLAS + LakeATLAS (~4 GB) once
python -m oikos_data build soils m1-swasia        # reads SoilGrids 1 km windows once (~1 min)
python -m oikos_data build radiocarbon_dates m1-swasia     # p3k14c + IntCal20 -> build/*.parquet
python -m oikos_data build radiocarbon_density m1-swasia
python -m oikos_data build rachis m1-swasia                # Allaby et al. 2017 barley rachis table
python -m oikos_data build sites m1-swasia                 # Palmisano et al. 2021 survey settlements
python -m oikos_data build obsidian m1-swasia              # DObsiSS volcano locations -> pack
python -m oikos_data build obsidian_assemblages m1-swasia  # DObsiSS sourced artefacts -> table
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
| `climate` | `temp_mean`, `temp_seasonality` (°C; SD of monthly means), `precip` (mm/year), `precip_cv` (%) per (time, cell), 92 century slices at midpoint years 12,050–2,950 BCE, linear in time | CHELSA-TraCE21k v1.0 bio1/bio4/bio12/bio15, 2×2-block means of 30″ pixels, land and sea; CC BY-SA 4.0 (open question 23) |
| `coastline` | `land_fraction` per (time, cell), same slices as `climate`, linear in time; cells at 0 are uninhabitable in the engine | CHELSA-TraCE21k paleo-DEM ocean mask (one global sea level per slice on modern topography and bathymetry, no regional isostasy); the Caspian, lakes and the Sea of Marmara count as land (the hydrology pack's lakes are subtracted in the engine) |
| `hydrology` | `river_length` (km of reaches with ≥ 1 m³/s), `river_discharge` (m³/s, largest reach), `river_order` (Strahler), `floodplain_fraction`, `lake_fraction` | RiverATLAS / LakeATLAS v1.0 (HydroATLAS, CC BY 4.0): modern hydrography as a proxy; floodplain from GIEMS-D15 maximum inundation per reach catchment; reservoirs excluded from lakes |
| `obsidian` | one field per traced source group (`cappadocia`, `eastern_anatolia`): 1 in cells within one step (a day's walk) of one of its volcanoes | DObsiSS 2.0 source table (CC BY-SA 4.0; shared under the same license) |
| `soils` | `clay`, `sand`, `silt` (%), `organic_carbon` (g/kg), `ph`, `coarse_fragments` (vol %), 0–30 cm depth-weighted | SoilGrids 2.0 1 km aggregates (CC BY 4.0), Homolosine samples located per cell; built-up, water and glaciers unmapped (nodata); modern soils as a proxy |

Observation tables (Parquet, locked like packs; docs/05 §5.1):

| Table | Rows | Source |
|---|---|---|
| `radiocarbon_dates` | one per p3k14c date in the region: lab id, 14C age/error, material, taxa, site, source database, location accuracy, cell, IntCal20 median year, probability share in the window | p3k14c 1.0.0 (MIT), IntCal20 (CC BY 4.0); repeated lab ids keep their first row |
| `radiocarbon_density` | cell × century (12,000–3,000 BCE): summed calibrated probability and number of dates, for dates located within 500 m or exactly | as above |
| `rachis` | one per archaeobotanical assemblage in the region: crop, site and phase, location, cell, median calendar year, non-shattering share and count, rachis fragments determined and excluded, primary source | Allaby et al. 2017 supplement, Table S4 (barley; CC BY 4.0) |
| `sites` | one per survey site phase in the region: survey window, site id, location, cell, start and end calendar years of its cultural period, size in hectares (NaN when unknown), source survey | Palmisano et al. 2021, Zenodo 4322979 (Central Anatolia and Upper Mesopotamia; CC BY 4.0) |
| `obsidian_assemblages` | one per sourced site context in the region with a dated ASPRO period: context id, site, location, cell, period and its start and end calendar years ("a or b" spans both), artefacts per traced source group and from other sources, characterization study | DObsiSS 2.0, Zenodo 55848 (CC BY-SA 4.0; shared under the same license) |

Rules:

- A pack only contains data whose license allows redistributing derived data
  (`manifest.require_redistributable`); record attribution in [CREDITS.md](../CREDITS.md).
- Store values compactly and reproducibly: integers with `scale_factor`/`add_offset`
  rather than floats from platform math, so rebuilds on any host give the same bytes.
- Geometry comes from the geometry pack; runs never compute it.
