"""Worlds on forcing-pack regions from Python (docs/03 §3.8a).

Packs are built with the data pipelines (../../data/oikos_data), as calibration code
would use them.
"""

import json
import sys
from pathlib import Path

import numpy as np
import pytest

import oikos

pytest.importorskip("h3")
sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "data"))
from oikos_data import geometry, pack, region  # noqa: E402

BOX = region.Region(name="test-box", resolution=5, bbox=(31.0, 33.0, 35.0, 37.0), sha256="0" * 64)


@pytest.fixture(scope="module")
def packs(tmp_path_factory):
    d = tmp_path_factory.mktemp("packs")
    g = geometry.build(BOX)
    cells = pack.read(g).raw("h3")
    t = pack.Builder("terrain", pack.region(BOX.name, BOX.resolution, cells))
    t.field("elev", "m", "test elevation", (np.arange(len(cells)) % 7 * 100).astype(np.uint16))
    t = t.build()
    paths = []
    for name, data in (("geometry", g), ("terrain", t)):
        (d / f"{name}.oikospack").write_bytes(data)
        paths.append(str(d / f"{name}.oikospack"))
    return {"paths": paths, "ids": sorted(pack.read(b).id for b in (g, t)), "geometry": pack.read(g).id, "cells": len(cells)}


def test_a_world_on_a_pack_region(packs):
    w = oikos.World.from_packs(packs["paths"], seed=4)
    assert w.cells == packs["cells"] and w.resolution == 5
    assert w.base_cell is None and w.geometry == packs["geometry"]
    assert w.inputs == packs["ids"]
    s = w.run_years(300, parts=3)
    assert s[-1]["population"] == int(w.population().sum()) > 0


def test_pack_worlds_are_partition_independent(packs):
    a, b = oikos.World.from_packs(packs["paths"], seed=8), oikos.World.from_packs(packs["paths"], seed=8)
    a.run_years(200, parts=1)
    b.run_years(200, parts=5)
    assert a.state_hash() == b.state_hash()


def test_checkpoints_and_forks_keep_their_packs(packs):
    w = oikos.World.from_packs(packs["paths"], seed=2)
    w.run_years(100)
    data = w.checkpoint()
    with pytest.raises(ValueError, match="forcing packs"):
        oikos.World.from_checkpoint(data)
    with pytest.raises(ValueError, match="forcing packs"):
        oikos.World.from_checkpoint(data, packs=packs["paths"][:1])
    restored = oikos.World.from_checkpoint(data, packs=packs["paths"])
    assert restored.state_hash() == w.state_hash() and restored.inputs == packs["ids"]
    twin = w.fork(seed=2)
    for x in (w, restored, twin):
        x.run_years(100)
    assert w.state_hash() == restored.state_hash() == twin.state_hash()


def test_work_units_with_inputs(packs, tmp_path):
    wu = {
        "format": 1,
        "world": {"geometry": packs["geometry"]},
        "seed": 4,
        "start": {"kind": "initial"},
        "years": 300,
        "partitions": 3,
        "summary_every_years": 100,
        "inputs": [{"id": pack.read(Path(p).read_bytes()).id, "path": Path(p).name} for p in packs["paths"]],
    }
    result = oikos.run_work_unit(json.dumps(wu), read_dir=str(Path(packs["paths"][0]).parent), write_dir=str(tmp_path))
    w = oikos.World.from_packs(packs["paths"], seed=4)
    w.run_years(300, parts=3)
    assert result["state_hash"] == w.state_hash()


def test_from_packs_rejects_bad_sets(packs, tmp_path):
    with pytest.raises(ValueError, match="geometry"):
        oikos.World.from_packs(packs["paths"][1:], seed=1)
    with pytest.raises(ValueError, match="not an Oikos forcing pack"):
        bad = tmp_path / "bad.oikospack"
        bad.write_bytes(b"garbage!" * 4)
        oikos.World.from_packs([str(bad)], seed=1)
