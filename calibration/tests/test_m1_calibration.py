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
    assert len(rc) == 10 and rc[0].observed < rc[4].observed, "dates rise into the Neolithic"
    g = problem.groups[0]
    assert g.indices == list(range(10)) and g.dof == 9
    assert np.allclose(np.diag(g.covariance), [t.variance for t in rc])
    assert all(0 < t.variance < 0.3 for t in rc)
    assert ts["domestic barley before 9000 BCE"].observed < 0.05 < 0.7 < ts["domestic barley after 7000 BCE"].observed
    assert problem.names[: len(problem.targets)] == list(ts)


def test_worlds_are_run_in_parallel_and_reproducibly(problem):
    from oikos_cal import m1

    a, b = m1.evaluate([({}, 1), ({}, 1)], workers=2)
    assert len(a) == len(problem.names) and np.isfinite(a).all()
    assert (a == b).all(), "a run is a function of its parameters and seed"
    i = history.implausibility(a[None, : len(problem.targets)], 0.0, problem.targets)[0]
    print("M1 default world implausibility: " + ", ".join(f"{t.name} {x:.1f}" for t, x in zip(problem.targets, i)))


def test_the_default_world_is_consistent_with_the_record(problem):
    """The registry defaults are a calibrated M1 world (calibration/runs/m1/round2/best.json,
    ADR-0057): with any seed, every target and the radiocarbon group pass history matching."""
    from oikos_cal import __main__ as cli
    from oikos_cal import m1

    out = m1.evaluate([({}, s) for s in (1, 2, 3)], workers=3)
    doc = {"targets": cli._targets_json(problem.targets), "groups": cli._groups_json(problem.groups), "outputs": out}
    i = history.implausibility(out[:, : len(problem.targets)], 0.0, problem.targets)
    assert cli.consistent(doc).all(), f"max implausibility per seed {i.max(axis=1)}"
