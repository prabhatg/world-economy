"""Calibration worlds through Coordinator v0 (ADR-0097, #21): jobs become M1 work units,
hosts compute them, and the accepted result's record is read back here."""

import io
import re
import subprocess
import tarfile
from pathlib import Path

import numpy as np
import pytest

from oikos_cal import coord

ENGINE = Path(__file__).resolve().parents[2] / "engine" / "target" / "release"
REFS = [{"kind": "geometry", "id": "a" * 64, "path": "/packs/geometry"},
        {"kind": "climate", "id": "b" * 64, "path": "/packs/climate"}]


def test_jobs_become_work_units_named_by_experiment_and_index():
    units = coord.units("w1", [({"growth_rate": 0.02}, 3), ({}, 4)], refs=REFS)
    assert [u["id"] for u in units] == ["w1-00000", "w1-00001"]
    wu = units[1]["unit"]
    assert wu["id"] == "w1-00001" and wu["seed"] == 4 and wu["params"] == {}
    assert wu["activity"] == {"dir": coord.RECORD, "every_years": 10}, "relative: it lands in the run directory"
    assert [i["id"] for i in wu["inputs"]] == ["a" * 64, "b" * 64]
    with pytest.raises(ValueError, match="experiment"):
        coord.units("../w1", [({}, 1)], refs=REFS)


def _tar(members):
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w") as t:
        for name, data in members:
            info = tarfile.TarInfo(name)
            info.size = len(data)
            t.addfile(info, io.BytesIO(data))
    return buf.getvalue()


def test_archives_unpack_inside_their_directory_only(tmp_path):
    coord.unpack(_tar([("./world/activity.zarr/zarr.json", b"{}")]), tmp_path / "ok")
    assert (tmp_path / "ok" / "world" / "activity.zarr" / "zarr.json").read_bytes() == b"{}"
    with pytest.raises(tarfile.TarError):
        coord.unpack(_tar([("../escape", b"x")]), tmp_path / "bad")
    assert not (tmp_path / "escape").exists()
    coord.unpack(_tar([("/escape", b"x")]), tmp_path / "abs")
    assert (tmp_path / "abs" / "escape").exists(), "an absolute name lands inside, its slash dropped"


def test_a_world_computed_through_the_coordinator_records_what_the_bindings_do(tmp_path):
    oikos = pytest.importorskip("oikos", reason="build the bindings: maturin develop -m engine/crates/oikos-py/Cargo.toml")
    from oikos_cal import m1, record

    try:
        packs = m1.packs()
    except FileNotFoundError as e:
        pytest.skip(str(e))
    for b in ("oikos", "oikos-coord"):
        if not (ENGINE / b).exists():
            pytest.skip(f"build it: cargo build --release -p {'oikos-cli' if b == 'oikos' else b}")
    serve = subprocess.Popen([str(ENGINE / "oikos-coord"), "serve", "--dir", str(tmp_path / "coord"), "--addr",
                              "127.0.0.1:0"], stderr=subprocess.PIPE, text=True)
    try:
        url = re.search(r" on (http://\S+)", serve.stderr.readline()).group(1)
        params, end = {"humid_period_scale": 6.0}, -11_000
        ids = coord.submit(url, "t", [(params, 3)], end=end)
        for host in ("a", "b"):  # two hosts agree: the quorum of new hosts
            subprocess.run([str(ENGINE / "oikos-coord"), "work", "--coordinator", url, "--host", host,
                            "--oikos", str(ENGINE / "oikos"), "--cache", str(tmp_path / host / "packs"),
                            "--scratch", str(tmp_path / host / "runs"), "--until-idle"], check=True)
        assert coord.wait(url, "t", poll=0) == {"open": 0, "done": 1, "failed": 0, "leased": 0}
        got = coord.collect(url, ids[0], tmp_path / "got", lambda a: a)
    finally:
        serve.kill()
    want = record.activity(oikos.World.from_packs(packs, seed=3, params=params), end=end)
    assert got.window == want.window
    for name in record.FIELDS:
        np.testing.assert_array_equal(getattr(got, name), getattr(want, name), err_msg=name)
    assert not (tmp_path / "got").exists(), "the archive is removed once scored"
