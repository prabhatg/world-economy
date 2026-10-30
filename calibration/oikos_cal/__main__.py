"""Calibration runs from the command line (docs/05 §5.2, ADR-0052).

    python -m oikos_cal screen --trajectories 10 --out runs/m1/screening.json
    python -m oikos_cal wave --runs 200 --screening runs/m1/screening.json --out runs/m1/wave1.json
    python -m oikos_cal wave --runs 200 --previous runs/m1/wave1.json --out runs/m1/wave2.json
    python -m oikos_cal best runs/m1/wave1.json runs/m1/wave2.json --out runs/m1/best.json

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


def active_factors(doc, threshold, max_active):
    """Indices of the factors a wave's emulators see: at most `max_active` of those whose
    largest μ*/tolerance in screening `doc` reaches `threshold`, most influential first."""
    _, scaled, _ = screening_ranks(doc)
    ranked = [int(i) for i in np.argsort(-scaled.max(axis=1), kind="stable") if scaled[i].max() >= threshold]
    return sorted(ranked[:max_active])


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
        if args.screening:  # refocus: later emulators may see more factors than earlier ones
            active = active_factors(json.loads(Path(args.screening).read_text()), args.threshold, args.max_active)
        waves = [_fit_wave(d, np.random.default_rng(0)) for d in previous]
        u, kept = history.nroy(waves, len(factors), args.runs, rng)
        print(f"NROY after {len(waves)} waves: {kept:.4f} of the space", file=sys.stderr)
        if len(u) < args.runs:
            raise SystemExit(f"only {len(u)} NROY points found; the space is (nearly) ruled out")
    else:
        factors = screening.free_parameters(oikos.registry())
        if args.screening:
            active = active_factors(json.loads(Path(args.screening).read_text()), args.threshold, args.max_active)
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


def consistent(doc):
    """Runs not ruled out by their own outputs: every target within the cutoff and every
    group within its chi-squared cutoff."""
    ts = _targets(doc["targets"])
    y = np.array(doc["outputs"])[:, : len(ts)]
    ok = history.implausibility(y, 0.0, ts).max(axis=1) <= CUTOFF
    for g in _groups(doc.get("groups")):
        ok &= g.implausibility(y, 0.0, ts) <= history.chi2_cutoff(g.dof)
    return ok


def candidates(docs, top):
    """The `top` consistent runs of waves `docs`, best first by the sum of their
    log-likelihood diagnostics, with only their active factors' values (`wave` counts from
    0 by the file's own wave number): the factors the emulators did not see were varied as
    noise, so their values say nothing and a candidate leaves them at the registry
    defaults."""
    found = []
    for k, doc in enumerate(docs):
        ll = [j for j, n in enumerate(doc["names"]) if n.startswith("log-likelihood")]
        y = np.array(doc["outputs"])
        active = [doc["factors"][i] for i in doc["active"]]
        for i in np.flatnonzero(consistent(doc)):
            found.append({"wave": doc.get("wave", k + 1) - 1, "run": int(i), "log_likelihood": float(y[i, ll].sum()),
                          "params": {f: doc["params"][i][f] for f in active}})
    found.sort(key=lambda c: -c["log_likelihood"])
    return found[:top]


def best(args):
    docs = [json.loads(Path(p).read_text()) for p in args.waves]
    cs = candidates(docs, args.top)
    seeds = list(range(1, args.seeds + 1))
    ts, names, groups = _problem_targets()
    out = _run([(c["params"], s) for c in cs for s in seeds], args.workers).reshape(len(cs), len(seeds), -1)
    check = {"targets": _targets_json(ts), "groups": _groups_json(groups)}
    ll = [j for j, n in enumerate(names) if n.startswith("log-likelihood")]
    for c, o in zip(cs, out):
        c["outputs"] = o.tolist()
        c["consistent"] = bool(consistent({**check, "outputs": o}).all())
        c["rerun_log_likelihood"] = float(o[:, ll].sum(axis=1).mean())
    ok = [i for i, c in enumerate(cs) if c["consistent"]]
    chosen = max(ok, key=lambda i: cs[i]["rerun_log_likelihood"]) if ok else None
    every = candidates(docs, top=None)
    values = {f: [c["params"][f] for c in every] for f in cs[0]["params"]} if cs else {}
    ranges = {f: dict(zip(["q10", "q50", "q90"], np.quantile(v, [0.1, 0.5, 0.9]).tolist())) for f, v in values.items()}
    _write(args.out, {"command": "best", "revision": _revision(), "waves": [str(p) for p in args.waves],
                      "seeds": seeds, "names": names, "consistent_runs": len(every), "ranges": ranges,
                      "candidates": cs, "chosen": chosen})
    report_best(json.loads(Path(args.out).read_text()))


def report_best(doc):
    print(f"{'wave':>4} {'run':>4} {'log-lik in wave':>15} {'rerun':>9}  consistent at defaults")
    for c in doc["candidates"]:
        print(f"{c['wave'] + 1:4} {c['run']:4} {c['log_likelihood']:15.1f} {c['rerun_log_likelihood']:9.1f}  "
              f"{'yes' if c['consistent'] else 'no'}")
    if doc["chosen"] is None:
        print("no candidate stays consistent with the other parameters at their defaults")
        return
    c = doc["candidates"][doc["chosen"]]
    print(f"chosen: wave {c['wave'] + 1} run {c['run']} ({doc['consistent_runs']} consistent runs give the ranges)")
    for f, v in c["params"].items():
        r = doc["ranges"][f]
        print(f"  {f:32} {v:<11.4g} consistent runs 10-90%: {r['q10']:.3g} to {r['q90']:.3g}")


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
    print(f"{'target':36} {'observed':>9} {'tol':>6} {'runs in 3 tol':>13} {'emulator |z|<3':>14}")
    for j, t in enumerate(ts):
        m, v = test.emulators[j].predict(x, noise=True)
        z = (y[cut:, j] - m) / np.sqrt(v)
        print(f"{t.name:36} {t.observed:9.3f} {np.sqrt(t.variance + t.discrepancy):6.3f} "
              f"{(run_i[:, j] <= CUTOFF).mean():13.2f} {(np.abs(z) < 3).mean():14.2f}")
    best = int(run_i.max(axis=1).argmin())
    kept = consistent(doc)
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
    w.add_argument("--screening", help="screening file choosing the emulators' active factors (else the last wave's)")
    w.add_argument("--threshold", type=float, default=0.5, help="active if max mu*/tol reaches this")
    w.add_argument("--max-active", type=int, default=10)
    w.add_argument("--seed", type=int, default=1)
    w.add_argument("--workers", type=int, default=workers)
    w.add_argument("--out", required=True)
    b = sub.add_parser("best", help="rerun the best consistent runs with the other parameters at their defaults")
    b.add_argument("waves", nargs="+", help="wave files, in order")
    b.add_argument("--top", type=int, default=10)
    b.add_argument("--seeds", type=int, default=3)
    b.add_argument("--workers", type=int, default=workers)
    b.add_argument("--out", required=True)
    r = sub.add_parser("report", help="report a screening, wave or best file (and the waves before it)")
    r.add_argument("files", nargs="+")
    args = ap.parse_args(argv)
    if args.command == "screen":
        screen(args)
    elif args.command == "wave":
        wave(args)
    elif args.command == "best":
        best(args)
    else:
        docs = [json.loads(Path(f).read_text()) for f in args.files]
        report = {"screen": lambda: report_screening(docs[0]), "best": lambda: report_best(docs[0])}
        report.get(docs[0]["command"], lambda: report_wave(docs))()


if __name__ == "__main__":
    main()
