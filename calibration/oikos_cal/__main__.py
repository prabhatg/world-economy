"""Calibration runs from the command line (docs/05 §5.2, ADR-0052).

    python -m oikos_cal screen --trajectories 10 --out runs/m1/screening.json
    python -m oikos_cal wave --runs 200 --screening runs/m1/screening.json --out runs/m1/wave1.json
    python -m oikos_cal wave --runs 200 --previous runs/m1/wave1.json --out runs/m1/wave2.json
    python -m oikos_cal best runs/m1/wave1.json runs/m1/wave2.json --out runs/m1/best.json
    python -m oikos_cal profile obsidian_loss 0.002 0.003 0.004 --out runs/m1/obsidian_loss.json
    python -m oikos_cal ensemble runs/m1/round10/best-wave6.json --out ../viewer/data/m1

Each command records what it ran (unit-cube points, parameter values, seeds, outputs,
targets with their variances, the code revision) in one JSON file, so waves can be
refitted, continued or audited on any machine.
"""

import argparse
import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path

import numpy as np

from . import emulator, history, screening

CUTOFF = history.CUTOFF
COORDINATOR = "an oikos-coord serve URL: run the worlds on the hosts working for it, score them here (ADR-0097)"
REDUCED = ("with --coordinator: hosts upload their records summed over the operators' zones, 1.7 MB instead of "
           "91 MB, for volunteers; outputs agree with the full record's to rounding (#47)")
LEAVE_OUT = "records not to judge or record (m1.OPTIONAL; default: those of the files continued, else none)"
SITES = "settlements"  # what the site targets compare with the surveys (ADR-0072)


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


def _run(jobs, workers, leave_out=(), coordinator=None, reduced=False, region="m1-swasia", rings=1):
    from . import m1

    start = time.time()
    out = m1.evaluate(jobs, workers, leave_out, coordinator, reduced=reduced, region=region, rings=rings)
    print(f"{len(jobs)} runs in {time.time() - start:.0f} s ({workers} workers)", file=sys.stderr)
    return out


def _problem_targets(leave_out=(), region="m1-swasia", rings=1):
    from . import m1

    p = m1.Problem(leave_out, region, rings)
    return p.targets, p.names, p.groups


def _left_out_of(doc):
    """The records a file left out. Files before the option record none; those with targets
    and no house Ginis judged without them (round 9 came before the record)."""
    if "left_out" in doc:
        return sorted(doc["left_out"])
    if "targets" in doc and not any(t["name"].startswith("house Gini") for t in doc["targets"]):
        return ["inequality"]
    return []


def left_out(given, docs):
    """The records a command leaves out: `given`, else those of the last of the files
    `docs` it continues, in order. A file may judge records the files before it left out
    (a later wave adding a record), never leave out one they judged: their emulators
    still judge it, so the waves would not describe one history match."""
    from . import m1

    theirs = [set(_left_out_of(d)) for d in docs]
    for k in range(1, len(theirs)):
        if not theirs[k] <= theirs[k - 1]:
            raise SystemExit(f"file {k + 1} left out {sorted(theirs[k] - theirs[k - 1])}, which the file before "
                             "it judged")
    chosen = sorted(given) if given is not None else sorted(theirs[-1]) if theirs else []
    if theirs and not set(chosen) <= theirs[-1]:
        raise SystemExit(f"--leave-out {chosen}: the files continued judged {sorted(set(chosen) - theirs[-1])}, "
                         "so it may not be left out")
    unknown = [r for r in chosen if r not in m1.OPTIONAL]
    if unknown:
        raise SystemExit(f"--leave-out: {unknown} cannot be left out; only {list(m1.OPTIONAL)} can be")
    return chosen


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
    leave = left_out(args.leave_out, [])
    ts, names, _ = _problem_targets(leave)
    out = _run([(p, args.seed) for p in params] + [({}, s) for s in noise], args.workers, leave, args.coordinator, args.reduced)
    runs, noisy = out[: len(params)], out[len(params) :]
    _write(args.out, {"command": "screen", "revision": _revision(), "seed": args.seed, "sites": SITES,
                      "left_out": leave, "levels": args.levels,
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
    peak = _defined(scaled).max(axis=1)
    ranked = [int(i) for i in np.argsort(-peak, kind="stable") if peak[i] >= threshold]
    return sorted(ranked[:max_active])


def _defined(scaled):
    """Effects with those no run defined (NaN) as -inf: a factor is ranked by the targets
    its effects are defined for."""
    return np.where(np.isnan(scaled), -np.inf, scaled)


def report_screening(doc):
    e, scaled, noise = screening_ranks(doc)
    ts = _targets(doc["targets"])
    order = np.argsort(-_defined(scaled).max(axis=1))
    print(f"{'factor':32} {'max mu*/tol':>11}  most affected target")
    for i in order:
        j = int(_defined(scaled[i]).argmax())
        print(f"{doc['factors'][i]:32} {scaled[i, j]:11.2f}  {ts[j].name}")
    print("seed noise / tol: " + ", ".join(f"{t.name} {n:.2f}" for t, n in zip(ts, noise)))


def extend_factors(previous, free, inactive=()):
    """Factors of a wave after waves over `previous`: those plus the free parameters new
    to the registry, appended so earlier waves' factor indices hold, and the indices of
    the new ones to emulate. Earlier waves never judged the new factors (they ran at the
    defaults), so NROY points take them uniformly and the new wave's emulators see them,
    except those `inactive`: parameters no record of the wave can see yet, sampled as
    noise (ADR-0068)."""
    new = [f for f in free if f not in previous]
    unknown = [n for n in inactive if n not in new]
    if unknown:
        raise SystemExit(f"--inactive: {unknown} not a parameter new to the registry; new: {new}")
    return list(previous) + new, [len(previous) + i for i, f in enumerate(new) if f not in inactive]


def activate(factors, active, names):
    """`active` plus the indices of the factors `names`: factors earlier waves varied as
    noise that a later wave's emulators should see (when a new mechanism makes them
    matter). NROY points still take them as earlier waves did, uniformly."""
    unknown = [n for n in names if n not in factors]
    if unknown:
        raise SystemExit(f"--activate: {unknown} not a factor of the earlier waves; factors: {factors}")
    return sorted(set(active) | {factors.index(n) for n in names})


def _fit_wave(doc, path=None, workers=1):
    """A wave file's emulators. Searching for their hyperparameters is the costly part of
    a later wave (#32), so they are kept in emulators/ beside the file, with a key of the
    runs they were fitted to, and rebuilt from there while the runs and the search stay
    the same."""
    ts = _targets(doc["targets"])
    u, y = np.array(doc["u"]), np.array(doc["outputs"])[:, : len(ts)]
    key = hashlib.sha256(json.dumps({"fit": emulator.FIT, "u": u.tolist(), "outputs": y.tolist(),
                                     "active": doc["active"]}).encode()).hexdigest()
    kept = Path(path).parent / "emulators" / Path(path).name if path else None
    thetas = None
    if kept and kept.exists():
        try:
            saved = json.loads(kept.read_text())
        except json.JSONDecodeError:
            saved = {}
        if saved.get("key") == key:
            thetas = saved["thetas"]
        else:
            print(f"{kept}: unreadable, or fitted to other runs or by another search; fitting again", file=sys.stderr)
    w = history.Wave.fit(u, y, ts, active=doc["active"], rng=np.random.default_rng(0), groups=_groups(doc.get("groups")),
                         thetas=thetas, workers=workers)
    if kept and thetas is None:  # whole or not at all, for a command reading it meanwhile
        kept.parent.mkdir(parents=True, exist_ok=True)
        part = kept.with_name(f"{kept.name}.{os.getpid()}.tmp")
        part.write_text(json.dumps({"key": key, "fit": emulator.FIT, "thetas": w.thetas}, indent=1) + "\n")
        part.replace(kept)
        print(f"wrote {kept}", file=sys.stderr)
    return w


def sampling_seed(seed, previous):
    """The seed of a wave's points: `seed` if given, else the wave's number, so waves never
    draw the same NROY candidates (and repeat the points earlier waves kept)."""
    return len(previous) + 1 if seed is None else seed


def wave(args):
    import oikos

    previous = [json.loads(Path(p).read_text()) for p in args.previous]
    require_sites(previous, "continue")
    chosen_by = json.loads(Path(args.screening).read_text()) if args.screening else None
    if chosen_by:
        require_sites([chosen_by], "choose factors by")
    leave = left_out(args.leave_out, previous)
    seed = sampling_seed(args.seed, previous)
    rng = np.random.default_rng(seed)
    if previous:
        factors, active = previous[-1]["factors"], previous[-1]["active"]
        if args.screening:  # refocus: later emulators may see more factors than earlier ones
            active = active_factors(chosen_by, args.threshold, args.max_active)
        factors, new = extend_factors(factors, screening.free_parameters(oikos.registry()), args.inactive)
        active = activate(factors, sorted(set(active) | set(new)), args.activate)
        if args.activate:
            print(f"activated: {args.activate}", file=sys.stderr)
        if new:
            print(f"new factors, active in this wave: {[factors[i] for i in new]}", file=sys.stderr)
        waves = [_fit_wave(d, p, args.workers) for d, p in zip(previous, args.previous)]
        u, kept = history.nroy(waves, len(factors), args.runs, rng)
        print(f"NROY after {len(waves)} waves: {kept:.4f} of the space", file=sys.stderr)
        if len(u) < args.runs:
            raise SystemExit(f"only {len(u)} NROY points found; the space is (nearly) ruled out")
    else:
        factors = screening.free_parameters(oikos.registry())
        if args.screening:
            active = active_factors(chosen_by, args.threshold, args.max_active)
        else:
            active = list(range(len(factors)))
        u = emulator.latin_hypercube(args.runs, len(factors), rng)
    number = len(previous) + 1
    params = [oikos.sample_params(dict(zip(factors, x))) for x in u]
    seeds = [1000 * number + i for i in range(len(u))]
    ts, names, groups = _problem_targets(leave)
    out = _run(list(zip(params, seeds)), args.workers, leave, args.coordinator, args.reduced)
    doc = {"command": "wave", "wave": number, "revision": _revision(), "seed": seed, "sites": SITES,
           "left_out": leave, "factors": factors,
           "active": active, "u": u.tolist(), "params": params, "seeds": seeds, "names": names,
           "outputs": out.tolist(), "targets": _targets_json(ts), "groups": _groups_json(groups),
           "previous": [str(p) for p in args.previous]}
    _write(args.out, doc)
    report_wave(previous + [doc], args.previous + [args.out], args.workers)


def consistent(doc):
    """Runs not ruled out by their own outputs: every target within the cutoff and every
    group within its chi-squared cutoff."""
    ts = _targets(doc["targets"])
    y = np.array(doc["outputs"])[:, : len(ts)]
    ok = history.implausibility(y, 0.0, ts).max(axis=1) <= CUTOFF
    for g in _groups(doc.get("groups")):
        ok &= g.implausibility(y, 0.0, ts) <= history.chi2_cutoff(g.dof)
    return ok


def summary_log_likelihood(doc, outputs):
    """`history.log_likelihood` of `outputs` [run, name] against `doc`'s targets and groups."""
    y = np.asarray(outputs, dtype=float)
    return history.log_likelihood(y, _targets(doc["targets"]), _groups(doc.get("groups")))


def _sites(doc):
    """What a file's site targets compared with the surveys: settled people-years before
    ADR-0072, settlements since."""
    return doc.get("sites", "settled people-years")


def require_sites(docs, what):
    """Refuse to `what` (continue, rerun) waves whose site targets used another model than
    the engine's now: emulators and scores of the two do not mix (ADR-0072)."""
    other = [f"wave {d['wave']}" if "wave" in d else "the screening" for d in docs if _sites(d) != SITES]
    if other:
        raise SystemExit(f"cannot {what} {', '.join(other)}: they judged sites by settled people-years, the engine "
                         f"now by {SITES} (ADR-0072); start a new round, from a new screening")


def candidates(docs, top):
    """The `top` consistent runs of waves `docs`, best first by how near their summaries
    sit to the record under history matching's own error model (ADR-0066), with only
    their active factors' values (`wave` counts from 0 by the file's own wave number): the
    factors the emulators did not see were varied as noise, so their values say nothing
    and a candidate leaves them at the registry defaults. The item log-likelihood
    diagnostics, summed, are kept beside it. Densities of different records do not
    compare, so every wave must have the same targets."""
    found = []
    for k, doc in enumerate(docs):
        if [t["name"] for t in doc["targets"]] != [t["name"] for t in docs[0]["targets"]]:
            raise ValueError(f"wave {doc.get('wave', k + 1)} is not judged against the same record as the first; "
                             "rank waves of one round of targets together")
        if _sites(doc) != _sites(docs[0]):
            raise ValueError(f"wave {doc.get('wave', k + 1)} judges sites by {_sites(doc)}, the first by "
                             f"{_sites(docs[0])} (ADR-0072); rank waves of one site model together")
        ll = [j for j, n in enumerate(doc["names"]) if n.startswith("log-likelihood")]
        y = np.array(doc["outputs"])
        fit = summary_log_likelihood(doc, y)
        active = [doc["factors"][i] for i in doc["active"]]
        for i in np.flatnonzero(consistent(doc)):
            found.append({"wave": doc.get("wave", k + 1) - 1, "run": int(i), "summary_log_likelihood": float(fit[i]),
                          "log_likelihood": float(y[i, ll].sum()), "params": {f: doc["params"][i][f] for f in active}})
    found.sort(key=lambda c: -c["summary_log_likelihood"])
    return found[:top]


def ranges(every):
    """10-50-90% quantiles of each factor over the consistent runs `every` whose waves
    emulated it: a wave that varied a factor as noise says nothing of it."""
    values = {}
    for c in every:
        for f, v in c["params"].items():
            values.setdefault(f, []).append(v)
    return {f: dict(zip(["q10", "q50", "q90"], np.quantile(v, [0.1, 0.5, 0.9]).tolist())) for f, v in values.items()}


def best(args):
    docs = [json.loads(Path(p).read_text()) for p in args.waves]
    require_sites(docs, "rerun the best runs of")
    leave = left_out(None, docs)
    cs = candidates(docs, args.top)
    seeds = list(range(1, args.seeds + 1))
    ts, names, groups = _problem_targets(leave)
    jobs = [(c["params"], s) for c in cs for s in seeds]  # none when no run of the waves is consistent
    out = _run(jobs, args.workers, leave, args.coordinator, args.reduced).reshape(len(cs), len(seeds), -1) if jobs else []
    check = {"targets": _targets_json(ts), "groups": _groups_json(groups)}
    ll = [j for j, n in enumerate(names) if n.startswith("log-likelihood")]
    for c, o in zip(cs, out):
        c["outputs"] = o.tolist()
        c["consistent"] = bool(consistent({**check, "outputs": o}).all())
        c["rerun_log_likelihood"] = float(o[:, ll].sum(axis=1).mean())
        c["rerun_summary_log_likelihood"] = float(summary_log_likelihood(check, o).mean())
    ok = [i for i, c in enumerate(cs) if c["consistent"]]
    chosen = max(ok, key=lambda i: cs[i]["rerun_summary_log_likelihood"]) if ok else None
    every = candidates(docs, top=None)
    _write(args.out, {"command": "best", "revision": _revision(), "rank": "summaries", "sites": SITES, "left_out": leave,
                      "waves": [str(p) for p in args.waves], "seeds": seeds, "names": names, "consistent_runs": len(every), "ranges": ranges(every),
                      "candidates": cs, "chosen": chosen})
    report_best(json.loads(Path(args.out).read_text()))


def profile_choice(doc):
    """The value (index) of a profile whose worlds are consistent with every seed and whose
    summaries sit nearest the record (`summary_log_likelihood`, averaged over seeds);
    None if none is."""
    ok, score = [], []
    for o in doc["outputs"]:
        ok.append(bool(consistent({**doc, "outputs": o}).all()))
        score.append(summary_log_likelihood(doc, o).mean())
    return max((i for i in range(len(ok)) if ok[i]), key=lambda i: score[i], default=None)


def _assignment(text):
    name, _, value = text.partition("=")
    if not name or not value:
        raise SystemExit(f"--set takes name=value, got {text!r}")
    return name, float(value)


def profile(args):
    """One parameter over `values`, every other at its registry default or held where
    `--set` says: for a parameter a new record constrains but which leaves the rest of the
    world alone (a tracer, as obsidian; ADR-0061), so the calibrated world stays the same
    world. `--region` and `--rings` judge the worlds on another grid (#40)."""
    leave = left_out(args.leave_out, [])
    held = dict(_assignment(a) for a in args.set or [])
    from .onset import CEREAL

    fixed = sorted({args.parameter, *held} & set(CEREAL))
    if fixed:
        raise SystemExit(f"{fixed}: the onset contrasts split the record by the wild-cereal parameters' "
                         "registry values, so a profile cannot change them")
    ts, names, groups = _problem_targets(leave, args.region, args.rings)
    seeds = list(range(1, args.seeds + 1))
    out = _run([({**held, args.parameter: v}, s) for v in args.values for s in seeds], args.workers, leave,
               args.coordinator, args.reduced, args.region, args.rings)
    doc = {"command": "profile", "revision": _revision(), "sites": SITES, "left_out": leave, "parameter": args.parameter,
           "values": args.values, **({"region": args.region, "rings": args.rings, "held": held}
                                     if (args.region, args.rings, held) != ("m1-swasia", 1, {}) else {}),
           "seeds": seeds, "names": names, "outputs": out.reshape(len(args.values), len(seeds), -1).tolist(),
           "targets": _targets_json(ts), "groups": _groups_json(groups)}
    doc["chosen"] = profile_choice(doc)
    doc["rank"] = "summaries"
    _write(args.out, doc)
    report_profile(doc)


def report_profile(doc):
    ll = [j for j, n in enumerate(doc["names"]) if n.startswith("log-likelihood")]
    print(f"{doc['parameter']:>14} {'summaries':>9} {'items':>9}  consistent")
    for v, o in zip(doc["values"], doc["outputs"]):
        print(f"{v:14.4g} {summary_log_likelihood(doc, o).mean():9.1f} {np.array(o)[:, ll].sum(axis=1).mean():9.1f}  "
              f"{'yes' if consistent({**doc, 'outputs': o}).all() else 'no'}")
    c, by = doc["chosen"], "chosen" if doc.get("rank") == "summaries" else \
        "chosen by the item log-likelihoods (before ADR-0066)"
    print("no value is consistent" if c is None else f"{by}: {doc['parameter']} = {doc['values'][c]:.4g}")


def report_best(doc):
    """Files before ADR-0066 ranked by the item log-likelihoods alone."""
    by_summaries = doc.get("rank") == "summaries"
    print(f"{'wave':>4} {'run':>4} {'in wave':>9} {'rerun':>9} {'items':>9}  consistent at defaults")
    for c in doc["candidates"]:
        fit = (c["summary_log_likelihood"], c["rerun_summary_log_likelihood"]) if by_summaries else \
            (c["log_likelihood"], c["rerun_log_likelihood"])
        print(f"{c['wave'] + 1:4} {c['run']:4} {fit[0]:9.1f} {fit[1]:9.1f} {c['rerun_log_likelihood']:9.1f}  "
              f"{'yes' if c['consistent'] else 'no'}")
    if not doc["candidates"]:
        print("no run of the waves is consistent with every record: nothing to rerun")
        return
    if doc["chosen"] is None:
        print("no candidate stays consistent with the other parameters at their defaults")
        return
    c = doc["candidates"][doc["chosen"]]
    print(f"chosen: wave {c['wave'] + 1} run {c['run']} ({doc['consistent_runs']} consistent runs give the ranges)")
    for f, v in c["params"].items():
        r = doc["ranges"][f]
        print(f"  {f:32} {v:<11.4g} consistent runs 10-90%: {r['q10']:.3g} to {r['q90']:.3g}")


def holdout_coverage(u, y, ts, active, workers=1):
    """Emulator check: per target, the share of held-out runs within 3 standard errors of
    emulators fitted to the others (the first 80% of the runs whose outputs are defined)."""
    defined = np.isfinite(y).all(axis=1)
    u, y = u[defined], y[defined]
    cut = int(0.8 * len(u))
    test = history.Wave.fit(u[:cut], y[:cut], ts, active=active, rng=np.random.default_rng(0), workers=workers)
    x = u[cut:][:, active]
    coverage = []
    for j, g in enumerate(test.emulators):
        m, v = g.predict(x, noise=True)
        coverage.append(float((np.abs(y[cut:, j] - m) / np.sqrt(v) < 3).mean()))
    return coverage


def best_run(implausibility):
    """The run [run, target] whose largest implausibility is smallest; runs with an
    undefined summary are ruled out, never best."""
    worst = implausibility.max(axis=1)
    return int(np.where(np.isnan(worst), np.inf, worst).argmin())


def report_wave(docs, paths=None, workers=1):
    doc = docs[-1]
    ts = _targets(doc["targets"])
    y = np.array(doc["outputs"])[:, : len(ts)]
    u = np.array(doc["u"])
    print(f"wave {doc['wave']}: {len(u)} runs; active factors: {[doc['factors'][i] for i in doc['active']]}")
    coverage = holdout_coverage(u, y, ts, doc["active"], workers)
    run_i = history.implausibility(y, 0.0, ts)
    gs = _groups(doc.get("groups"))
    run_g = np.stack([g.implausibility(y, 0.0, ts) for g in gs], axis=1) if gs else np.zeros((len(y), 0))
    print(f"{'target':36} {'observed':>9} {'tol':>6} {'runs in 3 tol':>13} {'emulator |z|<3':>14}")
    for j, t in enumerate(ts):
        print(f"{t.name:36} {t.observed:9.3f} {np.sqrt(t.variance + t.discrepancy):6.3f} "
              f"{(run_i[:, j] <= CUTOFF).mean():13.2f} {coverage[j]:14.2f}")
    best = best_run(run_i)
    kept = consistent(doc)
    print(f"best run {best}: max implausibility {run_i[best].max():.2f} ({ts[int(run_i[best].argmax())].name}); "
          f"runs not ruled out by their own outputs: {kept.sum()} ({(run_i.max(axis=1) <= CUTOFF).sum()} one by one)")
    for k, g in enumerate(gs):
        print(f"  group {k} ({len(g.indices)} targets, dof {g.dof}): cutoff {history.chi2_cutoff(g.dof):.1f}, "
              f"runs within {(run_g[:, k] <= history.chi2_cutoff(g.dof)).mean():.2f}, best {np.nanmin(run_g[:, k]):.1f}")
    print("  " + ", ".join(f"{k}={v:.4g}" for k, v in sorted(doc["params"][best].items())))
    waves = [_fit_wave(d, p, workers) for d, p in zip(docs, paths or [None] * len(docs))]
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
    s.add_argument("--leave-out", nargs="*", help=LEAVE_OUT)
    s.add_argument("--coordinator", help=COORDINATOR)
    s.add_argument("--reduced", action="store_true", help=REDUCED)
    s.add_argument("--out", required=True)
    w = sub.add_parser("wave", help="one history-matching wave")
    w.add_argument("--runs", type=int, default=200)
    w.add_argument("--previous", nargs="*", default=[], help="earlier waves' files, in order")
    w.add_argument("--screening", help="screening file choosing the emulators' active factors (else the last wave's)")
    w.add_argument("--threshold", type=float, default=0.5, help="active if max mu*/tol reaches this")
    w.add_argument("--max-active", type=int, default=10)
    w.add_argument("--activate", nargs="*", default=[], help="factors earlier waves varied as noise to emulate now")
    w.add_argument("--inactive", nargs="*", default=[],
                   help="parameters new to the registry to sample as noise, not emulate (no record sees them yet)")
    w.add_argument("--seed", type=int, help="seed of the wave's points (default: its number)")
    w.add_argument("--workers", type=int, default=workers)
    w.add_argument("--leave-out", nargs="*", help=LEAVE_OUT)
    w.add_argument("--coordinator", help=COORDINATOR)
    w.add_argument("--reduced", action="store_true", help=REDUCED)
    w.add_argument("--out", required=True)
    b = sub.add_parser("best", help="rerun the best consistent runs with the other parameters at their defaults")
    b.add_argument("waves", nargs="+", help="wave files, in order")
    b.add_argument("--top", type=int, default=10)
    b.add_argument("--seeds", type=int, default=3)
    b.add_argument("--workers", type=int, default=workers)
    b.add_argument("--coordinator", help=COORDINATOR)
    b.add_argument("--reduced", action="store_true", help=REDUCED)
    b.add_argument("--out", required=True)
    pr = sub.add_parser("profile", help="one parameter over values, the others at their defaults")
    pr.add_argument("parameter")
    pr.add_argument("values", nargs="+", type=float)
    pr.add_argument("--seeds", type=int, default=3)
    pr.add_argument("--workers", type=int, default=workers)
    pr.add_argument("--leave-out", nargs="*", help=LEAVE_OUT)
    pr.add_argument("--coordinator", help=COORDINATOR)
    pr.add_argument("--reduced", action="store_true", help=REDUCED)
    pr.add_argument("--set", nargs="*", action="extend", metavar="NAME=VALUE",
                    help="other parameters held at these values, not their defaults (repeat or list them)")
    pr.add_argument("--region", default="m1-swasia", help="the region whose packs and cells the worlds run on (#40)")
    pr.add_argument("--rings", type=int, default=1, help="cells around each site the operators read (#40)")
    pr.add_argument("--out", required=True)
    e = sub.add_parser("ensemble", help="run plausible worlds with outputs for the viewer")
    e.add_argument("best", help="a best file: its consistent candidates join the calibrated defaults")
    e.add_argument("--worlds", type=int, default=8, help="candidates besides the defaults")
    e.add_argument("--seed", type=int, default=1)
    e.add_argument("--every", type=int, default=100, help="years between recorded fields")
    e.add_argument("--oikos", default="../engine/target/release/oikos", help="the oikos CLI")
    e.add_argument("--workers", type=int, default=workers)
    e.add_argument("--out", required=True)
    r = sub.add_parser("report", help="report a screening, wave or best file (and the waves before it)")
    r.add_argument("files", nargs="+")
    r.add_argument("--workers", type=int, default=workers)
    args = ap.parse_args(argv)
    if args.command == "screen":
        screen(args)
    elif args.command == "wave":
        wave(args)
    elif args.command == "best":
        best(args)
    elif args.command == "profile":
        profile(args)
    elif args.command == "ensemble":
        from . import ensemble

        c = ensemble.build(args.best, args.out, k=args.worlds, seed=args.seed, every=args.every, oikos=args.oikos,
                           workers=args.workers)
        print(f"wrote {args.out}/catalog.json: {len(c['worlds'])} worlds, {len(c['years'])} times", file=sys.stderr)
    else:
        docs = [json.loads(Path(f).read_text()) for f in args.files]
        report = {"screen": lambda: report_screening(docs[0]), "best": lambda: report_best(docs[0]),
                  "profile": lambda: report_profile(docs[0])}
        left_out(None, docs)
        report.get(docs[0]["command"], lambda: report_wave(docs, args.files, args.workers))()


if __name__ == "__main__":
    main()
