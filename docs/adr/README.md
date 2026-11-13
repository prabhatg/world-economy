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
| 0057 | Calibrated M1 defaults: the registry defaults of the 12 emulated parameters are the best-fitting consistent world of round 2 (wave 2, run 28; joint radiocarbon and rachis log-likelihood), status `calibrated` with the run as source and the consistent worlds' 10-90% range in the note; parameters the emulators did not see keep their defaults, and `sedentary_growth_rate` follows `growth_rate` (no transition). `python -m oikos_cal best` reruns the top consistent runs with the other parameters at their defaults (three seeds) and records the choice in `calibration/runs/m1/round2/best.json`; a test keeps the default world consistent with the record. Hypothesis tests pin the pre-calibration values, so calibration never moves what they test | Accepted; values superseded by 0059; ranking by 0066 |
| 0058 | Calibration round 3: a third wave refocuses the emulators on the 17 factors whose round-2 screening effect reaches 0.4 tolerances (adding sedentary growth, soil critical share, NPP elasticity, dispersal crowding and wild yield ratio); earlier waves keep their own active sets and still rule out. `wave --previous ... --screening` refocuses. Defaults unchanged | Accepted |
| 0059 | Site-count operator and calibration round 4: survey site phases of Palmisano et al. 2021 (Anatolia, Upper Mesopotamia) counted aoristically in 500-year eras from 6,500 to 3,000 BCE; a world's cultivator activity in each survey's landscape (cells with any recorded site) is dated through the surveys' own period scheme; centered log shares, bootstrap covariance, one group per survey, discrepancy 0.25. A fourth wave judges radiocarbon, rachis and site counts together, and the best consistent world (wave 4, run 251) becomes the registry defaults: 17 parameters `calibrated`, sources in `calibration/runs/m1/round4/best.json` | Accepted |
| 0060 | Down-the-line exchange of obsidian as a tracer: cells hold a fixed-point availability per source group; every step, occupied cells within one res-5 step of a group's volcanoes (an `obsidian` forcing pack from DObsiSS 2.0) have all they want, other occupied cells take (1 - `obsidian_loss`) times the mean of themselves and their occupied neighbors, empty cells hold none; nothing else reads it, so it never changes history; checkpoint layout `OIKOSCK4` (`OIKOSCK3` restores with none held); sourced assemblages (DObsiSS, ASPRO periods) become an observation table | Accepted |
| 0061 | Obsidian sourcing operator and calibration round 5: the share of Cappadocian (against Eastern Anatolian) obsidian in each DObsiSS context with at least three artefacts of the two, against the obsidian in use in the site's cell and its neighbors over its period, beta-binomial (φ = 2); contexts no chain of habitable cells joins to a volcano (Cyprus) are dropped; targets are mean shares in four zones of relative distance to the sources (edges -250, 0, 250 km) before and after 6,900 BCE, bootstrap covariance, one group, discrepancy 0.1. A parameter new to the registry joins a later wave as an active factor. A round's best world replaces the defaults only if it fits clearly better than they do on the same seeds; `profile` calibrates one parameter at the defaults when a new record constrains it and leaves history alone: `obsidian_loss` 0.0028 (`calibration/runs/m1/round5/obsidian_loss.json`) | Accepted |
| 0062 | AI inside the simulation has four roles: the source of simulated people's behavior (behavior models distilled into per-archetype policy networks with calibrated free parameters), actors in the modern world (AI systems as a class of agents and capital; optional LLM notable actors replayed from content-addressed decision packs), analyst of ensembles, and scientist. Inside a run, AI is integer inference or replayed, never a live model call; behavior models used before the twin's start are trained on experiments and surveys, not on narratives of the period, and tested on held-out periods; every AI component is ablatable (docs/07) | Accepted |
| 0063 | Viewer v0: a TypeScript page (`viewer/`; Vite, deck.gl core and layers, h3-js) that plays back an ensemble of plausible worlds. It reads each world's own run outputs (`fields.zarr`, with a minimal uint64 + gzip Zarr reader) and a `catalog.json` written by `python -m oikos_cal ensemble` (the calibrated defaults and the best consistent worlds of a `best` record, run as ordinary work units by the CLI). Run outputs gain per-cell shares over time in parts per 10,000: cultivation, domestication, soil, dry land, obsidian per source group. The map shows one world on the night surface with no basemap; the charts show every world. Ensemble data is built locally, not committed | Accepted |
| 0064 | Cultivation-onset operator and calibration round 6: the share of crops (domesticates, crop progenitors, presumed crops) among the charred grains and seeds of dated site phases in the Origins of agriculture database (Wallace et al. 2018; under the ADS terms it is downloaded by hand and its phase table built in memory, never committed or packed), against a world's people-weighted cultivation effort in the site's cell and its neighbors over the phase. Shares are not diets: the share of a farming world is anchored on the record's Pottery Neolithic phases (mean 0.50 over 18 phases), and an era's mean share over the anchor is the effort the record implies; phases are not scored one by one. Targets are four eras (edges 10,000, 8,800 and 7,600 BCE), joint bootstrap over phases and anchors, one group, discrepancy 0.1. A sixth wave judges all five records, and its best consistent world (wave 6, run 236) becomes the registry defaults: 18 parameters `calibrated`, `obsidian_loss` included, sources in `calibration/runs/m1/round6/best.json` | Accepted |
| 0065 | Social learning of cultivation and calibration round 7: a band closes the gap between its cultivation effort and its target at `adoption_rate × (i + (1 − i) × seen)`, `seen` being the people-weighted mean effort in its cell after the previous step and `i` the new `innovation_share` (1 is individual learning, the model before; checkpoints without it restore with 1). Cultivation then spreads as an innovation does (Bass 1969; Henrich 2001): slowly while few plant, then fast. `wave --activate` lets a later wave emulate a factor earlier waves varied as noise (`adoption_rate`, which now sets when planting takes off). Three waves (7-9) with the five records; the best consistent world (wave 9, run 208) becomes the registry defaults, `adoption_rate` and `innovation_share` now `calibrated`, sources in `calibration/runs/m1/round7/best.json` | Accepted; values superseded by 0066 |
| 0066 | Calibrated defaults are ranked by the summaries, not the item log-likelihoods, and calibration round 8: `best` and `profile` choose the consistent world whose summaries of all five records sit nearest the record under history matching's own error model (`history.log_likelihood`: each group's observation covariance plus discrepancy, the other targets on their own; Kennedy and O'Hagan 2001), recording the item diagnostics beside it; waves ranked together must share their targets, and `ensemble` follows the ranking its best file records. Answers open question 32: the records do not pull apart, the per-date radiocarbon likelihood (no discrepancy) preferred early planting. A tenth wave (NROY 0.10%); `best` over waves 7-10 chooses wave 9, run 180, which plants 0.20, 0.55, 0.61 and 0.48 and becomes the registry defaults (20 parameters; sources in `calibration/runs/m1/round8/best.json`) | Accepted |
| 0067 | The M1 records do not identify the Neolithic demographic transition's growth rates (open question 27): populations follow carrying capacity, which at the defaults grows nearly 20 times slower than a band's growth rate even with sedentary growth at the prior's floor, so the rates only set a lag of a few percent, and the records measure stocks, not vital rates. Over the 534 consistent worlds of waves 7-10 the sedentary/forager ratio keeps its prior distribution, and a profile of `sedentary_growth_rate` at the defaults (0.1 to 4 times the forager rate) is flat. The transition in M1 worlds is in capacity, which cultivation raises; the rates stay as calibrated, marked unidentified. A test needs age structure and the juvenile index of rural cemeteries (#33) | Accepted |
| 0068 | Births and deaths are gross vital events, and the juvenile-index operator: growth keeps the logistic net change but counts births at the band's fertility (crude birth rate, moving with cultivation effort from `fertility` to `sedentary_fertility`, equal by default: no transition, nested) and deaths as the rest, net growth capped at fertility. `juvenile.Operator` reads a cemetery's juvenile index (15p5, Bocquet-Appel 2002) from the fertility and local growth of the people there and then, through a stable population with the average hunter-gatherer Siler mortality of Gurven and Kaplan (2007) at the level that gives that birth rate. Forager fertility is that schedule's stationary birth rate, 1/e0 = 0.0325. The Levant cemetery record (Guerrero, Naji and Bocquet-Appel 2008) is to come (#33) | Accepted |
| 0069 | Checkpoints restore parameters added since them as the model they were written with: every parameter is either stored by the oldest restorable checkpoints (`OIKOSCK2`, `params::IN_FIRST_CHECKPOINTS`) or has a `Restore` rule in `ADDED_SINCE_CHECKPOINTS` (a value: the hypothesis off; the same as another parameter; or inert, when the rules before it make it change nothing), and a test fails for a new parameter without one. Replaces ADR-0065's single entry and closes #31 | Accepted |
| 0070 | Fission by scalar stress, and farming villages: a group's size limit rises with its cultivation effort, in proportion, from foragers' camps (`fission_threshold`, 60) to sedentary villages (`village_fission_size`, 127: Hutterite fissioning reaches probability 0.5 there, Alberti 2014; Kosse 1990; Johnson 1982), and a group above it splits; the daughter founds a settlement in its own cell while the land has room, else is placed by crowding as before. Village size 0 is the fission before (restore rule, ADR-0069). The calibrated defaults fit as well (summary 5.1-5.3 against 4.9-5.2) | Accepted |
| 0071 | Aggregation of farming villages: each year a group joins, with probability `aggregation_rate` (0.05, placeholder) times its cultivation effort, the largest group of its cell at least its own size whose joined size stays below the scalar-stress limit (ADR-0070) of their pooled practice. Its drivers are social and psychological: settled farmers drew together for defence, exchange, marriage partners and ritual (Kuijt 2000) toward the larger community, up to the limit of face-to-face coordination (Johnson 1982; Dunbar 1993). Foragers never join; rate 0 is off (restore rule). Typical farming group 58-68 people (was 31-36) | Accepted |
| 0072 | Survey sites are compared with settlements, not settled people: the recorder adds settlement-years per cell and century (`World::settlements`: each group counted as far as it is settled, its cultivation effort), and the site targets use them. Run files record their site model (`sites`), and waves of different models are not ranked together. The calibrated defaults stay consistent | Accepted |
| 0073 | Calibration round 9, a new history for the model of villages and settlements (ADR-0070 to ADR-0072): a fresh Morris screening of all 33 free parameters, then three waves of 300 M1 worlds over the prior emulating the 24 whose effect reaches one tolerance (NROY 3.9%, 1.8%, 1.35%; 105 consistent runs). A world that leaves a surveyed landscape without settlements for an era has no site summary (NaN) and is ruled out by its own outputs, left out of emulators and elementary effects. The waves keep the calibrated defaults (largest emulated implausibility 2.11-2.44), and no candidate sits nearer the record when rerun (best 2.4 against the defaults' 3.8-4.1), so the defaults stay. The records do not see village size, aggregation, fertility or sedentary growth: consistent runs spread over their priors as all runs do; a record of village sizes is needed (open question 33). A wave's NROY points now default to a seed of its own number (wave 3 repeated 63 of wave 2's points) | Accepted |
| 0074 | Wealth inequality within groups, and the house-size record: each group carries the variance of its households' log wealth, which every generation keeps a share `beta` of and gains chance shocks (Borgerhoff Mulder et al. 2009, eq. 2), updated each period by the equivalent fraction of a 28-year generation. Transmission and shocks move from their values where land is open (foragers: beta 0.19, Gini 0.25) to where farmland is owned (agriculturalists: 0.36, 0.48) with the group's ownership: its cultivation effort times how closed in its land is (its neighbors' cultivation effort, sea and desert counting as closed; Demsetz 1967, Carneiro 1970, Bowles and Choi 2013). Nothing else reads it yet. Checkpoints become `OIKOSCK5`; older ones restore at foragers' inequality. An operator reads the house-size Gini a site's few excavated houses would show (house area elasticity 0.7 to wealth; small-sample Gini, Dixon et al. 1987) against five dated sites of Kohler et al. 2017 (built locally from Europe PMC, never committed). The calibrated defaults fit it (implausibility under 0.7 at every site); it is weak, with two to nineteen houses a site. +8.7% instructions per agent-step (the trait column most of it) | Accepted |
| 0075 | A density-dependent disease burden: the endemic infection settlement adds, spread by crowding, waste and fouled water. Transmission rises with a group's size, so the infection persists only above a critical size and then at prevalence 1 - 1/R0, R0 the size over the critical size (SIS endemic equilibrium, Anderson and May 1991; critical community size, Bartlett 1957); the infected die at `disease_mortality` per year (0.005, placeholder) on top of logistic growth, stochastically rounded on a new stream. The drivers are behavioral and social: mobile foragers move camp away from waste, vermin and the sick, acting on disgust (the behavioral immune system, Schaller and Park 2011; Curtis et al. 2011; Kelly 2013), while villagers stay with fields, stores and each other (Armelagos et al. 1991). So contact moves with cultivation effort from `mobile_contact` (0.4) to 1 and the critical size from 125 to `disease_threshold` (50): foragers' camps add none (their own persistent infections, endemic even in isolated bands, are in their baseline mortality: Black 1975; that camps add none is an assumption), and the large villages aggregation draws people into pay for their size, as the Neolithic skeletons of the southern Levant show more infection than the Natufian (Eshed et al. 2010). Left out: avoidance changing fission, aggregation or migration; hygiene norms; care of the sick; zoonoses from herds; epidemics between villages; immunity and age-specific mortality. It runs after growth and before fission; mortality 0 is off (restore rule). The calibrated defaults fit as well (largest implausibility 2.32-2.35 against 2.31-2.36 without disease, seeds 1-3), while four times the default mortality is ruled out by the domestic-barley record (4.4-4.5): domestication lags, with a domestic share after 7000 BCE of 0.24 against the observed 0.80. The golden summaries are unchanged (foragers only). +2.4% instructions per agent-step (1,347.35 to 1,379.57); native within noise | Accepted |
| 0076 | Learning cultivation from neighbors and by success: what a band learns from (the `seen` of ADR-0065) is the share of cultivation among the people of its cell and, each counting `neighbor_learning` (0.1, placeholder) as much, of its six neighboring cells, which kinship, marriage and visiting link it to (Wobst 1974); each farmer counts farming's payoff (what a unit of effort feeds cultivating against foraging, from `farmed_capacity`) to the power `success_bias`, foraging counting 1; and a band copies the practice it moves toward, farmers' when taking cultivation up and foragers' when giving it up (under ADR-0065 both were paced by farmers seen, so farming that pays nothing, invisible to success-biased learners, was barely given up). The drivers are psychological and social: people copy those who visibly do well (payoff bias, among the best supported learning strategies, McElreath et al. 2008, Laland 2004; success and prestige bias, Boyd and Richerson 1985, Henrich and Gil-White 2001). Both 0 run the learning before, bit for bit (restore rule). Left out: conformist bias beyond frequency, prestige of individuals, teaching, in-group bias across cultural boundaries, and learners comparing their own payoff with their models'. Wider learning evens practice out across a landscape (thin country next to rich stands plants sooner, the rich stands less fast); success bias speeds cultivation where it pays and holds it back where wild-cereal cultivation feeds fewer than foraging. Each cell's weights are tabulated once a step for the world's parameters; a cultivation mechanism with other settings computes its own. With neighbor learning the calibrated defaults fit slightly better (largest implausibility 2.26-2.28 against 2.32-2.34, seeds 1-2). Success bias is 0 by default until calibrated: at 1 it brings domestication and cultivation forward (domestic barley 9000-8000 BCE 0.51 against the record's 0.14; cultivation effort before 10,000 BCE 0.38 against 0.09) to the edge of the record (2.86-2.98), since `adoption_rate` and `innovation_share` were fitted without it; round 10 is to fit the three together. Golden summaries unchanged (the toy world does not farm); instructions per agent-step unchanged (1,379.57); M1 runs 80 s to 81-82 s (+2%; with success bias 1 about +6%, mostly its earlier farming) | Accepted |
| 0077 | Calibration round 10 (a new history, as round 9): the model now carries inequality (ADR-0074), disease (ADR-0075) and learning from neighbors and by success (ADR-0076), so its 43 free parameters are screened afresh and history matched in new waves. A round may leave out a record whose observed values cannot be published with its files (`--leave-out`; the waves and `best` continuing a file leave out what it left out): Kohler et al. 2017 publish their house sizes with no licence, so round 10 leaves the house-size Ginis out, and the wealth parameters, which no other record sees, are varied as noise; the Ginis can join a later round once their licensing is settled. A screening (448 worlds; selection_coefficient, cultivation_variability_weight, adoption_rate, soil_erosion_rate and growth_rate lead; success_bias 1.34 and disease_threshold 1.20 tolerances, neighbor_learning 0.67, disease_mortality 0.36, the wealth parameters 0) and three waves of 300 over the prior, 24 factors emulated, leave 0.70% of the prior and 54 consistent worlds. `best` over the waves reruns the ten nearest with three seeds: four stay consistent, the nearest (wave 2, run 54) with summary log-likelihood -0.3 (seeds 0.5, 0.3, -1.7) against 3.5-3.9 for the calibrated defaults (largest implausibility 2.26-2.28), so the defaults stay; registry and golden unchanged. The records do not identify success bias: over the consistent worlds it spreads across its prior (u 0.11-0.92, median 0.49), as do disease_threshold, adoption_rate and innovation_share, each traded against the others; it stays 0 (open question 36). What the records do narrow: selection_coefficient (u 0.20-0.45), low cultivation_pressure_weight (median u 0.17) and introgression_rate (u 0.07-0.61), high cultivation_variability_weight (u 0.30-0.93). A fourth wave adds the village-size record and the house-size Ginis (ADR-0078; the Ginis may be in committed run files since the maintainer's decision of 2026-11-03, #43), and `best` over it alone, since waves ranked together must be judged on the same records, keeps the defaults too. | Accepted |
| 0078 | Village sizes join the M1 record (#34): `villages.Operator` summarizes each 500-year survey era of Central Anatolia and Upper Mesopotamia (Palmisano et al. 2021, CC BY 4.0) by the mean log area of its sized site phases, counted aoristically, and puts a world's settlements through the dating of the sized phases (phases without a size neither enter the record nor date it): settlement-years times the log of their people (`World::village_sizes`, summed in canonical agent order, the same at any partition count), over settlement-years. Area becomes people at 121 per hectare with a shared log-scale error of 0.45, 50 to 294 as a 95% range, a judgment: the top is the North Yemeni tell village (van Beek 1982, doi:10.2307/1356674) that Kuijt (2000, doi:10.1006/jaar.1999.0352) used with Kramer's (1982) 90 from Iran; the bottom allows for mounds not lived in all at once, as Kuijt and Marciniak (2024, doi:10.1016/j.jaa.2024.101573) find at Çatalhöyük (600-800 people in an average year of its Middle phase, against 3,500-10,000 when every building is counted as lived in at once). The shared error makes the absolute size a loose constraint; each era's discrepancy (0.25) covers density differing between eras and surveys, so change through time and between surveys is judged more tightly. All fourteen eras are one group; a bootstrap over sites gives their sampling covariance, and surveys too sparse for it are refused. At the calibrated defaults settlements hold about 50 people against the record's 126-218 at the middle density: consistent (largest implausibility 2.87-2.88, Central Anatolia 6500-6000 BCE; village group 10.4-10.5 against a cutoff of 31.3) only toward the low densities, and level through time where Upper Mesopotamian sites grow. Round 10's wave 4 (300 worlds, the first judged on village sizes and the house-size Ginis, with `aggregation_rate` and the four wealth parameters emulated) leaves 0.44% of the prior and 27 consistent worlds, whose settlements hold 49-163 people. Size follows how densely farming settles a landscape (rank correlation with selection_coefficient -0.30, farm_density 0.27, fission_threshold 0.26) more than the size at which villages split (village_fission_size 0.19), which few settlements reach; the record narrows no parameter's range on its own. One consistent world grows its Upper Mesopotamian villages by over 0.2 in log size, against the record's 0.36 (median 0). Rerun with three seeds, three of the ten nearest stay consistent; one (run 130: dispersal_probability 0.02, aggregation_rate 0.18, village_fission_size 273) reaches the record's size, 179 people (village group 4.7), at a cost to the radiocarbon and onset records. The nearest (run 269, summary log-likelihood -1.0) does not beat the calibrated defaults (7.9-8.2 on all seven records; largest implausibility 2.87-2.88), so they stay; registry and golden unchanged. Villages as large and as growing as the record's want groups that stay and join rather than disperse, or settlements of several groups (#39). | Accepted |
| 0079 | Radiocarbon by subregion joins the M1 record (#35): the radiocarbon operator's blocks are grouped into four subregions, boxes of block centers (the southern Levant; Anatolia; the northern Levant, Cyprus and Upper Mesopotamia; the Zagros and Iran), and each is summarized by the calendar years by which 10, 25 and 50% of its binned date mass had accrued from 12,000 BCE: years rather than log shares, since early eras can be empty. The bootstrap resamples whole sites within each subregion, so a site's many dates move together, and research intensity still cancels per block. The twelve are judged together, discrepancy 300 years on each (three century bins, a judgment). Northeast Africa is recorded, not judged: CHELSA-TraCE21k has no African Humid Period (under 15 mm a year in the Western Desert throughout), so no world can occupy the desert when its dates say people did (Kuper and Kröpelin 2006; #44). The region-wide eras hid what this shows: worlds consistent with the other records date the first tenth of activity 900-2,200 years too early in Anatolia and 350-1,500 in the Zagros, and the Levant's half 250-1,000 years too late (open question 37), and the calibrated defaults of ADR-0066 fail it (northern Levant 50% at 3.08 sd). Round 10's fifth wave (300 worlds, judged on it) leaves 0.22% of the prior and 14 consistent worlds; the record narrows no parameter on its own. | Accepted |
| 0080 | Onset contrasts by wild-cereal stands join the M1 record (#36): the onset phases are split by the engine's own wild-cereal suitability at them (the same precipitation and temperature ramps, CHELSA-TraCE21k slices interpolated to each century and the registry's fixed cereal parameters; tested equal to `World::suitability`), rich at 0.95 or more, and the record's contrast is the cultivation effort the rich phases imply minus the poor phases', before and after 7,600 BCE, bootstrapped over phases and anchors: 0.21 and 0.62. Judged together, discrepancy 0.1 on each. It was to separate success bias, which copies those who do best where wild stands are richest, from adoption, which copies anyone. It does not: all 41 worlds consistent with the other records fit it, the contrast follows `cultivation_abundance_weight` (Spearman 0.77-0.81) and `selection_coefficient` (-0.51), not success bias (-0.18 to -0.31), and success bias spans its prior among wave 6's consistent worlds (u 0.04-0.94; open question 36). Reported as a record that constrains nothing new (doc 15 §15.2). | Accepted |
| 0081 | Round 10, wave 6 and new calibrated defaults: a sixth wave judges every record, the subregions and contrasts included (300 worlds; NROY 0.16% of the prior, 18 consistent), and `best` over it alone reruns its ten nearest with three seeds, six staying consistent. The defaults of ADR-0066 are not consistent with the subregions (summary log-likelihood -88.7), so the nearest, wave 6 run 216 (-86.7), becomes the registry defaults for 24 of its 29 emulated parameters; five keep their literature values: `fertility` (0.0325; run 216's 0.054 would give foragers more births than sedentary farmers, the reverse of the sign the juvenile index fixes, and no stock record sees it, ADR-0067) and the four wealth parameters (Borgerhoff Mulder et al. 2009; only the house-size Ginis read them, and the record leaves them over their priors). The defaults are consistent with every seed: summary log-likelihood -85.5, -85.6 and -86.5, largest implausibility 2.70 (obsidian after 6,900 BCE, Cappadocian source nearer by 0-250 km); 2.4 million people at 3,000 BCE, villages of about 67 people (geometric mean; 50 before). Most changed: `dispersal_probability` 0.30 to 0.024, `village_fission_size` 127 to 186, `disease_threshold` 50 to 147, `aggregation_rate` 0.05 to 0.13, `success_bias` 0 to 0.23 (one consistent world's value, not identified). Golden change, on purpose: hash 2f07e8aa, the toy world 304,533 people by 9,000 BCE (506,203 before); 1,260.95 wasm instructions per agent-step (1,379.57). | Accepted |
| 0082 | Settlements of several groups (#39, fidelity plan F7): each group carries a settlement label (0 when it lives by itself; otherwise unique within its cell, drawn from the cell's birth counter), and the groups of a cell sharing one form a settlement, the site a survey finds. A group keeps its scalar-stress limit (ADR-0070); a settlement of several groups holds together up to `settlement_integration` times the village limit of its pooled practice, plus `settlement_hierarchy` more groups' worth per unit of its households' variance of log wealth. Its drivers are social and psychological: institutions that bind households across groups without command, communal ritual, shared buildings and descent (Bandy 2004, doi:10.1525/aa.2004.106.2.322; Hodder 2014, doi:10.1017/S0066154614000027; Kuijt 2000), and leaders whose decisions others accept, which unequal wealth allows (vertical hierarchy, Johnson 1982); what limits them is scalar stress between groups, as between households. A group joining (ADR-0071) goes to the largest other settlement of its cell at least as large as its own that has room, merging into the largest of its groups it fits, else living beside them; a farmer's daughter that stays in her cell stays in her settlement while it is below its limit (a mobile forager's camps alone); a settlement at or above its limit sheds its smallest group, with the fission probability, on a new stream (`Settlement`). The endemic infection (ADR-0075) persists in a settlement as in one group of all its people, so towns pay for their size, and surveys count settlements, not groups (`World::settlements`, `World::village_sizes`, ADR-0072, ADR-0078). Integration 1 and no hierarchy are settlements of one group, the model before (restore rule; both placeholders, off until calibrated; integration calibrated to 4 in ADR-0083). Left out: wealth differences between groups (a settlement's inequality is its groups' mean), conflict and defence as a pull, towns drawing on neighboring cells' land, central-place functions and craft, and settlement area that grows slower than population. Why the model's villages are small: farmers settle at about 0.38 of their effort cultivating, where a group's limit is about 92 people, and they reach it; a cell holds five or six such groups, about 400 people. At the calibrated defaults (seed 1), two to six groups' worth gives villages of 112-160 people (66 before) against the record's 126-218, the village group's implausibility 4.5-4.8 against 7.8 and a summary log-likelihood of -84.5 to -85.1 against -85.3, all consistent; hierarchy reads an inequality that hardly changes once the land has filled (0.28), so neither driver makes villages grow through time (open question 38). Checkpoints become `OIKOSCK6` (a u32 label per agent); older ones restore with every group alone. A world-level flag skips the search for settlements until a label is made, so worlds without towns pay only for the column: 1,311.14 wasm instructions per agent-step against 1,260.95 (+4.0%), natively 9.4 ns against 8.9 (+5%). The golden toy world's summaries are unchanged; only its hash moves (9da779c4) | Accepted |
| 0083 | Round 10, wave 7 and settlements calibrated (#39): a seventh wave activates `settlement_integration` and `settlement_hierarchy` beside the 29 factors (300 worlds; NROY 0.10% of the prior, 20 consistent). It ran first on the commit of ADR-0082, where a mobile band's daughter that stayed in her cell founded a settlement with her mother (the toy world, all foragers, formed 65 camps of two to seven bands); foragers' daughters now camp alone, as foragers never join one, and the wave was run again (its results hardly changed). The records constrain neither parameter (consistent worlds span 1.6 to 5.3 and 0.6 to 3.7, 10-90%), and `best` over waves 6 and 7 keeps wave 6 run 216 (rerun -86.6; wave 7's nearest, -89.9, is not consistent with the other parameters at their defaults), so the defaults stay. A profile of integration at the defaults (ADR-0061; 1 to 6, three seeds each) finds every value consistent and more than one group's worth nearer the record: summary log-likelihood -85.7 at 1, -85.1 to -84.3 above, the village group's implausibility 7.8-8.0 at 1 against 4.3-5.9. The nearest, 4 (-84.5, -83.9, -84.5), becomes the default (`calibrated`), though the values from 1.5 to 6 differ by less than the spread between seeds: the record says settlements hold several groups, not how many. Hierarchy stays off (placeholder): inequality hardly changes once the land has filled, so it does what integration does. Village size does not rise steadily with integration (66, 88, 113 and 147 people at 1 to 2.5, 110 at 3.5, 100 at 4, 155 at 5 and 6): as towns grow (the median settled settlement at 5,500 BCE, seed 1: 180 people at 3, 287 at 4, 337 at 5), a cell's land holds one town and the small groups it sheds, which pull down the mean log size the surveys see, and towns above the disease threshold (147) pay for their size in infection (2.29 million people at 3,000 BCE against 2.40). At the new defaults villages are about 100 people in every era of both surveys (the record: 126-218, rising in Upper Mesopotamia; open question 38), every seed consistent, largest implausibility 2.70 (obsidian after 6,900 BCE, as before). Cost: an M1 world runs 20% longer (91 s against 76 s on one worker), as farmers' daughters who stay keep 17% more groups; the time per group does not rise. The toy world, all foragers, forms no towns: summaries unchanged (304,533 people by 9,000 BCE), golden hash 60340b07 (the default), 1,308.71 wasm instructions per agent-step (1,311.14 at ADR-0082) | Accepted |
| 0084 | The African Humid Period in the forcing (#44, fidelity plan F13, step 1): CHELSA-TraCE21k's CCSM3 climate leaves the Eastern Sahara as dry as today throughout (median 0 mm a year at 24-26 N, 1-5 mm at 26-28 N, in every century), where lakes, pollen and leaf waxes record a wetter Sahara from about 11,000 to 5,000 years ago. A `humid_period` pack holds per cell the rain the Humid Period adds at its height, and the engine adds `humid_period_scale` times it, along a time course, wherever it reads precipitation (forager productivity, wild-cereal suitability, climate stress and variability). The anomaly comes from the only quantitative Eastern Saharan reconstructions, Bartlein et al. (2011; NOAA, CC0): two 6 ka grid cells in Sudan where lakes were still full then (+605 mm at 19 N, 27 E, the West Nubian Palaeolake, which needed 500-900 mm, Hoelzmann et al. 2000; +177 mm at 21 N, 29 E), through which an exponential in latitude is extrapolated north over Egypt, where none exists (+28 mm at 24 N, +11 at Dakhla, +4 at 27 N, under 1 north of 29 N), over Africa only (west of the Suez isthmus and the Red Sea, north of 18 N and south of the Mediterranean, beyond which the fit would only extrapolate (the first build's mask let 180 Mediterranean cells near 34 N get 0.1 mm; corrected in ADR-0085, pack id 7a593de3); Arabia's humid phase, open question 39). The western Saharan leaf-wax reconstructions (Tierney et al. 2017) are not used for amounts: they give several hundred mm a year at 19-31 N on the Atlantic margin, far wetter than the Eastern Sahara. The time course rises and falls linearly over `humid_period_transition_years` (500, fixed: abrupt at each site, Shanahan et al. 2015) centered on `humid_period_onset_bp` (11,000, fixed: Tierney et al. 2017) and `humid_period_end_bp` (6,500; prior 8,500 to 5,000, as the end came earlier northward, about 5,000 years ago at 20 N); the amplitude's prior allows four times the fit (the Gilf Kebir's estimated 100 mm at 23 N is twice it); an onset less than a transition before the end is refused. Archaeological chronologies of the desert's occupation (Kuper and Kröpelin 2006) are not used to build the forcing: the radiocarbon record that judges the model is that occupation, so the timing and amplitude are parameters it constrains. Finding: the Humid Period alone hardly moves northeast Africa's dates (10%, 25% and 50% by 10,470, 9,090 and 6,960 BCE at the calibrated defaults, by 10,450, 9,030 and 6,930 at four times the amplitude, against the record's 7,390, 6,520 and 5,390): the model keeps about 4,300 foragers in the Nile valley and 3,000 elsewhere in Africa, mostly on the coast, from 12,000 BCE, and the green desert adds about 600 at four times the amplitude, while the record has almost no dates before 9,000 BCE (5 of 1,101), the desert's from 9,000 to 5,000 BCE and the Nile's from 5,000 BCE, once it was farmed (the model's Nile is not: its farmland follows rain). So the subregion stays recorded, not judged, until the Nile is farmed (step 2) and its early emptiness is explained (open question 40). The Python onset operator reads the climate pack's rain alone; its phases lie in Southwest Asia, where the anomaly is 0. Checkpoints that predate it restore with `humid_period_scale` 0. Golden change, on purpose: four parameters (the toy world has no climate; summaries unchanged, 304,533 people by 9,000 BCE), hash 130a331d | Accepted |
| 0085 | Floodplain farming (#44, fidelity plan F13, step 2): where a river floods its plain each year, fields need warmth but no rain, so domestic crops grow in deserts (the Nile's basins, Butzer 1976; the Mesopotamian alluvium, farmed by short canals from the Ubaid on, Adams 1981, which this treats alike). Each cell's floodplain fields are the share its rivers flood at their highest (the hydrology pack's `floodplain_fraction`, GIEMS-D15 along modern reaches) times what floods grow beyond what rain already does: `floodplain_yield` times the cereal warmth ramp less the cell's wild-cereal suitability, so rain-fed land is not counted twice (nothing where rain grows full crops, unless the yield exceeds 1). No wild cereals grow there: floodplain fields feed only the domestic share of the crop (`farmed_capacity`, farming's payoff), and only bands with domestic seed see it as land to sow (the target's suitability is wild suitability plus floodplain times the band's domestic share, at most 1). Floods renew the silt, so floodplain fields do not erode. `floodplain_yield` 1 (placeholder; prior 0-2): at the calibrated defaults every seed stays consistent and the summary log-likelihood improves a little (-83.8, -84.0, -84.0 against -84.7, -84.1, -84.3), with 2.30 million people at 3,000 BCE (2.29). Finding: it does not farm the Nile, so northeast Africa's dates do not move (10/25/50% by 10,480, 9,100 and 6,970 BCE). By 5,000 BCE farmers on Sinai's edge grow crops 50-60% domestic, but the Nile's cells are full of riverine foragers, so farmers' daughters, who go to the least crowded neighbor, do not move in, and the Nile's foragers have no domestic seed to sow. Tried and rejected: seed exchange, by which a band learning from farmers takes up their crops' domestic share (as the Faiyum's foragers took up the Levant's emmer and barley, Shirai 2010): it stops where no land can be farmed, as only farmers pass seed on, so it does not cross northern Sinai's 170 km of desert, and it spreads domestication through Southwest Asia faster than the rachis record allows (summary log-likelihood -88.0 at 0.1 a period, domestic barley 9,000-8,000 BCE the worst target). What could cross the gap: herders, who carried sheep, goats and grain across Sinai (F5, #37), or migrants who travel farther than a neighboring cell; and the record's near-empty Nile before 9,000 BCE remains unexplained (open question 40). Performance: an M1 world takes 89-91 s on one worker with floodplain fields or without (two runs each). Golden change, on purpose: one parameter (the toy world has no climate; summaries unchanged, 304,533 people by 9,000 BCE), hash cba106a4 | Accepted |
| 0086 | Lesions of infection as a record (#42, fidelity plan F4): Eshed et al. (2010) found lesions of infection (periosteal reactions over a quarter of a tibia or long bone, or systemic infection) on 2 of 200 Natufian (10,500-8,300 BC) and 9 of 205 Neolithic (8,300-5,500 BC) skeletons from 13 sites of the southern Levant. The counts, the sample by site (Table 1) and the sites' locations are transcribed into a manifest (`eshed2010-lesions`) by the maintainer's decision; the article (Wiley) is not stored. A skeleton shows a lesion for an unknown share of the infected, those who lived with it long enough and were preserved (the osteological paradox, Wood et al. 1992), so the record cannot say how common infection was: given that 11 skeletons show lesions, the share of them that are Neolithic is binomial in n_N P_N / (n_N P_N + n_A P_A), from which that share cancels. The engine reports each cell's infected people (`World::infected`: each settlement's people times the endemic prevalence at its size and pooled practice, ADR-0075, whether or not it kills), the recorder their people-years, and the operator each period's prevalence around its sites (cells within one ring, its centuries) weighted by their skeletons. One target, the Neolithic share of the lesioned (observed 0.82, binomial variance 0.0136), judged by itself with discrepancy 0.1: lesions sample local infections as well as the crowd infection the model has, and a few sites a region. A world with no infection in either period is undefined, so ruled out, as no skeleton could show a lesion. At the calibrated defaults (three seeds) 13.8% of the people around the Natufian sites carry the infection and 26% around the Neolithic ones, so 0.66 of the lesioned would be Neolithic (implausibility 1.0); the record's fourfold rise against the model's twofold leans toward foragers who move camp spreading less infection: at `mobile_contact` 0.2, 0.77 (0.33), at 0.9, 0.58 (1.53), every value consistent with every record (summary log-likelihood -83.1 at 0.2 to -85.1 at 0.9; calibration/runs/m1/round11/profile-mobile-contact.json, round 11's first file, as the records judged changed). It does not see disease mortality, which kills the infected whether or not they show it. Golden unchanged: an observer and a record | Accepted |
| 0087 | Herding of sheep and goats (#37, fidelity plan F5, steps 2-3): bands manage caprines as they cultivate, a second practice within one effort budget. Where: a `caprines` pack gives each cell's upland share, where wild goat (rugged slopes) and mouflon (hills, foothills, plateaus) lived (Uerpmann 1987; in the southern Levant too, Yeomans et al. 2017): the larger of ramps of mean elevation (300-900 m) and slope (1-4 degrees), 0 in Africa, whose wild caprines were never domesticated; each year the wild herds' range is it times a moisture ramp (`pasture_precip_min` 150 to `pasture_precip_full` 300 mm, judgments), and pasture for domestic herds the moisture ramp alone. How: each group carries `herding` (effort) and `herd_domestication` (the managed herd's domestic share; OIKOSCK7). Its target is `herding_weight` times the land it can herd on: the range, and with domestic animals the pasture beyond it in proportion to their share (as floodplains for domestic crops, ADR-0085); it is learned from the share of herding among the people of its cell and, by `neighbor_learning`, around it, at the same rate and innovation share as cultivation. Cultivation and herding move from one snapshot and share the budget, proportionally less of each where together they would exceed it. Herds become domestic by selection at `herd_selection` times effort, logistic from the wild animals' domestic-type share only where they range (`herd_wild_frequency` times the range), and gene flow from wild herds pulls them back only there (`herd_introgression`). Herds feed people: `herd_density` per km2 of pasture under full domestic herding, wild herds `wild_herd_ratio` of it on their range only; foraging takes the effort left. Its drivers are social and psychological: herds are learned from neighbors as crops are; risk buffering, herds as inheritable wealth (ADR-0074) and prestige are left out of the target for now, as are wealth transmission through herds, cattle and pigs, secondary products, zoonoses and transhumance. Parameters: `herding_weight` 0.12 (cultivation's abundance weight), `herd_density` 1.0, `wild_herd_ratio` 0.5, `herd_selection` 0.05 (herds mostly domestic within a millennium of management at a tenth of effort, as the size reduction follows management by about that, Zeder 2008; the crop's 0.009 would take five); all placeholders until a round calibrates them against sheep body size (step 4). At the calibrated defaults (seed 1) herding starts in the uplands about 10,000 BCE and takes a tenth of effort by 8,000 BCE, and herds are 72% domestic in Anatolia by 8,000 BCE and 95% by 7,000 BCE, as sheep shrink in the record by 7,400 BCE; nobody herds in Egypt, as herders do not cross Sinai (open question 40). Every seed stays consistent with every record (summary log-likelihood -83.7, -83.6, -83.8 against -83.3, -83.6, -83.6 without), with 2.53 million people at 3,000 BCE (2.30). Performance: the toy world, which never cultivates, takes 1,393.24 wasm instructions per agent-step (1,393.16 with the columns alone, 1,308.73 before them); an M1 world, timed in pairs under load, 1-2% more with herding. Golden change, on purpose: eight parameters (summaries unchanged), hash 18f7b037 | Accepted |
| 0088 | Sheep size as a record (#37, fidelity plan F5, step 4): domestic sheep are smaller than wild ones, so a faunal assemblage's mean log size index (LSI) falls as more of its sheep bones come from domestic animals. The record: 42 dated assemblages of Anatolia and its neighbours (Arbuckle et al. 2014, CC BY; `fauna` table), in six eras split at 8,500, 7,500, 7,000, 6,500 and 6,000 BCE, each the bone-weighted mean LSI (+0.063, +0.031, -0.004, -0.032, -0.033, -0.035), judged together. The forward model: around each assemblage (cells within one ring, its centuries) the domestic share q of the caprine bones its people discarded, from herds in proportion to herding effort and from wild sheep and goats hunted on their range at `HUNTING` (0.1) bones per unit of foraging effort for one of herding (a judgment: foraging goes to plants and many prey, while a herd yields animals for slaughter with far less effort; at 1, foraging's half of everyone's effort keeps the bones wild whatever the herds), so q is 0 before anyone herds; expected LSI (1 - q) mu_wild + q mu_domestic. The endpoints are judgments from the assemblages: wild +0.045 (the six before management) and domestic -0.045 (the 22 from 6,500 BCE), with uncertainties 0.02 and 0.015 shared by all eras, carried in the observation covariance at the observed eras' q; sampling variance from each assemblage's sd and count (0.035 where none is given); discrepancy 0.015 on each era (sex ratios, regional sizes, bones measured, hunting choices). The recorder keeps herders, their domestic animals and foragers on the range in people-years; the engine's range is a new binding (`caprine_range`). At the calibrated defaults (seed 1) the eras come out +0.045, -0.003, -0.016, -0.017, -0.017, -0.017: consistent (the group's implausibility 4.5 on 6 degrees of freedom), sheep a little too small by 8,500-7,500 BCE and not small enough later; worlds without herding (27.2) or with slow herd selection (0.01: 25.5) are ruled out, so the record sees whether and how fast herds became domestic. It does not see Capra (its LSI is kept, diagnostic), juveniles (they need a culling model) or caprine abundance (confounded with hunting) | Accepted |
| 0089 | Herders' wealth and movement (#45, fidelity plan F5): two parts of herding deferred from #37 so round 11 could calibrate a fixed model. Herds as wealth: animals are owned, inherited and given as bridewealth on open land too, so inequality (ADR-0074) gets a third economy beside foraging and owned land, mixed by a group's effort (effort left to foraging, effort on owned land, effort herding), per generation; pastoralists' importance-weighted transmission 0.43 +- 0.06 and Gini 0.42 +- 0.05 (Borgerhoff Mulder et al. 2009, Table 2) give `pastoral_wealth_transmission` 0.43 and `pastoral_wealth_shock` 0.4992 (lognormal wealth), with priors of three standard errors; herding effort, not the herds' domestic share, counts, as managed herds are owned whatever their size. The per-period coefficients are tabulated over transmission instead of ownership (1,024 steps; a generation errs by under 1e-7). Checkpoints from before restore both as the forager values, the model they were written with. Migrants' feasibility: dispersing groups and daughters leaving a full cell judged a neighbor by its residents' capacity, so pasture that feeds herders but not foragers turned herders away; now each neighbor is judged by what its land would feed with the group there, their practice pooled with the residents', people-weighted, as its capacity will be (`Sense::capacity_joined`; the group's own cell keeps its capacity). Herders move onto pasture where foragers find nothing, foragers do not, and nobody moves where neither forage nor pasture feeds them. Without farming the capacities are the residents', so the toy world is unchanged (summaries equal, hash c1a66fd6 for the two parameters; 1,404.49 wasm instructions per agent-step against 1,393.24). At the calibrated defaults (seeds 1-3) every seed stays consistent, summary log-likelihood -66.4, -66.2, -66.3 against -67.7, -67.7, -67.8 before (about 1.1 from migration, 0.3 from herd wealth), 2.58 million people at 3,000 BCE (2.53); the house Ginis rise a little (0.20-0.22 against 0.20-0.21; the record 0.20-0.36). Validation, not a design target: herders still do not cross Sinai to the Nile (#44): they reach the Negev, but Sinai's median rain is about 45 mm and a tenth of its cells reach `pasture_precip_min` (150 mm), so it offers no grazing (open question 40). A calibration round with the two parameters follows round 11 With round 11's calibrated herding (ADR-0090, herd_selection 0.035) merged in, the gain holds: summary log-likelihood -65.9, -65.9, -65.7 against -67.0, -66.8, -66.9 (herd wealth 0.3 of it), every seed consistent, house Ginis 0.20-0.22; golden hash 4b74c9a7. | Accepted |
| 0090 | Calibration round 11 and herding calibrated (#37, fidelity plan F5, step 5): a new history after the lesions (ADR-0086), floodplain farming (ADR-0085), the African Humid Period (ADR-0084) and herding with its sheep-size record (ADR-0087, ADR-0088), judged on every record, the house-size Ginis included. A screening of the 52 free parameters (538 worlds; selection_coefficient 12.3 tolerances, soil_erosion_rate 4.9, cultivation_abundance_weight 4.5, soil_formation_rate 4.0, adoption_rate 3.6, then herding_weight 3.4 and herd_selection 3.3, both through the sheep-size eras) and six waves of 300, 24 factors emulated (`--threshold 1.0 --max-active 24`), leave 1.36%, 0.38%, 0.17%, 0.08%, 0.06% and 0.04% of the prior, with 0, 3, 7, 9, 11 and 15 consistent worlds. `best` over waves 2-6 reruns the ten nearest with three seeds and only their emulated factors set: one stays consistent (wave 5, run 160; summary log-likelihood -80.5) and the nearest (wave 5, run 141; -71.7) does not, against -67.7, -67.7 and -67.8 for the calibrated defaults, so the defaults stay. The consistent worlds put herd_selection at 0.0027-0.039 and herding_weight at 0.16-0.82 (10-90%), away from their placeholders (0.05 and 0.12), but by trading them against the parameters they move with; at the calibrated defaults, profiles (ADR-0061, ADR-0083; three seeds each) find herd_selection consistent from 0.02 to 0.1 (not at 0.01 and below: sheep stay wild-sized too long) and nearest the record at 0.035 (-66.9, seeds -67.0 to -66.8, against -67.7 at 0.05 and -68.6 at 0.1), and herding_weight consistent only from 0.1 to 0.2 (not at 0.08 and below or 0.35 and above), nearest at 0.1 alone (-67.0). The two trade off: together, 0.035 with 0.12 gives -67.0, -66.8 and -66.9, with 0.1 -69.5 to -69.4, and 0.04 with 0.1 -67.7 to -68.1, every seed consistent. herd_selection becomes 0.035 and herding_weight stays 0.12, both `calibrated` with the profiles as source (`calibration/runs/m1/round11/`); golden hash a2d44aba, its summaries unchanged (the toy world never herds). herd_density and wild_herd_ratio, below one tolerance in the screening, stay placeholders: the record sees when herds became domestic, not how many people they fed. Herd wealth and herders' movement (#45) come after this round, so their parameters join a later one | Accepted |
| 0091 | Herd wealth not identified (#45, fidelity plan F5): profiles of `pastoral_wealth_transmission` (0.25 to 0.61) and `pastoral_wealth_shock` (0.33 to 0.68) at the merged calibrated defaults (ADR-0061, ADR-0083; three seeds each, `calibration/runs/m1/round12/`) find every value consistent with every record and the summaries flat: log-likelihood -65.8 across transmission, -66.0 to -65.7 across the shock, less than the spread between seeds. Herding is a small share of effort in M1 (about a tenth in the Negev by 4,000 BCE) and the five dated houses are farmers', so herd wealth moves their Ginis by under 0.01 (Çatalhöyük 0.214 to 0.229 across the shock's prior). The literature values (Borgerhoff Mulder et al. 2009) stay, status `literature`; the record that would identify them is the wealth of herders' households, which M1 does not have. The migration rule of ADR-0089 has no parameter. #45 is done; herders crossing Sinai stays with #44 (open question 40) | Accepted |
| 0092 | Herders and the Sinai (#44, fidelity plan F13, open question 40): herders stopped at Sinai for two reasons found by probing M1. (1) Herding's target was `herding_weight` (0.12) times the land a band can herd on, so nobody spent more than an eighth of effort on flocks and no specialized herders existed to cross a desert: the Negev's herders at 4,000 BCE herded 10-12% and farmed 20-38%. Lowering the grazing threshold alone (to 20 mm) moved nothing; with the cap lifted (weight 1) and grazing from 40 mm herders crossed to the Nile by 7,000 BCE, but the Negev then herded 73%. (2) The route is Sinai's north coast, 117-162 mm, below the 150 mm where open grazing begins. Two additions, both nested: need drives herding as it drives cultivation, `herd_pressure_weight` times the population pressure of people beyond what foraging feeds (the same measure), on any land the band can herd, in proportion to its herd's domestic share, as wild animals cannot be worked harder to feed more people and specialized herding came with docile stock (Zeder 2008); and wadi beds, `wadi_share` (0.03, a judgment) of dry land, graze with their rain plus `runoff_coefficient` (0.1, the average used for the Negev's ancient terraced wadi fields, Bruins et al. 2019) of the rain on the rest (`environment::pasture_share`). Need without the domestic share broke the sheep-size record (sheep domestic-sized by 12,000-8,500 BCE). With it, at the calibrated defaults (seed 1), need keeps every record consistent but fits worse: summary log-likelihood -65.6 off, -67.9 at 0.4, -74.9 at 0.8; with herd_selection 0.02 the best is -69.0 and at 0.01 none is consistent, since more herding breeds sheep domestic sooner than their size falls. Herders cross to the Nile only with need 0.8 and wadis at the top of their prior (0.1, 0.3), from about 4,000 BCE, against goats in Egypt by about 5,900 BCE (via the herder-gatherers of the Negev and Sinai) and the oldest African caprines at Sodmein Cave, 6,200-5,000 BCE (Vermeersch et al. 2015). So need is off by default (placeholder) and wadis on: over three seeds -65.6, -65.8, -65.7 against -65.9, -65.9, -65.7 without them, every seed consistent; a round varying need with the herding parameters can test it again. Crossing does not move northeast Africa's dates (10/25/50% by about 10,480, 9,100 and 6,970 BCE in every setting, against 7,390, 6,520 and 5,390 in the record): the model's Nile holds some 3,200 foragers from 12,000 BCE whose sites the record does not have, so the subregion stays recorded, not judged, until the radiocarbon operator says why (open question 40). Older checkpoints restore need 0 and no wadis; golden hash d5d631d3, summaries unchanged (the toy world never herds) | Accepted |
| 0093 | The Humid Period's rain over Egypt from the Western Desert's own estimates (#44, fidelity plan F13, open question 40): northeast Africa's early dates come mostly from three Western Desert research blocks (Dakhla and Kharga, Gilf Kebir and Abu Ballas, Farafra and Djara) that hold 69 of the subregion's about 173 dates, from about 9,000 to 5,000 BCE, and where the model had nobody at any time: an empty block is scored as a flat spread over 11,500-3,500 BCE. The anomaly through Bartlein et al.'s two Sudan cells alone (ADR-0084) fell off by e every 1.6 degrees, +11 mm a year at Dakhla and +4 mm at 27 N, while estimates for the desert from its sites' charcoal and fauna give tens of millimetres: at most 50 mm in the early and 100 mm in the mid-Holocene from the Gilf Kebir to Djara (Neumann 1989, as cited by Bubenzer and Riemer 2007, doi:10.1002/gea.20176), 50-100 mm from charcoal and 100-200 mm from fauna at Nabta Playa (as cited by Wendorf and Schild 1998, doi:10.1006/jaar.1998.0319). The fit now includes them, each as the geometric mean of its range (`humid.SITES`: 100 mm at 22.5 N, 71 mm at 25 N), and falls off by e every 2.8 degrees (+79 mm at 24 N, +46 at Dakhla, +27 at 27 N, +9 at Cairo); pack id 562a89f2. Left out: Kroepelin's (1987) 100-150 mm for the Gilf Kebir (only secondary citations seen), about 30 mm from sediments (Kroepelin 1993, place unclear), Kuper and Kroepelin's (2006) isohyets (over 50 mm over the whole desert, over 150 mm south of about 23 N, 8,500-7,000 BCE), which draw on the occupation the radiocarbon record judges, and a teleconnection estimate of +20 mm at 20 N (Molnar and Rajagopalan 2020) that its authors say falls short of the field evidence; the estimates are amounts from what sites hold, not the sites' dates, so the forcing still does not use the record that judges it. Tried and rejected: burying floodplain activity in the radiocarbon operator (by each cell's floodplain fraction, deeper with age, e-folding 500-2,000 years) moves the subregion's dates by under 50 years and lowers the region-wide radiocarbon log-likelihood by 3-4; and raising `humid_period_scale` on the old fit, which settles the desert only near 10, about 1,800 mm a year in northern Sudan. A profile of `humid_period_scale` at the calibrated defaults (three seeds each; `calibration/runs/m1/round12/profile-humid-period-scale.json`), every world consistent: summary log-likelihood -65.5 to -65.9 at 0, -65.6 to -65.7 at 0.5, -65.4 to -65.5 at 1, -65.5 to -66.0 at 2, -64.6 to -64.7 at 4; northeast Africa's 10/25/50% by about 10,490, 9,120 and 6,980 BCE at 0, 10,350, 8,800 and 6,640 at 1, 9,180, 7,070 and 5,410 at 2, and 8,510, 6,960 and 5,820 at 4, against the record's 7,390, 6,520 and 5,390. The blocks show why (seed 1): at 1 the desert stays empty; at 2 the Dakhla and Gilf Kebir blocks hold a few hundred people only from 6,000 to 4,000 BCE, so their dates fall late for the wrong reason; at 4 the Dakhla block is settled from about 8,000 BCE and the Gilf Kebir's from 7,000 BCE, both peaking at 6,000-5,000 BCE (their dates rise from 8,500 and peak at 6,500-5,500) and emptying after the Humid Period. `humid_period_scale` becomes 4, the top of its prior (still a placeholder): 315 mm at 24 N, 186 at Dakhla, 110 at Farafra, above the estimates, as the model spreads rain evenly where the desert's foragers lived by playas and pools that gathered it (Bubenzer and Riemer 2007); the colonists came from the south (Kuper and Kroepelin 2006), which the region, starting at 24 N, lacks. A best at the prior's edge: a calibration round should test a wider prior or a playa term. The subregion stays recorded, not judged: its 10% point is about 1,100 years early (implausibility about 3.2), from the Delta and Cairo block, the largest (36 dates), where the model keeps 2,200-3,500 foragers from 12,000 BCE and the record has nothing before 5,500 BCE (open question 40). Golden change, on purpose: one default (the toy world has no climate; summaries unchanged), hash 25a4370b | Accepted |
| 0094 | The Humid Period's scale over a wider prior (#44): the maintainer widened `humid_period_scale`'s prior from 0-4 to 0-10, the parameter's limit, as its best (ADR-0093) lay at the old edge. A profile at the calibrated defaults (three seeds each, `calibration/runs/m1/round12/profile-humid-period-scale-wide.json`), every world consistent: summary log-likelihood -64.6 to -64.7 at 4, -64.5 to -64.7 at 5, -64.5 to -64.6 at 6, -64.4 to -64.6 at 8, -64.4 to -64.7 at 10, flat; northeast Africa's 10/25/50% by about 8,510, 6,960 and 5,820 BCE at 4, 8,180, 6,820 and 5,810 at 8, and 8,000, 6,730 and 5,740 at 10, against the record's 7,390, 6,520 and 5,390, the 10% point's implausibility 3.2, 2.3 and 1.8. The default stays 4: the summaries cannot tell 4 from 10, 4 is the nearest the Western Desert's estimates (10 puts 790 mm at 24 N and 460 at Dakhla, against 30-200 mm), and higher values bring the subregion's first dates nearer partly by making up with desert dates for the Delta's foragers, whose cause is open (open question 40). The subregion stays recorded, not judged, until the Delta is understood; then a round can take the scale over the wider prior with the subregion judged. Golden hash unchanged (a prior, not a default) | Accepted |
| 0095 | Dates out of sight, and northeast Africa judged (#44, fidelity plan F13, open question 40): the radiocarbon operator multiplies activity by its visibility before scoring it (`radiocarbon.Operator(visibility=)`). (1) Drowned land: the share of a cell's land in a century that is still land at the coastline pack's last slice (3,000 BCE, the sea within a few metres of today's; today's coast, from another elevation model, differs along every shore by resolution alone, and fit the same) leaves dates; submerged sites are missing from terrestrial compilations. (2) Buried lowlands: on cells flatter than 0.2 degrees and lower than 100 m that a river floods (GIEMS-D15 floodplain over 0.02) or runs within two cells of (the Nile delta, the Mesopotamian alluvium, the Kura lowland; not Qattara or Siwa), nothing before 7,500 BCE is in sight, all after 5,500 BCE, linear between: deltas began to build as the sea's rise slowed, 8,500-6,500 radiocarbon years ago (Stanley and Warne 1994), and bury their early sites under fluvial sediment and a shallow water table (Stanley and Toscano 2009); southern Mesopotamia was habitable from the eleventh millennium BCE yet has no site older than the Ubaid (Altaweel et al. 2019). Why: the Delta and Cairo block, whose record begins about 5,500 BCE, held 2,200-3,500 of the model's foragers from 12,000 BCE, three quarters of them north of 30.5 N, in the delta, and at first 29% of them on land drowned since. At the calibrated defaults (three seeds), hiding drowned land alone takes northeast Africa's 10% implausibility from 3.2 to 2.8 and the region's radiocarbon log-likelihood up by about 6; with buried lowlands, to 1.9-2.0 (25% and 50%: 0.5-0.9) and up by about 14; the judged subregions' worst stays 2.4, and the Zagros and Iran's 10% point, which holds the Mesopotamian alluvium and was not the target, improves (0.46 to 0.15). A gradual burial (an exponential in age) was tried first and moves nothing: it hides the plain at every age alike. The ramp's window is the delta-initiation window, not fitted: 6,000 BCE as a step, 8,000-6,000, 7,000-5,000 and 8,500-5,500 give 10% implausibilities of 1.9-2.0 and log-likelihoods within 2. The visibility was chosen after seeing the misfit, so it is a hypothesis about the record that the improvement supports but does not prove; a sediment map would replace the terrain proxy, and lakes the model counts that are young or man-made (the Bitter Lakes, filled by the Suez canal; the Delta's lagoons) still give coastal foragers from 12,000 BCE. Northeast Africa becomes the fifth judged subregion (three targets more; the unjudged diagnostics are gone). A profile of `humid_period_scale` with it judged (`calibration/runs/m1/round13/profile-humid-period-scale.json`, three seeds each): 0 and 2 are now inconsistent (northeast Africa's 10% implausibility 8.5 and 3.8), 4 to 10 consistent, summaries -87.5 at 4 to -85.9 at 10, the subregion's 10% implausibility 1.9-2.0 at 4, 1.6-1.7 at 6, 1.2-1.3 at 8 and 0.8 at 10. The default stays 4, nearest the sites' rain estimates (10 means about 790 mm a year at 24 N, against 30-200 mm); a round varies it. Calibration only: golden unchanged. | Accepted |
| 0096 | The calibration record as a work-unit output (#46, toward Coordinator v0, #21): a work unit's `activity` block makes the engine write `activity.zarr`, float64 arrays [century, cell] of everything `oikos_cal.record.activity` sampled through the bindings (people-years and their weightings by cultivation, domestic crop, inequality, herding, domestic herds and hunting; settlement-years, their log size; infected people-years; obsidian in use per source), every `every_years` (dividing a century and a whole number of steps), with each cell's H3 index; the Zarr writer gains float64. The engine follows the Python arithmetic operation by operation, so a 1,000-year M1 world with non-default parameters gives the same arrays bit for bit through the CLI as through the bindings (`calibration/tests/test_m1_workunit.py`); recording only reads the world (state hash unchanged, bytes identical for any partition count). `m1.work_unit(params, seed, out)` builds the work unit and `record.read` loads the record for `Problem.outputs`, so a calibration world runs wherever the CLI runs and the operators score it on the calibration host. Why the full record rather than summaries: the operators live in Python on the host (ADR-0050), and the record is what they read; a full M1 world's record is 94 MB in 3,258 files (161 s of CPU, 55 MB of memory, on a loaded host), fine for cloud spot workers, the first client class, too large for volunteers, for whom it must be reduced to the blocks, sites and zones the operators read. Golden unchanged. | Accepted |
| 0097 | Coordinator v0 (#21, docs/11 §11.3-11.4): a new native-only crate, `oikos-coord`. The dispatcher and validator are one state machine without I/O (`Coordinator`): experiments' units, leases with deadlines (an expired lease returns its unit to the pool; its late result still counts), and per-host records of agreements and times outvoted. Validation is a quorum on the state hash, exact because the engine is bit-for-bit deterministic: two distinct hosts agreeing, or one trusted host alone (adaptive replication: trusted after `trusted_after` agreements, default 10, and never outvoted), with one unit in `spot_check_one_in` (default 10, by an FNV-1a hash of its id, stable across versions) replicated anyway; a disagreement asks for one more host, up to `max_replicas` (default 5), then the unit fails. Whether a result may stand alone is fixed when it arrives, by its host's trust then (so a result reported before its host earned trust still waits for a second host), and a later result disagreeing with one accepted alone reopens the unit for a quorum to decide. A host is never given a unit twice (not after its lease expired either), and a host whose run fails reports the error; a unit that fails on `max_replicas` hosts is failed. Hashes compare case-insensitively. Every decision is a function of the calls and the times passed in, so the state snapshots to JSON and restores exactly. `oikos-coord serve` wraps it in HTTP/JSON (tiny_http) for one trusted network, with no accounts or signatures; each submitted unit must parse as a work unit, start from scratch (v0 ships no checkpoints) and keep every output path relative and inside its run directory; experiments' forcing packs are checked against their BLAKE3 ids on submission and copied into the coordinator's own store, then served by id. `oikos-coord work` (ureq) is the headless host: it fetches packs into a cache and verifies them, checks the unit's output paths itself rather than trust its coordinator, runs the `oikos` CLI as a child process in a fresh directory (docs/13: the engine stays in its own process), and returns the directory as a tar archive with the result. Why our own small service rather than BOINC: the units, the validation (one hash) and the clients are ours, and a state machine this size is easier to test exactly than a BOINC deployment is to configure; Postgres, object storage and signed URLs replace the JSON snapshot and local archives when it outgrows one machine. Why native only: it serves and fetches over TCP, which wasm32-wasip1 lacks, so it is not a default member and the gate runs its clippy and tests by name, plus two workers on the golden unit through the real CLI, requiring the golden hash. `oikos_cal` submits a wave through it (`--coordinator URL` on `screen`, `wave`, `best` and `profile`): jobs become one experiment of M1 work units writing their record (ADR-0096), and each accepted archive is fetched, scored by the same operators and removed (`calibration/tests/test_coord.py`: a world computed by two hosts gives the bindings' record bit for bit). Golden unchanged. | Accepted |
| 0098 | Lakes a modern work filled are not lakes (#44, fidelity plan F13, open question 40): the hydrology pack leaves out of `lake_fraction` the lakes HydroLAKES counts as natural that a modern work filled, named by where they pour (`MODERN_LAKES`): the Bitter Lakes and Timsah, a dry salt basin and a marsh until the Suez Canal filled them in 1869 (Por 1978), with the ponds the canal feeds; and the Toshka lakes, Lake Nasser's overflow from 1998 (Chipman and Lillesand 2007), south of the M1 region. Their surface is not floodplain either: before the work it was dry. The Delta's lagoons stay: natural, if Holocene. In the M1 region 7 cells change, 0.75 cells' worth of lake in all; at the calibrated defaults (three seeds) northeast Africa's radiocarbon dates move by under 20 years and the log-likelihoods by under 2, so the change is made for honesty, not fit. Engine golden unchanged; the hydrology pack's id changes (afee4872 to e1ea1410). The change waited for round 13 to end (ADR-0104), whose waves ran on the old pack; round 14 judges with it. | Accepted |
| 0099 | The reduced calibration record (#47, toward volunteer uploads, docs/11 §11.2): every M1 operator reads the record as per-century sums over cell sets (the cells near a site, a survey region, a radiocarbon block weighted by visibility per cell and century, ADR-0095), so each operator lists its sets (`cells()`) and `m1.Problem.zones` gathers them: 191 zones, 44 weighted. A zone file (`oikos_cal.zones`, format `oikos-zones/1`: cell indices, the weighted zones, and per weighted cell a row of weights written as IEEE 754 bits, so both sides read the same doubles; BLAKE3 id; 2.2 MB, 0.4 MB deflated) is named by a work unit's `activity.zones`, checked against the world's cells and the record's centuries, and travels like a forcing pack (the coordinator copies and serves it by id; workers cache it). The engine then writes [century, zone] sums, each zone's cells added in ascending order as `Zones.sums` adds them, so the engine's sums equal Python's bit for bit; `zones.Zoned` answers the operators' reads (`a[cells]`, `a[cells, centuries]`, `a[np.ix_(...)]`, `a[:, k]`) from the sums, addressed by the same cell sets, so the operators score a reduced record unchanged. An M1 world's record: 1.7 MB in 1,094 files instead of 91 MB in 3,258; its outputs agree with the full record's to 5e-16 relative (the operators' own sums run in another order), so it is opt-in (`--reduced`, for volunteers) and the full record stays the default where it is affordable. Both sides refuse the same malformed files (cells unsorted or outside the world, a zone listed twice, weighted zones not last, weight rows of another length); `Zoned` raises on any read or arithmetic its sums cannot answer (paired index arrays, masks of another size, a constant added per cell, products of fields), and radiocarbon refuses zones weighted by another visibility than its own. File references must be `{id, path}` objects, so hosts always rewrite their paths. Float32 sums or fewer chunks would bring it under 1 MB if needed. Engine golden unchanged. |
| 0100 | Browser client v0 (#22, docs/11 §11.5): a browser tab is a host like `oikos-coord work`, running the engine the gate checks, the `oikos` CLI built for wasm32-wasip1, under a JavaScript WASI shim (`@bjorn3/browser_wasi_shim`) in a Web Worker, with the run directory and the packs (read-only, at `/packs`) in memory, rather than a second engine build with wasm-bindgen; so browser results are bit-identical to native ones and validate in the same quorum. The page (`viewer/volunteer.html`) is served by the coordinator itself (`serve --client viewer/dist`, at `/client/`), so it reaches the API from the same origin with no CORS. It checks a leased unit as the headless host does (initial start, outputs inside the run directory, file references as `{id, path}`), checks each pack against its BLAKE3 id (`@noble/hashes`) and keeps it with the Cache API, uploads the run directory as a ustar archive and reports the result. The CLI gains `--progress` (a JSON line per century on stderr; `run_work_unit_with` reports whole years done and leaves the run unchanged) so the page shows the simulated year and time left. A volunteer's Stop abandons the lease rather than report a failure, so stops never count towards giving a unit up. The build ships the wasm stripped of its custom sections (1.8 MB instead of 21 MB, code byte for byte). An M1 calibration unit with zones runs in headless Chromium in 178 s, its packs fetched from a local coordinator included (wasmtime: 188 s of CPU, 117 MB peak memory), with the native state hash. Nothing runs until the volunteer presses Start; it pauses on battery where the browser says so. The gate runs the golden unit through the shim (golden hash) and an end-to-end smoke: Chromium computes two units, the native host the second replicas, and both validate. After review: units and results travel as exact JSON (`parseExact`: a number JavaScript would not write back as given, such as a seed above 2^53 or `1e-7`, keeps its text), Stop aborts the round's requests and never reports a failure, paths too long for ustar go in PAX headers, and the coordinator follows no link out of the client directory. Engine golden unchanged. |
| 0101 | Emmer and einkorn sown with the crop (#38, fidelity plan F6): every group carries the domestic shares of the emmer and the einkorn it sows with its crop, in the same fields under the same effort, and each wheat is selected at its own fraction of the crop's selection (log-uniform priors 0.05-2 around the ratios of Allaby et al.'s 2017 early rates within the same region, Table S13, as each rests on a few assemblages: emmer 0.00068 against barley 0.00091 in the southern Levant, so `emmer_selection_ratio` 0.74; einkorn 0.00053 against 0.0037 in the northern, 0.14, which the record rules out, so `einkorn_selection_ratio` is calibrated, below). Selection stands for how people harvested each crop, a practice learned with cultivation itself: reaping ripe ears with sickles selects strongly, beating or uprooting them little (Hillman and Davies 1990). The crop's own share is barley's, which still stands for the package in carrying capacity and in where domestic crops can be sown, so the wheats feed nobody and change no world's history; they are judged against their own rachis records, which add assemblages where barley has few (Chogha Golan in the Zagros, Aşıklı Höyük in Anatolia, Qaramel). The supplement's Tables S7 (emmer: 9 assemblages, 672 fragments) and S10 (einkorn: 8, 504) are EMF pictures that keep their text; `rachis.emf_rows` reads them (rows by baseline, columns from the header's phrases, each cell to the nearest column centre, the header checked), and the rows match the supplement's totals and the values its own charts cache; rachis table id 6afcd490. Emmer has barley's eras (before 9,000, to 8,000, to 7,000 and after 7,000 BCE); einkorn's last two are one, after 8,000 BCE, each era holding at least one assemblage. El Kowm 2, an oasis of the Syrian steppe, would be einkorn's only assemblage after 7,000 BCE, and the model barely farms there: the hydrology lacks its springs, so few groups sow and their plots stay wild (barley there has 65 of 65 ears domestic, the model's share 0.15, with a sixtieth of the effort sown near the core's sites). Its crops most likely came domestic from farming kin and exchange partners. The wheats leave it out (`m1.OASES`) until springs and seed exchange are modelled (#48); barley's eras keep it among some ten sites an era, as round 13 judges them. Each era is judged on its own with discrepancy 0.1. The record keeps `emmer_domestic` and `einkorn_domestic` (Rust and Python bit-identical). Checkpoints become `OIKOSCK8` (52 bytes per agent); `OIKOSCK7` restores each wheat as domestic as the crop, with both ratios 1, which is what its one crop stood for. At the calibrated defaults emmer's eras fit at the literature ratio (|z| below 0.8, three seeds); einkorn's do not at 0.14, as it stays wild (a share of 0.006 against 0.81 at Cafer Höyük by 7,600 BCE). A profile with every other parameter at its default (`calibration/runs/m1/round13/profile-einkorn-selection-ratio.json`, 0.14 to 1.5, three seeds) finds 0.5 to 1.1 consistent with every record and 0.9 best (summary log-likelihood -83.0, against -89.1 at 0.14), so `einkorn_selection_ratio` is calibrated to 0.9: einkorn about as fast as barley and faster than emmer, the reverse of the early rates' order, each of which rests on one or two assemblages. Left out: where each wheat's wild ancestor grew (one cereal suitability serves every crop), pulses and flax, and the crop mix in carrying capacity; #38 stays open for them. Golden change, on purpose: the checkpoint layout and two parameters (the toy world never cultivates; summaries unchanged), hash 7190d21c. Performance: 1,492.98 wasm instructions per agent-step against 1,404.55 at cf2f8ca (+6.3%, the same 679,289 agent-steps); the wheats share one `[u32; 2]` column, as two separate columns cost +8.9%. | Accepted |
| 0102 | Seed exchange, a hypothesis off by default (#48, fidelity plan F14): seed moves between people, not only with them. Each period a band replaces `seed_exchange` times its cultivation effort of its seed stock with seed at the domestic shares (crop, emmer, einkorn) sown by the people it learns from, its cell's and, by `neighbor_learning`, the cells around it, weighted by the effort each sows: farmers renew, replace and gift seed along kinship, marriage and exchange ties, out of reciprocity and trust (Pautasso et al. 2013). The world keeps each cell's sums of effort times each wheat's share beside the crop's (derived, never checkpointed). Why off: at the calibrated defaults (seed 1, the crop alone in a first trial) the rachis assemblages favour it, as the item log-likelihood rises from -86.5 off to -82.9 at 0.02, -80.3 at 0.1 and -77.5 at 0.3: the steppe's farming villages turn domestic (El Kowm 2 from 0.15 to 0.86 against 65 of 65 ears, Wadi Fidan C from 0.28 to 0.92 against 82 of 90), and the core's early crops slow toward the record (Dja'de 0.58 to 0.36 against 2 of 155). But the wild Jilat camps of the eastern desert turn domestic too (Jilat 7: 0.24 to 0.69 against 0 of 85), and the era summaries that rank worlds (ADR-0066) are flat (sum of squared z 95.8 to 97.3), so they do not identify it. Its log-uniform prior (0.001-0.5) lets the next round weigh it; the El Kowm assemblage stays out of the wheats' eras (ADR-0101) until then. What would separate the camps from the villages is the share of the harvest gathered from wild stands, which the domestication operator does not yet model. Golden change, on purpose: the registry gains the parameter (state hash 14b95b41; summaries unchanged). Performance: 1,466.84 wasm instructions per agent-step against 1,492.98 at 4036233; the toy world never farms, so the change is code generation. | Accepted |
| 0103 | Wild cereal gathered beside the fields (#48, fidelity plan F14): the domestication operator counts the wild ears that people near an assemblage gathered as well as those they sowed. Cultivators and the foragers beside them kept bolstering their stores from wild stands (Allaby et al. 2017), so each people-year of foraging effort (neither sowing nor herding) brings `m1.GATHERED` times the ears of a people-year sown, all wild (at the wild mutant frequency): a camp that sows a little among wide wild stands reads wild, a village that sows most of its effort reads as its fields. This is what ADR-0102 found missing, as seed exchange turned the Jilat camps domestic with the villages. No source gives the share directly, so it is calibrated against the rachis assemblages of all three crops at the calibrated defaults, seeds 1-3, over 0, 0.01, 0.03, 0.1, 0.3 and 1: 0.03 scores them best with seed exchange off (item log-likelihood -135.7, -140.3, -137.7 against -136.9, -140.6, -138.9 without gathering) and within 0.2 of best with it at 0.1 (-126.6, -127.1, -126.9 against -130.2 to -130.3); the era targets barely move with exchange off (sum of squared z 6.8 against 6.7, seed 1) and improve with it (5.6 against 8.1), so the two together fit better than either alone, and the next round can weigh seed exchange. Larger shares turn the late eras too wild: barley's after 7,000 BCE reaches z -2.6 at 0.3 and -4.2 at 1, which the record rules out. The operator gathers wild cereal anywhere at one rate: where each crop's wild ancestor grew is not yet modelled (#38). Calibration only: the engine, its golden hash and the round-13 waves already run are unchanged; rounds from 14 judge with it. | Accepted |
| 0104 | Calibration round 13 (#19, #44): the history after dates out of sight and northeast Africa's judging (ADR-0095), at the calibrated defaults of ADR-0090 with the Humid Period's scale 4 (ADR-0094). A screening of the 57 free parameters (588 worlds; forager_density_ref 9.2 tolerances and initial_occupancy 6.8, through northeast Africa's radiocarbon, selection_coefficient 9.1, growth_rate 4.9, npp_elasticity 4.6, humid_period_scale 4.3) and six waves of 300, 24 factors emulated, leave 0.92%, 0.34%, 0.13%, 0.07%, 0.05% and 0.03% of the prior, with 0, 0, 0, 0, 3 and 2 consistent worlds; wave 3 was sampled from wave 2's emulators alone, by mistake, and the later waves chain every earlier one. The regional arrival dates (radiocarbon subregions, ADR-0079) bind throughout: about a fifth of each wave's runs pass them. `best` over waves 2-6 reruns the five nearest with three seeds and only their emulated factors set: one stays consistent (wave 5, run 226; summary log-likelihood -94.5, items -4,315.0) against -87.5 and -4,320.5 for the calibrated defaults (`profile-humid-period-scale.json` at 4), which rank first by the summaries (ADR-0066) and stay. The consistent worlds narrow selection_coefficient to 0.0044-0.0063 and herd_selection to 0.0022-0.0055 (10-90%), the latter below its calibrated 0.035, traded against herding_weight (0.39-0.80), as in round 11. These waves ran before the wheats (ADR-0101), seed exchange (ADR-0102) and wild gathering (ADR-0103); round 14 judges them. No golden change. | Accepted |
| 0105 | Calibration round 14 (#19, #48): the wheats judged (ADR-0101), seed exchange (ADR-0102) and wild gathering (ADR-0103), with modern lakes kept out of the land (ADR-0098), at the calibrated defaults. A screening of the 60 free parameters (618 worlds; selection_coefficient 8.4 tolerances through domestic barley before 9000 BCE, sedentary_growth_rate 6.5 and fertility 6.3 through northeast Africa's radiocarbon, farm_density 6.0, forager_density_ref 5.8; emmer_selection_ratio 3.8, einkorn_selection_ratio 3.3; seed_exchange 1.5, 37th) and six waves, 24 factors emulated in the first and seed_exchange added from the second, leave 0.66%, 0.14%, 0.06%, 0.03%, 0.01% and under 0.005% of the prior, with 0, 0, 0, 0, 1 and 0 consistent worlds. The last two waves ran 250 and 70 worlds, as many as a million uniform draws found in what was left. `best` over waves 2-6 reruns the one consistent world (wave 5, run 231; seed_exchange 0.0032) with three seeds and only its emulated factors set: it scores the summaries at -153.4 and is no longer consistent, so the calibrated defaults stay. Seed exchange is then calibrated by profile at those defaults (`profile-seed-exchange.json`, 0 to 0.3, and `profile-seed-exchange-0.5.json`): every value is consistent, and the summary log-likelihood rises from -82.9 off to -81.1 at 0.3 and -81.2 at 0.5, with the rachis assemblages' item log-likelihood from -86.7 to -73.9, as the barley and einkorn eras turn domestic nearer when the record does. So `seed_exchange` is 0.3, calibrated: seed passed along kinship, marriage and exchange ties is now part of the model rather than a hypothesis, though the record identifies its rate only weakly (under 2 log-units of summaries across the prior). The golden hash changes (14b95b41 to a6da1980; the golden world's population is unchanged). | Accepted |
| 0106 | Soil depth (#24, fidelity plan F12): fields wear the same depth from every soil (Montgomery 2007), so a thin soil over rock loses its share faster. `soil_erosion_rate` is now the share lost by a soil 2 m deep or more, and a soil `d` times as deep (a `bedrock` pack's `depth_share`, at least 0.01) loses 1/`d` times it; without the pack, or data for a cell, the soil is deep, so the golden world is unchanged. The pack holds each cell's mean depth to bedrock within 2 m from SoilGrids250m 2017 (BDRICM, Shangguan et al. 2017), read at its 1/16 overview (~3.7 km): SoilGrids 2.0 has no depth layer, and the absolute depth to bedrock (BDTICM, up to tens of metres of regolith) is not the soil roots and ploughs reach. The 2017 layers are under the Open Database License v1.0, whose share-alike clause covers the derived pack, so it is shared under ODbL v1.0, apart from the other packs (as CHELSA's and DObsiSS's are under CC BY-SA). SoilGrids sees M1's soils as mostly deep on that scale: half the land is at 2 m, 90% above 1.76 m, the hills only a little shallower (Judean hills 0.95, Taurus 0.93; the full-resolution pixels agree, and its probability of rock within 2 m is about a fifth there against 1-2% on the plains), though the hills' terra rossa is often thinner. At the calibrated defaults the M1 worlds are bit-identical with and without the pack, and at erosion rates from 0.0001 to 0.0007 (`profile-soil-erosion-rate-bedrock.json`): no field wears below `soil_critical` before 3000 BCE, so soil depth constrains nothing yet; at 0.0015 (beyond the prior's 0.001) the summaries fall to -82.5. It will matter in the longer eras and in worlds that erode faster. No golden change. | Accepted |
| 0107 | Decadal climate anomalies (#41, fidelity plan F9): the 8.2 ka event and the Holocene's other abrupt anomalies, which the climate pack's century means smooth away (ADR-0042), enter from data. A `climate_anomaly` pack holds one index for the region: the NGRIP 20-year δ18O means on GICC05 (Andersen et al. 2007, PANGAEA, CC BY 3.0) less the line through the means within 500 years of each, a 1,000-year window like the background Morrill et al. (2013, doi:10.5194/cp-9-423-2013) measured the event against, from 11,600 BP, after the Younger Dryas the forcing's model simulates itself; 0 outside. Packs gain time-series fields (dims `["time"]`). Each decade every cell's temperature shifts by `climate_anomaly_temp` (1 °C per ‰) times the index and its rain is multiplied by 1 + `climate_anomaly_precip` (0.15 per ‰) times it: literature values from the event's size in the eastern Mediterranean, about −1 °C and −15% for NGRIP's −1‰ (Morrill et al. 2013; Pross et al. 2009, doi:10.1130/G25739A.1). Climate stress, comparing capacity now with 158 years before, sees the event at decadal resolution; climate variability takes the index's century means, so an event of a century or more counts and decadal swings that cancel do not. Greenland gives the timing, not regional differences, and its decadal noise (sd 0.4‰) is mostly its own (open question 41; regional speleothems, #49). In M1 the century ending 6,200 BCE is the Holocene's deepest (−1.0‰). People meet the event through the model's existing behaviour: climate stress moving effort into cultivation, moving away from crowding, and seed passed along social ties; how communities buffered bad decades by storage, sharing and diversifying (Halstead and O'Shea 1989) is not modelled. Profiles at the calibrated defaults (three seeds each, `calibration/runs/m1/anomaly/`) keep every world consistent: the summaries fall from −81.6 to −83.8 as the rain scale goes from 0 to 0.3, and from −82.6 to −85.4 as the temperature scale goes from 0 to 3 (−81.1 without the pack). The event brings Anatolia's site counts toward the record's fall in 6500–6000 BCE but not its rebound after, and northeast Africa's radiocarbon earlier, away from the record. Most of the cost is the stable-climate hypothesis: the century-to-century swings the record has (the Preboreal oscillation, the 9.3 and 8.2 ka events) raise climate variability, and at the weight calibrated against the forcing's smooth centuries (7.202, ADR-0081) farming at sites like Abu Hureyra falls to a third from 9,500 to 8,000 BCE. Profiled with the anomalies (`profile-variability-weight.json`), `cultivation_variability_weight` fits the summaries best at 2 (−81.5; −82.0 at 0, −83.5 at 4, −83.2 at 7.202) and the items better than without the anomalies (−4,293.7 against −4,297.7), so it becomes 2: against real swings the hypothesis needs a quarter of the weight. The anomaly scales keep their literature values, with uniform priors (0–3 °C and 0–30% per ‰), for a calibration round to vary with the weight. Golden changes (two parameters and the weight's default; its statistics are unchanged). | Accepted |
| 0108 | Wild wheat ranges (#38, fidelity plan F6): wild emmer and wild einkorn grow far less widely than wild barley (Harlan and Zohary 1966, doi:10.1126/science.153.3740.1074; Zohary, Hopf and Weiss 2012), and villages beyond their ranges, as on the Damascus basin's plain and the central Anatolian plateau, sowed seed brought from them, out of reach of wild stands that could mix back in (Willcox 2005, doi:10.1007/s00334-005-0075-x). A `wild_wheats` pack holds each wheat's range per cell: 1 within two cells (about 30 km) of a GBIF occurrence record of its wild ancestor, else 0 (6,636 records, CC0 and CC BY 4.0 only, chiefly the crop-wild-relatives database of Castañeda-Álvarez et al. 2016, doi:10.1038/nplants.2016.22, and genebank passport data; living specimens and whole-degree coordinates dropped; 883 cells for emmer, 2,486 for einkorn). GBIF's search results change as publishers update them, so the snapshot is kept in `data/snapshots` (38 kB) and fingerprinted in a lock, and every contributing dataset is credited in the pack's sources under the licenses of the records used. The engine pulls a wheat back by gene flow only in proportion to its range (the herds' rule, ADR-0087), while sown wild seed carries its domestic mutants wherever it is sown; without the pack the wheats grow wild everywhere, as before, so the golden is unchanged and no parameter is added. The rachis operator counts wild ears gathered by foragers only from the near cells in each wheat's range, a zone of its own in the reduced record (#47); barley's wild stands stay everywhere. Of the wheat sites, Aşıklı Höyük, Tell Aswad and Seker al-Aheimar fall outside emmer's range and El Kowm 2 outside einkorn's. Profiles at the calibrated defaults (three seeds; `calibration/runs/m1/ranges/`) keep emmer's literature ratio 0.74 (0.37 and 1.48 inconsistent) and einkorn's calibrated 0.9 (0.5 and 1.3 worse by 4-5 log-units), with summaries -81.6 against -81.5 without the ranges: emmer reads more domestic, 0.46 against 0.38 in 8000-7000 BCE (the record's 0.30) and 0.84 against 0.81 after (the record's 0.84). The ranges are modern, standing for the early Holocene's (open question 42). | Accepted |
| 0109 | Regional speleothems not adopted (#49, open question 41): the climate anomaly stays NGRIP's (ADR-0107). SISALv3 (Kaushal et al. 2024, doi:10.5194/essd-16-1933-2024; Oxford Research Archive doi:10.5287/ora-2nanwp4rk, CC BY 4.0) holds five early-Holocene δ18O records from the region at sub-century resolution: Soreq 2N-HR and the Soreq composite, Jeita-1, Sofular SO-1 and Katalekhor KT-3. Treated as ADR-0107 treats NGRIP (20-year means on each record's own chronology, less the line through the means within 500 years), they share no signal: correlations between records run from -0.34 to +0.11 at 20 years and -0.44 to +0.19 between century means, Soreq's two records correlating negatively with each other (-0.15 and -0.23), and with NGRIP from -0.17 to +0.15 and -0.26 to +0.21. They disagree on the 8.2 ka event (6300-6160 BCE): drier in the Soreq composite (+2.2 sd), Jeita (+1.1) and Sofular (+0.8), wetter in Soreq 2N-HR (-1.1), near zero in Katalekhor (-0.4). At these time scales the records carry their U-Th dating uncertainty (tens to a few hundred years at 8 ka) and local noise, not a regional climate; an index built from them, or one per cell weighted by distance, would make the forcing noisier, not more regional. No pack or manifest is added. Revisit if a regional stack on a common, layer-counted or wiggle-matched chronology appears. | Accepted |
| 0110 | Springs not added yet (#48, fidelity plan F14): no spring dataset is even enough across the region's countries, and at the calibrated defaults the record has little left for springs to explain. OpenStreetMap holds 6,455 `natural=spring` nodes in the region's box, its mapping the measure of each country's volunteers (32 within 30 km of Wadi Fidan, nearly all on the Israeli side of the Arava, against 1 to 3 at El Kowm, Jilat, Azraq and Abu Hureyra). GeoNames (CC BY 4.0), built from national gazetteers, holds 25,469 springs and 96 oases, but its density follows the gazetteer, not the water: about 200 per 1,000 km2 in Lebanon, 100 in Armenia, 27 in Israel, 4 in Syria and Jordan, 2 in Turkey and 0.3 in Iraq. Of the 9,295 cells with under 250 mm of rain at 7000 BCE, 413 hold one, El Kowm's among them (nine springs). Meanwhile seed exchange (ADR-0102, ADR-0105) has done most of what springs were wanted for: at the calibrated defaults (seeds 1-3, with the wheat ranges) El Kowm 2 reads 0.81 against 65 of 65 domestic ears and Wadi Fidan C 0.80 against 82 of 90, where without exchange they read 0.15 and 0.28. Spring gardens at `farm_density` would add tens of people to a cell whose rain already feeds about a hundred, and Azraq's spring lies by the Jilat camps, the assemblages the model already reads too domestic (Jilat 7: 0.65 against 0 of 85 ears; Jilat 13: 0.83 against 1 of 23), the larger misfit by far (open question 43). #48 stays open; springs return with a dataset of perennial springs or groundwater discharge whose coverage does not change at borders. | Accepted |
| 0111 | M1 stays at H3 resolution 5 (#40, fidelity plan F8); resolution 6 is buildable and measured. A region `m1-swasia-r6` (the same box, 155,352 cells of about 36 km2 against 22,198 of 252 km2) builds every M1 pack from the cached inputs; the packs a world reads come to 150 MB against 21.5 MB, the climate pack 114 MB of it. A world costs 2.3 times as much, not 7 (310 s against 138 s to 3,000 BCE on four partitions, seed 1, one host), as only about 90,000 of the cells are ever occupied. The calibration judges a world on any region (`Problem(region, rings)`; `profile --region --rings --set`), and two resolution-bound reaches scale by sqrt(7) a level: the wild wheats' ranges and the rivers that bury dates. At the calibrated defaults the record fits worse at resolution 6 (`calibration/runs/m1/f8/`, three seeds; summary log-likelihood with the lesion target held neutral): -80.9 at resolution 5, no target beyond its cutoff; -196.1 at resolution 6 as calibrated, four beyond; -124.6 with the parameters that act per cell step rescaled to keep their reach in km (`dispersal_probability` times 7, keeping diffusion; `obsidian_loss` per step 1 - (1 - l)^(1/sqrt 7)), three beyond. Operators reading one ring (a village's catchment, 252 km2) or three (1,333 km2, about resolution 5's one) score alike (-123.9 and -124.6; items -4,379.9 and -4,378.9 against -4,293.7). What breaks is what the model ties to a cell: villages gather only their own cell's groups (ADR-0082), so they are smaller (log size about 4.0 against 4.6 and the record's 5.1); infection persists only above a critical size counted within a cell, so none reaches the southern Levant's sites and the lesion summary is undefined (ADR-0086); northeast Africa's dated activity comes 2,700 years early (10% by 10,100 BCE against the record's 7,390), not yet traced; and sites after 6,900 BCE draw more on their nearer obsidian source than the record (0.97 against 0.54). So resolution 6 would need settlement and contact defined by distance rather than by cell, and a calibration round of its own, before it could judge local ecology; until then it would cost more and fit worse. The nested design (resolution 6 in the core) waits on the same. The region's packs stay in the lock so the comparison can be rerun; the GBIF snapshot is kept per box searched, one for both regions, which changes the resolution-5 wild_wheats pack's id (its manifest), not its values. | Accepted |
| 0112 | Catchments (#50, follow-up to F8): land and settlement are counted per catchment, not per cell. Farmers worked fields within about an hour's walk of home and foragers ranged within about two (site catchment analysis, Vita-Finzi et al. 1970, doi:10.1017/S0079497X00013074; Chisholm 1968), so a new parameter `catchment_radius_km` (8 km, literature, prior 5-10) sets the land a group works and the people it lives among. A catchment is a block of cells: the coarser H3 cell nearest a circle of that radius in area (each level seven times the last), so catchments never overlap and each cell's land counts once; overlapping ones were tried and gave the land between two villages to both (on the same land cut finer, 131,000 people against 82,000). Per catchment: the people, their practice sums and what the land feeds that mechanisms see (`Sense`), so growth, room to stay, crowding and the choice to farm are reckoned on the catchment's land; the one-person floor of a cell's capacity and its rounding apply once per catchment; water at hand anywhere in it feeds fishers on all its land; a farming group may join the largest settlement of another cell of its catchment that has room (aggregation, ADR-0071); groups that leave (fission, dispersal) found new homes in a neighboring catchment, at its cell nearest theirs, and a catchment buds as often as a cell did; learning, seed exchange and the closure of land read neighboring catchments; obsidian passes between catchments as between cells (ADR-0060). Disease stays per settlement (ADR-0075), and settlements now reach their size. The world's per-cell fields and every output stay the cells'. At 8 km every M1 cell at resolution 5 (217-304 km2) is its own catchment, so M1 is unchanged bit for bit (3,000 years compared, agents and all); the golden work unit changes only by the parameter set in its state. On the same land cut at resolutions 3 and 4, people (76,834 against 81,943), settlements (532 against 545) and mean log size (4.28 against 4.45) agree; cell by cell the finer grid held villages of at most 75 people (3.48). At resolution 6, catchments are the resolution-5 cells, and the record fits nearly as well at the resolution-5 calibration (`calibration/runs/m1/f50/compare-r6.json`, three seeds, lesion target held neutral as in ADR-0111): -85.2 against -80.9 (-124.6 before), every world consistent, no target beyond its cutoff, without rescaling dispersal or obsidian loss. Villages reach log size 4.2-4.4 (4.6 at resolution 5, 4.0 before), infection reaches the southern Levant's sites (Neolithic share of the lesioned 0.66 against 0.65), northeast Africa's 10% date is 7,230 BCE (7,750 at resolution 5, the record 7,390; 10,100 before: every desert cell had fed one forager, 15,022 of 15,492 there), and obsidian after 6,900 BCE draws on its nearer source as at resolution 5 (0.88). A resolution-6 world now takes 260 s to 3,000 BCE (310 s before, 138 s at resolution 5; seed 1, four partitions, one host), its groups being larger. M1 stays at resolution 5, cheaper and slightly better; resolution 6 can now be judged and calibrated with the same parameters. | Accepted |
| 0113 | Egypt's first domestic animals (#51, fidelity plan F15, open question 40): a record of when herds reached a land with no wild sheep or goats, and two hypotheses for how. **Record.** Linseele et al. (2014, PLoS ONE 9: e108517, CC BY 4.0) tabulate the domestic and hunted mammal bones of Egypt's Neolithic assemblages (Tables 3 and 6); the 15 in the region dated to the 6th and 5th millennia BCE (Djara, Hidden Valley in Farafra, Sodmein, Qasr el-Sagha XI/81, IX/81 and VII A/81, Kharga's KS 43, Merimde I and II-V, Kom K, Kom W, El-Omari, Saïs I and II, Maghar Dendera 2) are transcribed into a manifest (`oikos_data.egypt`), each located by its radiocarbon samples in p3k14c, and every one holds domestic animals. The operator (`oikos_cal.stock`) scores presence, not share: a world's domestic share q of the mammal bones discarded around a site is its herds' domestic effort over herding plus `GAME` (0.1, the sheep-size operator's judgment) times foraging, and an assemblage of n bones shows some with probability 1 - (1 - q)^n; the share itself depends on how much foraging went to fish and game, 33-99% of bones in the desert and under 8% on the Nile, which the model does not represent. Eras split at 5,000 BCE (6,200-5,000: mobile herders without crops; 5,000-3,900: farming villages). Recorded with every run as diagnostics, not judged: at the calibrated defaults (seed 1) no assemblage shows any, so judging it would rule out every world. **Why.** Herding is learned from neighbours, but animals were not passed on: a band takes up herding only with land to herd, the wild herds' range or, with domestic stock, pasture (ADR-0087), and in Africa no wild caprines lived, so domestic animals could reach Egypt only with people who moved there; and Egypt's rain grows no grazing along the Nile. **Hypotheses, both off.** `herd_exchange` (1/period, prior 0.001-0.5): a band replaces that share of its stock each period with animals at the domestic share of the herds of the people it is tied to (its cell's and, by `neighbor_learning`, its neighbours', by herding effort), bands that herd nothing too, as herders give, lend and marry with animals along social ties that pool the risk of losing a herd (the Maasai's osotua, Aktipis, Cronk and de Aguiar 2011, doi:10.1007/s10745-010-9364-9); and `floodplain_grazing` (prior 0-1): that share of the land a river floods grazes domestic herds whatever the rain, on what rain does not graze already (Butzer 1976), the wild herds' range unchanged. **Findings (seed 1 unless stated).** Alone or together (exchange 0.01-0.2, floodplain grazing 1) neither brings herds to Egypt: exchange makes eastern Sinai's stock 87% domestic by 5,000 BCE and western Sinai's 29-33%, but bands there herd almost nothing (a target of herding_weight times pasture times their domestic share, about 0.0004 on wadi grazing), so nobody near them is seen herding and adoption, which copies what people around do, hardly starts. With need at the top of ADR-0092's range as well (herd_pressure_weight 0.8, wadis 0.1 and 0.3) herds reach the Nile and the Fayum: a profile of need with exchange 0.2, floodplain grazing 1 and those wadis (three seeds, `calibration/runs/m1/f51/profile-need-with-exchange.json`) is consistent at 0, 0.4 and 0.8 with summary log-likelihoods -81.2, -84.8 and -88.4 (the calibrated defaults -81.1); the 5th millennium's assemblages show domestic animals at 0, 0.06-0.14 and 0.80 of the record's 1, and northeast Africa's 10, 25 and 50% dates come to 7,480-7,580, 6,310-6,420 and 5,400-5,430 BCE at 0.8 (record 7,390, 6,520 and 5,390), but the Levant's sheep turn domestic-sized by 8,500-7,500 BCE, a millennium early, and the 6th millennium's desert assemblages show none in any setting (0-0.40, the Fayum's alone). So the defaults stay. What is missing is the record's own reading: the desert's first herders were hunters keeping a few animals as a food reserve on the hoof (Linseele et al. 2014), a risk-buffering motive ADR-0087 left out of the herding target. The calibration CLI's `profile --set` now adds repeated flags (it kept only the last). Golden: the registry's two new parameters change the state hash (f889ebcf to 86b20dc1); every summary and output is unchanged. | Accepted |
| 0114 | A reserve of animals, and wild barley left native to Egypt (#51, fidelity plan F15, open question 40). **Reserve.** Linseele et al. (2014) read Egypt's first herders as hunters who kept a handful of sheep and goats among their game as a food reserve on the hoof, and herders themselves explain herds beyond present need as insurance against lean years (Dahl and Hjort 1976): a motive ADR-0087 left out of the herding target. `herd_reserve` (prior 0-0.2, off): a band whose herd is domestic keeps at least that share of effort on it, times the herd's domestic share, wherever its land grazes any (`Cultivation::kept`); the land and need set more where they repay it. It is a floor applied at once, not a target approached by copying others: animals a band holds are tended, not learned, and with the calibrated innovation share (1.6e-5 a year) a target alone was never acted on where nobody around herded (seed 1, reserve 0.05 as a target: 0.025 of the 5th millennium's assemblages show domestic animals; as a floor 0.60). **Profiles** (three seeds each, `calibration/runs/m1/f51/`), with stock exchange 0.2 and floodplain grazing 1 (ADR-0113): reserve 0, 0.05, 0.1 and 0.2 are all consistent, summary log-likelihoods -81.3, -81.8, -82.0 and -82.1 (the calibrated defaults -81.1); Egypt's 6th- and 5th-millennium assemblages show domestic animals at 0 and 0, 0.01 and 0.59, 0.03 and 0.69, and 0.19 and 0.79 of the record's 1 and 1; the Levant's sheep are unchanged up to 0.1 and nearer the record at 0.2 (-0.027 after 6,500 BCE against -0.033 to -0.035; -0.015 without); northeast Africa's dates move by under 60 years. With the reserve at 0.2, stock exchange at 0.05 is consistent (-81.6) but slower (0.003 and 0.63) and at 0.5 inconsistent (-87.5): exchange with wild-caught stock in the core keeps the Levant's sheep wild-sized (+0.004 after 6,000 BCE). So the reserve, unlike need (ADR-0113), brings herds to the Nile and the Fayum in the 5th millennium at almost no cost to the other records, but no setting brings them to the desert's 6th-millennium hunters in time: animals cross Sinai a band at a time, a millennium late. The three stay off and the record unjudged; the next candidate is how far herders themselves travelled, the desert groups moving up to 200 km between the oases and the Nile (Linseele et al. 2014). **Wild barley stays native to Egypt.** GBIF's openly licensed records place wild barley in Egypt only near Mersa Matruh (3 of 5,546 in the region's box), but Euro+Med, POWO and Boulos's checklist of Egypt's flora list it as native there, and in Asia the records cover only about half the model's wild-barley land (52% within six rings, 41% east of 42 E; Çatalhöyük beyond any), so a records-only range would cut where it grew; barley's wild stands stay everywhere the climate allows. The model's Mediterranean coast of Egypt therefore still cultivates and domesticates its own barley from about 8,000 BCE, on land since drowned or under the Delta's silt that no record sees (open question 40). Golden: the new parameter changes the state hash (86b20dc1 to 15e9905e); every summary and output is unchanged. | Accepted |
| 0115 | Herders travel farther (#51, fidelity plan F15, open question 40). With a reserve of animals (ADR-0114) herds reached the Nile in the 5th millennium BCE, a band at a time across Sinai, but not the desert's 6th-millennium sites: a group leaving a crowded place founds its new home only in a neighbouring catchment, while Egypt's desert groups moved between the oases and the Nile, Djara's about 150 km, Farafra's sites holding things from the valley 200 km away (Linseele et al. 2014). `herder_reach_km` (prior 0-250 km, off): in forager dispersal, a band whose group cannot go to the least crowded neighbour goes instead, with the chance that it lives by domestic herds (herding effort times the herd's domestic share), to the nearest place within that distance over habitable land, from catchment to neighbouring catchment, that is less crowded than its own and can feed the group and its flocks (`Dispersal::farther`), the least crowded of the nearest ring. People move as short a way as will do, to the nearest opportunity (intervening opportunities, Stouffer 1940), and herders cross land that feeds them nothing for grazing beyond. The distance becomes steps by the spacing of hexagons of the catchments' mean area (`Catchments::step_km`), so it spans as many km however finely the land is cut (a test at resolutions 3 and 4). A review (Codex) found that the search first skipped neighbours with room but more crowded than the least crowded one, could return into the group's own catchment where catchments hold several cells, and allocated per attempt: the search now starts at the first ring, keys what it has seen by catchment, and reuses its buffers within a tile. **Profile** (three seeds, `calibration/runs/m1/f51/profile-reach.json`), with the reserve 0.2, stock exchange 0.2 and floodplain grazing 1: reach 0, 100, 150, 200 and 250 km are all consistent, summary log-likelihoods -82.1, -82.1, -81.9, -82.2 and -82.4 (the calibrated defaults -81.1), the records' items -4296 to -4275; Egypt's 6th-millennium assemblages show domestic animals at 0.18-0.20, 0.18-0.28, 0.20-0.42, 0.46-0.60 and 0.61-0.66 of the record's 1, the 5th's at 0.79 to 0.99 (seed 1 at 200 km: Djara 0.52, Sodmein 0.31, Hidden Valley in Farafra 0); northeast Africa's 10% date moves from about 7,750-7,800 to 7,460-7,510 BCE (record 7,390), the 25% from 6,540-6,590 to 6,280-6,340 (6,520) and the 50% from 5,545-5,550 to 5,320-5,360 (5,390); the Levant's sheep are unchanged. So herders' longer moves bring herds to the desert's sites at about half the record at no cost to the others; the four hypotheses stay off and the record unjudged until a round varies them together. Golden: the new parameter changes the state hash (15e9905e to 9c4497df); every summary and output is unchanged. | Accepted |
| 0116 | Egypt's domestic animals judged, and the four Egypt hypotheses calibrated on (#51, fidelity plan F15, open question 40). **Record.** With a reserve of animals (ADR-0114) and herders' longer moves (ADR-0115) some worlds bring herds to Egypt in time, so the record of ADR-0113 is judged: each era's share of assemblages holding domestic animals (the 6th millennium's 5, the 5th's 10, all of which hold some) against the share a world expects, judged together (`targets.DomesticStock`), discrepancy 0.1 on each, as on the other shares. Its sampling variance is that of Jeffreys' posterior of the share of such assemblages that held domestic animals, Beta(k + 1/2, n - k + 1/2) (`stock.Operator.sampling_variance`): with every assemblage holding some the binomial variance would be none, yet five assemblages cannot show that all of the era's did (sd 0.10 for five, 0.06 for ten). A world must then show domestic animals in about 57% of the 6th millennium's assemblages. The calibrated defaults, which bring none, fail it (implausibility 8.6). **Profiles** (three seeds each, every other parameter at its calibrated default; `calibration/runs/m1/f51/judged-*.json`), each hypothesis over its prior with the other three held: reach 150, 200 and 250 km (reserve 0.2, exchange 0.2, floodplain grazing 1) give summaries -91.9, -85.0 and -83.3, consistent only at 250 (at 200 two seeds of three show the 6th millennium's share under 0.57); the reserve 0.1, 0.15 and 0.2 (reach 250) -101.6, -87.2 and -83.3, consistent only at 0.2; exchange 0.05, 0.1, 0.2 and 0.3 -84.0, -83.6, -83.3 and -85.3, consistent but 0.3 (exchange with wild-caught stock keeps the Levant's sheep wild-sized after 6,500 BCE); floodplain grazing 0, 0.1, 0.25, 0.5 and 1 -84.7, -83.1, -83.0, -83.0 and -83.3, consistent but 0 (the 5th millennium's Nile villages without herds). **Chosen**, the profiles' common world: reserve 0.2, exchange 0.2, floodplain grazing 0.25 and reach 250 km, now the registry defaults (calibrated). Every record is consistent for three seeds, summaries -83.0; Egypt's 6th millennium shows domestic animals at 0.60-0.67, the 5th at 0.999 (seed 1: Djara 1.00, the Fayum's Qasr el-Sagha sites 1.00, Sodmein 0.32, Farafra's Hidden Valley 0.01); northeast Africa's 10, 25 and 50% dates come to 7,430-7,480, 6,250-6,310 and 5,320-5,340 BCE (record 7,390, 6,520 and 5,390; the old defaults about 7,750, 6,530 and 5,550); the Levant's sheep after 6,500 BCE are at -0.026 to -0.027 (record -0.033 to -0.035). The other records cost 1.1 log-units (-82.2 against the old defaults' -81.1), each within its tolerance. The reserve and the reach are best at the tops of their priors (a fifth of effort, a little beyond the record's 200 km), as the humid period's scale was (ADR-0093): a round should take them over wider priors and vary them with the rest. The records' psychology stays explicit: animals kept as insurance against lean years, passed along ties that pool the risk of losing a herd, and herders moving no farther than the nearest land that can take them. Engine tests that isolate one herding mechanism turn the four off (stock exchange alone carries animals across their test deserts along the one-person bands a cell's capacity keeps). Golden: the four defaults change the state hash (9c4497df to 31ea2eb6); the toy world never herds, so every summary is unchanged. | Accepted |
| 0117 | Egypt's crops judged (#51, fidelity plan F15, open question 40): the crops half of the record #51 planned. **Record.** Linseele et al. (2014) state, for 13 of their 15 assemblages, whether people grew crops there: none in the 6th millennium BCE ("For the 6th millennium BC, there is no evidence for cultivated crops in Egypt"; Djara's grinding stones served wild plants), none in the deserts (Kharga's KS 43 holds a few cleaned grains brought from the Nile valley), and crops from Southwest Asia at Merimde I and II-V (emmer, a free-threshing wheat, hulled barley), Kom K and Kom W (the Upper K granaries; cultivated plants in Kom K's hearths), El-Omari and Saïs I and II, the first domesticated plants in Egypt. Qasr el-Sagha VII A/81 and Maghar Dendera 2, of whose plants the article says nothing, are left out. Transcribed as `crops` in the manifest (`oikos_data.egypt`). **Operator** (`crops.Operator`, sharing the placing of assemblages and Jeffreys' variance with `stock.Operator`): the domestic share q of the cereals discarded around a site, sown by cultivation effort at the crop's domestic share and gathered wild at `m1.GATHERED` (0.03, the rachis operators') per unit of foraging, and an assemblage shows crops with probability 1 - (1 - q)^10, ten cereal remains a judgment the record does not count; cultivating wild cereal shows none. Eras as the animals' (6th millennium: 0 of 5; 5th: 7 of 8), judged together, discrepancy 0.1 on each (`targets.AssemblageShares`, renamed from `DomesticStock`, serves both records). **At the calibrated defaults** (three seeds, `calibration/runs/m1/f51/judged-crops.json`) every record is consistent, summaries -81.1 with both Egypt records: the 6th millennium shows crops at 0.001-0.019, the 5th at 0.86-0.87 (record 0.875); seed 1: Merimde, El-Omari and Saïs 1.00, Kom K and Kom W 0.996 (q 0.42, where 3% of effort is sown), the deserts and Kharga 0. **What it sees:** without seed exchange (seed 1) the Fayum's 6th-millennium neighbours farm (Qasr el-Sagha XI/81 and IX/81 at 0.84 and 0.95), the era's share 0.36 against 0, implausibility about 2.5: the record leans against such worlds but does not rule them out alone; why seed exchange delays the Fayum's farming in the model is not yet examined. Golden unchanged (no engine change). | Accepted |
| 0118 | The reserve and the reach beyond their priors (#51): both were best at the tops of their priors (ADR-0116), so profiles at the calibrated defaults took them further (three seeds each, `calibration/runs/m1/f51/wide-reach.json` and `wide-reserve.json`), with both Egypt records judged. **Reach** 300, 350 and 400 km (the parameter's limit): every world consistent, summaries -81.9, -81.7 and -80.7 against -81.1 at 250 km (`judged-crops.json`), each seed's from -79.8 to -83.6: flat within the seeds' spread of 1-3 log-units. The 6th millennium's desert assemblages show domestic animals at 0.60-0.78 throughout, and Hidden Valley stays near 0 (0.026 at 400 km, seed 1). The default stays 250 km, the nearest the record's 150-200 km among values the summaries cannot tell apart, as `humid_period_scale` stayed nearest the estimates (ADR-0094). **Reserve** 0.3 and 0.4: inconsistent, as the crops record (ADR-0117) sees the Fayum's 6th-millennium neighbours farm (crops at 0.31-0.53 of the era's assemblages at 0.3, 0.46-0.56 at 0.4, implausibility up to 3.9), though domestic animals then reach every 6th-millennium assemblage (0.98-1.00). So the records bound the reserve from above at about 0.2, and the prior's top, a fifth of effort, is not what holds it there; why a larger reserve brings crops to the Fayum early is not yet examined. Golden unchanged (notes only). | Accepted |
| 0119 | Merging bands keep their domestic animals and crops (#51): small bands of a cell merge into one (`merge_threshold`), and their traits were people-weighted, the herd's and the crops' domestic shares too (ADR-0049, ADR-0087, "a proxy, as the model keeps no herd sizes"). But a share belongs to the herd or the fields, not the people: a herding band whose stock is half domestic, merging with forager bands nine times its size that keep no animals, came out with stock a twentieth domestic, as if the foragers had brought wild sheep, and the domestic animals (people x herding x share), the measure stock exchange and the calibration's operators read, were not kept: they fell by the merged herders' share of the people. Now the crop's, emmer's and einkorn's domestic shares are weighted by what each band sows (people x cultivation) and the herd's by what it herds (people x herding), so merging keeps the domestic crop sown and the domestic animals herded, to rounding; where none of the merging bands sows or herds, the shares are of the seed and stock they hold, by people. Effort and inequality stay people's. Found tracing why Hidden Valley gets no animals: the herds' domestic share fades westward from the Nile through the populated desert (0.27 at 30.5 E, 0.10 at 29.5 E, 0.01 at Hidden Valley, 6th millennium, seed 1), where no wild animals pull it down, and small desert bands merging with foragers were a sink. The fix does not reach Hidden Valley (0.008, seed 1): the gradient remains, as a band's herding effort follows its herd's domestic share and the desert grazes little, so the frontier's few animals stay few. **At the calibrated defaults** (three seeds, `calibration/runs/m1/f51/merged-defaults.json`) every record is consistent, summaries -81.7 (seeds -83.3, -79.1, -82.7) against -81.1 before, within the seeds' spread; Egypt's 6th-millennium assemblages show domestic animals at 0.67-0.81 (before 0.60-0.67), and the Fayum's 6th-millennium neighbours keep domestic crops (0.05-0.39 of the era's assemblages show crops, before 0.001-0.019; implausibility up to 2.7). Engine tests: herders now creep across the tests' deserts without a reach or without need along the one-person bands a cell's capacity keeps there, their stock no longer diluted, so the tests of the reach and of need compare (with a reach 9 to 32 times as many herd beyond the desert as without; with need 2.8 times as many cross as without, where ADR-0092 found it the only way across: part of that was the dilution). Cost: the effort-weighted sums are u128, skipped for bands that neither sow nor herd: wasm instructions per agent-step 1,494.6 to 1,521.2 (+1.8%, `scripts/instructions.sh` at resolution 4, 50 years), native `bench --res 6 --years 20 --parts 4` within noise (median 29.7 and 25.4 ns per agent-step over five runs). Golden unchanged: the toy world's merging bands hold no domestic shares. | Accepted |
| 0120 | Calibration round 15 (#19, #51): Egypt's domestic animals (ADR-0116) and crops (ADR-0117) judged, merging bands keeping their domestic shares (ADR-0119), at the calibrated defaults (`calibration/runs/m1/round15/`). A screening of the 67 free parameters (688 worlds; selection_coefficient 8.8 tolerances through domestic barley before 9000 BCE, forager_density_ref 8.6 through northeast Africa's radiocarbon, soil_erosion_rate 7.0, herding_weight 6.3, growth_rate 6.1) ranks the four Egypt hypotheses low: herd_exchange 1.6 (39th), herd_reserve 1.4 (44th), floodplain_grazing 1.0 (49th), herder_reach_km 0.3 (62nd), so they are varied as noise, as seed exchange was in round 14's first wave. Seed noise reaches 1.09 tolerances on Egypt's 6th-millennium crops and 0.45 on its 6th-millennium animals, 0.08 or under elsewhere. Six waves, 24 factors emulated, leave 0.39%, 0.09%, 0.03%, 0.02%, 0.01% and under 0.005% of the prior, with no consistent world among their 1,449 (the last two ran 196 and 53, as many as a million uniform draws found in what was left; `best` now records such a round instead of failing). So the calibrated defaults stay, consistent with every record (summaries -81.7 over three seeds, ADR-0119). What rules the waves' worlds out: in waves 4-6, Egypt's crops (group 10) in 81% of worlds, mostly the 5th millennium's, where crops never reach the Nile; the regional arrival dates (group 6) in 72%; domestic einkorn after 8000 BCE in 73%, domestic emmer after 7000 BCE in 61%, Egypt's 6th-millennium animals in 53%; no world fails only one or two. **A new constraint on the pace of domestication.** Of the 1,449 worlds, those whose 5th-millennium Egypt shows crops have selection_coefficient 0.0042-0.136 (10-90%, median 0.0102), those whose Egypt does not 0.0022-0.018 (median 0.0056); seed exchange does not separate them (medians 0.024 and 0.020). The Levant's early barley caps the pace (wave 1 keeps u 0.09-0.43 of its log prior), and Egypt's crops now hold it from below: 110 worlds fit both, at 0.0035-0.0128 (median 0.0063), and the default 0.0091 lies inside. Domestic crops must be selected fast enough to reach Egypt by the 5th millennium, slowly enough not to appear in the Levant before 9000 BCE. The Egypt hypotheses' values stay those their profiles gave (ADR-0116, ADR-0118); a later round could emulate herd_reserve and seed_exchange from its first wave, since the records that see them rule out most of the space they are drawn from. Hidden Valley (#52) and the Fayum's early farming without seed exchange (#53) go on as their own issues. | Accepted |
| 0121 | Seed exchange no longer delays the Fayum's farming (#53): ADR-0117 found that without seed exchange the Fayum's 6th-millennium neighbours (Qasr el-Sagha XI/81 and IX/81) show crops at 0.84 and 0.95 (seed 1), against 0.001-0.019 of the era's assemblages with it, as if exchange held farming back. That was measured before merging bands kept their domestic shares (ADR-0119; ADR-0118 changed no default). Since, at the calibrated defaults (`seed_exchange` 0.3), the two sites show crops at 0.75 and 0.88 (seed 1), 0.13 and 0.13 (seed 2), 0.98 and 0.94 (seed 3), and without exchange at 0.01 and 0.01, 0 and 0, 0.60 and 0.65: exchange brings them crops earlier, as it should. Seed 1's trace: domestic seed reaches the Fayum's foragers about 5500 BCE with exchange (the share sown domestic 0.57 by 5500, 0.81 by 5300), against 5300 without, and they sow 0.2-0.8% of their effort; the operator counts cereals from fields and from wild grasses gathered (ADR-0117), so a little domestic seed shows. So the reversal belonged to the merge bug; how it produced it is not traced, the code being gone. What remains is the lean ADR-0119 recorded: at the defaults the era's assemblages show crops at 0.05-0.39 against the record's none (implausibility up to 2.7), now through exchange reaching foragers who sow a little; open question 40. No code or default changes. | Accepted |
| 0122 | Harvest risk (#13): years differ. Until now a cell's capacity changed only with its century's climate (recomputed every ten years), so no year was bad and storage, the first answer to risk, would have had nothing to buffer. Now each year a band lives on its cell's capacity times 1 + its harvest's spread times the cell's shock, at least 0 (`mechanisms::Growth`). **The shock** (`harvest_shock`) is standard, the square root of `harvest_correlation` (0.3) times the region's shock that year plus the square root of the rest times the cell's own, each the sum of four uniform Philox draws scaled to unit variance (a new stream, `Harvest`), so a drought strikes many cells at once; years are independent, multi-year droughts left out. **The spread** is `harvest_cv` (0.4) where rain no longer limits wild cereals (`cereal_precip_full`, 350 mm, and wetter), growing as the square root of the shortfall as rain falls, to at most 1 (`environment::harvest_cv`): rain varies more from year to year where it is scarce, to about 500 mm (Golodets et al. 2013); Jordan's cereal lands have rainfall CVs of 36% at 400 mm, and dry-farmed yields swing further than rain (FAO 2008: from nothing to three times the mean). Floodplain fields swing at `harvest_cv`, the cell's spread being its fields' mean (`harvest_cv_of`). **Who feels it.** A band feels a share of its cell's spread that moves with its cultivation effort from `forage_risk_ratio` to all of it: foragers widen their diet and move, herders move their herds to where the rain fell (Winterhalder 1986; Kelly 2013), farmers are tied to their fields (Halstead and O'Shea 1989). **Deaths.** A bad year kills at most the people its harvest cannot feed, or births' worth if more: the logistic would kill everyone in a year that feeds nobody. The cap applies everywhere, so it also changes the toy world, where crude fission sometimes sends a large band into a cell of tiny capacity. **At the calibrated defaults**, without storage (ADR-0123; three seeds each, `calibration/runs/m1/f13/`, shocks keyed by year): with `forage_risk_ratio` 0.5, harvest_cv 0.2 is consistent (summaries -81.7, against -82.1 without risk), 0.4 and 0.6 are not (-96.4 and -161.1), always through Egypt's 6th-millennium desert assemblages, too few holding domestic animals as bad years thin the desert's foragers and herders (and at 0.6 Egypt's 5th-millennium animals and crops too); every other record stays consistent. With harvest_cv 0.4, forage_risk_ratio 0.1 is consistent (-80.6), 0.2 and 0.35 are not (-87.5 and -87.6). That record sits near its cutoff at the defaults and moves by 0.45 tolerances with the seed alone, so single values flip: an earlier run of these profiles with shocks keyed by day (before Codex's review) found 0.2 consistent. Storage settles it (ADR-0123): `harvest_cv` stays at the literature's 0.4 and `forage_risk_ratio` is calibrated at 0.2 (its prior widened to 0.05-1), foragers and herders feeling a fifth of a farmer's swings, which stands for their moving to where the rain fell, not yet modelled. **One harvest a year**: shocks are keyed by the simulated year (`harvest_at`), so growth periods shorter than a year see the year's harvest (Codex review). **Tests** (`tests/harvest.rs`): the spread's known answers; shocks of mean 0, variance 1 and correlation `harvest_correlation`; the cap; in a world at 400 mm farmers' numbers fall 18% under risk (harvest_cv 0.3, forage_risk_ratio 0.5), foragers' 2%; farmers' numbers swing more from year to year at 250 mm than at 600 mm; partition independence and restore; checkpoints from before restore with every year alike. Cost: wasm instructions per agent-step 1,521.2 to 1,554.4 (`scripts/instructions.sh`, resolution 4, 50 years, where no shock is drawn: about 0.9% the code, the rest the toy world's own change through the cap, 679,660 agent-steps against 679,289); native `bench --res 6 --years 20 --parts 4` medians 11.40 and 11.55 ns per agent-step over five runs each, within noise; one M1 world (seed 1, one partition) 119.7 s without harvest risk, 123.9 s with it. Golden changes (31ea2eb6 to 5f0a2732): the new parameters enter the state hash, and the cap changes the toy world from 11,500 BCE (13,896 people against 13,911). Storage follows in ADR-0123. | Accepted |
| 0123 | Storage (#13): the first answer to a bad year (Halstead and O'Shea 1989). Groups carry a store, years of their people's need per person (`Traits::store`, fixed point). Each year, after the harvest shock (ADR-0122), a group's share of its cell's people fed is `fed` = its realized capacity over the people; in a year that feeds fewer it eats from its store while it lasts, in one that feeds more it puts the surplus by up to `storage_years` (0.5) times its cultivation effort, and `storage_loss` (0.1) of the store rots or is eaten by pests each year (`Growth::stored`). **Why effort.** Storing takes a delayed-return economy and norms that let a household keep its store against demands to share (Woodburn 1982; Testart 1982): settled cultivators have them, as granaries before domestication at Dhra' (Kuijt and Finlayson 2009) and household stores at Catalhoyuk (Bogaard et al. 2009) show, mobile foragers mostly do not, so foragers keep none. Delayed-return foragers' stores (Testart's storing hunter-gatherers) are left out, and so are sharing between groups and the inequality stores feed. **Bookkeeping.** Merging and joining groups pool stores by people, so the food stored is kept; migrants carry their group's store per person. Checkpoints carry it (`OIKOSCK9`); `OIKOSCK8` restores with every store empty, and parameters from before restore with nobody storing. **At the calibrated defaults** (three seeds each, `calibration/runs/m1/f13/`, with groups joining villages as ADR-0124 has them and the corrections below): storage_years 0.25, 0.5 and 1 are consistent (summaries -81.3, -82.6 and -83.8, against -81.7 before harvest risk), 0 is not (-84.2), on Egypt's 6th-millennium desert animals; with storage, forage_risk_ratio 0.1 and 0.2 are consistent (-82.8 and -82.6), 0.35 and 0.5 not (-84.8 and -88.9), and harvest_cv 0.2 and 0.4 are consistent (-83.7 and -82.6), 0.6 not (-86.3). So the records identify the three weakly, within the seeds' spread of 1-3 log-units, and bound them through one noisy record; the defaults are the literature's harvest_cv 0.4, forage_risk_ratio 0.2 and the judgment storage_years 0.5, consistent for every seed. **Corrections from Codex's review**: births share a group's store and the dead leave theirs (`Growth::shared`), so growth keeps the food stored; a store moves at its yearly pace whatever the growth period (`period_ticks`), its losses compounded over the period; pooled stores are summed in u128, as stores reach u32::MAX. Disease deaths still take their share of a store with them. **Tests** (`tests/storage.rs`): the store's known answers, at yearly and shorter periods; births and deaths keep the food stored; in a farming world at 400 mm with harvests swinging (harvest_cv 0.3), half a year's storage raises farmers' mean numbers from 1.10 to 1.26 million (1.34 without risk) and cuts their year-to-year falls by 37%; foragers keep none; merges keep the food stored; partition independence and restore; old checkpoints and parameters. Cost: wasm instructions per agent-step 1,554.4 to 1,629.4 (+4.8%; `scripts/instructions.sh` at resolution 4, 50 years: 3.1% the new column, which sorting, merging and migration copy, and 1.7% the check in Growth, which foragers with nothing stored skip); native `bench --res 6 --years 20 --parts 4` median 11.55 to 12.72 ns per agent-step over five runs; one M1 world (seed 1, one partition) 127.7 s without storage, 133.1 s with it. Golden changes (5f0a2732 to 06817ca3): the checkpoint layout and the new parameters; the toy world's numbers are unchanged, as nobody there cultivates. | Accepted |
| 0124 | Groups joining a village keep their domestic crops and herds (#13, found while adding stores): when aggregation joins a group to another in its settlement it pooled every trait by people, the domestic shares of the crop, the wheats and the herd too, the dilution ADR-0119 fixed for merges: foragers joining farmers came out as if they had brought wild seed and wild sheep. Joining now uses the merge pool (`Traits::pooled`): effort, inequality and stores by people, the domestic shares by what each group sows and herds. **At the calibrated defaults** (three seeds, with ADR-0123's corrections; `calibration/runs/m1/f13/profile-storage-years.json` at 0.5): every record consistent, summaries -82.6 against -81.9 before both (seeds' spread 1-3 log-units). **Engine tests.** Herders' stock kept domestic as groups join crosses the test deserts more readily (`tests/herding.rs`): without a reach 161,300 herd beyond the desert after 1,000 years, with 150 and 300 km 179,512 and 423,494, so a reach now more than doubles them, where it multiplied them five times and more; without need (herd_pressure_weight) 253,044 cross in 500 years against 156,077 with it, so need no longer gets herders across, though it still keeps four and a half times as many living in the desert itself (13,739 against 2,985), and wadis remain the way across (2,210 without them). As in ADR-0119, part of ADR-0092's finding that need carried herders across Sinai was dilution. Cost: wasm instructions per agent-step 1,629.4 to 1,659.0 (+1.8%, `scripts/instructions.sh`, with ADR-0123's corrections: the joining pool 0.5%, the store's checks the rest). Golden unchanged: the toy world's joining groups hold no domestic shares. | Accepted |
| 0125 | Moving in bad years (#13), a hypothesis the M1 records reject for now. Moving is the first answer to a bad year (Halstead and O'Shea 1989): foragers leave camps whose returns have fallen and herders take their flocks to where the rain fell (Kelly 2013), and people weigh what they have just lived through heavily (decisions from experience, Hertwig et al. 2004). **The mechanism** (`mechanisms::Dispersal`): crowding, a neighbor's room and how crowded it is are reckoned on what the year's harvest feeds (`harvest_share`, the share Growth lives on, ADR-0122), as far as `bad_year_moves` weighs (`Dispersal::this_year`: the capacity moved toward the year's by that share); the origin is judged at its people's mean cultivation effort, a destination at the moving group's. A group with food stored stays through a bad year (`Dispersal::stays`): it eats from its store, and its fields and stores tie it down. **In a test world** of foragers feeling all of a harvest's swings at 250 mm, judging by the mean the margin empties (a thinned cell is never crowded enough to send anyone to refill the cells bad years empty) and judging by the year it stays peopled, about 1,200 people: the rescue effect (Brown and Kodric-Brown 1977). Where foragers feel a fifth of the swings, as calibrated (`forage_risk_ratio` 0.2), it changes their numbers by under 1% at 150 and 250 mm, and where they feel it all at 400 and 600 mm it lowers them by 3-4%: years are independent, so a move after a bad year takes a group to land the next year treats as any other, and `forage_risk_ratio` already stands for moving within the year. **On the M1 records** (three seeds each, `calibration/runs/m1/f13/profile-bad-year-moves.json`): 0 is consistent (summaries -82.6), 0.5 and 1 are not (-83.7 and -84.6): the share of Egypt's 6th-millennium desert assemblages holding domestic animals (the record: all) falls from 0.61-0.65 over the seeds to 0.42-0.70 and 0.39-0.56, past its cutoff for two seeds of three at each, every other record as before. So `bad_year_moves` is 0, the land judged by its mean as before, and the hypothesis stays in the code, switched off, for the data to judge again (after multi-year droughts, or sharing between groups, change what a move buys). **Correction from Codex's review**: a place that feeds nobody, empty, compared as no less crowded than any other, so where it came first among a group's neighbors it hid every place with room; dispersal now leaves such places out (`least_crowded_among`). It changes nothing in the M1 worlds at the defaults (their outputs are bit-identical); judging by the year it did, and the profile above is with it (before it, 1 scored -90.2). **Tests** (`tests/moves.rs` and a unit test): the judged capacity's known answers; groups with food stored stay through a bad year; the rescue effect; partition independence and restore; parameters from before restore judging by the mean. Cost: wasm instructions per agent-step 1,659.0 to 1,659.8 (`scripts/instructions.sh`, resolution 4, 50 years, where nothing is judged by the year). Golden changes (06817ca3 to 0b7a4fec): the new parameter enters the state hash; the toy world's numbers are unchanged. | Accepted |
| 0126 | Sharing between groups (#13). A group pools `harvest_sharing` (0.1) of its year's harvest with the people of its neighboring cells: it lives on its own share of its cell's capacity plus that share of the difference between its neighbors' mean and its own, each at its own practice (`mechanisms::pooled_harvest` in Growth), so it draws on neighbors the year favoured and gives in turn when they are short. **Why.** Among foragers, partners in other camps share food and access to land in bad years (hxaro, Wiessner 1982); among early farmers, households and villages exchange food and obligations against future need (social storage, Halstead 1989); transfers that follow need rather than accounts pool risk best where harvests are volatile (Aktipis et al. 2016). Pooling helps only as far as neighbors' years differ from one's own (`harvest_correlation`), and unlike moving after a bad year (ADR-0125) it works within the year. **What is left out.** The pooling is in expectation: what a group draws is not taken from its neighbors' people, so food is conserved on average over years, not each year. Everyone shares alike; that people with stores or herds lean on reciprocity less (Cashdan 1985; since ADR-0127), that sharing follows kinship and distance, and that giving builds obligation and rank (Halstead and O'Shea 1982) are not modelled. Partners are each neighboring cell's own people and harvest, whatever the catchments (Codex's review: one cell per neighboring catchment, judged peopled by the catchment's people, could lend an empty cell's good year). **In a test world** of farmers at 400 mm with harvests swinging (harvest_cv 0.3, correlation 0.3, nobody storing), sharing 0.25 raises their mean numbers from 1.10 to 1.20 million and 0.5 to 1.21 million, with smaller falls from year to year; where every cell has the same year (correlation 1) nothing changes. **On the M1 records** (three seeds each, `calibration/runs/m1/f13/profile-harvest-sharing.json`): 0 and 0.1 are consistent (summaries -82.6 and -81.0), 0.25 and 0.5 are not (-83.2 and -83.1): sharing keeps more of Egypt's 6th-millennium desert assemblages holding domestic animals, as the record has them, but past 0.1 brings crops into some of its 6th-millennium assemblages (implausibility 3.65 for seed 2 at 0.25), which held none. The summaries tell 0 and 0.1 apart by less than the seeds' spread; 0.1 is consistent for every seed and fits best, so it is the default. **Tests** (`tests/sharing.rs`): the pool's known answers; more farmers through bad years, with smaller falls; nothing pooled where every cell has the same year; partition independence and restore; parameters from before restore without sharing. Older tests of harvest risk, storage and moving hold sharing at 0, as when they were written. Golden changes (0b7a4fec to 5e95381a): the new parameter enters the state hash; the toy world, without a climate, has no harvest risk and is otherwise unchanged. | Accepted |
| 0127 | Stores and herds weaken sharing (#13). A group withholds `sharing_withdrawal` (0.5) of its `harvest_sharing` times how far its own buffers insure it: its store as a share of `storage_years` of need (full at that), and its herding effort, together as 1 − (1 − stored)(1 − herding) (`Growth::sharing_of`). It then neither gives in its neighbors' bad years nor draws in its own. **Why.** People with buffers of their own lean on reciprocity less: among the Basarwa, households with stored crops or livestock took less part in exchange networks than those without (Cashdan 1985), and early farmers' household stores were kept from demands to share (Testart 1982; Bogaard et al. 2009). Self-insurance and pooling are substitutes. **What is left out.** How much a full store weakens sharing is not measured; the form (linear in how full the store is, herds insuring like a store) is the simplest that keeps an empty store's group fully in the network. Groups that withdraw are not punished or shunned. **In a test world** of farmers at 400 mm storing up to a year's need (harvest_cv 0.3, correlation 0.3, sharing 0.5), full withdrawal keeps 46-78% of what sharing gains in mean numbers (seeds 3 to 6), since stores empty in bad years and groups share again; where nobody stores or herds it changes nothing. **On the M1 records** (three seeds each, `calibration/runs/m1/f13/profile-sharing-withdrawal.json`, at 3f8563b): 0, 0.5 and 1 are all consistent (summaries -81.0, -81.2 and -81.7), apart by less than the seeds' spread, so no record bounds it. The default 0.5, the prior's middle, is a judgment that keeps the behavior the literature finds without claiming its size. **Tests** (`tests/withdrawal.rs`): known answers of the withheld share; farmers with full stores share less, between sharing fully and not at all; no withdrawal without stores or herds (Codex's review: the stylized fact alone would pass a withdrawal that ignored the buffers); partition independence and restore; parameters from before restore without withdrawal. Golden changes (5e95381a to 895f086d for the new parameter at 0, then to 55fbb762 at the default 0.5): the toy world, without a climate, has no harvest risk and is otherwise unchanged. | Accepted |
| 0128 | Stores are wealth (#13). A group's owned share of wealth, which sets its intergenerational transmission and shocks (ADR-0074), is its cultivation effort times its land's closure plus `stored_wealth` (1) times how full its store is (against `storage_years`) times the land still open (`Inequality::owned`, `mechanisms::store_fill`). **Why.** Where norms let a household keep its store against demands to share, what it puts by is its own and passes to its heirs: storage made sedentary foragers unequal (Testart 1982), and at Catalhoyuk households kept private pantries beside celebrated surplus (Bogaard et al. 2009). So wealth can be owned on an open frontier, before farmland closes in. **What is left out.** The store is the farming economy's: a group that has left farming lives by foragers' levelling norms, which share out what it holds (Woodburn 1982), so its leftover store, which rots away, is not owned (Codex's review raised the case). Shared and feasted surplus, and the rank giving it builds, are not modelled. The store's size is not itself wealth's spread: it changes which transmission and shocks apply, not the variance directly. **In a test world** four centuries into the farming frontier, with stores about four-fifths full, owned stores raise the settled variance of log wealth from 0.39 to 0.65; by 1,500 years, with farmland closed in, both are near the landed level. **On the M1 records** (three seeds each, `calibration/runs/m1/f13/profile-stored-wealth.json`, at d04dd06): 0, 0.25, 0.5 and 1 are all consistent (summaries -81.2, -81.1, -81.1 and -80.9). Every one of the five house-size Ginis moves toward its record as stores are owned, but by at most 0.01 (Catalhoyuk 0.231 to 0.240 against 0.28; Tell Brak 0.228 to 0.236 against 0.36), so stores explain little of the record's rise (open question 34). 1 fits best and becomes the default. **Tests** (`tests/stored_wealth.rs`): known answers of the fill and owned share; frontier farmers with owned stores grow unequal; nothing changes where nobody stores or stores rot within the year (Codex's review: the frontier test alone would pass if every store counted as full); partition independence and restore; parameters from before restore with stores unowned. Golden changes (55fbb762 to d362603e for the new parameter at 0, then to 3c47af2a at the default 1). | Accepted |
| 0129 | Droughts that last (#13, open question 44). With `harvest_persistence` above 0, a cell's harvest shock is a first-order autoregression of its yearly draws: the sum of each past year's draw times the persistence to the power of its age, scaled back to unit variance and cut where the weights' squares fall below a millionth (`mechanisms::harvest_shock`, `PERSISTENCE_TAIL`). Each year's shock stays standard and cells share the region's part as before; at 0 it is ADR-0122's shock bit for bit. The default stays 0. **Why.** Multi-year droughts recur in the eastern Mediterranean's tree-ring records (Touchan et al. 2005; Cook et al. 2016), and a run of bad years, not one, outlasts stores and forces people to move (Halstead and O'Shea 1989). ADR-0125 left open whether moving in bad years, which weighs the latest year, would hold where the latest year foretells the next. **In test worlds:** storing gains farmers 13-15% in mean numbers with independent years and 7-8% with persistence 0.6 (seeds 3 to 5); judging where to go by the year's harvest costs foragers at 400 mm 3% of their numbers with independent years and nothing with persistence 0.6, and on the 250 mm margin keeps about 11,000 people where it kept 1,200-1,900. Weighing recent experience is adaptive only where the past predicts the future. **On the M1 records** (three seeds each, at d671ae8): `harvest_persistence` 0, 0.3 and 0.6 are all consistent (summaries -80.9, -81.3 and -81.4, `profile-harvest-persistence.json`), apart by less than the seeds' spread. With it at 0.6, `bad_year_moves` 0 is consistent (-81.4) and 0.5 and 1 are not (-90.4 and -82.6, `profile-bad-year-moves-persistent.json`): moving still empties Egypt's 6th-millennium deserts of herds (their assemblages holding domestic animals 0.19-0.61 of the time, the record all). So lasting droughts do not save the hypothesis on these records, and persistence stays 0 until a measured autocorrelation of the region's yearly rain or harvests sets it. **Cost.** A persistent shock draws every past year it keeps: at 0.6 some 14 times the draws, and sharing and moving recompute their neighbors' (Codex's review); a cache of each cell's yearly shock would remove the repetition if the default rises. **Tests** (`tests/droughts.rs`): persistence 0 reproduces the old shock exactly; persistent shocks stay standard, share the region's part and carry 0.3 or 0.6 into the next year; one harvest a year when persistent; a store buffers lasting droughts less but still helps; moving in bad years stops costing where droughts last; partition independence (through moving and sharing too) and restore; parameters from before restore with independent years. Golden changes (3c47af2a to cff3e8ae): the new parameter enters the state hash; the toy world is unchanged. | Accepted |
| 0130 | Calibration round 16 (#13): harvest risk (ADR-0122), storage (ADR-0123) and pooled traits on joining (ADR-0124) in, at 79597db, before sharing (ADR-0126) and what followed it (`calibration/runs/m1/round16/`). A screening of the 72 free parameters (738 worlds) puts harvest_cv first (8.7 tolerances, through northeast Africa's radiocarbon), then selection_coefficient (8.3, early domestic barley), growth_rate (7.5), dispersal_probability (7.1) and forage_risk_ratio (7.0); storage_years 10th (4.1), harvest_correlation 30th (2.7), storage_loss 62nd (0.6). Seed noise reaches 0.69 tolerances on Egypt's 6th-millennium crops, 0.33 on domestic barley 8000-7000 BCE, 0.06 or under on most records. Six waves, 24 factors emulated, leave 0.34%, 0.15%, 0.07%, 0.04%, 0.02% and under 0.005% of the prior, with no consistent world among their 1,652 (waves 5 and 6 ran 292 and 160, as many points as the sampler found in what was left); the best of each wave misses by 5.16, 4.29, 3.92, 3.84, 3.64 and 4.05 implausibility. So the calibrated defaults stay, consistent with every record by their profiles (ADR-0123 to ADR-0129). What rules the waves' worlds out, waves 4-6: domestic einkorn after 8000 BCE in 68-74% of worlds and domestic emmer after 7000 BCE in 62-74%; Anatolia's sheep size 8500-7500 BCE in 58-65%; Egypt's 6th-millennium animals in 53-61% and its 5th-millennium crops in 49-59%; Anatolia's radiocarbon (25% of its dates by) in 55-62%; no world fails fewer than three records. Of harvest risk and storage, the records bound only how much less of a bad year foragers and herders feel: forage_risk_ratio in waves 4-6 runs 0.08-0.63 (10-90%, median 0.26, against a prior 0.05-1), in line with its profiled default 0.2. Harvest_cv (0.23-0.56, median 0.37) and storage_years (0.09-0.95) stay as spread as their priors, so harvest_cv stays the literature's 0.4 and storage_years a judgment. selection_coefficient keeps round 15's window (median 0.0079; the default 0.0091). The round leaves the wheats' late domestication and Egypt's records as what no setting of the present model reaches together; a next round waits on the mechanisms since (sharing, owned stores, droughts that last, rank built by giving). | Accepted |
| 0131 | Rank built by giving (#13). Households short in a bad year are fed by those with food to spare, and a gift binds the one who cannot return it (Mauss 1925; the norm of reciprocity, Gouldner 1960): in years of real shortage the needy repay with labour, land and deference, and social storage turns into rank (Halstead and O'Shea 1982). Settled farmers, tied to their fields and stores, cannot walk away from what they owe as mobile foragers can (Woodburn 1982). So in Growth a period's unmet need, after harvest, sharing and store, adds `giving_rank` times cultivation effort times the need squared times the period's years to a group's variance of log wealth, which Inequality carries on; 0 runs the model before, bit for bit (restore rule). In a test world (harvest_cv 0.3, 800 years) giving_rank 1 raises settled farmers' log-wealth variance from 0.79 to 0.90, and by nothing where years are alike. Profiled on M1 at the calibrated defaults (mechanism commit be03b86; `calibration/runs/m1/f13/profile-giving-rank.json`, `profile-giving-rank-high.json`, three seeds each): 0, 0.5, 1, 2, 3 and 4 are all consistent (summaries -80.9, -80.9, -80.8, -80.6, -80.5, -80.4), a little better in every seed as it rises, because each unit lifts every site's house-size Gini by about 0.008 (at 2: Jerf el Ahmar 0.212 to 0.228 against 0.20, Catalhoyuk 0.240 to 0.254 against 0.28, Tell Brak 0.236 to 0.250 against 0.36) and the records' tolerances, 0.07-0.15, reward any rise. Giving lifts the level, not the rise from the PPNA to the Late Chalcolithic, and the records do not bound it, so the default is a judgment, 1, the middle of its prior (open question 34). Left out: rank between groups and settlements (the debts are within a group), herds lent and owed (clientship among herders), and who gives to whom. A group splitting in the year keeps slightly more of that year's debt in its daughter, which skips Inequality that year. Golden 9a78be52 to 9a11f66c: the default changes (params); the toy world is unchanged, since it does not farm. Instructions per agent-step 1656.81 to 1661.81 with the check. | Accepted |

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

## ADR-0062 rationale (AI inside the simulation)

The project's aim (doc 1) needs simulated people whose responses to conditions and
policies are realistic, including responses to policies never tried; hand-written
decision rules are the weakest part of agent-based models, and models trained on human
choice data now predict behavior across many tasks (doc 7 §7.2). Using them directly
for billions of agents is unaffordable and not reproducible, so they are teachers:
their answers on sampled situations are distilled into small per-archetype policy
networks that run in integer arithmetic (exact on every platform, so volunteer results
stay validatable by hash, ADR-0016 and ADR-0017) and whose few free parameters are
calibrated like any mechanism. The record keeps the last word.

The main risk is leakage. A model that has read history knows its outcome; letting it
decide agents' choices in a hindcast would steer worlds toward what happened and turn
calibration into an echo. Behavior models used before the present are therefore
trained on experiments, surveys and ethnography rather than narratives of the period,
and their contribution is tested on held-out periods and regions. Full LLMs act only
after the twin's start, where there is nothing to leak and where judging reactions to
untried policies matters most; even there their decisions are recorded once into a
content-addressed decision pack and replayed, so a work unit never calls a model.

AI also appears as part of the world: from about 2020 AI systems do tasks, earn
returns for their owners and advise decisions, so they are a class of agents and
capital (task-based production, Acemoglu and Restrepo 2018, 2019), not a single
productivity parameter. And it helps people read the results: grouping worlds into
pathways, finding what separates better worlds from the rest, answering questions with
the runs as evidence, and narrating a world's history. Every such component can be
switched off, and results report how much they depend on it.

## ADR-0063 rationale (viewer v0)

The viewer has two jobs: to let people see what the simulated worlds do, and to keep
the uncertainty in front of them. One world on a map invites the reader to take it as
the forecast. So the unit the viewer shows is an ensemble: the calibrated defaults and
the best worlds that the calibration found consistent with the record, all from the
same start. The map shows one of them at a time, and the charts show all of them, with
the mapped world as one strong line among faint ones (design system §6). In M1 the nine
worlds end between 0.55 and 12 million people, and the viewer says so in words.

The viewer reads what the engine recorded and computes nothing of its own. Each world is
an ordinary work unit run by the `oikos` CLI with outputs, so what the viewer shows is
what any volunteer's run of the same unit would produce, down to the state hash in the
catalog. Run outputs therefore gain the fields a map needs: cultivation, domestication,
soil, dry land and obsidian per source group. They are per-cell shares in parts per
10,000, computed by integer division from the engine's fixed-point state, so they are
exact on every platform. Recording them changes no history (tested), and an output time
costs one pass over the cells. The engine writes uint64 Zarr with gzip, and browsers
decompress gzip natively, so a 120-line reader replaces a Zarr library. Chunks are
fetched lazily, so the map loads one band of times at a time.

deck.gl is the stack ADR-0007 chose. The viewer takes only its core and layers packages:
the umbrella package pulls in 3D tiles, glTF and CARTO loaders that the viewer never
uses, along with 13 advisories in them. Hexagons come from h3-js boundaries, drawn as
polygons. There is no basemap. Cells without dry land in that century are not drawn,
so the paleo-coastline (the dry Gulf basin, Cyprus apart from the mainland) appears
from the coastline pack itself, and no tile service sees the viewer's users. Fields are
drawn in six fixed classes of a single-hue ramp, which is new to the design system
(`map-ramp-1..6`). The classes step about evenly in the logarithm where values span
orders of magnitude, and colors compare across worlds and times.

An ensemble is about 13 MB per world in M1 (91 times, 22,198 cells), so ensembles are
built locally (`viewer/data/`, not committed) and served next to the page. Publishing
one for the public site waits for the coordinator's analysis store (doc 11).
Dependencies are pinned exactly in the lock file. The gate typechecks, unit-tests and
builds the viewer (`npm ci` when the lock changes), and smoke-renders it at phone and
desktop widths when an ensemble is built. It skips these with a notice where Node,
Chromium or an ensemble is missing.

Seeing the worlds found what the calibration's numbers had not. Every consistent world
gets more than half its food from planted crops by 11,500 BCE, more than a millennium
before cultivation is first seen and at far too high a share. None of the four records
dates the onset of cultivation (open question 30, #29). The page says so, next to its
claim that the worlds match the evidence checked so far (design review, ADR-0032).

## ADR-0064 rationale (cultivation onset; calibration round 6)

The viewer's ensemble showed every world consistent with the four earlier records
living mostly on planted crops by 11,500 BCE (open question 30), while cultivation of
wild cereals is first seen in the Pre-Pottery Neolithic A and at a small share. None
of those records dates the onset: rachis dates domestication, the surveys start at
6,500 BCE, and radiocarbon and obsidian say nothing of diet.

What dates it is what people burnt. The Origins of agriculture database (Wallace,
Livarda, Charles and Jones 2018) compiles plant remains per sample for dated site
phases of southwest Asia, Pre-Pottery Neolithic and earlier, with the Pottery
Neolithic. Items are counted as Wallace et al. (2019) select them for their analysis of
deliberate collection: samples the compilers did not exclude, grains and seeds,
charred. Crops are domesticates, crop progenitors and presumed crops; taxa classed
"progenitor or wild" or indeterminate count as neither. Phases with fewer than 50
items are dropped (their analysis uses 50 or more), leaving 59 target phases and 18
Pottery Neolithic ones.

Counts are not diets: arable weeds come with the harvest, dung fuel brings wild seeds,
and a cereal grain is many small seeds' worth of food. So the operator does not read a
share as a diet. It anchors the share of a farming world on the record's own Pottery
Neolithic phases (the mean over phases, 0.50), when people across the region lived on
crops, and reads an era's mean share over that anchor as the cultivation effort the
record implies. Both simplifications make the test lenient: Pottery Neolithic people
also gathered and herded, so full farming would show more crops than the anchor (less
implied effort than the record's), and foragers gathered wild crop progenitors, which
this leaves out. A world the record rules out under them is ruled out.

The model's side is a world's cultivation effort (cultivator people-years over
people-years) over the site's cell and its six neighbors and the centuries of the
phase, each weighted by its overlap. Phases where a world has nobody take the mean
effort of the phases it reached, so every world is judged on the same phases.

Phases are not scored one by one. Within a period their shares vary enormously at the
same level of farming (logit standard deviation 2-5: villages burning crop waste near
1, steppe sites burning dung near 0), and a beta-binomial per phase (φ = 2, as for
rachis) scored a late start below an early one, because shares near zero say almost
nothing under that spread. Only era means over many phases carry information. Four
eras split at 10,000, 8,800 and 7,600 BCE (Late Epipalaeolithic, PPNA, Early-Middle
PPNB, Late PPNB) give implied efforts of 0.09, 0.52, 0.64 and 0.62, with a joint
bootstrap over the eras' phases and the anchors (the anchor is shared, so the targets
correlate): one group of four, discrepancy 0.1 on effort, as on the domestic share. The
earliest era is thin: four phases at three sites (two at Abu Hureyra, Iraq ed-Dubb,
Mureybet), so its standard deviation (0.04) comes from few phases, and the discrepancy
dominates it.

The record rules out the round-4 defaults: they put in 0.57, 0.79, 0.79 and 0.58
(implausibility 4.4 before 10,000 BCE; group distance 21.2 against a cutoff of 14.9).
The discrepancy is a judgment: at 0.2 the defaults would pass (implausibility 2.3), so
the conclusion rests on effort being known to about 0.1 from the anchor argument above,
not on the record alone.

Wave 6 (300 runs, 18 active factors, the earlier waves as previous) judges all five
records: 0.23% of the prior space survives (0.8% after round 5), and 55 of 300 worlds
are consistent; the onset group passes in 42% of runs, the earliest era alone in 29%.
`best` reruns the 12 best with three seeds; 11 stay consistent and run 236 fits best
(-4,274.8). Because the record rules the defaults out, the defaults cannot stay, and
run 236 replaces them: its 18 emulated values become the registry defaults, with
`obsidian_loss` now emulated in the wave instead of profiled (0.0041, against 0.0028).
Its maximum implausibility is 2.3-2.4 (the earliest era) and its groups 6.2, 8.0, 12.7,
12.7 and 8.6 (cutoffs 23.6, 18.5, 18.5, 22.0, 14.9); it has 5.4 million people at
3,000 BCE (2.3 million in the round-4 world; consistent worlds 0.8 to 4.5 million,
10-90%). Its forager growth (0.0022) and wild-abundance weight (0.024) are below the
consistent worlds' 10-90% ranges and its farm density (6.9) above: it is one world
among many, not an estimate.

The finding is structural. Run 236 puts in an effort of 0.34, 0.39, 0.39 and 0.34: it
passes by planting about a third throughout, not by starting low and rising. No world
of wave 6, consistent or not, rises like the record (0.09 to 0.62): the largest rise
from the first era to the last is 0.22, and among consistent worlds 0.19 (median 0.04).
The cultivation mechanism moves effort toward a target set by pressure, stress and
abundance, which reaches its level early and holds it; nothing in it makes planting
grow with time (open question 31, #30).

The ADS terms allow research use but not passing on the data or anything derived from
it except to registered users. The seven tables are downloaded by hand into the local
cache and checked against the manifest's SHA-256 sums; the phase table is built in
memory for each calibration and never written to `build/`, locked or committed, and
the targets and wave files record only the four era summaries (with the citation), as
a publication of results would.

The golden toy world has no cultivation packs; its forager growth falls to 0.0022, so
it grows to 7,505 people by 9,000 BCE (205,140 before) and its hash changes (5efe3afc).
Mechanism tests that need foragers to fill a toy region in centuries (dispersal, sea
flooding) pin fast-spreading values instead of the defaults. The stylized test that
domestic crops do not dominate before 9,500 BCE now weights cells by cultivators: in
the round-6 world two cells at the region's northwest edge (243 people), where no wild
stands dilute the crop, drift past half domestic, while rapid selection (0.02) would put
98% of cultivators in such cells.

## ADR-0065 rationale (social learning of cultivation; calibration round 7)

Round 6 found that no world plants more over time as the onset record does (0.09 before
10,000 BCE to about 0.6 in the PPNB; open question 31, #30). Across wave 6 the first
era's effort and the last era's correlate at 0.92, and the world of the defaults
reaches 0.29 by 11,000 BCE and holds near 0.4 after: every band moves toward its target
from the first year, so the onset is set by when the run starts, not by anything in
the world. Climate variability, the only driver that could hold planting back early,
differs little on average between the late glacial and the Holocene in the region.

Practices are not reinvented by each band; they are learned from others. Diffusion
studies find S-shaped adoption curves, slow while few have adopted and then fast, which
Henrich (2001) reads as the mark of biased cultural transmission and Bass (1969) fits
as innovation plus imitation; Ackland et al. (2007) model farming spreading with its
practitioners in the same way. The mechanism nests this: the rate at which a band closes
the gap to its target is `adoption_rate × (i + (1 − i) × seen)`, where `seen` is the
people-weighted mean cultivation effort in the band's cell after the previous step
(`effort_sum`, recomputed at each commit with the population), and `i`, the innovation
share, is the part a band learns on its own. `i = 1` is individual learning, the model
before; small `i` makes planting an innovation that takes off only after about
`ln(1/i) / adoption_rate` years. `seen` is read from committed state, so it is the same
for any partition count; checkpoints written before the parameter existed restore with
`i = 1`. The test world with abundance alone shows the S-curve: with `i = 10⁻³` effort is
0.02 after 300 years, 0.22 after 600 and at the target (0.5) by 1,200, while individual
learning is there in 300.

A profile at the round-6 defaults (`runs/m1/round7/innovation_share.json`, three seeds)
shows the effect and its cost: `i` = 1, 0.1, 0.01 and 0.001 put in a first-era effort of
0.34, 0.27, 0.08 and 0.00, but below 0.1 the world is no longer consistent (domestic
barley after 7,000 BCE is too low: planting starts later, so selection has less time).
Other parameters have to move with it, so it is calibrated with them.

Wave 7 (300 runs, `innovation_share` the new active factor) is drawn from the NROY space
of waves 1-6, whose emulators ran at `i = 1`: the earlier waves' judgment of the other
factors is conditional on individual learning, and NROY points take `i` uniformly over
its prior. 101 of 300 worlds are consistent (55 in wave 6) and the largest rise among
them is 0.25 (0.19); worlds that rise like the record exist (0.02, 0.58, 0.80, 0.64) but
domesticate too slowly. They all have `adoption_rate` near 0.005, the bottom of its
prior: with the default 0.02 a take-off near 10,000 BCE would need `i` far below its
prior. `adoption_rate` had been varied as noise in every wave, so no emulator saw it and
`best` set it back to its default; `wave --activate adoption_rate` makes it active from
wave 8. In wave 8, 110 of 300 worlds are consistent and the largest rise among them is
0.36; in wave 9 (NROY 0.13% of the prior space), 154, and 0.46. `best` over waves 7-9
(the top 16, three seeds) keeps 12 consistent at the defaults. It ranks waves with
different active factors; each factor's consistent range now comes from the waves that
emulated it.

The model can now plant as the record does. Wave 9, run 84 (`i` = 1.1 × 10⁻⁴,
`adoption_rate` 0.0069) puts in 0.09, 0.57, 0.75 and 0.54 against the record's 0.09,
0.52, 0.64 and 0.62, consistent with all five records for each of three seeds, with an
onset group distance of 0.8 (cutoff 14.9). That settles the structural finding of round
6 (#30).

It does not become the default. The best consistent world by the log-likelihood
diagnostics, the rule of every round (ADR-0057, ADR-0059, ADR-0064), is wave 9, run 208
(-4,260.0 at the defaults against -4,275.0 for the round-6 world), and it becomes the
registry defaults: 20 parameters `calibrated`, `adoption_rate` (0.017) and
`innovation_share` (0.0037) among them. It learns cultivation socially, and puts in
0.40, 0.59, 0.61 and 0.47: nearer the record in the PPNA and PPNB than the round-6
world (0.34, 0.39, 0.39, 0.34), farther before 10,000 BCE (implausibility 2.8-2.9, the
largest of its targets). Its groups are 6.9, 7.3, 14.2, 14.6 and 9.4 (cutoffs 23.6,
18.5, 18.5, 22.0, 14.9) and it has 5.0 million people at 3,000 BCE. Its forager growth
(0.0033) and productivity elasticity (0.51) are below the consistent worlds' 10-90%
ranges and its dispersal probability (0.47) above: one world among many, not an
estimate. Run 84 fits radiocarbon (-4,058.5 against -3,992.7), rachis (-115.3 against
-75.1) and obsidian (-230.0 against -192.2) worse. The diagnostics cover those three
records only, while the onset and the site counts are judged as targets, so the ranking
does not see that run 84 fits the onset far better; whether the records really pull
apart or the ranking should weigh the onset is open question 32.

The new defaults change the golden toy world (no cultivation packs; forager growth
0.0033 and dispersal 0.47): it grows to 23,297 people by 9,000 BCE (7,505 before), and
its hash changes (f93c62f9). The cultivation mechanism tests pin individual learning and
the earlier adoption rate with the other values they were written for, and the capacity
tests (the toy world and the M1 capacity map) pin a productivity elasticity of 1, so
later rounds leave them alone. Checkpoint parameters missing a key take the registry
default, which is now social learning; `Params::from_checkpoint` restores one written
before ADR-0065 with `innovation_share` 1 instead, as it was run (generalized in ADR-0069).

## ADR-0066 rationale (ranking worlds by their summaries; calibration round 8)

Round 7 left open question 32. The consistent world that plants as the onset record
does (wave 9, run 84) fit radiocarbon, rachis and obsidian worse than the world chosen
as the defaults (run 208), which plants 0.40 before 10,000 BCE. Every round since
ADR-0057 chose its defaults by the item log-likelihood diagnostics, summed: the
multinomial of the binned radiocarbon date mass over blocks, classes and centuries, the
beta-binomial of each rachis assemblage and the beta-binomial of each obsidian context.
Those see every item. But they allow for no model discrepancy, and they leave out the
onset and site records. ADR-0064 found that the onset record cannot be scored item by
item: phases vary too much at the same level of farming.

History matching judges a world by a different model: each record's summaries, with
their observation covariance plus a discrepancy that says how far a good world may
honestly sit from them. `history.log_likelihood` is that model's density, with each
group joint and the other targets on their own. It is the likelihood of Kennedy and
O'Hagan (2001) at the best input, which weighs records by what their summaries tell,
not by how many items they hold. Calibrating a model without its discrepancy gives
biased, overconfident parameters (Brynjarsdóttir and O'Hagan 2014).

Across the 365 consistent worlds of waves 7-9 the two rankings agree only loosely
(correlation 0.38). Under the summaries, the nearest worlds plant late. The leaders by
the summaries, rerun at the defaults over three seeds, include wave 9, run 180. It puts
in 0.20, 0.55, 0.61 and 0.48 against the record's 0.09, 0.52, 0.64 and 0.62, and its
summaries sit 6.0 log units nearer the record than run 208's (seed spread 0.3 at most).
By record, run 180 is nearer on the onset (3.8 log units), sites (1.1), obsidian (0.9)
and domestic barley (0.7), and 0.4 farther on the radiocarbon eras. Its item
diagnostics are 44 lower: 35 from radiocarbon, 7 from rachis and 2 from obsidian.

So the records do not pull apart. A world can plant as late as the onset record says
and sit as near or nearer every other record's summaries. What preferred early planting
was the radiocarbon date likelihood: about 35 log units over 938 units of binned date
mass, without discrepancy. That is detail of where and when dates fall within the
eras, which a model this coarse cannot honestly claim. Choosing by it gave the
radiocarbon record the deciding vote over the onset record, which the item diagnostics
do not cover at all.

`best` and `profile` now rank consistent worlds by `history.log_likelihood`, and the
item diagnostics are recorded beside it. Densities over different targets do not
compare, so waves ranked together must share their targets. `ensemble` follows the
ranking its best file records; files before this ADR keep their item ranking, and their
reports say so.

Round 8 adds a tenth wave of 300 worlds from the NROY space of waves 1-9 (0.10% of the
prior space): 169 are consistent with all five records, and the largest rise in
planting among them is 0.54. `best` over waves 7-10 (the top 16 by the summaries,
three seeds) keeps 10 consistent at the defaults and chooses wave 9, run 180, 0.5 log
units ahead of wave 9, run 179 (seed spread under 0.1). It becomes the registry
defaults: 20 parameters `calibrated`, with the consistent worlds' 10-90% ranges (over
534 consistent runs) in their notes. At the defaults it puts in 0.20, 0.55, 0.61 and
0.48 of the record's 0.09, 0.52, 0.64 and 0.62 (run 208: 0.40, 0.59, 0.61, 0.47);
domestic barley is 0.01, 0.11, 0.22 and 0.79 (record 0.01, 0.14, 0.34, 0.80). Its
largest single implausibility is 2.3 (Upper Mesopotamian sites 3,500-3,000 BCE), against
2.9 before, and its groups are 7.7, 6.2, 13.1, 12.8 and 1.8 (cutoffs 23.6, 18.5, 18.5,
22.0, 14.9; run 208: 6.9, 7.3, 14.2, 14.6, 9.4). It has 4.1 million people at 3,000
BCE. Its forager growth (0.023 per year) is near the top of the consistent range and
its sedentary growth only 1.14 times that: little Neolithic demographic transition.
Like run 208, it is one world among many, not an estimate.

The new defaults change the golden toy world (no cultivation packs, so forager growth
0.023 and dispersal 0.30 drive it): it fills its 2,401 cells and reaches 506,594 people
by 9,000 BCE (23,297 before), and its hash changes (31e6b2fb). The domestication-speed
test now pins the wild mutant frequency and introgression it was written for, as the
other cultivation tests do. With more people per agent the toy world costs 1,139 wasm
instructions per agent-step (1,178 before), and an M1 run 65 s (97 s).

Profiles chosen before this ADR keep their values: `obsidian_loss` is now emulated, so
the chosen world sets it (0.0040).

## ADR-0067 rationale (the transition's growth rates are not identified)

Open question 27 asked whether cultivation speeds population growth in M1. Rounds 3 to 8
each found consistent worlds with sedentary growth from under half to several times the
forager rate. Round 8's chosen world has a ratio of 1.14, and its forager growth is
near the top of the consistent range.

The consistent worlds learn nothing about the ratio. Over the 534 consistent runs of
waves 7-10, the ratio of sedentary to forager growth has the 10, 50 and 90% quantiles
0.28, 0.94 and 3.69, against 0.25, 1.0 and 4.0 under the prior (both rates uniform on
0.001-0.03). Each rate spans its prior: forager growth 0.0038 to 0.027, sedentary
0.0039 to 0.026. The summary log-likelihood is unrelated to either rate, to their ratio
or to their difference (Spearman |rho| at most 0.06), also among the worlds with slow
foragers. At the defaults, a profile of `sedentary_growth_rate` from 0.0023 to 0.093,
0.1 to 4 times the forager rate (three seeds each;
`calibration/runs/m1/round8/sedentary_growth_rate.json`), keeps every world consistent.
It moves the summary log-likelihood only between 4.7 and 5.2, about its seed spread, and
the item diagnostics by 11 log units.

The reason is structural. Logistic growth toward a capacity that itself grows at rate g
lags it by N/K = 1 - g/r. At the defaults, M1 capacity grows at most about 0.05% a year,
as cultivation spreads over millennia. A band's effective rate, between the forager and
sedentary rates by its cultivation effort (at most 0.65), is at least 0.9% a year: nearly
20 times faster, even with sedentary growth at the prior's floor. So worlds sit at 95-100% of capacity
from 11,000 BCE. Population at 3,000 BCE is 4.04, 4.10 and 4.11 million at sedentary
rates 0.1, 1.14 and 4 times the forager rate. Its rise from 1.0 million at 11,000 BCE
to 4.5 million at 5,000 BCE is cultivation raising capacity, whatever the rates. The
records measure stocks: dated activity, sites, crop shares and obsidian in use. Their
summaries cannot see a lag of a few percent. As ADR-0053 found for the radiocarbon
rise, the transition in M1 worlds is in capacity.

A rate would show only if it were near g, below the prior's floor, or if the record
measured vital rates. The juvenile index of rural cemeteries, the share of 5-19 year
olds among skeletons over 5 (Bocquet-Appel 2002), rises a few centuries after farming
arrives and is the transition's own signature. Testing against it needs age structure in
the bands and an operator for the index (#33). Until then the two rates stay at the
calibrated world's values, marked in the registry as unidentified. Later waves may drop
`sedentary_growth_rate` from the emulators' active factors as noise.

## ADR-0068 rationale (gross births and deaths; the juvenile index)

ADR-0067 found that the M1 records cannot see the Neolithic demographic transition: they
measure stocks, and populations follow capacity. The skeletal record measures something
else. The juvenile index of a cemetery, the share of 5-19 year olds among the dead over 5,
rises with the birth rate and hardly depends on mortality, and it rises a few centuries
after farming arrives (Bocquet-Appel 2002). It is a test of fertility, not of numbers.

The model's bands had a net growth rate and no births or deaths of their own. Two ways
to give them a juvenile index were weighed: age classes inside each band (counts of
0-4, 5-19 and 20+ year olds, ageing, deaths by age), or gross vital rates with a stable
population read off them. Age classes would read the index from simulated deaths, but
cost three arrays per agent, a new checkpoint format and age-aware split and merge. The
index is read here through stable-population theory instead, which is how it is
interpreted in the first place.

`Growth` now draws births at the band's fertility, linear in its cultivation effort as
its growth rate is, and deaths as the rest of the logistic change, so the expected net
is unchanged. Net growth beyond fertility is capped at it: a population cannot grow
faster than it bears children. Priors keep fertility (0.03-0.06) at or above growth
(0.001-0.03). At capacity, higher fertility means more births and as many more deaths,
not more people (`cultivators_bear_more_children_at_the_same_numbers`).

`juvenile.Operator` reads a cemetery's index from the people within a ring of its site
over the centuries of its use. Their birth rate is their people-weighted fertility,
exact since fertility is linear in effort. Their growth is the trend of log
people-years, up to the bands' intrinsic growth rate either way: people arriving or
leaving faster than bands grow are migrants, whose ages a stable population does not
describe. Rates are per year, converted from the registry's per-period ones. Their dead
are those of a stable population with that birth and growth rate (Lotka), integrated to
300 years so that low mortality keeps its old ages. Its mortality has the Siler shape of the average of five hunter-gatherer
populations (Gurven and Kaplan 2007, Table 2: a1 0.422, b1 1.131, a2 0.013, a3 1.47e-4,
b3 0.086), scaled to give that birth rate. The schedule reproduces the paper's own
survivorship (57% to 15; 30.8 years at birth), and its stationary birth rate, 1/e0 =
0.0325, is the forager fertility default. At it the stationary index is 0.184.

In the default M1 world, scored at Abu Hureyra in 500-year slices from 12,000 to 6,000
BCE, the index stays at 0.184 without a transition. With sedentary fertility 0.06 it
follows the cultivation effort, from 0.184 while people forage to 0.21-0.25 as they farm
(effort 0.17-0.46). A fertility transition shows in the index whether or not it shows in
numbers.

Sedentary fertility stays equal to forager fertility, the no-transition hypothesis
nested, until a record can calibrate it. The Levant juvenile indices are tabulated in
Guerrero, Naji and Bocquet-Appel (2008, doi:10.1007/978-1-4020-8539-0_4), a chapter
without an open licence. They join as the sixth record once the chapter is in hand
(#33). Until then the two fertility parameters change only noise in the five records,
so a wave before that record names them in `--inactive`: new registry parameters are
then sampled as noise, not emulated.

Gross deaths empty small cells more often: a lone survivor at capacity now dies about
one year in thirty, where the net change almost never removed one. That exposed a stale
state. Obsidian exchange runs at the start of a step, so a cell emptied during the step
kept its obsidian until the next exchange cleared it. Cells that end a step empty now
lose their obsidian at once (`Obsidian::abandon`). The exchange never reads empty cells,
so no history changes, only what the world shows between steps
(`only_people_hold_obsidian`).

The golden toy world changes on purpose: births and deaths are rounded separately, so
its history differs by chance (506,094 people by 9,000 BCE against 506,594; hash
f91b4dec). Two roundings per band cost 1,217.28 wasm instructions per agent-step against
1,139.23 (+6.9%). Native time is unchanged within noise (`bench --res 6 --years 20
--parts 4`: 13.4-14.6 ns per agent-step against 14.0-16.8, alternating runs).

## ADR-0069 rationale (checkpoints restore the model they were written with)

Parameters in a checkpoint were read as overrides of the registry defaults, so one
written before a parameter existed restored with its current default. For a nested
hypothesis added switched off and later calibrated on, that resumes an old run under a
different model: `cultivation_variability_weight` was added at 0 and is now 15,
`soil_erosion_rate` at 0 and now 2.1e-4. ADR-0065 fixed one parameter
(`innovation_share`). Others had the same gap (#31), and so would the next one.

A checkpoint stores every parameter of the engine that wrote it, so a missing parameter
means the checkpoint predates it. The oldest format that still restores is `OIKOSCK2`
(ADR-0049), whose parameters are listed once, from the registry at that commit (41).
Each of the nine added since has a rule saying how a checkpoint without it restores:

- `sedentary_growth_rate` the same as `growth_rate` (ADR-0053: one rate for all bands);
- `cultivation_variability_weight` 0 (ADR-0054 off);
- `soil_erosion_rate` 0 (ADR-0055: soil stays full, as `OIKOSCK2` checkpoints restore
  it); `soil_formation_rate` and `soil_critical` are then inert;
- `obsidian_loss` inert (a tracer, ADR-0060);
- `innovation_share` 1 (ADR-0065: individual learning);
- `fertility` 0, and `sedentary_fertility` the same: births and deaths not split
  (ADR-0068). Fertility is not inert, since splitting caps net growth at it and rounds
  births and deaths apart. So fertility 0 now means the growth before ADR-0068, the net
  change alone rounded once with the same draw. With it, the golden work unit
  reproduces every summary of the engine before ADR-0068 (506,594 people by 9,000 BCE).
  The branch costs 1,222.29 wasm instructions per agent-step against 1,217.28.

A "same as" rule reads the checkpoint's stored value, else that parameter's own rule or
default. A test fails when a registry parameter is neither in the first checkpoints nor
has a rule, so adding a parameter now means saying what it was before. No checkpoints
from those versions are known to exist (no volunteers yet), so nothing stored changes.
Schema versions in the header were the alternative. They would say that something
changed, not what the old model was, so each version would still need these rules.

## ADR-0070 rationale (fission by scalar stress; farming villages)

Fission was a placeholder: any group of 60 or more split with probability 0.2 a year,
and the daughter went to the least crowded of its cell and neighbors, or explored a neighbor. That caps every group at about 60 people, and a
farming cell of 200-750 people became 8-24 groups of about 31 (default M1 world, 7,000
BCE). Early Neolithic villages held hundreds.

Scalar stress is the reason groups split: decisions among more than about six units
overload a group, which splits unless it builds hierarchy (Johnson 1982). Face-to-face
farming communities reach that point at about 100-200 people (Kosse 1990). Hutterite
colonies, communal farmers without that hierarchy, fission with probability 0.5 at 127
people (95% CI 122-132) and 0.99 at 158. The same fit holds for early villages of the
Titicaca basin and elsewhere (Alberti 2014). Mobile foragers split far smaller, as camps
of a few families. A group's limit therefore moves, in proportion (log-linear), from
`fission_threshold` (60, still a placeholder at about twice a forager residential group)
to `village_fission_size` (127) with its cultivation effort. Above it the group splits
with the existing probability per year. A village that splits founds another in its
own cell while the land feeds more people (population below capacity). Only a full
cell places the daughter by crowding, as all fission did before.

In the default M1 world, farming groups now reach about 100 people (99th percentile;
up to 123, against a cap of about 63), with 27% fewer agents by 3,000 BCE and the same
population. The calibrated defaults fit the five records as well: summary
log-likelihood 5.1-5.3 over three seeds against 4.9-5.2 before, largest implausibility
2.32 both ways. So the defaults stand and no new wave is needed. The median farming
group stays near 35 people. In a full cell nobody grows, so the two halves of a split
village stay small, and nothing brings groups together. Aggregation into larger
villages (defence, exchange, ritual; Kuijt 2000) is the other half of #14. So are
settlement counts in the site-count operator, which still compares surveys with
settled people-years.

`village_fission_size` 0 is the fission before: one limit, daughters placed by crowding whatever the room. With
fertility 0 it reproduces the golden summaries of the engine before ADR-0068 (the
regression test now switches both off), and checkpoints from before restore with it
(ADR-0069). The golden toy world changes on purpose: it never cultivates, so its limit
stays 60, but daughters now stay in cells with room (506,203 people and 18,582 agents
by 9,000 BCE against 506,094 and 18,891; hash 6bc3bc28). Fewer, larger groups cut
agent-steps by 6% (787,203 against 837,656 in `instructions.sh`), at the same cost per
agent-step (1,219.88 wasm instructions against 1,222.29).

## ADR-0071 rationale (aggregation of farming villages)

Fission by scalar stress (ADR-0070) lets villages grow to their limit, but in a full
cell nobody grows. The two halves of a split village stayed small: the typical farming
group in the default M1 world held 31-36 people against a limit of about 104 at its
effort. Early farming villages drew people together, and this mechanism models that.

Drivers, as the project's standing rule asks of every mechanism:

- **Why people gather (social):** sedentism ties people to fields and stores, and a
  village gives defence, exchange partners, marriage partners and shared ritual. The
  late PPNB villages grew this way (Kuijt 2000). The pull grows with sedentism, so the
  probability scales with cultivation effort and mobile foragers never join.
- **Where they go (social attraction):** to the larger community. A group joins the
  largest group of its cell that is at least its own size, never a smaller one.
- **What stops them (cognitive):** the limit of face-to-face coordination. Decisions
  among more than about six units overload a group (Johnson 1982), and that matches the
  social-brain group size of about 150 (Dunbar 1993). A group joins only a village whose
  joined size stays below the scalar-stress limit of their pooled practice, where fission
  would split it again.
- **What they bring (social learning):** joined groups pool their practice, weighted by
  people, as merging bands already do. Imitation of successful neighbors is modelled in
  cultivation (ADR-0065).
- **Left out:** conflict and status within villages, kin-group (household) structure,
  and leaders who raise the limit by adding hierarchy. Those belong to emerging
  inequality (#16). Integration that let PPNB megasites pass the limit is not modelled,
  so villages here stay below it.

The first rule joined only the cell's largest group, which usually sits near its limit.
That raised the typical group only to 47-52. The rule above, the largest group with
room, gives villages in the default M1 world of 58-68 people (median group; 86-92 for
the group the typical farmer lives in), up to about 109 (99th percentile), with 31% fewer
agents by 3,000 BCE. The rate, 0.05 a year at full effort (a join within about twenty years of
settling), is a placeholder to calibrate. The calibrated defaults stay consistent with
all five records: largest implausibility 2.30-2.32 either way, summary log-likelihood
4.2-4.7 over three seeds against 5.1-5.3 without. That small drop comes through pooled
practice; the records see people-years, not group sizes. Settlement counts in the
site-count operator, the rest of #14, would let the survey record judge villages
directly.

Joining a village pools practice, and less farming lowers the limit, so the check uses
the pooled effort (`Aggregation::fits`).

The golden toy world never cultivates, so nobody joins: its summaries are unchanged and
only its hash moves, since the parameters are part of the state (539e0f22). A tile with
no farmers is skipped after one scan. Cost: 1,239.14 wasm instructions per agent-step
against 1,219.88 in the toy world. A full M1 world takes 55.1 s against 54.7 s on one
core: every farming group draws once a year, and there are fewer groups.

## ADR-0072 rationale (survey sites are settlements)

Surface surveys count sites: places where people lived long enough, and built enough,
to leave pottery and rubble that a walker finds. The site-count operator (ADR-0059)
compared the survey counts with settled people-years, cultivator people-years per cell
and century. That was the best available while bands had a fixed size. Since villages
form (ADR-0070, ADR-0071), a world's groups are its settlements. So the model can count
what surveyors count, and the era shares of the survey record now judge settlements.

How people live shapes what they leave. A settled group, living year-round in houses
near its fields and stores, leaves a site. A mobile camp leaves little that surveys
recover. So each group counts as far as it is settled: its cultivation effort, the
model's measure of sedentism. Settlement-years per cell and century are summed in fixed
point by the engine (`World::settlements`), so they are exact whatever the partitions
and cost a fraction of reading every group. Left out: larger villages are easier to
find, and reoccupied places count once in surveys, not once per occupation. Both would
need survey-specific detection models.

In the default M1 world the change is small: settlements track settled people closely
(villages of 58-68 people). The two survey groups score 7.1-7.2 and 13.2-13.8 over
three seeds (cutoff 18.5), against 6.2 and 13.1 for the same world judged by settled
people-years in round 8. The largest implausibility is 2.31-2.36 (Upper Mesopotamian
sites, 3,500-3,000 BCE), and the summary log-likelihood is 3.8-4.1. The defaults stay
consistent with all five records.

Wave files before this ADR judged sites by settled people-years, under the same target
names. Wave, profile and best files now record `sites` ("settlements"). `candidates`
refuses to rank waves of different site models together, as it does waves of different
targets (ADR-0066). `wave --previous` and `best` refuse waves of the old model: their
emulators would rule out parameters, and their scores pick candidates, by what the
engine no longer compares. The next wave starts a new round.

## Notes

- H3 cells are not exactly equal-area; mechanisms use per-cell areas, never assume
  uniform cells.
