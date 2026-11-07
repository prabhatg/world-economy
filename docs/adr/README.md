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
