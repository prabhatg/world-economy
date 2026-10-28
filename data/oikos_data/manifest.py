"""Dataset manifests (data/manifests/*.toml): what a dataset is, where it comes from,
what it may be used for, and what it measures. Pipelines only build packs from
datasets whose license allows redistributing derived data.
"""

import hashlib
import tomllib

from . import DATA

REQUIRED = {"dataset", "title", "version", "homepage", "citation", "license", "access", "coverage", "variables"}
OPTIONAL = {"doi", "notes"}
LICENSE = {"id", "url", "derived_redistribution", "attribution", "verified"}


class Manifest(dict):
    sha256 = ""


def load(path):
    raw = path.read_bytes()
    m = Manifest(tomllib.loads(raw.decode()))
    m.sha256 = hashlib.sha256(raw).hexdigest()
    where = f"manifest {path.name}"
    if unknown := set(m) - REQUIRED - OPTIONAL:
        raise ValueError(f"{where}: unknown keys {sorted(unknown)}")
    if missing := REQUIRED - set(m):
        raise ValueError(f"{where}: missing {sorted(missing)}")
    if missing := LICENSE - set(m["license"]):
        raise ValueError(f"{where}: license needs {sorted(missing)} (verified = date the terms were checked)")
    if not isinstance(m["license"]["derived_redistribution"], bool):
        raise ValueError(f"{where}: license.derived_redistribution must be true or false")
    if "kind" not in m["access"]:
        raise ValueError(f"{where}: access needs a kind")
    for v in m["variables"]:
        if missing := {"name", "unit", "description"} - set(v):
            raise ValueError(f"{where}: variable {v.get('name', '?')} needs {sorted(missing)}")
    return m


def require_redistributable(m):
    if not m["license"]["derived_redistribution"]:
        raise ValueError(f"dataset {m['dataset']}: its license does not allow redistributing derived data, so no pack may contain it")


def all_manifests():
    return [load(p) for p in sorted((DATA / "manifests").glob("*.toml"))]
