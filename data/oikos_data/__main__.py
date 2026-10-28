"""Build and check forcing packs.

    python -m oikos_data build <kind> <region> [--out DIR] [--update-lock]
    python -m oikos_data check        # rebuild every locked pack, compare ids

Packs go to data/build/ (not committed). data/packs.lock.json records each pack's id:
a build that differs from the lock fails unless --update-lock says the change is on
purpose (like the engine's golden file).
"""

import argparse
import json
import sys
from pathlib import Path

from . import DATA, geometry, pack, region

LOCK = DATA / "packs.lock.json"
BUILDERS = {"geometry": geometry.build}


def load_lock():
    return json.loads(LOCK.read_text()) if LOCK.exists() else {}


def build(kind, region_name):
    if kind not in BUILDERS:
        raise SystemExit(f"unknown pack kind {kind} (known: {', '.join(BUILDERS)})")
    data = BUILDERS[kind](region.load(region_name))
    return data, pack.read(data).id


def cmd_build(args):
    data, pid = build(args.kind, args.region)
    key = f"{args.region}/{args.kind}"
    lock = load_lock()
    locked = lock.get(key, {}).get("id")
    if locked and locked != pid and not args.update_lock:
        raise SystemExit(f"{key}: built {pid}, lock has {locked}; if the change is intended, rerun with --update-lock")
    args.out.mkdir(parents=True, exist_ok=True)
    path = args.out / f"{args.region}.{args.kind}.oikospack"
    path.write_bytes(data)
    if locked != pid:
        lock[key] = {"id": pid, "bytes": len(data)}
        LOCK.write_text(json.dumps(lock, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"pack": key, "id": pid, "bytes": len(data), "path": str(path)}))


def cmd_check(_args):
    bad = 0
    for key, want in sorted(load_lock().items()):
        region_name, kind = key.split("/")
        _, pid = build(kind, region_name)
        ok = pid == want["id"]
        bad += not ok
        print(f"{key:32s} {pid[:16]} {'OK' if ok else 'MISMATCH (lock ' + want['id'][:16] + ')'}")
    sys.exit(1 if bad else 0)


def main():
    ap = argparse.ArgumentParser(prog="python -m oikos_data")
    sub = ap.add_subparsers(dest="cmd", required=True)
    b = sub.add_parser("build", help="build one pack")
    b.add_argument("kind")
    b.add_argument("region")
    b.add_argument("--out", type=Path, default=DATA / "build")
    b.add_argument("--update-lock", action="store_true")
    b.set_defaults(fn=cmd_build)
    sub.add_parser("check", help="rebuild locked packs and compare ids").set_defaults(fn=cmd_check)
    args = ap.parse_args()
    args.fn(args)


if __name__ == "__main__":
    main()
