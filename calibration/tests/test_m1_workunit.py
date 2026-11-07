"""A calibration world as a work unit (#46): the engine's activity record is what
`record.activity` computes through the bindings, bit for bit (runs when the M1 packs,
the bindings and the release CLI are built)."""

import json
import subprocess
from pathlib import Path

import numpy as np
import pytest

CLI = Path(__file__).resolve().parents[2] / "engine" / "target" / "release" / "oikos"


def test_an_m1_work_unit_records_what_the_bindings_do(tmp_path):
    oikos = pytest.importorskip("oikos", reason="build the bindings: maturin develop -m engine/crates/oikos-py/Cargo.toml")
    from oikos_cal import m1, record

    try:
        packs = m1.packs()
    except FileNotFoundError as e:
        pytest.skip(str(e))
    if not CLI.exists():
        pytest.skip("build the CLI: cargo build --release -p oikos-cli")
    params, end = {"humid_period_scale": 6.0, "herd_pressure_weight": 0.4}, -11_000
    wu = m1.work_unit(params, seed=3, out="world", end=end)
    assert wu["activity"] == {"dir": "world", "every_years": 10} and "outputs" not in wu
    (tmp_path / "world.json").write_text(json.dumps(wu))
    subprocess.run([str(CLI), "run", "world.json", "-o", "world.result.json"], cwd=tmp_path, check=True)
    got = record.read(tmp_path / "world" / "activity.zarr")

    want = record.activity(oikos.World.from_packs(packs, seed=3, params=params), end=end)
    assert got.window == want.window
    for name in record.FIELDS:
        np.testing.assert_array_equal(getattr(got, name), getattr(want, name), err_msg=name)
    assert list(got.obsidian) == list(want.obsidian) and want.obsidian
    for k in want.obsidian:
        np.testing.assert_array_equal(got.obsidian[k], want.obsidian[k], err_msg=k)
    assert got.people.sum() > 0

    # The same world summed over zones (#47): what `zones.Zones.reduce` gives, bit for bit.
    from oikos_cal import zones
    from oikos_data import region

    rng = np.random.default_rng(0)
    cells = region.load("m1-swasia").cells()
    n = len(cells)
    weights = np.ones((n, want.window.centuries))
    weights[100:200] = rng.uniform(0, 1, (100, want.window.centuries))
    z = zones.Zones(cells, want.window, [np.arange(n), np.arange(0, n, 7)], weighted=[np.arange(50, 400)],
                    weights=weights)
    (tmp_path / "zones.json").write_bytes(z.encode())
    wu = m1.work_unit(params, seed=3, out="zoned", end=end, zones={"id": z.id, "path": "zones.json"})
    (tmp_path / "zoned.json").write_text(json.dumps(wu))
    subprocess.run([str(CLI), "run", "zoned.json", "-o", "zoned.result.json"], cwd=tmp_path, check=True)
    reduced = record.read(tmp_path / "zoned" / "activity.zarr", zones=z)
    expect = z.reduce(want)
    for name in record.FIELDS:
        np.testing.assert_array_equal(getattr(reduced, name).data, getattr(expect, name).data, err_msg=name)
    for k in want.obsidian:
        np.testing.assert_array_equal(reduced.obsidian[k].data, expect.obsidian[k].data, err_msg=k)
    with pytest.raises(ValueError, match="zones"):
        record.read(tmp_path / "zoned" / "activity.zarr")
