"""Calibration runs from the command line (docs/05 §5.2, ADR-0052).

    python -m oikos_cal screen --trajectories 10 --out runs/m1/screening.json
    python -m oikos_cal wave --runs 200 --screening runs/m1/screening.json --out runs/m1/wave1.json
    python -m oikos_cal wave --runs 200 --previous runs/m1/wave1.json --out runs/m1/wave2.json

Each command records what it ran (unit-cube points, parameter values, seeds, outputs,
targets with their variances, the code revision) in one JSON file, so waves can be
refitted, continued or audited on any machine.
"""

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path

import numpy as np

from . import emulator, history, screening

CUTOFF = history.CUTOFF


def _revision():
    try:
        rev = subprocess.run(["git", "rev-parse", "--short", "HEAD"], capture_output=True, text=True, check=True)
        dirty = subprocess.run(["git", "status", "--porcelain"], capture_output=True, text=True, check=True)
        return rev.stdout.strip() + ("-dirty" if dirty.stdout.strip() else "")
    except (OSError, subprocess.CalledProcessError):
        return "unknown"


def _targets_json(ts):
    return [{"name": t.name, "observed": t.observed, "variance": t.variance, "discrepancy": t.discrepancy} for t in ts]


def _targets(js):
    return [history.Target(t["name"], t["observed"], t["variance"], t["discrepancy"]) for t in js]


def _groups_json(gs):
    return [{"indices": g.indices, "covariance": g.covariance.tolist(), "dof": g.dof} for g in gs]


def _groups(js):
    return [history.Group(g["indices"], g["covariance"], g["dof"]) for g in js or []]


def _run(jobs, workers):
    from . import m1

    start = time.time()
    out = m1.evaluate(jobs, workers)
    print(f"{len(jobs)} runs in {time.time() - start:.0f} s ({workers} workers)", file=sys.stderr)
    return out


def _problem_targets():
    from . import m1

    p = m1.Problem()
    return p.targets, p.names, p.groups


def _write(path, doc):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(doc, indent=1) + "\n")
    print(f"wrote {path}", file=sys.stderr)


def screen(args):
    import oikos

    factors = screening.free_parameters(oikos.registry())
    design = screening.morris(len(factors), args.trajectories, args.levels, np.random.default_rng(args.seed))
    points = design.reshape(-1, len(factors))
    params = [oikos.sample_params(dict(zip(factors, u))) for u in points]
    noise = list(range(args.seed, args.seed + args.noise_seeds))
    ts, names, _ = _problem_targets()
    out = _run([(p, args.seed) for p in params] + [({}, s) for s in noise], args.workers)
    runs, noisy = out[: len(params)], out[len(params) :]
    _write(args.out, {"command": "screen", "revision": _revision(), "seed": args.seed, "levels": args.levels,
                      "factors": factors, "design": design.tolist(), "params": params, "names": names,
                      "outputs": runs.tolist(), "noise_seeds": noise, "noise_outputs": noisy.tolist(),
                      "targets": _targets_json(ts)})
    report_screening(json.loads(Path(args.out).read_text()))


def screening_ranks(doc):
    """Per factor, the largest μ* over the targets in units of each target's tolerance
    sqrt(V_obs + V_discrepancy), and the seed noise of each target in the same units."""
    design = np.array(doc["design"])
    y = np.array(doc["outputs"]).reshape(design.shape[:2] + (-1,))
    ts = _targets(doc["targets"])
    tol = np.sqrt([t.variance + t.discrepancy for t in ts])
    e = screening.elementary_effects(design, y[..., : len(ts)])
    noise = np.array(doc["noise_outputs"])[:, : len(ts)].std(axis=0, ddof=1) / tol
    return e, e.mu_star / tol, noise


def report_screening(doc):
    e, scaled, noise = screening_ranks(doc)
    ts = _targets(doc["targets"])
    order = np.argsort(-scaled.max(axis=1))
    print(f"{'factor':32} {'max mu*/tol':>11}  most affected target")
    for i in order:
        j = int(scaled[i].argmax())
        print(f"{doc['factors'][i]:32} {scaled[i, j]:11.2f}  {ts[j].name}")
    print("seed noise / tol: " + ", ".join(f"{t.name} {n:.2f}" for t, n in zip(ts, noise)))


def _fit_wave(doc, rng):
    ts = _targets(doc["targets"])
    y = np.array(doc["outputs"])[:, : len(ts)]
    return history.Wave.fit(np.array(doc["u"]), y, ts, active=doc["active"], rng=rng, groups=_groups(doc.get("groups")))


def wave(args):
    import oikos

    previous = [json.loads(Path(p).read_text()) for p in args.previous]
    rng = np.random.default_rng(args.seed)
    if previous:
        factors, active = previous[-1]["factors"], previous[-1]["active"]
        waves = [_fit_wave(d, np.random.default_rng(0)) for d in previous]
        u, kept = history.nroy(waves, len(factors), args.runs, rng)
        print(f"NROY after {len(waves)} waves: {kept:.4f} of the space", file=sys.stderr)
        if len(u) < args.runs:
            raise SystemExit(f"only {len(u)} NROY points found; the space is (nearly) ruled out")
    else:
        factors = screening.free_parameters(oikos.registry())
        if args.screening:
            doc = json.loads(Path(args.screening).read_text())
            _, scaled, _ = screening_ranks(doc)
            active = [i for i in np.argsort(-scaled.max(axis=1)) if scaled[i].max() >= args.threshold]
            active = sorted(int(i) for i in active[: args.max_active])
        else:
            active = list(range(len(factors)))
        u = emulator.latin_hypercube(args.runs, len(factors), rng)
    number = len(previous) + 1
    params = [oikos.sample_params(dict(zip(factors, x))) for x in u]
    seeds = [1000 * number + i for i in range(len(u))]
    ts, names, groups = _problem_targets()
    out = _run(list(zip(params, seeds)), args.workers)
    doc = {"command": "wave", "wave": number, "revision": _revision(), "seed": args.seed, "factors": factors,
           "active": active, "u": u.tolist(), "params": params, "seeds": seeds, "names": names,
           "outputs": out.tolist(), "targets": _targets_json(ts), "groups": _groups_json(groups),
           "previous": [str(p) for p in args.previous]}
    _write(args.out, doc)
    report_wave(previous + [doc])


def report_wave(docs):
    doc = docs[-1]
    ts = _targets(doc["targets"])
    y = np.array(doc["outputs"])[:, : len(ts)]
    u = np.array(doc["u"])
    print(f"wave {doc['wave']}: {len(u)} runs; active factors: {[doc['factors'][i] for i in doc['active']]}")
    # emulator check: fit on 80% of the runs, standardized errors on the rest
    n = len(u)
    cut = int(0.8 * n)
    test = history.Wave.fit(u[:cut], y[:cut], ts, active=doc["active"], rng=np.random.default_rng(0))
    x = u[cut:][:, doc["active"]]
    run_i = history.implausibility(y, 0.0, ts)
    gs = _groups(doc.get("groups"))
    run_g = np.stack([g.implausibility(y, 0.0, ts) for g in gs], axis=1) if gs else np.zeros((len(y), 0))
    fails_group = (run_g > np.array([history.chi2_cutoff(g.dof) for g in gs])).any(axis=1)
    print(f"{'target':36} {'observed':>9} {'tol':>6} {'runs in 3 tol':>13} {'emulator |z|<3':>14}")
    for j, t in enumerate(ts):
        m, v = test.emulators[j].predict(x, noise=True)
        z = (y[cut:, j] - m) / np.sqrt(v)
        print(f"{t.name:36} {t.observed:9.3f} {np.sqrt(t.variance + t.discrepancy):6.3f} "
              f"{(run_i[:, j] <= CUTOFF).mean():13.2f} {(np.abs(z) < 3).mean():14.2f}")
    best = int(run_i.max(axis=1).argmin())
    kept = (run_i.max(axis=1) <= CUTOFF) & ~fails_group
    print(f"best run {best}: max implausibility {run_i[best].max():.2f} ({ts[int(run_i[best].argmax())].name}); "
          f"runs not ruled out by their own outputs: {kept.sum()} ({(run_i.max(axis=1) <= CUTOFF).sum()} one by one)")
    for k, g in enumerate(gs):
        print(f"  group {k} ({len(g.indices)} targets, dof {g.dof}): cutoff {history.chi2_cutoff(g.dof):.1f}, "
              f"runs within {(run_g[:, k] <= history.chi2_cutoff(g.dof)).mean():.2f}, best {run_g[:, k].min():.1f}")
    print("  " + ", ".join(f"{k}={v:.4g}" for k, v in sorted(doc["params"][best].items())))
    waves = [_fit_wave(d, np.random.default_rng(0)) for d in docs]
    space = np.random.default_rng(1).random((200_000, len(doc["factors"])))
    ok = np.ones(len(space), dtype=bool)
    for w in waves:
        ok[ok] = w.plausible(space[ok])
    print(f"NROY after wave {doc['wave']}: {ok.mean():.4f} of the prior space")
    if ok.any():
        lo, hi = np.quantile(space[ok], [0.05, 0.95], axis=0)
        for i in doc["active"]:
            print(f"  {doc['factors'][i]:32} u in [{lo[i]:.2f}, {hi[i]:.2f}]")


def main(argv=None):
    ap = argparse.ArgumentParser(prog="python -m oikos_cal", description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="command", required=True)
    workers = max(1, (os.cpu_count() or 2) - 1)
    s = sub.add_parser("screen", help="Morris screening of the free parameters")
    s.add_argument("--trajectories", type=int, default=10)
    s.add_argument("--levels", type=int, default=4)
    s.add_argument("--noise-seeds", type=int, default=8, help="runs of the defaults with different seeds")
    s.add_argument("--seed", type=int, default=1)
    s.add_argument("--workers", type=int, default=workers)
    s.add_argument("--out", required=True)
    w = sub.add_parser("wave", help="one history-matching wave")
    w.add_argument("--runs", type=int, default=200)
    w.add_argument("--previous", nargs="*", default=[], help="earlier waves' files, in order")
    w.add_argument("--screening", help="screening file choosing the emulators' active factors (wave 1)")
    w.add_argument("--threshold", type=float, default=0.5, help="active if max mu*/tol reaches this")
    w.add_argument("--max-active", type=int, default=10)
    w.add_argument("--seed", type=int, default=1)
    w.add_argument("--workers", type=int, default=workers)
    w.add_argument("--out", required=True)
    r = sub.add_parser("report", help="report a screening or wave file (and the waves before it)")
    r.add_argument("files", nargs="+")
    args = ap.parse_args(argv)
    if args.command == "screen":
        screen(args)
    elif args.command == "wave":
        wave(args)
    else:
        docs = [json.loads(Path(f).read_text()) for f in args.files]
        report_screening(docs[0]) if docs[0]["command"] == "screen" else report_wave(docs)


if __name__ == "__main__":
    main()
