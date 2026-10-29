# Calibration

Observation operators and, later, the calibration and assimilation loops built on them
([docs/05](../docs/05-calibration-assimilation.md)). Operators map what a simulated
world did to what the historical record measures, biases included, and score it
against observation tables built by [`data/`](../data/README.md). They read worlds
through the Python bindings (`oikos`) and never change them.

| Module | What it is |
|---|---|
| `oikos_cal/record.py` | `activity(world, end)`: people-years (and cultivator people-years) per cell and century while a world runs |
| `oikos_cal/radiocarbon.py` | Radiocarbon-density operator (ADR-0050): taphonomic loss, calibration kernel from IntCal20, site-phase bins, multinomial over centuries per block; `synthetic_dates` for validation |
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

Operators run on the calibration host, so they use NumPy freely; work-unit results are
validated by the engine's state hash, never by an operator's output.
