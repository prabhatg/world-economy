"""Vector datasets: verified downloads, bbox-filtered reads cached outside the repo."""

import hashlib

import numpy as np
import pyogrio.raw
import pytest
import shapely

from oikos_data import raster, vector


@pytest.fixture
def cache(tmp_path, monkeypatch):
    monkeypatch.setenv("OIKOS_DATA_CACHE", str(tmp_path / "cache"))
    return tmp_path / "cache"


def write_lines(path):
    geoms = [shapely.LineString([(40, 35), (41, 36)]), shapely.LineString([(10, 10), (11, 11)]),
             shapely.LineString([(40.5, 35.5), (40.6, 35.1)])]
    pyogrio.raw.write(str(path), geometry=shapely.to_wkb(geoms), field_data=[np.array([5.0, 1.0, 2.5]), np.array([3, 1, 2])],
                      fields=["DIS", "ORD"], driver="GPKG", geometry_type="LineString", crs="EPSG:4326", layer="rivers")


def test_reads_features_in_a_box_and_caches_them(tmp_path, cache):
    f = tmp_path / "r.gpkg"
    write_lines(f)
    src = vector.Source(path=str(f), identity="test-1", layer="rivers")
    geoms, attrs = vector.read(src, (34.0, 37.0, 39.0, 42.0), ["DIS", "ORD"])
    assert len(geoms) == 2 and list(attrs["DIS"]) == [5.0, 2.5] and list(attrs["ORD"]) == [3, 2]
    assert shapely.equals(geoms[1], shapely.LineString([(40.5, 35.5), (40.6, 35.1)]))
    f.unlink()
    again, attrs2 = vector.read(src, (34.0, 37.0, 39.0, 42.0), ["DIS", "ORD"])  # from the cache
    assert all(shapely.equals(a, b) for a, b in zip(geoms, again)) and list(attrs2["ORD"]) == [3, 2]
    with pytest.raises(raster.MissingInputs):
        vector.read(src, (0.0, 1.0, 0.0, 1.0), ["DIS"], offline=True)


def test_downloads_are_verified(tmp_path, cache):
    payload = tmp_path / "payload.bin"
    payload.write_bytes(b"x" * 1000)
    url = payload.as_uri()
    good = hashlib.md5(b"x" * 1000).hexdigest()
    path = vector.download(url, "p.bin", good)
    assert path.read_bytes() == b"x" * 1000 and path.parent == cache / "downloads"
    assert vector.download(url, "p.bin", good, offline=True) == path
    with pytest.raises(ValueError, match="md5"):
        vector.download(url, "q.bin", "0" * 32)
    assert not (cache / "downloads" / "q.bin").exists()
    with pytest.raises(raster.MissingInputs):
        vector.download(url, "r.bin", good, offline=True)
