"""Forcing-pack writer and reader, the Python side of oikos-io::pack (docs/03 §3.8a).

Layout: "OIKOSPK1" | header_len u32 | canonical JSON header | zero padding | arrays,
each array little-endian at an 8-byte-aligned offset. The pack id is the BLAKE3 hash
of the file.
"""

import json

import blake3
import numpy as np

MAGIC = b"OIKOSPK1"
FORMAT = 1
DTYPES = {"u8": "<u1", "u16": "<u2", "u32": "<u4", "u64": "<u8", "i16": "<i2", "i32": "<i4", "i64": "<i8", "f32": "<f4", "f64": "<f8"}
NAMES = {(np.dtype(v).kind, np.dtype(v).itemsize): k for k, v in DTYPES.items()}


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def cells_hash(h3):
    """BLAKE3 of H3 indices as little-endian u64: the identity of a region's cells."""
    return blake3.blake3(np.asarray(h3, dtype="<u8").tobytes()).hexdigest()


def region(name, resolution, h3):
    h3 = np.asarray(h3, dtype=np.uint64)
    if len(h3) == 0 or np.any(h3[1:] <= h3[:-1]):
        raise ValueError(f"region {name}: cells must be non-empty and strictly ascending")
    return {"name": name, "resolution": int(resolution), "cells": len(h3), "cells_hash": cells_hash(h3)}


class Builder:
    def __init__(self, kind, region):
        self.kind, self.region = kind, region
        self.time_ = None
        self.sources, self.fields = [], []

    def time(self, calendar_year, rule):
        years = [int(y) for y in calendar_year]
        if not years or any(a >= b for a, b in zip(years, years[1:])) or rule not in ("step", "linear"):
            raise ValueError("time needs strictly ascending years and rule 'step' or 'linear'")
        self.time_ = {"calendar_year": years, "rule": rule}
        return self

    def source(self, *, dataset, version, license, citation, manifest_sha256):
        self.sources.append(dict(dataset=dataset, version=version, license=license, citation=citation, manifest_sha256=manifest_sha256))
        return self

    def field(self, name, unit, description, values, *, scale=1.0, offset=0.0, nodata=None):
        values = np.asarray(values)
        if any(f[0]["name"] == name for f in self.fields):
            raise ValueError(f"duplicate field {name}")
        dtype = NAMES.get((values.dtype.kind, values.dtype.itemsize))
        if dtype is None or values.dtype.kind not in "uif" or dtype == "u8" and values.dtype.kind != "u":
            raise ValueError(f"field {name}: unsupported dtype {values.dtype} (use {', '.join(DTYPES)})")
        cells = self.region["cells"]
        if values.ndim == 1:
            dims = ["cell"]
            if len(values) != cells:
                raise ValueError(f"field {name}: expected {cells} cells, got {len(values)}")
        elif values.ndim == 2:
            dims = ["time", "cell"]
            if self.time_ is None:
                raise ValueError(f"field {name}: set the pack's time slices first")
            if values.shape != (len(self.time_["calendar_year"]), cells):
                raise ValueError(f"field {name}: expected shape (time, cell) = ({len(self.time_['calendar_year'])}, {cells}), got {values.shape}")
        else:
            raise ValueError(f"field {name}: values must be (cell,) or (time, cell)")
        data = np.ascontiguousarray(values, dtype=DTYPES[dtype]).tobytes()
        spec = {"name": name, "unit": unit, "description": description, "dims": dims, "dtype": dtype,
                "scale_factor": float(scale), "add_offset": float(offset)}
        if nodata is not None:
            spec["nodata"] = int(nodata)
        self.fields.append((spec, data))
        return self

    def build(self):
        start = 0
        while True:
            at, fields = start, []
            for spec, data in self.fields:
                at = -(-at // 8) * 8
                fields.append({**spec, "offset": at, "len": len(data)})
                at += len(data)
            header = {"format": FORMAT, "kind": self.kind, "region": self.region, "fields": fields,
                      "sources": self.sources, "builder": {"tool": "oikos_data", "format": FORMAT}}
            if self.time_ is not None:
                header["time"] = self.time_
            text = canonical(header).encode()
            data_start = -(-(12 + len(text)) // 8) * 8
            if data_start == start:
                break
            start = data_start
        out = bytearray(MAGIC + len(text).to_bytes(4, "little") + text)
        for f, (_, data) in zip(fields, self.fields):
            out.extend(b"\0" * (f["offset"] - len(out)))
            out.extend(data)
        return bytes(out)


class Pack:
    def __init__(self, data):
        if data[:8] != MAGIC:
            raise ValueError("not an Oikos forcing pack")
        n = int.from_bytes(data[8:12], "little")
        self.header = json.loads(data[12 : 12 + n])
        if self.header["format"] != FORMAT:
            raise ValueError(f"unsupported pack format {self.header['format']}")
        self.id = blake3.blake3(data).hexdigest()
        self._data = data

    def _def(self, name):
        for f in self.header["fields"]:
            if f["name"] == name:
                return f
        raise ValueError(f"pack has no field {name}")

    def raw(self, name):
        """Stored values: (cell,) or (time, cell)."""
        f = self._def(name)
        a = np.frombuffer(self._data, dtype=DTYPES[f["dtype"]], count=f["len"] // np.dtype(DTYPES[f["dtype"]]).itemsize, offset=f["offset"])
        return a.reshape(-1, self.header["region"]["cells"]) if f["dims"] == ["time", "cell"] else a

    def values(self, name):
        """Physical values (float64, scale and offset applied); nodata is NaN."""
        f = self._def(name)
        raw = self.raw(name)
        v = raw.astype(np.float64) * f["scale_factor"] + f["add_offset"]
        if "nodata" in f:
            v[raw == f["nodata"]] = np.nan
        return v


def read(data):
    return Pack(bytes(data))
