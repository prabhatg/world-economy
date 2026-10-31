# Architecture decision records

| ADR | Decision | Status |
|---|---|---|
| 0001 | Purpose: policy laboratory for human welfare; history is calibration data | Accepted |
| 0002 | Three run modes (free, assimilated reanalysis, scenario forks) on one engine | Accepted |
| 0003 | Start epoch ~12,000 BCE | Accepted |
| 0004 | Adaptive resolution via super-individual agents with split/merge | Accepted |
| 0005 | Multi-dimensional welfare: health, material, capabilities, sustainability | Accepted |
| 0006 | Core in Rust + GPU kernels; Python (PyO3) for orchestration/calibration/AI | Accepted |
| 0007 | Web viewer in TypeScript with deck.gl/CesiumJS; not Flutter/Firebase | Accepted |
| 0008 | Storage: Zarr (fields), Parquet/Arrow (agents, events) on object storage; DuckDB for queries | Accepted |
| 0009 | H3 geodesic hex grid with hierarchical refinement | Accepted |
| 0010 | Bulk-synchronous phased ticks + counter-based RNG for determinism | Accepted |
| 0011 | Cloud-first (spot ensembles), HPC later — amended by 0014 (volunteer tier) | Accepted |
| 0012 | Open source, open provenance | Accepted |
| 0013 | First milestone: Neolithic Southwest Asia vertical slice | Accepted |
| 0014 | Volunteer computing (SETI@home-style) for ensemble workloads; central cluster for coupled runs | Accepted |
| 0015 | Own cloud-native coordinator (Rust, GCP), not BOINC; backend-neutral work units | Accepted |
| 0016 | Bitwise cross-platform determinism is a hard requirement on CPU/WASM | Accepted |
| 0017 | Two-tier numerics: integer/fixed-point conserved quantities, restricted IEEE f64 + own math crate; integer GPU kernels where validated bitwise | Accepted |
| 0018 | Engine compiled to WASM for browser clients; native builds for desktop/mobile/headless | Accepted |
| 0019 | All client types: browser, desktop, mobile, headless | Accepted |
| 0020 | All volunteer types, with separate `science` / `explore` queues | Accepted |
| 0021 | Nonprofit, open-source governance (fiscal sponsor → foundation) | Accepted |
| 0022 | Native client UI shell: Flutter + flutter_rust_bridge, presentation + IPC only; Tauri v2 as desktop fallback | Accepted |
| 0023 | Per-user Rust daemon (`worldd`) = headless client; sandboxed per-work-unit engine workers with no network; local IPC to UI | Accepted |
| 0024 | Globe in native clients = embedded TS web viewer; daemon serves it on localhost | Accepted |
| 0025 | Licenses: Apache-2.0 for code; CC-BY 4.0 for docs and results; datasets per source | Accepted |
| 0026 | Names: ManyWorlds (project, volunteer network, apps); Oikos (engine, crates `oikos-*`) — pending trademark/domain check | Accepted |
| 0027 | GPU compute: wgpu first (Vulkan/Metal/DX12/WebGPU), CUDA backend later behind the same trait | Accepted |
| 0028 | Test-driven development is mandatory; golden/determinism tests are the oracle for every refactor and optimization | Accepted |
| 0029 | No hosted CI for now: verification gate runs on our own compute; cross-host determinism via compared reports | Accepted |
| 0030 | Performance is a first-class requirement: measured benchmarks, targets and an optimization backlog (doc 14) | Accepted |
| 0031 | ManyWorlds Design System (design/): research-grounded principles, shared tokens for web and Flutter, no harsh boxes, diagrams first | Accepted |
| 0032 | Every user-facing change is reviewed by the `design-reviewer` agent before it ships | Accepted |
| 0033 | Project website is a static page in `docs/index.html`, served by GitHub Pages from `/docs` (no build step) | Accepted |
| 0034 | Mechanism API: declared reads/writes/period/after; schedule rejects ambiguous order; multi-rate via gcd base period; cache-tiled, cell-local decide phase | Accepted |
| 0035 | Parameter registry: one versioned TOML per model declaring unit, range, prior, status and sources for every parameter; engine embeds it and validates all overrides; placeholders must say so | Accepted |
| 0036 | Outputs: own minimal Zarr v3 writer (uint64, gzip, deterministic) + Parquet via the `parquet` crate; written through the host sink; observers are read-only | Accepted |
| 0037 | Python bindings: `oikos-py` (PyO3, abi3 wheel via maturin, module `oikos`) as thin wrappers over `oikos-sim`; JSON for structured values, NumPy for arrays, `ValueError` for engine errors; outside default members; tested in the gate against the golden hash | Accepted |
| 0038 | Partitions are contiguous cell ranges of near-equal estimated cost (8 × agents + cells), re-split every 16 steps when > 10% out of balance; performance settings, not model parameters | Accepted |
| 0039 | Deterministic performance metric: exact wasm instruction counts (wasmtime fuel, found by bisection) of `oikos bench --no-timing`, instead of callgrind (not on every host) | Accepted |
| 0040 | M1 region: H3 res-5 cells whose centers lie in 24–42°N, 26–58°E (22,198 cells over base cells 15, 16, 22, 31, 33, 41; includes Cyprus, the paleo-Gulf basin and the Nile valley and delta), chosen offline from a manifest; a run gets its cell list from the geometry pack and uses only H3 topology | Accepted |
| 0041 | Forcing packs: one little-endian file (magic, canonical JSON header, aligned arrays), identified by its BLAKE3 hash; per-cell static and time-sliced fields with unit, dtype, scale/offset, nodata and sources; packs of a region share a cell-list hash | Accepted |
| 0042 | M1 climate: CHELSA-TraCE21k v1.0 bioclim (bio1, bio4, bio12, bio15) as per-cell means of 2×2-block-averaged 30″ pixels, land and sea alike, for century slices placed at their midpoint years and interpolated linearly; the pack follows the stricter of the dataset's two licence statements (CC BY-SA 4.0) | Accepted |
| 0043 | Coastlines: a `coastline` pack (`land_fraction[time, cell]`) from the CHELSA-TraCE21k paleo-DEM's per-century ocean mask (a bathtub model: one global sea level per slice on modern topography and bathymetry) rather than regional ICE-6G_C relative sea level; a cell is habitable while its interpolated land fraction is above zero; worlds without the pack treat every cell as habitable (golden unchanged) | Accepted |
| 0044 | Hydrology: a static `hydrology` pack from RiverATLAS and LakeATLAS (HydroATLAS, CC BY 4.0; not the standalone HydroRIVERS, whose HydroSHEDS licence restricts redistribution): river length, largest reach's natural discharge and Strahler order, floodplain fraction (GIEMS-D15 maximum inundation) and natural-lake fraction per cell; dry land = coastline land − lake fraction decides habitability | Accepted |
| 0045 | Soils: a static `soils` pack from SoilGrids 2.0's 1 km aggregates (CC BY 4.0): clay, sand, silt, organic carbon, pH and coarse fragments as 0–30 cm depth-weighted means of 2×2-block samples located through the Homolosine projection; modern soils as a proxy, non-soil pixels skipped | Accepted |
| 0046 | Radiocarbon observations: p3k14c (public release, MIT compendium) dates in the region, de-duplicated by lab id and calibrated with IntCal20 (annual grid, curve error in quadrature, normalized), as two locked Parquet tables (dates; probability per cell and century for LocAccuracy ≥ 2), built like forcing packs and identified by BLAKE3 | Accepted |
| 0047 | Foraging carrying capacity: with a climate pack, capacity = dry-land area × (terrestrial density from Miami-model NPP, `ref × (NPP/NPP_ref)^elasticity`, + an aquatic density where a perennial river, lake shore or sea coast is at hand), recomputed every `capacity_period_ticks`; all coefficients in the registry; toy worlds keep the random capacity (golden trajectory unchanged, hash changed by the new parameters) | Accepted |
| 0048 | Forager dispersal: a default mechanism after fission; a cell at least `dispersal_crowding` full (N/K) buds off one group with `dispersal_probability` per year (draw keyed by cell and tick): `dispersal_fraction` of its first band whose share fits moves to the least crowded habitable neighbor if it is less crowded and has room (as of last step); new `Stream::Dispersal` | Accepted |
| 0049 | Cultivation and domestication: agents carry practice (`cultivation` effort and `domestication` share of the sown crop, fixed-point parts per billion), inherited by split-off groups and people-weighted on merge; a deterministic `cultivation` mechanism (last in the step) moves effort toward a target that nests three competing drivers as weights (population pressure beyond `cultivation_crowding` × forager capacity, climate stress = decline of forager capacity over `stress_window_years`, wild-cereal abundance) and selects for domestic traits logistically at `selection_coefficient` × effort against wild gene flow; capacity = forager capacity × (1 − effort) + farm capacity × (wild yield × effort + domestic gain × effort × domestication); climate worlds start peopled (`initial_occupancy`); checkpoint magic `OIKOSCK2` | Accepted |
| 0050 | Observation operators live in `calibration/` (Python package `oikos_cal`), read activity recorded from a world through the bindings and never change it; first operator: radiocarbon date density, forward-modelled (people-years per cell and century × Surovell 2009 taphonomic survival, blurred by a calibration kernel built from IntCal20 per error class) and compared with site-phase-binned calibrated dates as a multinomial over centuries within coarse H3 blocks, so research intensity per block cancels; 1% background | Accepted |
| 0051 | Domestication-timing operator: rachis assemblages (Allaby et al. 2017 Table S4, barley, a locked Parquet table) scored beta-binomially (concentration 2) against the model's domestic share of the crop sown within one ring of the site over the phase's century ± 1 (wild mutant frequency where nobody cultivates); the recorder keeps effort × domestication people-years | Accepted |
| 0052 | First calibration waves on the calibration host (`oikos_cal`): Morris screening (μ* in units of each target's tolerance) picks the factors emulators see; Gaussian-process emulators (linear trend, ARD, nugget; SciPy pinned) per summary; history matching with implausibility cutoff 3 on the max over summaries; summaries are 5 radiocarbon eras (centered log shares, block-bootstrap variance) and 4 barley domestic-share eras, discrepancy sd 0.25 and 0.1; every run recorded in `calibration/runs/` | Accepted |
| 0053 | Neolithic demographic transition as a nested hypothesis: a band's intrinsic growth rate moves linearly with its cultivation effort from `growth_rate` (mobile foragers) to `sedentary_growth_rate` (sedentary cultivators); the default sets them equal (no transition) until calibration says otherwise; the golden hash changes only because the parameter set grew (summaries identical) | Accepted |
| 0054 | Stable-climate hypothesis of cultivation (Richerson, Boyd and Bettinger 2001) as a fourth nested driver: the cultivation target is scaled by 1 - `cultivation_variability_weight` × climate variability (mean change of wild-cereal suitability per century over the stress window, between century boundaries, recomputed once per century); weight 0 (the default) switches it off; with century-resolution forcing it does not delay the onset | Accepted |
| 0055 | Land degradation as a nested hypothesis: cells keep a fixed-point soil share, eroded at `soil_erosion_rate` × effort per year and re-forming at `soil_formation_rate` × what is missing, updated each capacity period; field yields scale by min(1, soil / `soil_critical`); checkpoint layout `OIKOSCK3` (soil per cell; `OIKOSCK2` restores with whole soil); erosion 0 (the default) switches it off | Accepted |
| 0056 | Calibration round 2: radiocarbon eras of 800 years (10), judged one by one and together by a multivariate implausibility (Mahalanobis distance with the block-bootstrap covariance plus discrepancy and emulator variances, against the 99.5% chi-square quantile, Vernon et al. 2010); 27 free parameters including the nested hypotheses of ADR-0053-0055; runs in `calibration/runs/m1/round2/` | Accepted |
| 0057 | Calibrated M1 defaults: the registry defaults of the 12 emulated parameters are the best-fitting consistent world of round 2 (wave 2, run 28; joint radiocarbon and rachis log-likelihood), status `calibrated` with the run as source and the consistent worlds' 10-90% range in the note; parameters the emulators did not see keep their defaults, and `sedentary_growth_rate` follows `growth_rate` (no transition). `python -m oikos_cal best` reruns the top consistent runs with the other parameters at their defaults (three seeds) and records the choice in `calibration/runs/m1/round2/best.json`; a test keeps the default world consistent with the record. Hypothesis tests pin the pre-calibration values, so calibration never moves what they test | Accepted; values superseded by 0059 |
| 0058 | Calibration round 3: a third wave refocuses the emulators on the 17 factors whose round-2 screening effect reaches 0.4 tolerances (adding sedentary growth, soil critical share, NPP elasticity, dispersal crowding and wild yield ratio); earlier waves keep their own active sets and still rule out. `wave --previous ... --screening` refocuses. Defaults unchanged | Accepted |
| 0059 | Site-count operator and calibration round 4: survey site phases of Palmisano et al. 2021 (Anatolia, Upper Mesopotamia) counted aoristically in 500-year eras from 6,500 to 3,000 BCE; a world's cultivator activity in each survey's landscape (cells with any recorded site) is dated through the surveys' own period scheme; centered log shares, bootstrap covariance, one group per survey, discrepancy 0.25. A fourth wave judges radiocarbon, rachis and site counts together, and the best consistent world (wave 4, run 251) becomes the registry defaults: 17 parameters `calibrated`, sources in `calibration/runs/m1/round4/best.json` | Accepted |
| 0060 | Down-the-line exchange of obsidian as a tracer: cells hold a fixed-point availability per source group; every step, occupied cells within one res-5 step of a group's volcanoes (an `obsidian` forcing pack from DObsiSS 2.0) have all they want, other occupied cells take (1 - `obsidian_loss`) times the mean of themselves and their occupied neighbors, empty cells hold none; nothing else reads it, so it never changes history; checkpoint layout `OIKOSCK4` (`OIKOSCK3` restores with none held); sourced assemblages (DObsiSS, ASPRO periods) become an observation table | Accepted |
| 0061 | Obsidian sourcing operator and calibration round 5: the share of Cappadocian (against Eastern Anatolian) obsidian in each DObsiSS context with at least three artefacts of the two, against the obsidian in use in the site's cell and its neighbors over its period, beta-binomial (φ = 2); contexts no chain of habitable cells joins to a volcano (Cyprus) are dropped; targets are mean shares in four zones of relative distance to the sources (edges -250, 0, 250 km) before and after 6,900 BCE, bootstrap covariance, one group, discrepancy 0.1. A parameter new to the registry joins a later wave as an active factor. A round's best world replaces the defaults only if it fits clearly better than they do on the same seeds; `profile` calibrates one parameter at the defaults when a new record constrains it and leaves history alone: `obsidian_loss` 0.0028 (`calibration/runs/m1/round5/obsidian_loss.json`) | Accepted |

## ADR-0007 rationale (frontend)

Simulation output is terabytes of gridded fields and agent snapshots. Firestore's
document model and per-read pricing are a poor fit; chunked arrays/columnar files on
object storage are the right substrate. The hard UI problem is GPU rendering of
millions of geospatial features on a globe, where deck.gl/CesiumJS/WebGPU are far
ahead of Flutter. Firebase may still be used for auth/hosting if convenient.

## ADR-0017 rationale (numerics)

Integers make conservation of people, goods and money exact over 14,000 simulated years
and make results trivially comparable across hosts. `f64` keeps accuracy for continuous
dynamics; restricting it to IEEE correctly-rounded operations plus our own
transcendental functions makes it bit-reproducible on x86_64, aarch64 and wasm32.
Converting probabilities to integer thresholds makes every stochastic decision exact.
GPU vendors and WGSL do not guarantee correctly rounded division/transcendentals, hence
integer kernels where bitwise validation is needed.

## ADR-0042 rationale (M1 climate)

CHELSA-TraCE21k is the only transient climate reconstruction at ~1 km covering the whole
M1 window, so terrain-driven contrasts (Zagros foothills against the alluvium) survive
aggregation to ~250 km² cells. Its four bioclimatic summaries are what the first
foraging and cultivation mechanisms need (warmth, water, and how seasonal both are);
monthly fields would cost 12× the bytes for detail those mechanisms do not yet use.
The files are stored in one-row strips, not tiles, so a region costs whole rows (~58 MB
per file, ~21 GB for M1 once); 2×2 block means keep the cache at ~2 GB while leaving
~85 samples per cell. Slices are 100-year means, so a slice is placed at its midpoint
and the engine interpolates linearly between midpoints; events shorter than a century
(the 8.2 ka event) are smoothed out, and the climate is a single model run (CCSM3), a
structural uncertainty to carry into calibration. A seeded weather generator from
per-cell statistics replaces raw fields later (doc 11 §11.2).

## ADR-0043 rationale (coastlines)

ICE-6G_C gives relative sea level on a ~1° grid; turning it into coastlines needs
high-resolution bathymetry, which CHELSA-TraCE21k has already combined with a global
sea-level curve (and ICE-6G ice) for every century slice. Using its mask keeps the coastline
consistent with the climate pack (same slices, same licence, same reader) at the cost
of regional isostatic and tectonic effects, which are metres against the ~120 m
postglacial rise. The mask covers only the ocean: the Caspian, lakes and the Sea of
Marmara count as land until a lake mask arrives with hydrology (#9). Habitability is a
mask the engine derives each step from the pack and the year, never checkpointed, so a
restored run rebuilds it; bands in flooded cells do not split, and the `inundation`
mechanism (last in the step) moves them.

## ADR-0047 rationale (foraging capacity)

The Miami model needs only annual temperature and precipitation, which the climate
pack has for every century, and is the standard first-order productivity model; it is
crude (no seasonality, soils or CO2), and a better productivity model can replace it
behind the same function. Forager density rising with productivity is the best-
supported cross-cultural regularity (Binford 2001; Kelly 2013; Tallavaara et al. 2018);
its level and elasticity are uncertain, so both are priors for calibration. Aquatic
resources add density rather than multiplying it, because river and coastal foraging
does not depend on local rain: the Nile valley feeds people in a rainless desert.
Soils matter for cultivation, not foraging, and enter with #13. Capacity is
recomputed every ten years, not every step: the climate changes on century slices, and
recomputing 22,198 cells' productivity every year would cost more than the agents.

## ADR-0048 rationale (dispersal)

On the real M1 region a res-5 cell feeds about one forager band (~20-50 people), so
fission by group size (bands of 60 or more) never fires and populations stay where they
start. Foragers fill a landscape by budding: part of a group founds a new territory when
the old one is full, a reaction-diffusion wave of advance whose speed grows with growth
rate and dispersal rate (Fisher-KPP; Ammerman and Cavalli-Sforza 1984). Moving toward
less crowded land, only where the group can be fed, follows the ideal free distribution.
Events are per cell (a res-5 cell is about one band's territory), which also keeps the
cost low: one draw per crowded cell, and the neighbor scan only when it hits; per-band
draws and scans cost ~45% more per agent-step in a fully crowded world.
Room is judged from last step's population (the partition-independent snapshot), so
groups from several neighbors arriving in the same step can overshoot a cell a little;
logistic decline corrects it, and reserving room across partitions within a step would
break partition independence. Group-size (scalar-stress) fission, the Neolithic
demographic transition and village aggregation remain in #14; they need sedentism and
cultivation (#13).

## ADR-0049 rationale (cultivation and domestication)

Domestication was a co-evolution of plants and practice, not a switch (Rindos 1984):
people cultivated wild cereals for centuries to millennia while domestic traits such as
non-shattering ears spread through the sown crop (Fuller et al. 2014). Two quantities
carry that: how much effort a band puts into cultivation, and how domestic its crop is.
Both belong to bands, not cells, so they travel with migrants: farming spreads by people
moving (demic diffusion) without a separate rule, and a desert or a forest a band moves
into does not inherit a crop it never sowed. Domestic cereals cannot reseed themselves,
so a crop with no one cultivating it reverts to the wild type.

Where the literature disagrees, the model nests the hypotheses instead of choosing
(docs/04 §4.2; open question 2): why people began to cultivate is a sum of three drivers
with separate weights (population pressure: Binford 1968, Cohen 1977; climate stress
such as the Younger Dryas: Moore and Hillman 1992, Bar-Yosef 1998; abundance and niche
construction: Smith 2007, Zeder 2016), so a weight of zero switches one off and
calibration weighs them against the record; how fast crops domesticated is the range of
the selection coefficient, whose prior spans the protracted estimate from the
archaeobotanical record (s ~ 0.003; Purugganan and Fuller 2011; Allaby et al. 2008) and
the rapid one from experimental cultivation (decades to two centuries; Hillman and
Davies 1990). Gene flow from wild stands opposes selection in proportion to the effort
not spent cultivating, so light cultivation never domesticates.

Pressure is measured against forager capacity with a tolerance above 1: foragers at
equilibrium feel none, and dispersal absorbs small local overshoots, so pressure comes
when the climate or a rising sea cuts capacity under people, or when cultivation has let
people outgrow it. Because any cultivation of rich wild stands raises capacity, effort
then ratchets up; that feedback is the hypothesis' content, and how readily it ignites
is open question 24. Capacity blends land use linearly by effort, a first-order
approximation; wild crops yield `wild_yield_ratio` of domestic ones, and early
cultivation was not more productive per hour than foraging (Bowles 2011).

Traits are fixed-point integers so merging (integer weighted means, rounded down) and
checkpoints are exact on every host, and updates round in the direction of change so a
decaying practice reaches zero instead of stalling at a rounding floor. The mechanism
draws no random numbers. It runs last (after inundation), and split-off groups copy
their parent's traits, so `fission`, `dispersal` and `inundation` read `AgentTraits` and
`cultivation` writes it. Worlds without a climate pack never cultivate, so the M0 toy's
trajectory is unchanged; its hash changed with the checkpoint layout and parameter set.
Climate worlds now start with a forager band in every habitable cell at
`initial_occupancy` of capacity, because Epipaleolithic foragers lived across Southwest
Asia at 12,000 BCE; starting from a dozen founders left most of the region empty for
millennia and made the founders' crowding read as pressure. Agents grew from 20 to 28
bytes: +8% wasm instructions and +14% native time per agent-step on the toy benchmark
after copying kept bands in runs during merges and skipping trait moves for bands
already in place (from +21% at first). Sedentism, storage and the Neolithic demographic
transition build on these traits next (#13, #14).

## ADR-0050 rationale (observation operators, radiocarbon density)

The record never measures people; it measures what survived, was found, dated and
calibrated. Comparing a model's population with a summed probability distribution of
dates (SPD) would treat calibration wiggles, taphonomic loss and excavation history as
demography. So the operator moves the model into the data's space instead of the data
into the model's: it predicts the calibrated date mass a history would produce and
scores that against the observed mass (the logic of simulation-based SPD tests,
Shennan et al. 2013, Timpson et al. 2014, here as a likelihood).

- **Calibration** is a fixed linear map. The kernel gives, for a sample whose true date
  is uniform in a century and whose error is typical of its class (≤ 50, ≤ 100,
  ≤ 200 years; larger errors are dropped), the calibrated mass in each century, through
  the same calibration as the dates pipeline. Plateaus such as the late-glacial ones
  then blur the model exactly as they blur the data. It is built once per table (4 s
  on M1), so scoring a history is a matrix product.
- **Taphonomy** uses the global survival curve of Surovell et al. (2009), validated by
  Bluhm and Surovell (2019); over 12,000-3,000 BCE survival falls about threefold, so
  ignoring it reads loss as growth.
- **Research intensity** varies far more across space (Çatalhöyük, the Jordan valley)
  than through time within a region, and we have no model of it. Scoring only the
  distribution over time within each block (H3 resolution 2, ~87,000 km², 44 blocks
  with dates in M1) conditions it out exactly: any per-block factor cancels. The price
  is that the operator says nothing about where in space dates are dense; that needs
  site counts or a research-intensity model (open question 25).
- **Clustering:** one well-dated site is one observation per phase. Dates of a site
  within 200 years share one date's weight (Shennan et al. 2013): in M1, 4,398 usable
  dates are 1,403 site-phases, and Çatalhöyük's 216 dates count about 14.
- **Model error:** 1% of each block's mass is spread uniformly, so dates where the
  model has nobody are costly but finite.

Operators run on the calibration host, not in work units, so they use NumPy; results
are validated by the engine's state hash, never by an operator's output. Activity is
recorded from the bindings (population and cultivation sampled every 10 years); people
and cultivators are kept apart so an operator can make sedentary cultivators more
visible without re-running a world. Validation is on synthetic worlds with known
histories, linear and plateau curves: the true history outscores onsets and peaks moved
by 500 years, and research intensity and scale do not change the score.

## ADR-0051 rationale (domestication timing)

The share of non-shattering rachis in a dated assemblage is the most direct record of
domestication, and the quantity the two hypotheses of ADR-0049 disagree about: a
protracted rise over millennia (Purugganan and Fuller 2011) or one within decades to two
centuries of cultivation starting (Hillman and Davies 1990). The model's counterpart is
the domestic share of the crop sown near the site, weighted by the people and effort
sowing it, over the phase's century and its neighbours (phase medians are uncertain by
centuries). Where nobody cultivates nearby, ears come from wild stands.

Assemblages are far more variable than binomial sampling: they tend to be nearly all
wild or nearly all domestic, from mixed contexts, gathered wild stands and changing
identification criteria (Tanno and Willcox 2012). A beta-binomial absorbs this; on the
M1 barley record a regional logistic fits best with concentration 1-2, and the
conclusions below hold from 1 to 30.

Only Table S4 (barley, 26 assemblages, 8,095 fragments) is machine-readable; emmer and
einkorn are images, and transcribing them by hand would make an unverifiable dataset.
On barley alone the record is a protracted domestication: a regional logistic fits best
at 0.0015-0.003 per year, half domestic around 7,500 BCE, and 0.05 per year is worse by
about 50 log-units. Full M1 worlds agree: the default protracted selection
(`selection_coefficient` 0.005) scores -92.8, the rapid one (0.05) -157.5, since rapid
selection makes the crop domestic as soon as cultivation starts (by 11,800 BCE at the
defaults, open question 24), against wild Natufian and PPNA assemblages. The model's
coefficient is not the logistic rate: selection acts in proportion to effort, so the
best worlds sit at 0.005-0.008 and 0.002 never domesticates by 6,000 BCE.

## ADR-0052 rationale (first calibration waves)

Calibration starts with history matching rather than a posterior (docs/05 §5.2): with a
model known to be incomplete, the first question is which parameter values cannot
reproduce the record however generously the model's errors are judged. The answer
rules out regions, and says whether any region survives at all; an empty space would
mean a missing mechanism, not a bad parameter.

Summaries rather than full likelihoods: an implausibility needs an observed value and
variances it can be judged against, and a handful of summaries can be emulated. Both
come from the operators (ADR-0050, ADR-0051), so the model side goes through the same
forward model. Radiocarbon eras keep only the shape over time (scale and research
intensity cancel as in the operator); the block bootstrap is the honest observation
error for 44 blocks occupied and studied very differently. Discrepancies are judgments:
0.25 on an era's log share, 0.1 on an era's domestic share. Seed noise is below 5% of
every tolerance, so the emulators' nuggets mostly absorb the factors they do not see.

Screening (238 M1 worlds, 10 trajectories over the 22 free parameters) found eight
factors with an effect of at least half a tolerance on some summary: selection
coefficient (8.8 tolerances), cultivation abundance weight, growth rate, forager
density, introgression, the wild domestic-type frequency, adoption rate and climate
stress weight. Dispersal, fission, aquatic resources and the stress window move no
summary by more than half a tolerance.

Two waves of 200 worlds each (Latin hypercube, then the space wave 1 did not rule out)
leave 22% of the prior space; 62 of the 400 worlds are consistent with all nine
summaries. Only selection is sharply constrained: 0.001-0.01 per year (prior to 0.5),
median 0.003, so rapid domestication is ruled out by history matching as well as by
the likelihood (ADR-0051). The radiocarbon eras rule out nothing at these tolerances:
the record's early era is about 1.0 log units below the model's median (the gap of open
question 25), within three tolerances of 0.37. The second wave shrank the space only
from 23% to 22% with emulators validated (95-100% of held-out runs within three
standard deviations), so further waves at these summaries would not narrow it: the next
constraint has to come from new observations or a tighter, justified discrepancy, not
more runs.

## ADR-0053 rationale (Neolithic demographic transition)

Skeletal series show a jump in the share of juveniles a few centuries after farming
arrives, read as a rise in fertility with sedentism and cereal weaning foods
(Bocquet-Appel 2002, 2011); across societies, fertility rises with dependence on
cultivation (Sellen and Mace 1997), and settled hunter-gatherers have shorter birth
intervals but higher child mortality (Page et al. 2016). The model encodes the net
effect as the band's intrinsic growth rate, interpolated by its cultivation effort, so
the transition travels with practice and with people. Equal rates nest the
no-transition hypothesis, which stays the default: the literature fixes the sign, not
the size, of the net change.

It was built to test whether the transition explains the radiocarbon rise (open
question 25). It does not. M1 populations sit at carrying capacity from 11,000 BCE
(N/K 0.94-1.01), so the growth rate changes only how fast they follow it. With
foragers at 0.002 and cultivators at 0.02 per year, the first radiocarbon era moves
from -0.62 to -0.84 (record -1.36), and from an equilibrium start (`initial_occupancy`
1) only to -0.67. The model's rise is set by capacity, and capacity has already
doubled by 11,500 BCE through cultivation of wild cereals (open question 24). Single
capacity-side changes (no climate stress, wild yields 0.2, farm density 8) do not
reproduce the record's shape either: the record's date mass rises about eightfold to
6,700-5,100 BCE and then falls, every world rises about 2.6-fold and keeps rising.

## ADR-0054 rationale (stable-climate hypothesis; onset timing)

Open question 24 names three explanations for an onset of cultivation that comes too
early; the stable-climate one can be tested on the forcing. Richerson, Boyd and
Bettinger (2001) argue that late-glacial climates swung too fast for sowing to pay,
whatever the motive, so the hypothesis scales the target rather than adding a driver.
Variability is measured between century boundaries, the climate packs' native step:
between them the climate is interpolated, and 100-year lags from mid-century land on
smoothed values. It changes only at boundaries, so it is computed once per century
(M1 run time +2%; computing it every land update cost +11%).

With century-resolution TraCE climate it does not delay the onset. Suitability changes
about 0.08 per century before 9,000 BCE and 0.03 after, a real contrast, but
suitability saturates at 1 in the core of the cereal belt (37% of suitable cells), where
variability is zero however the climate swings, and cultivation ignites there and
spreads. Weights 5-15 leave the radiocarbon eras unchanged and make late domestication
worse (domestic share after 7,000 BCE 0.20-0.22 against 0.33). Testing the hypothesis
properly needs decadal variability (ice cores, speleothems), the time scale its
authors argue from. The default stays 0.

Is onset timing the lever at all? A counterfactual that forbids cultivation until a
given year and then forks the world with the default parameters says: partly. With an
onset at 10,500 BCE the first two radiocarbon eras match the record (-0.96 and -0.25
against -1.36 and -0.30), but the last era overshoots (0.77 against 0.51), barley after
7,000 BCE is only 6% domestic, and the radiocarbon log-likelihood, which sees every
century, gets worse (-4,036; onsets at 9,700 and 9,000 BCE: -4,053 and -4,063; the
default: -4,028). What the record has and no world has is the fall after 5,100 BCE.

## ADR-0055 rationale (land degradation: boom and bust)

The radiocarbon record of M1 rises to a plateau at 6,600-5,600 BCE and falls by a
quarter by 5,000 BCE; under the taphonomic correction that the record itself prefers
(Surovell 2009: log-likelihood -4,028 against -4,056 without it), activity has to fall by
about 40%. Every world kept rising: nothing in the model lowered capacity once farming
had raised it. A counterfactual that halves farm density over 7,000-5,000 BCE improved
the radiocarbon fit by 24 log-units, and with a later onset and faster selection by 44,
with all nine summaries within 1.2 tolerances: a capacity decline after the boom is what
the record asks for (the boom and bust of Neolithic populations, Shennan et al. 2013;
the Late PPNB crisis, Rollefson and Kohler-Rollefson 1989).

Soil erosion is the process that fits: tilled land erodes one to two orders of
magnitude faster than soil forms (Montgomery 2007), so it is cumulative, slow and hardly
reversible on the time scale of a civilization. Yields are insensitive to soil depth
until roots run short (Lal 1998), so loss of soil is a loss of depth at a steady rate,
and field yields hold until the soil falls below a critical share, then fall linearly.
That gives the shape the record has, a plateau and then a decline, where a proportional
loss would decline from the start. Soil is cell state, kept as a fixed-point share and
updated with integer rounding toward the change, so checkpoints stay exact.

In M1 at the defaults otherwise, erosion of 3e-4 of the soil per year at full effort
improves the radiocarbon log-likelihood from -4,028 to -4,004 with the domestication score
unchanged (-94 against -93); 5e-4 is too much (-4,145). With faster selection (0.01) and
a critical share of 0.3 the radiocarbon fit reaches -3,991, but with the onset as early
as it is, faster selection makes Natufian barley 54% domestic. Decline, onset and
selection have to be calibrated together; the default stays off until they are.

## ADR-0056 rationale (calibration round 2)

Round 1's five radiocarbon eras ruled out nothing (ADR-0052): 1,600-year eras averaged
the peak and the fall away, and judging each era alone ignores that a record's eras
err together (they share sites, blocks and research history). Eras of 800 years resolve
the plateau at 6,700-5,100 BCE and the fall after it with observation errors of 0.12-0.27
(0.50 for the first); the block bootstrap gives their full covariance, so the shape is
also judged as a whole. Both checks must pass.

Screening (288 worlds, 27 free parameters): selection (11.5 tolerances), then soil
erosion (3.2), the abundance weight (2.5) and soil formation (1.7); the emulators see the
12 most influential. Two waves of 300 worlds leave 4.7% of the prior space (round 1:
22%), and 99 of the 600 worlds are consistent with all 14 summaries and the group. The
radiocarbon record now binds: in wave 1 only 45-48% of the worlds pass the two earliest
eras. Selection stays at 0.0017-0.0067 per year (10-90%); erosion above about 8.6e-4 of the
soil per year is ruled out.

History matching does not require erosion (13 consistent worlds have almost none),
but the likelihoods favour it: the consistent worlds that fit both records best all
erode at 1.1-1.6e-4 per year with slow soil formation (~2e-5), scoring -3,980 to -3,990
on radiocarbon and -84 to -91 on domestication, better on both than the default world
(-4,028, -93) and than any world of round 1. Their dated activity rises to a plateau and
falls, but by less than the record (peak era 0.41 against 0.80), and barley after
7,000 BCE is 64% domestic against 80%.

## ADR-0057 rationale (calibrated M1 defaults)

Until now every later experiment started from placeholder defaults that the record
rules out on radiocarbon (-4,028) and on late domestication. A point in the NROY space
is not a posterior, but it is a world the record does not rule out, and the best such
world is the natural reference run until simulation-based inference gives a posterior
(#19). The candidate runs varied all 27 free parameters, the 15 the emulators did not
see as noise; their values say nothing, so a candidate keeps only its 12 emulated values
and the others stay at their defaults. `best` reruns the 12 best consistent runs that
way with three seeds: all 12 stay consistent, and wave 2 run 28 remains the best
(radiocarbon -3,990, domestication -80). Values keep four significant figures: two
cost 14 log-units, more than seed noise, so the fit is sharper than the ranges suggest.

`sedentary_growth_rate` was not emulated (screening 0.41 tolerances). Left at 0.01 it
would make cultivators grow at a third of the calibrated forager rate (0.029), an
inverted demographic transition no source supports, so it follows `growth_rate`, the
neutral value its note promises; the record mildly prefers the slower rate (-3,990
against -3,997 on radiocarbon), which the next round should emulate (open question
27). The adopted world: radiocarbon -3,997, domestication -82, max implausibility 2.1
and group distance 8.1 (cutoff 23.6) for seeds 1-3; 319,000 people at 3,000 BCE
(placeholder defaults: 1.7 million). Most calibrated values are only one point of wide
ranges (e.g. farm density 1.1 within 0.7-6.5 people/km2); selection (0.0021 within
0.0017-0.0067) and erosion (1.4e-4, below 8.6e-4) are constrained.

The golden toy world changes with them (hash b2f4be8f): growth three times faster
fills its land by 9,500 BCE (506,594 people at the end against 190,526). Instructions
per agent-step 1,068.19 -> 1,081.51 (+1.2%); an M1 run takes 27.7 s against 31.3 s
(fewer people).

## ADR-0058 rationale (calibration round 3)

History matching may change which factors the emulators see from wave to wave: each
wave's implausibility uses its own emulators, and a point stays only if every wave
keeps it (Vernon et al. 2010). Round 2's emulators saw 12 factors; the next five by
screening (0.40-0.54 tolerances) were noise to them, among them sedentary growth (open
question 27) and the soil's critical share, which shapes the bust. Wave 3 (300 worlds
from round 2's NROY) leaves 3.6% of the prior space (4.7% before); 127 of its worlds
are consistent with all summaries, and the record leaves all five new factors free.

Likelihoods agree: consistent worlds score alike whether cultivators grow at under 0.7,
0.7-1.4 or over 1.4 times the forager rate (mean radiocarbon -4,031, -4,030, -4,037),
and soil critical share does not correlate with either log-likelihood. The best
wave-3 world (run 162) ties the default world (joint -4,069.1 against -4,068.2), so
the defaults stay. It shows the bust is within reach: its dated activity falls 0.21
from the peak era to the last against the record's 0.26 (the default world: 0.44),
and its barley after 7,000 BCE is 76% domestic against 80%.

## ADR-0059 rationale (site counts; calibration round 4)

Radiocarbon dates are dense where people dug, not only where people lived (open
question 25); surface surveys walk a landscape and record every site they see, so
they measure settlement differently. Palmisano et al. (2021) compiled the surveys of
Southwest Asia with each site phase dated to the cultural period of its pottery.
Periods are long and uneven, so a phase counts in each era by its share of the period
(aoristic sums; Crema 2012), and a world is seen the same way: each century of its
settled activity goes to the periods that contain it, weighted by how many phases
surveyors dated to each per century, and spreads over them. Recovering a known
history to 0.01 on synthetic surveys tests that the two sides match. Only villages
with pottery are visible to survey, so eras start at 6,500 BCE; only the cells holding
some recorded site count, because survey coverage is unknown elsewhere. The archive has
no Southern Levant settlements (DAAHL is not included), so only the northern surveys
count. Counts are shares, so the operator needs no detection
rate.

The surveys contradict the region-wide bust: Upper Mesopotamia's sites fall gently to
5,000 BCE and then multiply into the Late Chalcolithic, and Anatolia's swing between
eras, while every round-2 and round-3 world, the defaults included, falls after
6,000 BCE (the defaults' Upper Mesopotamian log share drops from 1.5 to -2.4; group
distances 20-215 against 18.5). A fourth wave, sampled from round 3's NROY space and
judged on all three records, keeps 2.3% of the prior space; 91 of 300 worlds are
consistent with all of them (148 with radiocarbon and rachis alone). Soil formation
separates them: worlds whose soils recover (formation in the upper part of its prior)
keep the bust local.

`best` reruns the 12 best consistent wave-4 worlds with three seeds; 10 stay
consistent, and run 251 fits best (joint log-likelihood -4,089 against -4,079 for the
round-2 defaults, which the site record rules out). Its 17 emulated values become the
defaults. They change the world: forager growth 0.010 against 0.029, sedentary growth
0.028 (a Neolithic demographic transition, although the record does not identify one;
question 27), forager density 0.29 against 0.043 people/km2, faster soil formation
(1.9e-4) and introgression ten times weaker. The adopted world: radiocarbon -4,000,
domestication -88, max implausibility 2.5, group distances 9.6 (radiocarbon, cutoff
23.6), 8.8 (Anatolia) and 15.0 (Upper Mesopotamia, cutoff 18.5) for seeds 1-3;
2.3 million people at 3,000 BCE. It fits both records by having almost no bust: its
radiocarbon log share keeps rising after 5,900 BCE (0.35 to 0.59, the record 0.73
to 0.54), and its Upper Mesopotamian share still falls where the surveys rise. The
radiocarbon fall may be southern Levantine or research focus rather than region-wide
(question 25).

The golden toy world changes with the defaults (hash 362621da): slower growth leaves
205,140 people at the end against 506,594. Instructions per agent-step 1,081.51 ->
1,136.58 (+5.1%); an M1 run takes 62 s against 43 s at the round-2 defaults (measured
interleaved), seven times as many people.

## ADR-0060 rationale (obsidian exchange)

Obsidian is the one Neolithic good whose origin can be read from the object: each
volcano has its own trace-element signature, so sourced assemblages map who received
whose obsidian. Renfrew, Dixon and Cann (1966, 1968) found a supply zone around each
source and, beyond it, a share of obsidian that falls off steadily with distance, and
Renfrew (1977) showed that down-the-line exchange produces such a fall-off: each group
keeps part of what it receives and passes the rest on. The model needs the pattern,
not the goods: what M1 can test is whether the settlement network its worlds grow
would carry Cappadocian obsidian to the Levant and Eastern Anatolian obsidian to the
Zagros, and where the two mix.

Availability is a share per source group, updated each year as the mean over a cell
and its settled neighbors less `obsidian_loss`; along settled land it falls by an
e-fold every 1.5 cells (25 km) at loss 0.1 to 10.6 cells (180 km) at 0.001, measured
on the res-5 grid. It runs every step rather than every capacity period, because at
small losses the network needs about 1/loss steps to settle. People cannot pass on
what nobody holds, so empty cells hold none and deserts and seas break the chain.
Exchange in the model is a tracer: obsidian has no use value, nobody moves for it,
and nothing reads it, so worlds with and without the pack have identical histories
(tested). Agent-based models of the same network (Ortega et al. 2016) add exchange
between villages by kinship and distance; that, and goods that matter, can replace
this when exchange starts to change what people do.

Only the two groups that supplied most Neolithic obsidian are traced: Cappadocia
(Göllü Dağ, Nenezi Dağ, Hasan Dağ, Erciyes, Acıgöl) and Eastern Anatolia (Bingöl,
Nemrut Dağ, Meydan, Tendürek, Süphan, Muş). Their volcanoes reach the cells within
one res-5 step (about a day's walk). Northeastern Anatolian and Transcaucasian sources
(8% of artefacts in the region, mostly after 6,000 BCE) count as "other". DObsiSS is
CC BY-SA 4.0, so the pack and table carry its attribution and are shared under the
same license. Both groups supply equally; whether one source was preferred (Göllü
Dağ's quality, Bingöl's accessibility) is left to the data.

The golden toy world has no obsidian pack, so its history is unchanged (same
summaries); its hash changes with the checkpoint layout and the new parameter
(3409a5cb). Instructions per agent-step 1,136.58 -> 1,138.42 (+0.2%, no pack); an
M1 run with the pack takes 3-5% longer (51.0-51.7 s against 48.6-50.3 s).

## ADR-0061 rationale (obsidian sourcing; calibration round 5)

What sourced obsidian records is a share: of the artefacts analysed from a context, how
many came from each volcano. The operator compares it with the share the world's people
held, using the same exchange tracer (ADR-0060), over the context's ASPRO period
(each century weighted by its overlap) and over the site's cell and its six neighbors,
so a site on the edge of a settled patch still counts. Only Cappadocian and Eastern
Anatolian artefacts count; contexts with fewer than three of them say little and are
dropped, leaving 129. Assemblages vary more than sampling would make them (a few large
contexts, phases mixed, artefacts chosen for analysis), so counts are beta-binomial
with φ = 2, as for rachis.

Five contexts are never reached in any world: all are on Cyprus (Shillourokambos,
Khirokitia, Kalavasos Tenta, Cap Andreas, Klepini-Troulli), supplied with Cappadocian
obsidian by boat across some 70 km of sea. A chain of settled cells cannot cross
the sea, so those contexts would only add a share of 0.5 for every world. They are
dropped when the operator is built, by asking which contexts a chain of habitable
cells (land minus lake fraction above zero in some century, the engine's rule) joins
to a volcano; dropping them per world instead would let worlds be judged on different
contexts. 124 contexts are judged; sea crossings are open question 28.

History matching needs summaries. Mean shares in four zones of relative distance (how
much nearer the Cappadocian than the eastern sources, with edges at -250, 0 and 250 km),
before and after 6,900 BCE (by the period midpoint), give eight targets: the record's
eastern obsidian moves west after 6,900 BCE (Cappadocian 0-250 km zone 0.91 to 0.54).
Their bootstrap covariance makes one group (dof 8), with discrepancy 0.1. At the
round-4 defaults the record rules out small losses: 0.0003 leaves
every zone mixed (group distance 37.7 against a cutoff of 22.0), and 0.03 supplies the
later Cappadocian side wholly (0.99 against 0.54).

`obsidian_loss` is new to the registry, so wave 5 adds it to the factors of the
earlier waves (`extend_factors`; earlier waves' emulators never see it) as an active
factor. Wave 5 judges all four records: 0.8% of the prior space survives (2.3% after
round 4), 70 of 300 worlds are consistent, and losses below 0.002 are ruled out. `best`
reruns the 12 best with three seeds; all stay consistent, and run 31 fits best
(-4,280.2). The defaults score -4,282.9 on the same seeds and are consistent with
every record (obsidian group 14.6). A gap of 2.7 is small next to the spread of the
candidates' reruns (-4,280 to -4,374), and run 31 is a different world (sedentary growth 0.4
times the forager rate, 3.9 million people at 3,000 BCE against 2.3 million), so it is
not adopted: a round's best world replaces the defaults only when it fits clearly
better than they do on the same seeds. That the two fit alike is the finding:
sedentary growth stays unidentified (question 27).

Obsidian changes no history, so its loss can be calibrated in the default world
itself: `profile` reruns the defaults over ten losses with three seeds and chooses the
consistent one with the best log-likelihood. 0.0028 is best (-4,282.8); 0.002 to
0.0038 lie within 1.4 of it, and 0.0016 or less are inconsistent. Every other
log-likelihood is the same along the profile (a tracer). The zone distance keeps
falling up to 0.0056 (11.5) while the context likelihood falls away beyond 0.0038:
the zones see the boundary's sharpness and the contexts the far tails.

The golden toy world has no obsidian pack, so its history is unchanged (same
summaries); its hash changes with the default (ae727f79). Performance is unchanged:
no code on the step path changed (instructions per agent-step 1,138.42 -> 1,138.40).

## Notes

- H3 cells are not exactly equal-area; mechanisms use per-cell areas, never assume
  uniform cells.
