"""The reduced calibration record (#47): sums over the cell sets the operators read.

A world's full record (`record.Activity`) is every field per cell and century, about
94 MB for M1: too much for a volunteer to upload. Every operator reads it the same way,
as per-century sums over a set of cells (the cells near a site, a survey region, a
radiocarbon block), and radiocarbon weights each cell by its visibility in each century
(ADR-0095). So the operators' cell sets, gathered once on the calibration host, are
the record's zones: the engine sums each field over each zone and century
(`activity.zones` in a work unit), and `Zoned` answers the operators' reads from those
sums, addressed by the same cell sets, so they score a reduced record unchanged.

A zone file (`encode`) lists the zones by cell index, which zones count each cell by
its weight in each century, and the weights (cells not listed weigh 1), each weight
written as its IEEE 754 bits so both sides read the same double. Its BLAKE3 hash is its
id. Sums follow the zone's cells in ascending order, as the engine adds them; they
match a sum over the full record to rounding, not bit for bit.
"""

import json

import numpy as np

from .radiocarbon import Window

FORMAT = "oikos-zones/1"


def _key(cells, n):
    """A cell selector as its zone's key: integer indices, or a mask over all `n` cells."""
    cells = np.asarray(cells)
    if cells.dtype == bool:
        if cells.shape != (n,):
            raise KeyError(f"a mask of {cells.shape} cells, not the world's {n}")
        cells = np.flatnonzero(cells)
    elif cells.size and not np.issubdtype(cells.dtype, np.integer):
        raise KeyError(f"cells by {cells.dtype}, not by integer index")
    return np.sort(cells.astype(np.int64).ravel()).tobytes()


class Zones:
    """`sets` and `weighted`: cell index arrays; `weights` [cell, century] or None
    (every weight 1)."""

    def __init__(self, cells, window: Window, sets, weighted=(), weights=None):
        self.cells, self.window = np.asarray(cells, dtype=np.uint64), window
        n = len(self.cells)
        if weights is not None:
            weights = np.asarray(weights, dtype=float)
            if weights.shape != (n, window.centuries):
                raise ValueError(f"weights must be [cells, centuries] = {(n, window.centuries)}, not {weights.shape}")
        self.weights = weights
        self.zones, self.index = [], {}
        for flag, group in ((False, sets), (True, weighted)):
            for s in group:
                s = np.sort(np.asarray(s, dtype=np.int64))
                if s.size and (s[0] < 0 or s[-1] >= n):
                    raise ValueError(f"zone {s.tolist()} has cells outside the world's {n}")
                if (flag, s.tobytes()) not in self.index:
                    self.index[(flag, s.tobytes())] = len(self.zones)
                    self.zones.append((flag, s))

    def encode(self):
        if getattr(self, "_encoded", None) is None:
            self._encoded = self._encode()
        return self._encoded

    def _encode(self):
        weighted = [z for z, (flag, _) in enumerate(self.zones) if flag]
        w = {"cells": [], "row": [], "rows": []}
        if self.weights is not None:
            rows = {}
            for c in np.nonzero((self.weights != 1).any(axis=1))[0]:
                bits = self.weights[c].view(np.uint64).tolist()
                w["cells"].append(int(c))
                w["row"].append(rows.setdefault(tuple(bits), len(rows)))
            w["rows"] = [list(r) for r in rows]
        from oikos_data import pack

        doc = {"format": FORMAT, "cells_hash": pack.cells_hash(self.cells), "cells": len(self.cells),
               "first_year": self.window.first, "centuries": self.window.centuries,
               "zones": [s.tolist() for _, s in self.zones], "weighted": weighted, "weights": w}
        return json.dumps(doc, separators=(",", ":")).encode()

    @property
    def id(self):
        import blake3

        return blake3.blake3(self.encode()).hexdigest()

    @classmethod
    def decode(cls, data, cells):
        from oikos_data import pack

        doc = json.loads(data)
        cells = np.asarray(cells, dtype=np.uint64)
        if doc.get("format") != FORMAT:
            raise ValueError(f"not a zone file ({FORMAT})")
        if doc["cells"] != len(cells) or doc["cells_hash"] != pack.cells_hash(cells):
            raise ValueError("the zones are for other cells than these")
        window = Window(doc["first_year"], doc["first_year"] + 100 * doc["centuries"])
        w, n = doc["weights"], len(doc["zones"])
        if doc["weighted"] != list(range(n - len(doc["weighted"]), n)):
            raise ValueError("the weighted zones are the last ones, in order")
        if len(w["cells"]) != len(w["row"]) or any(not 0 <= r < len(w["rows"]) for r in w["row"]):
            raise ValueError("each weighted cell needs a row that exists")
        if any(len(r) != window.centuries for r in w["rows"]):
            raise ValueError(f"each weight row has the record's {window.centuries} centuries")
        if w["cells"] != sorted(set(w["cells"])) or any(not 0 <= c < len(cells) for c in w["cells"]):
            raise ValueError("weighted cells ascending, each once, in the world")
        weights = None
        if w["rows"]:
            rows = np.array(w["rows"], dtype=np.uint64).view(float)
            weights = np.ones((len(cells), window.centuries))
            weights[w["cells"]] = rows[w["row"]]
        flagged = set(doc["weighted"])
        sets = [np.array(s) for z, s in enumerate(doc["zones"]) if z not in flagged]
        weighted = [np.array(s) for z, s in enumerate(doc["zones"]) if z in flagged]
        out = cls(cells, window, sets, weighted, weights)
        if [s.tolist() for _, s in out.zones] != doc["zones"]:
            raise ValueError("a zone file lists unweighted zones first, each once, cells ascending")
        out._encoded = bytes(data)  # its id is the hash of the file as given
        return out

    def sums(self, field):
        """[zone, century] sums of `field` [cell, century]."""
        out = np.zeros((len(self.zones), self.window.centuries))
        w = self.weights
        for z, (flag, s) in enumerate(self.zones):
            for c in s:
                out[z] += field[c] * w[c] if flag and w is not None else field[c]
        return out

    def reduce(self, activity):
        """The reduced record of a full one: the same `record.Activity`, its arrays `Zoned`."""
        from . import record

        if activity.window != self.window:
            raise ValueError(f"activity covers {activity.window}, the zones {self.window}")
        arrays = {n: Zoned(self, self.sums(getattr(activity, n))) for n in record.FIELDS}
        obsidian = {g: Zoned(self, self.sums(a)) for g, a in activity.obsidian.items()}
        return record.Activity(self.window, obsidian=obsidian, **arrays)


class Zoned:
    """A field's [zone, century] sums, read as an operator reads the full [cell, century]
    array: indexing by one of the zones' cell sets gives a single row whose sum over
    cells is the zone's (`a[cells]`, `a[cells, centuries]`, `a[np.ix_(cells,
    centuries)]`), and `a[:, k]` reads the zone of every cell."""

    def __init__(self, zones: Zones, data):
        self.zones, self.data = zones, data

    def _zone(self, cells, weighted=False):
        z = self.zones.index.get((weighted, _key(cells, len(self.zones.cells))))
        if z is None:
            raise KeyError(f"cells {np.sort(np.asarray(cells)).tolist()[:8]}... are not a zone"
                           f"{' weighted' if weighted else ''}: add them to their operator's zones")
        return z

    def __getitem__(self, key):
        if not isinstance(key, tuple):
            return self.data[[self._zone(key)]]
        cells, when = key
        if isinstance(cells, slice):
            if cells != slice(None):
                raise KeyError("only every cell (:) or a zone's cells")
            return self.data[self._zone(np.arange(len(self.zones.cells)))][when]
        cells = np.asarray(cells)
        if cells.ndim == 2:  # np.ix_(cells, centuries)
            if cells.shape[1] != 1 or np.ndim(when) != 2 or np.shape(when)[0] != 1:
                raise KeyError("only np.ix_(cells, centuries) of two index arrays")
            return self.data[[self._zone(cells.ravel())]][:, np.asarray(when).ravel()]
        if not isinstance(when, (slice, int, np.integer)):
            raise KeyError("a zone's cells with a slice or one century; paired index arrays pick one value per cell")
        return self.data[[self._zone(cells)], when]

    def weighted(self, sets):
        """[set, century] weighted sums over `sets`, each a weighted zone."""
        return self.data[[self._zone(s, weighted=True) for s in sets]]

    def __add__(self, other):
        """Fields add per cell, so their zone sums add; a constant per cell does not."""
        if not isinstance(other, Zoned):
            raise TypeError("a zoned field adds only to a field of the same zones")
        if other.zones is not self.zones:
            raise ValueError("fields of different zones")
        return Zoned(self.zones, self.data + other.data)

    __radd__ = __add__

    def __mul__(self, other):
        """Scaling by a constant scales the zone sums; a product of fields is not a sum's."""
        if isinstance(other, Zoned) or not np.isscalar(other):
            raise TypeError("a zoned field multiplies only by a scalar")
        return Zoned(self.zones, self.data * other)

    __rmul__ = __mul__
