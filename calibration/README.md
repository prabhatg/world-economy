# Calibration

Observation operators and the calibration loops built on them
([docs/05](../docs/05-calibration-assimilation.md)). Operators map what a simulated
world did to what the historical record measures, biases included, and score it
against observation tables built by [`data/`](../data/README.md). They read worlds
through the Python bindings (`oikos`) and never change them.

| Module | What it is |
|---|---|
| `oikos_cal/record.py` | `activity(world, end)`: people-years, cultivator people-years, domestic-crop people-years, obsidian in use per source group and settlement-years (ADR-0072), per cell and century while a world runs |
| `oikos_cal/radiocarbon.py` | Radiocarbon-density operator (ADR-0050): taphonomic loss, calibration kernel from IntCal20, site-phase bins, multinomial over centuries per block; `synthetic_dates` for validation |
| `oikos_cal/domestication.py` | Domestication-timing operator (ADR-0051): domestic share of the crop sown near each dated rachis assemblage, beta-binomial against its counts |
| `oikos_cal/sites.py` | Site-count operator (ADR-0059): aoristic survey site counts per era in each survey's landscape, and a world's settlements (ADR-0072; settled people-years before) dated through the surveys' own period scheme |
| `oikos_cal/obsidian.py` | Obsidian sourcing operator (ADR-0061): share of Cappadocian obsidian in use around each sourced assemblage over its period, beta-binomial against its counts; contexts no chain of land cells joins to a volcano (Cyprus) are dropped |
| `oikos_cal/juvenile.py` | Juvenile-index operator (ADR-0068): a cemetery's share of 5-19 year olds among its dead over 5, from the world's fertility and local growth there, through a stable population with hunter-gatherer Siler mortality; the Levant record is to come (#33) |
| `oikos_cal/onset.py` | Cultivation-onset operator (ADR-0064): people-weighted cultivation effort at dated site phases, against the effort their crops among charred plant remains imply (anchored on Pottery Neolithic phases) |
| `oikos_cal/screening.py` | Morris elementary-effects screening of the registry's free parameters (ADR-0052) |
| `oikos_cal/emulator.py` | Gaussian-process emulators (linear trend, ARD length scales, nugget) and Latin hypercubes |
| `oikos_cal/history.py` | History matching: targets, implausibility (one by one and for correlated groups), waves, sampling the space not yet ruled out |
| `oikos_cal/targets.py` | Summaries of the record for history matching: radiocarbon eras, domestication eras, site eras, obsidian zones, onset eras |
| `oikos_cal/ensemble.py` | Ensembles for the viewer (ADR-0063): the calibrated defaults and the best consistent worlds of a `best` record, run as work units by the `oikos` CLI with outputs, and a `catalog.json` |
| `oikos_cal/m1.py` | The M1 problem: operators, targets, worlds run in worker processes |
| `oikos_cal/__main__.py` | `python -m oikos_cal screen / wave / best / profile / ensemble / report` |
| `runs/m1/` | Every screening and wave run: points, parameters, seeds, outputs, targets, revision |
| `tests/` | pytest: synthetic worlds with known answers; M1 tests run when p3k14c, IntCal20 and the M1 packs are cached |

```sh
source engine/scripts/env.sh
(cd engine && maturin develop --release -m crates/oikos-py/Cargo.toml)   # the oikos module
pytest calibration/tests
```

From the repository root, with `PYTHONPATH=calibration:data`:

```python
import oikos, pyarrow.parquet as pq
from oikos_cal import radiocarbon, record
from oikos_data import radiocarbon as dates, region

cells = region.load("m1-swasia").cells()
curve = dates.inputs(offline=True)[2]
op = radiocarbon.Operator(pq.read_table("data/build/m1-swasia.radiocarbon_dates.parquet").to_pydict(), curve, cells)
w = oikos.World.from_packs([f"data/build/m1-swasia.{k}.oikospack" for k in ("geometry", "climate", "coastline", "hydrology")], seed=1)
a = record.activity(w, end=-3000)            # ~15 s for 9,000 years of M1
op.log_likelihood(a.visible(0))              # higher is better; compare histories, not absolute values
```

Screening and history-matching waves, from `calibration/` with `PYTHONPATH=.:../data`
(each run is a 9,000-year M1 world, about 30 s on one core; runs use all cores but one):

```sh
python -m oikos_cal screen --trajectories 10 --out runs/m1/screening.json
python -m oikos_cal wave --runs 220 --screening runs/m1/screening.json --out runs/m1/wave1.json
python -m oikos_cal wave --runs 220 --previous runs/m1/wave1.json --out runs/m1/wave2.json
python -m oikos_cal wave --runs 300 --previous ... --activate adoption_rate --out runs/m1/wave8.json
python -m oikos_cal report runs/m1/wave1.json runs/m1/wave2.json
python -m oikos_cal best runs/m1/wave1.json runs/m1/wave2.json --out runs/m1/best.json
python -m oikos_cal profile obsidian_loss 0.002 0.003 0.004 --out runs/m1/obsidian_loss.json
python -m oikos_cal ensemble runs/m1/round8/best.json --out ../viewer/data/m1   # needs the release CLI
```

`best` reruns the best consistent runs with only their emulated parameters set and the
others at their defaults, with three seeds, and records which stay consistent; the
chosen world becomes the registry defaults (ADR-0057, ADR-0059). Runs are ranked by how
near their summaries sit to the record under history matching's own error model, every
record's observation covariance plus discrepancy (`history.log_likelihood`, ADR-0066);
the item log-likelihood diagnostics, which allow for no discrepancy, are recorded beside
it. Waves ranked together must be judged against the same targets.
`wave` and `report` refit every earlier wave's emulators. Searching for their
hyperparameters is the costly part, so it runs in worker processes (`--workers`), and the
hyperparameters found are kept in `emulators/` beside each wave file (committed with it),
keyed by the wave's runs and the search (`emulator.FIT`); while those stay the same the
emulators are rebuilt from there, exactly as the search left them (#32).
A later wave appends parameters new to the registry as active factors; `--activate`
also makes active a factor the earlier waves varied as noise, when a new mechanism makes
it matter (ADR-0065). NROY points take such factors uniformly, as the earlier waves did.
`--inactive` keeps parameters new to the registry as noise when no record of the wave
sees them yet (`fertility` and `sedentary_fertility` until the juvenile-index record
joins, ADR-0068).
`profile` varies one parameter with every other at its default and chooses the
consistent value whose summaries sit nearest (files before ADR-0066 chose by the item
log-likelihoods): for a parameter a new record
constrains but that leaves the world's history alone (ADR-0061).

Round 1 (ADR-0052) is in `runs/m1/`, round 2 (ten radiocarbon eras judged one by one
and together, 27 free parameters; ADR-0056) in `runs/m1/round2/`, with `best.json`, the
source of the calibrated defaults (ADR-0057). Round 3 (a third wave refocused on 17
factors, ADR-0058) is in `runs/m1/round3/`; `wave --previous ... --screening` refocuses.
Round 4 (site counts added, ADR-0059) is in `runs/m1/round4/`, with `best.json`, the
source of the current calibrated defaults (`best` given only wave 4, the one judged on
the sites).
Round 5 (obsidian zones added, ADR-0061) is in `runs/m1/round5/`: `wave5.json`,
`best.json` (whose best world ties the defaults, so it is not adopted) and
`obsidian_loss.json`, the profile that calibrates `obsidian_loss` at the defaults.
Round 6 (cultivation-onset eras added, ADR-0064) is in `runs/m1/round6/`: `wave6.json`
and `best.json` (wave 6, run 236, the defaults before round 7).
Round 7 (social learning of cultivation, ADR-0065) is in `runs/m1/round7/`:
`innovation_share.json` (a profile at the round-6 defaults), `wave7.json`, `wave8.json`
(`adoption_rate` activated), `wave9.json` and `best.json` over waves 7-9 (wave 9, run
208, the defaults before round 8; ranked by the item log-likelihoods).
Round 8 (ranking by the summaries, ADR-0066) is in `runs/m1/round8/`: `wave10.json` and
`best.json` over waves 7-10, the source of the current calibrated defaults (wave 9,
run 180), and `sedentary_growth_rate.json`, a profile at those defaults from 0.1 to 4
times the forager rate: flat, the records do not identify it (ADR-0067).
Waves up to round 8 judged sites by settled people-years; since ADR-0072 the engine
counts settlements, so `wave --previous` and `best` refuse them (and `wave --screening`
a screening of the old model).
Round 9 (ADR-0073), a new history, is in `runs/m1/round9/`: `screening.json` (all 33
free parameters), `wave1.json` (Latin hypercube over the prior, the 24 factors whose
effect reaches one tolerance emulated: `--threshold 1.0 --max-active 24`), `wave2.json`,
`wave3.json` and `best.json` over them, whose best world does not beat the calibrated
defaults, so they stay. Waves number afresh in a new round; a wave's NROY points take
its number as seed unless `--seed` is given (round 9's wave 3, drawn with wave 2's seed,
repeats 63 of its points, run again with new world seeds). The onset record's tables are downloaded by hand (`data/README.md`); without
them the M1 problem cannot be built.

Operators run on the calibration host, so they use NumPy freely; work-unit results are
validated by the engine's state hash, never by an operator's output.
