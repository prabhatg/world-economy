# 10. Roadmap

Principle: **vertical slices** — each milestone runs the whole loop (engine → mechanisms
→ outputs → calibration → viewer) end to end, at increasing breadth.

## M0 — Foundations

**Status:** first slice implemented in [`engine/`](../engine/README.md) — workspace,
Philox RNG (Random123 known answers), integer probabilities, pinned deterministic math,
H3 grid, super-individual agents with split/merge, phased partitioned step,
checkpoint/fork, JSON work units, `oikos` CLI + benchmark, persistent partitions on a
rayon pool (13 ns/agent-step on 4 cores), local verification gate; golden run
reproduced bit-for-bit on x86_64 and wasm32; pluggable mechanisms with a validated,
multi-rate schedule and cache-tiled decide phase (no performance cost vs. the hard-coded
step, golden unchanged); parameter registry with units, ranges, priors, status and
sources, enforced on every override; Zarr/Parquet outputs, byte-identical native vs
wasm and verified with zarr-python and pyarrow. Cross-host verified: x86_64 Linux
and aarch64 macOS (Apple Silicon) give identical golden, wasm and dense hashes.
Python bindings (`oikos` module, PyO3 + maturin, abi3): create, run, fork, checkpoint
and read worlds, run work units, sample priors; tested in the gate to reproduce the
golden hash. Cost-weighted partitions, re-balanced as population moves (up to 2.8×
faster on hotspot worlds, results unchanged), and an exact instruction-count metric
(wasm fuel). Remaining: run the gate on Windows.

- Rust workspace skeleton (core, grid, store, sched, io, params, sim, py).
- H3 grid, SoA store with super-individuals and split/merge, phase scheduler,
  Philox RNG, checkpoint/fork, Zarr/Parquet output.
- Two-tier numerics (integer conserved quantities, restricted `f64`), deterministic
  math crate.
- Build targets: x86_64, aarch64, **wasm32**.
- Local verification gate on our compute: determinism across partition counts *and*
  across targets, conservation property tests, golden-run hashes.
- Work-unit format v0 (segment run: state in → state + summaries out).
- Toy mechanism: logistic population growth on a grid with diffusion. Python can run,
  fork and read results.
- **Exit:** bit-identical results across 1/2/4/8 partitions and across x86_64 /
  aarch64 / wasm32; fork/restore works; a work unit runs in a browser tab.

## M1 — Neolithic Southwest Asia (12,000–3,000 BCE)

**Status:** started. Region chosen (ADR-0040); forcing-pack format and engine inputs
(ADR-0041: worlds on a geometry-pack region, work units and checkpoints naming their
packs); `data/` pipelines with locked pack ids; geometry, terrain, climate
(CHELSA-TraCE21k, ADR-0042), coastline (ADR-0043: moving coastlines; flooded cells
are uninhabitable), hydrology (ADR-0044: rivers, floodplains, lakes) and soils
(ADR-0045) packs for M1; radiocarbon observation tables from p3k14c with IntCal20
(ADR-0046). First mechanism on real forcing: forager carrying capacity from climate,
dry land and aquatic resources (ADR-0047), and forager dispersal by budding into less
crowded land (ADR-0048), so M1 worlds fill the region as a wave of advance.
Cultivation and domestication (ADR-0049): bands carry cultivation effort and crop
domestication, driven by competing hypotheses nested as weights; M1 worlds start with
foragers across the region.
First observation operator (ADR-0050): radiocarbon date density, scoring recorded
worlds against binned, calibrated p3k14c dates with taphonomy and calibration
forward-modelled (`calibration/`). Domestication timing (ADR-0051) against barley
rachis assemblages: the record and M1 worlds both favour protracted domestication over
rapid. First calibration waves (ADR-0052): Morris screening and two history-matching
waves with Gaussian-process emulators leave 22% of the prior space, ruling out rapid
domestication; radiocarbon summaries do not yet constrain. Neolithic demographic transition
(ADR-0053) as a nested hypothesis; it does not explain the radiocarbon rise. Stable-
climate hypothesis of cultivation (ADR-0054), nested and off: untestable on
century-resolution climate. Land degradation (ADR-0055), nested and off: fields wear
soils down, turning the farming boom into a bust. Calibration round 2 (ADR-0056): finer
radiocarbon eras judged together; 4.7% of the prior space survives, and the best
worlds erode their soils. The best consistent world is the registry default (ADR-0057). Round 3 (ADR-0058): a
third wave on 17 factors leaves 3.6%; sedentary growth is not identified. Site-count
operator (ADR-0059): survey settlements of Anatolia and Upper Mesopotamia, dated
aoristically; with them a fourth wave leaves 2.3%, and its best world is the new
registry default. Down-the-line exchange of obsidian as a tracer (ADR-0060), from the
DObsiSS volcanoes. Obsidian sourcing operator (ADR-0061): sourced assemblages judged in
zones of distance to the sources; with them a fifth wave leaves 0.8%, the defaults
stay (the best world only ties them) and the obsidian loss is calibrated at them.
Cultivation-onset operator (ADR-0064): crops among charred plant remains of dated site
phases, anchored on the Pottery Neolithic; they rule out the defaults, a sixth wave
leaves 0.23%, and its best world became the registry default. No world rose from
little planting to much as the record does (#30).
Social learning of cultivation (ADR-0065): bands learn planting mostly from those around
them; three more waves find a consistent world that plants as the record does, and the
best-fitting world, which learns socially, became the registry default; it still
planted too early. Ranking worlds by their summaries (ADR-0066), as history matching
judges them, shows the records agree: a tenth wave and the new ranking choose a world
that plants late, as the onset record does, and it is the registry default.
The records do not identify the Neolithic demographic transition's growth rates
(ADR-0067): populations follow carrying capacity; age structure and the juvenile
index would test it (#33). Births and deaths are now gross events with fertility
moving with cultivation, and a juvenile-index operator reads them (ADR-0068); the
Levant cemetery record is to come.
Fission by scalar stress (ADR-0070): farmers live in villages of up to about 127 people,
foragers in camps; the calibrated defaults fit as well. Farming groups gather into
villages up to that limit (ADR-0071), and survey sites are compared with settlements
(ADR-0072). Calibration round 9 (ADR-0073) starts a new history for that model: a new
screening and three waves leave 1.35% of the prior and keep the calibrated defaults,
which no candidate beats; no record yet sees village size or aggregation.
Wealth inequality within groups (ADR-0074): households pass on wealth more where farmland
is owned, so inequality rises as farming closes the land in; the house-size Ginis of five
dated sites (Kohler et al. 2017) join the record, and the calibrated defaults fit them.
An endemic infection within groups (ADR-0075) burdens large, settled villages and spares
foragers' camps; the calibrated defaults fit as well, and heavy disease (four times the
default mortality) is ruled out by the domestic-barley record.
Bands learn cultivation from neighboring cells too, and can copy what visibly pays
(ADR-0076); with neighbors the defaults fit slightly better, and success bias waits for
calibration round 10, fitted with the adoption rates it trades off against. This completes the first versions of
inequality, disease and social learning (#16). Calibration round 10 (ADR-0077) screens
the 43 free parameters of that model and history matches them in three waves, the
house-size record left out until its licensing is settled: 0.70% of the prior is left,
the calibrated defaults stay, and the records do not identify success bias, which stays 0.
Survey site areas join as a seventh record (ADR-0078, #34), and a fourth wave judges them
with the house-size Ginis: the defaults stay, their villages (about 50 people) smaller
than the record's (126-218), which only worlds whose groups seldom disperse approach.
Radiocarbon by subregion (ADR-0079, #35) and onset contrasts by wild-cereal stands
(ADR-0080, #36) follow. The subregions show what the region-wide eras hid, a periphery
that fills too early and a core that rises too late (open question 37), and rule out
the defaults; the contrasts constrain nothing new, so success bias stays unidentified.
A sixth wave judged on every record gives new calibrated defaults (ADR-0081): wave 6
run 216, with fertility and the wealth parameters at their literature values.
Settlements of several groups (ADR-0082, #39): groups keep their scalar-stress limit,
and integrative institutions and hierarchy hold several of them together in one
settlement, which pays for its size in infection; at the calibrated defaults, two or
more groups' worth gives villages of 110-160 people, inside the record's range, though
still level through time. A seventh wave and a profile calibrate it (ADR-0083): four
groups' worth fits best, though any value above one fits as well within the spread
between seeds, and villages are about 100 people, as a cell's land holds one town and
the small groups it sheds; hierarchy is not identified.
The forcing has the African Humid Period (ADR-0084, #44): rain from Eastern Saharan
reconstructions, timed and scaled by parameters; refitted over Egypt to the Western
Desert's own estimates (ADR-0093), it settles the desert while it lasts and brings
northeast Africa's 25% and 50% dates to the record's; with dates out of sight on drowned
land and under the delta's and the alluvium's silt (ADR-0095), the subregion is judged.
Floodplains can be farmed with domestic crops (ADR-0085), opening the Mesopotamian
alluvium, but the model's farming does not cross Sinai to the Nile.
Lesions of infection on Natufian and Neolithic skeletons are a record (ADR-0086, #42):
the defaults are consistent with their rise, which leans toward foragers who spread
less infection.
Bands herd sheep and goats (ADR-0087, #37), a second practice within one effort budget,
and the size of sheep in dated faunal assemblages is a record (ADR-0088). Round 11
(ADR-0090), judged on every record, keeps the calibrated defaults and calibrates the
herding parameters: herds turn domestic at 0.035 per period at full effort, and the
effort aimed at herds is 0.12 of the land's herding potential. Round 13 (ADR-0104), with
northeast Africa judged, keeps them again.
Bands sow emmer and einkorn with their crop (ADR-0101, #38), each selected at its own
fraction of the crop's selection, and the wheats' rachis records join barley's; einkorn's
selection is calibrated by profile. Seed exchange along kinship and exchange ties
(ADR-0102) and the wild cereal gathered beside the fields (ADR-0103) are in; round 14
(ADR-0105) keeps the calibrated defaults and calibrates seed exchange at 0.3 by profile.
The oases' springs wait (#48).
Egypt's first domestic animals are recorded (ADR-0113, #51) and judged (ADR-0116): a
reserve of animals (ADR-0114), stock exchange, floodplain grazing and herders' longer
moves (ADR-0115) are calibrated on by profile, and herds reach the desert's
6th-millennium sites and the Nile's 5th-millennium villages, every record consistent.
Their crops are judged too (ADR-0117): none in the 6th millennium or the deserts, crops in
the Nile valley, the delta and the Fayum in the 5th, as the defaults have it. Round 15
(ADR-0120) judges both and finds no world nearer the record than the calibrated defaults,
which stay; Egypt's crops now bound the pace of domestication from below. Hidden Valley
(#52) remains; the Fayum's early farming without seed exchange was the merge bug's (ADR-0121).
What comes next for the model's fidelity, records first, is ordered in the
[model fidelity plan](15-model-fidelity.md).
Work items are GitHub issues under the M1 milestone.

- Region: Levant, Anatolia, Zagros, Mesopotamia (res 5 H3): res-5 cells with centers
  in 24–42°N, 26–58°E, 22,198 cells incl. Cyprus, the paleo-Gulf basin and the Nile valley (ADR-0040).
- Forcing: CHELSA-TraCE21k climate, sea level, terrain, hydrology, soils, depth to bedrock
  (ADR-0106: thin soils erode first; no M1 field wears below the critical share before
  3000 BCE at the calibrated defaults), decadal climate anomalies from NGRIP (ADR-0107:
  the 8.2 ka event; against real century-to-century swings the stable-climate
  hypothesis' weight falls from 7.2 to 2, and the fit is as good as before), and where
  wild emmer and einkorn grow, from GBIF's records (ADR-0108).
- Mechanisms: foraging ecology; cultivation & domestication co-evolution; sedentism and
  storage; Neolithic demographic transition; band/village formation and fission;
  exchange networks (obsidian as tracer!); emerging inequality; simple disease;
  social learning of practices.
- Observation operators: radiocarbon density (p3k14c), site counts, obsidian sourcing
  distance-decay, domestication timing, cultivation onset.
- Calibration: sensitivity analysis + first history-matching waves on cloud batch.
- Viewer v0: TS + deck.gl map playing back fields and settlements over time.
  *Implemented (ADR-0063):* `viewer/` plays back an ensemble of plausible M1 worlds
  (`python -m oikos_cal ensemble`). The map shows one world's people, farming, soil
  loss or obsidian; the charts show every world.
- **Coordinator v0** (work generator, dispatcher, result ingest, validator) with cloud
  spot workers as the first client class. First step done (ADR-0096, #46): a
  calibration world is a work unit whose result carries its calibration record, so it
  runs wherever the `oikos` CLI runs and is scored on the calibration host. Second
  step done (ADR-0097, #21): `oikos-coord serve` leases units to hosts, validates
  their results by quorum on the state hash with adaptive replication, keeps each
  run's outputs as an archive and serves forcing packs by content hash; `oikos-coord
  work` is the headless host; `oikos_cal` submits waves through it (`--coordinator`).
  Third step done (ADR-0099, #47): hosts can record only the sums over the zones the
  operators read, 1.7 MB instead of 91 MB, small enough for volunteers to upload
  (`--reduced`). Next: spot VMs as hosts.
- **Browser client v0**: closed alpha, runs M1 work units and shows "your world".
  Client done (ADR-0100, #22): `viewer/volunteer.html`, served by the coordinator
  (`oikos-coord serve --client viewer/dist`), runs units in a Web Worker on the wasm
  engine the gate checks, with results bit-identical to native hosts, and shows the
  simulated year, the world it computed and whether a second computer confirmed it.
  Next: a coordinator volunteers can reach (TLS, accounts), then the alpha.
- **Exit:** plausible emergence of farming and villages in the right windows across
  ensembles, with quantified fit; competing domestication hypotheses compared;
  first calibration wave computed partly by alpha volunteers.

## M2 — Calibration & assimilation machinery + public volunteer launch
- Emulators (GP/NN), SBI pipeline, localized particle-filter assimilation on M1.
- Knowledge-ingestion pipeline v1 feeding the parameter registry; citizen-science
  review tool for extracted parameters.
- Behavior v1 (ADR-0062): first distilled, integer-inference policy network for one
  M1 archetype decision (move or stay, cultivate or forage), its free parameters
  calibrated against the M1 record and compared with the hand-written rule.
- Ensemble analyst v0: pathway clustering and scenario discovery over calibration
  runs, with questions answered from the analysis store.
- Desktop clients (Win/macOS/Linux) and headless/Docker client; accounts, teams,
  credits; adaptive replication.
- Nonprofit set up (fiscal sponsor), charter, advisory board; **public launch**.
- Android client (charging + Wi-Fi); iOS explore-first client.

## M3 — Global Holocene to 1 CE
- All continents at res 4; independent agricultural origins (Fertile Crescent, China,
  Mesoamerica, Andes, New Guinea, Sahel…) as validation; chiefdoms → early states;
  bronze, writing, cities, long-distance trade; Seshat validation.

## M4 — 1 CE–1800
- Empires, world religions, trade networks (Silk Road, Indian Ocean, Atlantic),
  pandemics, Columbian exchange, early modern finance, colonial extraction.
- First hindcast experiments.

## M5 — 1800–2026 and the digital twin
- Industrialization, demographic transition, nation-states, globalization, digital
  economy, AI. Dense assimilation; synthetic modern population; twin-2026 v1.
- AI as a class of agents and capital (task-based production; ADR-0062); decision
  packs for optional LLM notable actors.
- Multi-node scaling work (MPI, GPU kernels).

## M6 — Policy laboratory
- Policy DSL, scenario ensembles, RDM analysis, policy & adoption-pathway search,
  hindcast policy validation, public web viewer and reports.

A small **modern-era policy toy** (synthetic country) may be built alongside M2 to
exercise the policy DSL early, without waiting for M5.
