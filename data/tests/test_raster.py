"""Raster tiles read at reduced resolution (COG overviews), cached outside the repo,
with every input's hash locked so rebuilds use the same data."""

import numpy as np
import pytest
import rasterio
from rasterio.transform import from_origin

from oikos_data import raster


def write_cog(path, data, west=40.0, north=36.0):
    res = 1.0 / data.shape[0]
    profile = dict(driver="GTiff", width=data.shape[1], height=data.shape[0], count=1, dtype="float32",
                   crs="EPSG:4326", transform=from_origin(west, north, res, res), tiled=True, blockxsize=256, blockysize=256)
    with rasterio.open(path, "w", **profile) as ds:
        ds.write(data.astype(np.float32), 1)
        ds.build_overviews([2, 4], rasterio.enums.Resampling.average)


@pytest.fixture
def cache(tmp_path, monkeypatch):
    monkeypatch.setenv("OIKOS_DATA_CACHE", str(tmp_path / "cache"))
    return tmp_path / "cache"


def test_reads_an_overview_and_caches_it(tmp_path, cache):
    f = tmp_path / "t.tif"
    data = np.arange(1200 * 1200, dtype=np.float32).reshape(1200, 1200) % 1000
    write_cog(f, data)
    t = raster.read_overview(str(f), 4)
    assert t.data.shape == (300, 300) and t.data.dtype == np.float32
    assert (t.north, t.west) == (36.0, 40.0) and t.dlat == pytest.approx(1 / 300) and t.dlng == pytest.approx(1 / 300)
    f.unlink()
    again = raster.read_overview(str(f), 4)  # from the cache now
    np.testing.assert_array_equal(again.data, t.data)
    assert raster.fingerprint(again) == raster.fingerprint(t)
    assert any(cache.rglob("*.npz"))


def test_full_resolution_and_unknown_overviews(tmp_path, cache):
    f = tmp_path / "t.tif"
    write_cog(f, np.ones((400, 400)))
    assert raster.read_overview(str(f), 1).data.shape == (400, 400)
    with pytest.raises(ValueError, match="overview"):
        raster.read_overview(str(f), 8)


def test_input_lock_records_then_enforces(tmp_path):
    lock = tmp_path / "lock.json"
    raster.check_inputs(lock, {"a": "1", "b": "2"}, update=False)
    raster.check_inputs(lock, {"a": "1"}, update=False)
    with pytest.raises(ValueError, match="a"):
        raster.check_inputs(lock, {"a": "9"}, update=False)
    raster.check_inputs(lock, {"a": "9"}, update=True)
