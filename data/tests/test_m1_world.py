"""A world on the M1 region runs from its geometry pack through the engine CLI (#2)."""

import json
import subprocess
from pathlib import Path

import pytest

from oikos_data import geometry, pack, region

OIKOS = Path(__file__).resolve().parents[2] / "engine" / "target" / "release" / "oikos"


@pytest.mark.skipif(not OIKOS.exists(), reason="build the CLI first: cargo build --release -p oikos-cli")
def test_m1_world_runs_from_its_geometry_pack(tmp_path):
    data = geometry.build(region.load("m1-swasia"))
    gid = pack.read(data).id
    (tmp_path / "m1.geometry.oikospack").write_bytes(data)

    def run(parts):
        wu = {
            "format": 1, "id": f"m1-smoke-{parts}", "world": {"geometry": gid}, "seed": 1,
            "start": {"kind": "initial"}, "years": 100, "partitions": parts, "summary_every_years": 50,
            "inputs": [{"id": gid, "path": "m1.geometry.oikospack"}],
        }
        (tmp_path / "wu.json").write_text(json.dumps(wu))
        out = subprocess.run([str(OIKOS), "run", "wu.json"], cwd=tmp_path, check=True, capture_output=True, text=True)
        return json.loads(out.stdout)

    one, four = run(1), run(4)
    assert one["state_hash"] == four["state_hash"]
    assert one["summaries"][-1]["population"] > one["summaries"][0]["population"] > 0
