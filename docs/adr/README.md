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


- H3 cells are not exactly equal-area; mechanisms use per-cell areas, never assume
  uniform cells.
