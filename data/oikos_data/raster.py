"""Raster tiles read at reduced resolution, cached outside the repository.

Cloud-optimized GeoTIFFs carry overviews (2x, 4x, ... downsampled copies), so a
pipeline that aggregates to ~16 km cells reads a fraction of the bytes. Each tile read
is cached as .npz under $OIKOS_DATA_CACHE (default ~/.cache/manyworlds) and
fingerprinted; data/locks/<dataset>.json records the fingerprints, so a rebuild that
would use different input data (an updated upstream file) fails loudly.
"""

import hashlib
import json
import os
import struct
from dataclasses import dataclass
from pathlib import Path

import numpy as np


class MissingInputs(Exception):
    """Inputs are not in the cache and fetching was not allowed."""


@dataclass
class Tile:
    data: np.ndarray  # float32 (rows, cols), row 0 at the north edge
    north: float
    west: float
    dlat: float  # degrees per row
    dlng: float  # degrees per column


def cache_dir():
    return Path(os.environ.get("OIKOS_DATA_CACHE") or Path.home() / ".cache" / "manyworlds")


def _cache_file(src, factor):
    return cache_dir() / "rasters" / (hashlib.sha256(f"{src}|{factor}".encode()).hexdigest()[:32] + ".npz")


def read_overview(src, factor, offline=False):
    """Band 1 of a GeoTIFF (path or http URL) at 1/`factor` resolution (1 = full)."""
    f = _cache_file(src, factor)
    if f.exists():
        z = np.load(f)
        north, west, dlat, dlng = z["geo"].tolist()
        return Tile(z["data"], north, west, dlat, dlng)
    if offline:
        raise MissingInputs(f"{src} (1/{factor}) is not cached")
    import rasterio

    path = f"/vsicurl/{src}" if src.startswith(("http://", "https://")) else src
    with rasterio.Env(GDAL_DISABLE_READDIR_ON_OPEN="EMPTY_DIR", CPL_VSIL_CURL_ALLOWED_EXTENSIONS=".tif"):
        with rasterio.open(path) as ds:
            factors = ds.overviews(1)
        if factor != 1 and factor not in factors:
            raise ValueError(f"{src}: no overview with factor {factor} (has {factors})")
        level = None if factor == 1 else factors.index(factor)
        with rasterio.open(path, overview_level=level) if level is not None else rasterio.open(path) as ds:
            data = ds.read(1).astype(np.float32)
            t = ds.transform
    tile = Tile(data, t.f, t.c, -t.e, t.a)
    f.parent.mkdir(parents=True, exist_ok=True)
    tmp = f.with_suffix(".tmp.npz")
    np.savez_compressed(tmp, data=data, geo=np.array([tile.north, tile.west, tile.dlat, tile.dlng]))
    tmp.replace(f)
    return tile


def read_window(src, bounds, factor=1, offline=False, valid=False):
    """Band 1 of a GeoTIFF inside `bounds` (south, north, west, east), averaged over
    `factor` x `factor` blocks of the file's grid (NaN and nodata pixels skipped; blocks
    without data are NaN), or with `valid` the fraction of each block's pixels that have
    data (e.g. land in a DEM whose sea is nodata). For files stored in strips rather than
    tiles, where only whole rows can be fetched; the window is snapped outward to whole
    blocks."""
    key = f"{src}|{bounds}|{factor}" + ("|valid" if valid else "")
    f = cache_dir() / "windows" / (hashlib.sha256(key.encode()).hexdigest()[:32] + ".npz")
    if f.exists():
        z = np.load(f)
        north, west, dlat, dlng = z["geo"].tolist()
        return Tile(z["data"], north, west, dlat, dlng)
    if offline:
        raise MissingInputs(f"{src} (window {bounds}) is not cached")
    import math

    import rasterio
    from rasterio.windows import Window

    s, n, w, e = bounds
    path = f"/vsicurl/{src}" if src.startswith(("http://", "https://")) else src
    with rasterio.Env(GDAL_DISABLE_READDIR_ON_OPEN="EMPTY_DIR", CPL_VSIL_CURL_ALLOWED_EXTENSIONS=".tif"):
        with rasterio.open(path) as ds:
            t = ds.transform
            r0, r1 = (n - t.f) / t.e, (s - t.f) / t.e
            c0, c1 = (w - t.c) / t.a, (e - t.c) / t.a
            r0, c0 = (max(0, math.floor(x + 1e-6) // factor * factor) for x in (r0, c0))
            r1 = min(ds.height // factor * factor, -(-math.ceil(r1 - 1e-6) // factor) * factor)
            c1 = min(ds.width // factor * factor, -(-math.ceil(c1 - 1e-6) // factor) * factor)
            data = ds.read(1, window=Window(c0, r0, c1 - c0, r1 - r0)).astype(np.float64)
            if ds.nodata is not None:
                data[data == ds.nodata] = np.nan
    blocks = data.reshape((r1 - r0) // factor, factor, (c1 - c0) // factor, factor)
    ok = np.isfinite(blocks)
    count = ok.sum(axis=(1, 3))
    total = np.where(ok, blocks, 0).sum(axis=(1, 3))
    if valid:
        mean = (count / factor**2).astype(np.float32)
    else:
        mean = np.where(count > 0, total / np.maximum(count, 1), np.nan).astype(np.float32)
    tile = Tile(mean, t.f + r0 * t.e, t.c + c0 * t.a, -t.e * factor, t.a * factor)
    f.parent.mkdir(parents=True, exist_ok=True)
    tmp = f.with_suffix(".tmp.npz")
    np.savez_compressed(tmp, data=mean, geo=np.array([tile.north, tile.west, tile.dlat, tile.dlng]))
    tmp.replace(f)
    return tile


def read_text(url, offline=False):
    """A small text file (e.g. a tile list), cached like rasters."""
    f = cache_dir() / "text" / (hashlib.sha256(url.encode()).hexdigest()[:32] + ".txt")
    if f.exists():
        return f.read_text()
    if offline:
        raise MissingInputs(f"{url} is not cached")
    import ssl
    import urllib.request

    import certifi  # pinned CA bundle: some Python builds ship without system certificates

    with urllib.request.urlopen(url, timeout=60, context=ssl.create_default_context(cafile=certifi.where())) as r:
        text = r.read().decode()
    f.parent.mkdir(parents=True, exist_ok=True)
    f.write_text(text)
    return text


def fingerprint(tile):
    geo = struct.pack("<4d", tile.north, tile.west, tile.dlat, tile.dlng)
    return hashlib.sha256(geo + np.ascontiguousarray(tile.data, dtype="<f4").tobytes()).hexdigest()


def check_inputs(lock_path, hashes, update):
    """Compares input fingerprints with the lock; records new inputs."""
    lock = json.loads(Path(lock_path).read_text()) if Path(lock_path).exists() else {}
    changed = sorted(k for k, v in hashes.items() if k in lock and lock[k] != v)
    if changed and not update:
        raise ValueError(f"inputs differ from {Path(lock_path).name}: {', '.join(changed[:8])}"
                         f"{' ...' if len(changed) > 8 else ''}; if upstream data changed on purpose, rerun with --update-lock")
    merged = {**lock, **hashes}
    if merged != lock:
        Path(lock_path).parent.mkdir(parents=True, exist_ok=True)
        Path(lock_path).write_text(json.dumps(merged, indent=1, sort_keys=True) + "\n")
