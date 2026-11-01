"""Screening and wave files: what the command line records is enough to rank factors and
refit waves (docs/05 §5.2, ADR-0052)."""

import json

import numpy as np
import pytest

from oikos_cal import __main__ as cli
from oikos_cal import emulator, history, screening


def test_factors_are_ranked_in_units_of_each_target_tolerance():
    d = screening.morris(k=3, trajectories=6, levels=4, rng=np.random.default_rng(1))
    y = np.stack([d[..., 0], d[..., 1], np.zeros(d.shape[:2])], axis=-1)  # two targets and a diagnostic
    ts = [history.Target("a", 0, 0.0, 4.0), history.Target("b", 0, 0.01, 0.0)]
    doc = {"design": d.tolist(), "outputs": y.reshape(-1, 3).tolist(), "targets": cli._targets_json(ts),
           "noise_outputs": [[0.0, 0.1, 9], [0.0, -0.1, 9]]}
    doc = json.loads(json.dumps(doc))
    _, scaled, noise = cli.screening_ranks(doc)
    assert scaled[:, 0] == pytest.approx([0.5, 0, 0]) and scaled[:, 1] == pytest.approx([0, 10, 0])
    assert noise == pytest.approx([0, np.sqrt(0.02) / 0.1])


def test_a_wave_file_refits_to_the_same_emulators():
    rng = np.random.default_rng(2)
    u = emulator.latin_hypercube(30, 3, rng)
    y = np.stack([u[:, 0] + u[:, 1], u[:, 0]], axis=1)
    ts = [history.Target("a", 1.0, 0.01, 0.01), history.Target("b", 0.5, 0.01, 0.01)]
    doc = json.loads(json.dumps({"u": u.tolist(), "outputs": np.hstack([y, y[:, :1]]).tolist(),
                                 "targets": cli._targets_json(ts), "active": [0, 1]}))
    a, b = cli._fit_wave(doc, np.random.default_rng(0)), cli._fit_wave(doc, np.random.default_rng(0))
    x = rng.random((50, 3))
    assert (a.implausibility(x) == b.implausibility(x)).all()
    assert a.active == [0, 1] and len(a.emulators) == 2


def test_a_wave_file_keeps_its_groups():
    rng = np.random.default_rng(3)
    u = emulator.latin_hypercube(30, 2, rng)
    y = np.stack([u[:, 0], -u[:, 0]], axis=1)
    ts = [history.Target("a", 0.5, 0.01, 0.0), history.Target("b", -0.5, 0.01, 0.0)]
    g = history.Group([0, 1], [[0.01, -0.01], [-0.01, 0.01]], dof=1)
    doc = json.loads(json.dumps({"u": u.tolist(), "outputs": y.tolist(), "targets": cli._targets_json(ts),
                                 "groups": cli._groups_json([g]), "active": [0, 1]}))
    w = cli._fit_wave(doc, np.random.default_rng(0))
    assert len(w.groups) == 1 and w.groups[0].dof == 1
    assert np.array_equal(w.groups[0].covariance, g.covariance)
    assert cli._fit_wave({**doc, "groups": None}, np.random.default_rng(0)).groups == [], "round-1 files have none"


def _wave_doc(outputs, params):
    ts = [history.Target("a", 0.0, 0.01, 0.0), history.Target("b", 0.0, 0.01, 0.0)]
    g = history.Group([0, 1], [[0.01, 0.0], [0.0, 0.01]], dof=2)
    names = ["a", "b", "log-likelihood radiocarbon", "log-likelihood domestication", "people 3000 BCE"]
    return json.loads(json.dumps({"targets": cli._targets_json(ts), "groups": cli._groups_json([g]),
                                  "names": names, "outputs": outputs, "params": params,
                                  "factors": ["x", "y", "z"], "active": [0, 2]}))


def test_consistent_runs_pass_every_target_and_every_group():
    doc = _wave_doc([[0.0, 0.0, -1, -1, 5], [0.4, 0.0, -1, -1, 5], [0.25, 0.25, -1, -1, 5]], [{}] * 3)
    # run 1 is 4 sd off on a; run 2 is 2.5 sd off on both: fine one by one, not together (chi2 12.5 > 10.6)
    assert cli.consistent(doc).tolist() == [True, False, False]


def test_candidates_rank_consistent_runs_by_how_near_their_summaries_sit_and_keep_active_factors():
    p = [{"x": i, "y": 10 + i, "z": 20 + i} for i in range(4)]
    # The item log-likelihoods (columns 2-3) prefer (0, 0); the summaries prefer (0, 1).
    first = _wave_doc([[0.2, 0.1, -1, -1, 1], [0, 0, -10, -5, 1]], p[:2])
    second = _wave_doc([[0.1, 0, -3, -3, 1], [0.4, 0, 0, 0, 1]], p[2:])
    c = cli.candidates([first, second], top=5)
    assert [(k["wave"], k["run"]) for k in c] == [(0, 1), (1, 0), (0, 0)], "the inconsistent run is left out"
    ts, gs = cli._targets(first["targets"]), cli._groups(first["groups"])
    assert c[0]["summary_log_likelihood"] == pytest.approx(history.log_likelihood([[0, 0]], ts, gs)[0])
    assert c[0]["log_likelihood"] == -15, "the item log-likelihoods are kept as diagnostics"
    assert c[0]["params"] == {"x": 1, "z": 21}, "inactive y is left at its default"
    assert len(cli.candidates([first, second], top=1)) == 1
    assert cli.candidates([{**second, "wave": 4}], top=1)[0]["wave"] == 3, "a wave keeps its own number"
    renamed = {**second, "targets": [{**t, "name": "c"} for t in second["targets"]]}
    with pytest.raises(ValueError, match="the same record"):
        cli.candidates([first, renamed], top=5)


def test_ranges_take_each_factor_from_the_runs_that_emulated_it():
    # A later wave activated y: the earlier wave's runs varied it as noise, so say nothing of it.
    every = [{"params": {"x": 1.0}}, {"params": {"x": 3.0, "y": 10.0}}, {"params": {"x": 2.0, "y": 20.0}}]
    r = cli.ranges(every)
    assert list(r) == ["x", "y"]
    assert r["x"]["q50"] == 2.0 and r["y"]["q50"] == 15.0
    assert cli.ranges([]) == {}


def test_a_later_wave_can_refocus_on_more_factors():
    d = screening.morris(k=4, trajectories=6, levels=4, rng=np.random.default_rng(4))
    y = np.stack([d[..., 0] + 0.5 * d[..., 2] + 0.2 * d[..., 3]], axis=-1)
    ts = [history.Target("a", 0, 0.0, 0.01)]
    doc = json.loads(json.dumps({"design": d.tolist(), "outputs": y.reshape(-1, 1).tolist(),
                                 "targets": cli._targets_json(ts), "noise_outputs": [[0.0], [0.0]]}))
    assert cli.active_factors(doc, threshold=3, max_active=4) == [0, 2]
    assert cli.active_factors(doc, threshold=1, max_active=4) == [0, 2, 3]
    assert cli.active_factors(doc, threshold=1, max_active=2) == [0, 2], "the most influential first"


def test_a_later_wave_can_activate_factors_by_name():
    assert cli.activate(["a", "b", "c", "d"], [0, 3], ["c"]) == [0, 2, 3]
    assert cli.activate(["a", "b"], [1], []) == [1]
    assert cli.activate(["a", "b"], [1], ["b"]) == [1], "already active"
    with pytest.raises(SystemExit, match="not a factor"):
        cli.activate(["a", "b"], [1], ["z"])


def test_a_later_wave_adds_parameters_new_to_the_registry_as_active_factors():
    factors, new = cli.extend_factors(["a", "c", "d"], ["a", "b", "c", "d", "e"])
    assert factors == ["a", "c", "d", "b", "e"] and new == [3, 4], "appended, so earlier waves' indices hold"
    assert cli.extend_factors(["a", "b"], ["a", "b"]) == (["a", "b"], [])
    # Earlier waves judge only their own factors: a point's new coordinates never matter.
    u = emulator.latin_hypercube(30, 2, np.random.default_rng(1))
    ts = [history.Target("a", 0.5, 0.0001, 0.0)]
    w = history.Wave.fit(u, u[:, :1], ts, active=[0], rng=np.random.default_rng(0))
    x = np.random.default_rng(2).random((200, 3))
    assert (w.plausible(x) == w.plausible(np.c_[x[:, :2], 1 - x[:, 2]])).all()


def test_a_profile_chooses_the_consistent_value_whose_summaries_sit_nearest():
    doc = _wave_doc([], [])
    # three values of one parameter, two seeds each: outputs [value][seed]
    doc["outputs"] = [[[0.2, 0.0, -1, -1, 5], [0.2, 0.0, -1, -1, 5]],
                      [[0.0, 0.0, -9, -1, 5], [0.4, 0.0, -9, -1, 5]],  # nearest, but 4 sd off on one seed
                      [[0.1, 0.0, -5, -1, 5], [0.0, 0.1, -4, -1, 5]]]
    assert cli.profile_choice(doc) == 2, "not the best item log-likelihoods (value 0)"
    doc["outputs"][2][1][0] = 0.4
    assert cli.profile_choice(doc) == 0
    doc["outputs"] = [[[0.4, 0.0, -1, -1, 5]]]
    assert cli.profile_choice(doc) is None, "no value stays consistent"


def test_reports_say_what_an_older_file_ranked_by(capsys):
    doc = {**_wave_doc([], []), "command": "profile", "parameter": "x", "values": [1.0], "chosen": 0,
           "outputs": [[[0.0, 0.0, -1, -1, 5]]]}
    cli.report_profile(doc)
    assert "chosen by the item log-likelihoods (before ADR-0066): x = 1" in capsys.readouterr().out
    cli.report_profile({**doc, "rank": "summaries"})
    assert "chosen: x = 1" in capsys.readouterr().out
