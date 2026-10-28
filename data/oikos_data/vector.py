"""Vector datasets: whole-file downloads verified against a published MD5, and features
inside a region's box read with GDAL (pyogrio) and cached.

Large vector products (river networks, lake polygons) come as one archive per dataset.
The archive is downloaded once into $OIKOS_DATA_CACHE/downloads and checked against
the checksum the publisher lists; the features a region needs are read with a bbox
filter and cached as WKB, so rebuilds need neither the archive nor GDAL's reader.
"""

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import shapely

from .raster import MissingInputs, cache_dir


@dataclass(frozen=True)
class Source:
    path: str  # file or GDAL virtual path (e.g. /vsizip/...)
    identity: str  # what the features come from (the archive's checksum), for the cache key
    layer: str


def download(url, name, md5, offline=False):
    """The file at `url`, cached as downloads/`name` once its MD5 matches `md5`."""
    path = cache_dir() / "downloads" / name
    ok = path.with_name(name + ".md5ok")
    if path.exists() and ok.exists() and ok.read_text() == md5:
        return path
    if offline:
        raise MissingInputs(f"{name} is not downloaded")
    import ssl
    import urllib.request

    import certifi

    path.parent.mkdir(parents=True, exist_ok=True)
    part = path.with_name(name + ".part")
    h = hashlib.md5()
    ctx = ssl.create_default_context(cafile=certifi.where())
    with urllib.request.urlopen(url, timeout=120, context=None if url.startswith("file:") else ctx) as r, part.open("wb") as out:
        while chunk := r.read(1 << 20):
            h.update(chunk)
            out.write(chunk)
    if h.hexdigest() != md5:
        part.unlink()
        raise ValueError(f"{name}: md5 {h.hexdigest()} does not match the published {md5}")
    part.replace(path)
    ok.write_text(md5)
    return path


def _cache_file(src, bounds, columns):
    key = json.dumps([src.identity, src.layer, list(bounds), list(columns)])
    return cache_dir() / "vectors" / (hashlib.sha256(key.encode()).hexdigest()[:32] + ".npz")


def read(src, bounds, columns, offline=False):
    """Geometries (shapely) and `columns` ({name: array}) of the features intersecting
    `bounds` (south, north, west, east), in the source's order."""
    f = _cache_file(src, bounds, columns)
    if f.exists():
        z = np.load(f)
        blob, ends = z["wkb"].tobytes(), z["ends"]
        starts = np.concatenate([[0], ends[:-1]])
        geoms = list(shapely.from_wkb([blob[a:b] for a, b in zip(starts, ends)]))
        return geoms, {c: z[f"col_{c}"] for c in columns}
    if offline:
        raise MissingInputs(f"{src.layer} features of {src.identity} for {bounds} are not cached")
    import pyogrio.raw

    s, n, w, e = bounds
    meta, _, wkb, values = pyogrio.raw.read(src.path, layer=src.layer, columns=list(columns), bbox=(w, s, e, n))
    names = list(meta["fields"])
    cols = {c: np.asarray(values[names.index(c)]) for c in columns}
    geoms = list(shapely.from_wkb(wkb))
    blobs = shapely.to_wkb(geoms)
    f.parent.mkdir(parents=True, exist_ok=True)
    tmp = f.with_suffix(".tmp.npz")
    np.savez_compressed(tmp, wkb=np.frombuffer(b"".join(blobs), dtype=np.uint8),
                        ends=np.cumsum([len(b) for b in blobs]).astype(np.int64),
                        **{f"col_{c}": v for c, v in cols.items()})
    tmp.replace(f)
    return geoms, cols


def path_in_zip(archive, member):
    return f"/vsizip/{Path(archive)}/{member}"
