# Calibration

Observation operators and the calibration loops built on them
([docs/05](../docs/05-calibration-assimilation.md)). Operators map what a simulated
world did to what the historical record measures, biases included, and score it
against observation tables built by [`data/`](../data/README.md). They read worlds
through the Python bindings (`oikos`) and never change them.

| Module | What it is |
|---|---|
| `oikos_cal/record.py` | `activity(world, end)`: people-years, cultivator people-years and domestic-crop people-years per cell and century while a world runs |
| `oikos_cal/radiocarbon.py` | Radiocarbon-density operator (ADR-0050): taphonomic loss, calibration kernel from IntCal20, site-phase bins, multinomial over centuries per block; `synthetic_dates` for validation |
| `oikos_cal/domestication.py` | Domestication-timing operator (ADR-0051): domestic share of the crop sown near each dated rachis assemblage, beta-binomial against its counts |
| `oikos_cal/screening.py` | Morris elementary-effects screening of the registry's free parameters (ADR-0052) |
| `oikos_cal/emulator.py` | Gaussian-process emulators (linear trend, ARD length scales, nugget) and Latin hypercubes |
| `oikos_cal/history.py` | History matching: targets, implausibility (one by one and for correlated groups), waves, sampling the space not yet ruled out |
| `oikos_cal/targets.py` | Summaries of the record for history matching: radiocarbon eras, domestication eras |
| `oikos_cal/m1.py` | The M1 problem: operators, targets, worlds run in worker processes |
| `oikos_cal/__main__.py` | `python -m oikos_cal screen / wave / report` |
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
python -m oikos_cal report runs/m1/wave1.json runs/m1/wave2.json
python -m oikos_cal best runs/m1/wave1.json runs/m1/wave2.json --out runs/m1/best.json
```

`best` reruns the best consistent runs (by both log-likelihoods) with only their emulated
parameters set and the others at their defaults, with three seeds, and records which
stay consistent; the chosen world becomes the registry defaults (ADR-0057).

Round 1 (ADR-0052) is in `runs/m1/`, round 2 (ten radiocarbon eras judged one by one
and together, 27 free parameters; ADR-0056) in `runs/m1/round2/`, with `best.json`, the
source of the calibrated defaults (ADR-0057).

Operators run on the calibration host, so they use NumPy freely; work-unit results are
validated by the engine's state hash, never by an operator's output.
