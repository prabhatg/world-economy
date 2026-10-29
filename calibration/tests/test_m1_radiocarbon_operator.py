"""The radiocarbon operator on the real M1 record (runs when p3k14c and IntCal20 are
cached, and, for scoring a world, the M1 packs and the bindings)."""

import io

import numpy as np
import pyarrow.parquet as pq
import pytest

from oikos_cal import radiocarbon as rc
from oikos_data import DATA, raster, region
from oikos_data import radiocarbon as pipeline


@pytest.fixture(scope="module")
def m1():
    r = region.load("m1-swasia")
    try:
        inputs = pipeline.inputs(offline=True)
    except raster.MissingInputs:
        pytest.skip("p3k14c or IntCal20 not cached; run: python -m oikos_data build radiocarbon_dates m1-swasia")
    dates = pq.read_table(io.BytesIO(pipeline.build(r, *inputs)[0])).to_pydict()
    return r.cells(), dates, rc.Operator(dates, inputs[2], r.cells())


def test_the_intcal20_kernel_keeps_the_mass_of_interior_centuries(m1):
    *_, op = m1
    rows = op.kernel[:, 15:70].sum(axis=2)  # 10,500 to 5,000 BCE: wide errors on late-glacial plateaus leak earlier
    assert np.abs(rows - 1).max() < 1e-3
    assert (op.kernel.max(axis=2) < 0.9).all(), "calibration always spreads a date beyond its century"


def test_heavily_dated_sites_count_by_phase(m1):
    _, dates, op = m1
    w = dict(zip(op.obs.lab_ids, op.obs.weights))
    catal = [w[lab] for lab, site in zip(dates["lab_id"], dates["site_name"]) if site == "Çatalhöyük East" and lab in w]
    assert len(catal) > 200 and sum(catal) < 20, "over 200 dates, about a dozen 200-year phases"
    assert op.obs.weights.sum() < 0.4 * len(op.obs.weights)
    assert len(op.obs.blocks) > 30


def test_the_record_prefers_neolithic_growth_to_stasis_or_decline(m1):
    cells, _, op = m1
    t = op.settings.true.midpoints()
    trend = lambda r: np.ones((len(cells), len(t))) * np.exp(r * (t - t[0]))
    assert op.log_likelihood(trend(1e-4)) > op.log_likelihood(trend(0)) + 10 > op.log_likelihood(trend(-5e-4)) + 10


def test_an_m1_world_is_scored(m1):
    oikos = pytest.importorskip("oikos", reason="build the bindings: maturin develop -m engine/crates/oikos-py/Cargo.toml")
    from oikos_cal import record

    cells, _, op = m1
    packs = [DATA / "build" / f"m1-swasia.{k}.oikospack" for k in ("geometry", "climate", "coastline", "hydrology")]
    if not all(p.exists() for p in packs):
        pytest.skip("M1 packs not built; run python -m oikos_data build <pack> m1-swasia")
    w = oikos.World.from_packs([str(p) for p in packs], seed=1)
    a = record.activity(w, end=op.settings.true.end)
    assert a.window == op.settings.true and a.people.shape == (len(cells), a.window.centuries)
    score = op.log_likelihood(a.visible(0))
    assert np.isfinite(score)
    e = op.expected(a.visible(0)).sum(axis=(0, 1))
    assert e[-10:].sum() > e[:10].sum(), "foragers and farmers grow over the Holocene"
    print(f"M1 default world: {score:.1f}; flat history: {op.log_likelihood(np.ones_like(a.people)):.1f}")
