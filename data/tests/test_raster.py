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


def write_strips(path, data, west=40.0, north=36.0):
    """A strip-organized GeoTIFF (one row per strip), as CHELSA ships."""
    res = 1.0 / 100
    profile = dict(driver="GTiff", width=data.shape[1], height=data.shape[0], count=1, dtype="float32",
                   crs="EPSG:4326", transform=from_origin(west, north, res, res), blockysize=1, compress="deflate")
    with rasterio.open(path, "w", **profile) as ds:
        ds.write(data.astype(np.float32), 1)


def test_reads_a_window_averaged_over_blocks_and_caches_it(tmp_path, cache):
    f = tmp_path / "s.tif"
    data = np.arange(200 * 300, dtype=np.float32).reshape(200, 300)  # 2 x 3 degrees from 40E, 36N
    write_strips(f, data)
    t = raster.read_window(str(f), (34.5, 35.43, 40.51, 41.0), 2)
    # Snapped outward to whole 2x2 blocks of the file grid: rows 56..150, columns 50..100.
    assert (t.north, t.west) == pytest.approx((35.44, 40.50)) and t.dlat == pytest.approx(0.02)
    assert t.data.shape == (47, 25) and t.data.dtype == np.float32
    want = data[56:150, 50:100].reshape(47, 2, 25, 2).mean(axis=(1, 3))
    np.testing.assert_array_equal(t.data, want)
    f.unlink()
    again = raster.read_window(str(f), (34.5, 35.43, 40.51, 41.0), 2)
    assert raster.fingerprint(again) == raster.fingerprint(t)
    with pytest.raises(raster.MissingInputs):
        raster.read_window(str(f), (34.5, 35.0, 40.5, 41.0), 2, offline=True)


def test_window_block_means_skip_nan(tmp_path, cache):
    f = tmp_path / "s.tif"
    data = np.ones((100, 100), dtype=np.float32)
    data[0, 0] = 5.0
    data[0, 1] = data[1, 0] = data[1, 1] = np.nan
    data[2:4, 2:4] = np.nan
    write_strips(f, data)
    t = raster.read_window(str(f), (35.0, 36.0, 40.0, 41.0), 2)
    assert t.data[0, 0] == 5.0 and np.isnan(t.data[1, 1]) and t.data[1, 2] == 1.0


def test_window_nodata_is_nan_and_valid_fractions(tmp_path, cache):
    f = tmp_path / "m.tif"
    data = np.full((100, 100), 7, dtype=np.int16)
    data[0, :3] = -32768  # sea
    profile = dict(driver="GTiff", width=100, height=100, count=1, dtype="int16", nodata=-32768,
                   crs="EPSG:4326", transform=from_origin(40.0, 36.0, 0.01, 0.01), blockysize=1, compress="deflate")
    with rasterio.open(f, "w", **profile) as ds:
        ds.write(data, 1)
    mean = raster.read_window(str(f), (35.0, 36.0, 40.0, 41.0), 2)
    assert mean.data[0, 0] == 7 and mean.data[0, 1] == 7
    land = raster.read_window(str(f), (35.0, 36.0, 40.0, 41.0), 2, valid=True)
    assert land.data[0, 0] == 0.5 and land.data[0, 1] == 0.75 and land.data[0, 2] == 1.0
    assert raster.fingerprint(land) != raster.fingerprint(mean)
