"""The domestication operator on the real M1 barley record (runs when the rachis table's
supplement is cached and, for scoring worlds, the M1 packs and the bindings): the two
competing domestication hypotheses of ADR-0049 compared."""

import io

import numpy as np
import pyarrow.parquet as pq
import pytest

from oikos_cal import domestication as dom
from oikos_data import DATA, rachis, raster, region


@pytest.fixture(scope="module")
def m1():
    r = region.load("m1-swasia")
    try:
        inputs = rachis.inputs(offline=True)
    except raster.MissingInputs:
        pytest.skip("rachis supplement not cached; run: python -m oikos_data build rachis m1-swasia")
    return r.cells(), pq.read_table(io.BytesIO(rachis.build(r, *inputs))).to_pydict()


def best_logistic(table, selection):
    """Best score of a regional logistic rise at `selection` per year, over its midpoint."""
    y, k, n = (np.asarray(table[c], dtype=float) for c in ("median_year", "k_nonshattering", "n_rachis"))
    return max(dom.beta_binomial(k, n, (1 / (1 + np.exp(-selection * (y - h)))).clip(1e-3, 1 - 1e-3), 2.0).sum()
               for h in range(-9_500, -5_900, 100))


def test_the_barley_record_is_a_protracted_domestication(m1):
    _, t = m1
    scores = {s: best_logistic(t, s) for s in (0.001, 0.002, 0.003, 0.005, 0.01, 0.05)}
    assert max(scores, key=scores.get) in (0.002, 0.003), scores
    assert scores[0.05] < max(scores.values()) - 30, "decades-to-centuries domestication is ruled out"


def test_m1_worlds_with_protracted_domestication_fit_better_than_rapid(m1):
    oikos = pytest.importorskip("oikos", reason="build the bindings: maturin develop -m engine/crates/oikos-py/Cargo.toml")
    from oikos_cal import record

    cells, t = m1
    packs = [DATA / "build" / f"m1-swasia.{k}.oikospack" for k in ("geometry", "climate", "coastline", "hydrology")]
    if not all(p.exists() for p in packs):
        pytest.skip("M1 packs not built; run python -m oikos_data build <pack> m1-swasia")

    def score(selection):
        w = oikos.World.from_packs([str(p) for p in packs], seed=1, params={"selection_coefficient": selection})
        a = record.activity(w, end=-6_000)
        return dom.Operator(t, cells, a.window).log_likelihood(a)

    protracted, rapid = score(0.005), score(0.05)  # Purugganan and Fuller 2011; Hillman and Davies 1990
    print(f"M1 domestication: protracted {protracted:.1f}, rapid {rapid:.1f}")
    assert protracted > rapid + 30
