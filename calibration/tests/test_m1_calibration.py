"""The M1 calibration problem on the real records (runs when p3k14c, IntCal20, the rachis
supplement and the M1 packs are cached and the bindings are built)."""

import numpy as np
import pytest

from oikos_cal import history
from oikos_data import raster


@pytest.fixture(scope="module")
def problem():
    pytest.importorskip("oikos", reason="build the bindings: maturin develop -m engine/crates/oikos-py/Cargo.toml")
    from oikos_cal import m1

    try:
        m1.packs()
        return m1.Problem()
    except (raster.MissingInputs, FileNotFoundError) as e:
        pytest.skip(str(e))


def test_the_record_summaries(problem):
    ts = {t.name: t for t in problem.targets}
    rc = [t for t in problem.targets if t.name.startswith("radiocarbon")]
    assert len(rc) == 5 and rc[0].observed < rc[2].observed, "dates rise into the Neolithic"
    assert all(0 < t.variance < 0.25 for t in rc)
    assert ts["domestic barley before 9000 BCE"].observed < 0.05 < 0.7 < ts["domestic barley after 7000 BCE"].observed
    assert problem.names[: len(problem.targets)] == list(ts)


def test_worlds_are_run_in_parallel_and_reproducibly(problem):
    from oikos_cal import m1

    a, b = m1.evaluate([({}, 1), ({}, 1)], workers=2)
    assert len(a) == len(problem.names) and np.isfinite(a).all()
    assert (a == b).all(), "a run is a function of its parameters and seed"
    i = history.implausibility(a[None, : len(problem.targets)], 0.0, problem.targets)[0]
    print("M1 default world implausibility: " + ", ".join(f"{t.name} {x:.1f}" for t, x in zip(problem.targets, i)))
