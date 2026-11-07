"""Calibration worlds through Coordinator v0 (ADR-0097, #21).

A wave's (params, seed) jobs become M1 work units (`m1.work_unit`) submitted to an
`oikos-coord serve` as one experiment; hosts running `oikos-coord work` compute them,
and the coordinator accepts each result when hosts agree on its state hash. The
calibration host then fetches each accepted run's archive, reads its record and scores
it with the same operators as an in-process run, so `evaluate(..., coordinator=url)`
gives what `m1.evaluate` gives, bit for bit. A full M1 record is about 94 MB, so each
archive is unpacked, scored and removed in turn.
"""

import io
import json
import re
import shutil
import tarfile
import time
import urllib.request
from pathlib import Path

from . import m1, record

RECORD = "world"  # the record's directory inside each run's archive


def _call(url, data=None, method=None):
    req = urllib.request.Request(url, data=data, method=method, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req) as r:
        return r.read()


def units(experiment, jobs, refs=None, end=m1.END):
    """[{id, unit}] for `jobs` (params, seed): ids `<experiment>-<index>`."""
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]*", experiment):
        raise ValueError(f"experiment {experiment!r}: letters, digits, '-', '_' and '.' only")
    refs = m1.pack_refs() if refs is None else refs
    out = []
    for i, (params, seed) in enumerate(jobs):
        uid = f"{experiment}-{i:05d}"
        wu = m1.work_unit(params, seed, out=RECORD, end=end, refs=refs)
        wu["id"] = uid
        out.append({"id": uid, "unit": wu})
    return out


def submit(url, experiment, jobs, refs=None, end=m1.END):
    """Submits `jobs` as `experiment`; returns their unit ids, in order."""
    us = units(experiment, jobs, refs, end)
    _call(f"{url}/experiments/{experiment}", json.dumps(us).encode(), "POST")
    return [u["id"] for u in us]


def wait(url, experiment, poll=30, timeout=None):
    """Waits until no unit of `experiment` is open; returns its counts."""
    start = time.time()
    while True:
        counts = json.loads(_call(f"{url}/experiments/{experiment}"))["counts"]
        if counts["open"] == 0:
            return counts
        if timeout is not None and time.time() - start > timeout:
            raise TimeoutError(f"{experiment}: {counts}")
        time.sleep(poll)


def unpack(tar_bytes, dest):
    """Extracts a run's archive into `dest`, refusing members that would land outside it."""
    with tarfile.open(fileobj=io.BytesIO(tar_bytes)) as t:
        t.extractall(dest, filter="data")


def collect(url, unit_id, scratch, score):
    """`score(activity)` of a done unit's accepted record, fetched into `scratch` and
    removed afterwards."""
    doc = json.loads(_call(f"{url}/units/{unit_id}"))
    if doc["status"] != "done":
        raise RuntimeError(f"unit {unit_id} is {doc['status']}, not done")
    host = doc["report"]["host"]
    scratch = Path(scratch)
    try:
        unpack(_call(f"{url}/archives/{unit_id}/{host}.tar"), scratch)
        return score(record.read(scratch / RECORD / "activity.zarr"))
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
