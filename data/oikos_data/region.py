"""Regions: the H3 cells a run simulates, defined by a file in data/regions/."""

import hashlib
import tomllib
from dataclasses import dataclass

import h3
import numpy as np

from . import DATA


@dataclass(frozen=True)
class Region:
    name: str
    resolution: int
    bbox: tuple  # (south, north, west, east), degrees
    sha256: str  # of the region file: provenance for every pack of the region

    def cells(self):
        """Ascending H3 indices (uint64) of the cells whose centers lie in the box."""
        s, n, w, e = self.bbox
        poly = h3.LatLngPoly([(s, w), (s, e), (n, e), (n, w)])
        return np.array(sorted(h3.str_to_int(c) for c in h3.polygon_to_cells(poly, self.resolution)), dtype=np.uint64)


def load(name):
    path = DATA / "regions" / f"{name}.toml"
    raw = path.read_bytes()
    r = tomllib.loads(raw.decode())
    if r.get("rule") != "cell centers inside bbox":
        raise ValueError(f"{path}: unsupported rule {r.get('rule')!r}")
    b = r["bbox"]
    return Region(r["name"], r["resolution"], (b["south"], b["north"], b["west"], b["east"]), hashlib.sha256(raw).hexdigest())
