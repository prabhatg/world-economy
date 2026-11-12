"""The calibration command line (`python -m oikos_cal`)."""

import oikos_cal.__main__ as cli


def test_a_profile_holds_every_parameter_set_however_the_flags_are_written(monkeypatch):
    seen = {}
    monkeypatch.setattr(cli, "profile", lambda args: seen.update(held=args.set))
    cli.main(["profile", "herd_pressure_weight", "0", "0.8", "--set", "a=1", "--set", "b=2", "c=3", "--out", "x"])
    assert seen["held"] == ["a=1", "b=2", "c=3"], "a repeated --set adds to the held parameters"


def test_best_records_a_round_with_no_consistent_run_without_running_any(monkeypatch, tmp_path, capsys):
    import json

    wave = tmp_path / "wave1.json"
    wave.write_text(json.dumps({"wave": 1, "sites": cli.SITES, "left_out": []}))
    monkeypatch.setattr(cli, "candidates", lambda docs, top: [])
    monkeypatch.setattr(cli, "_problem_targets", lambda leave: ([], ["x"], []))
    monkeypatch.setattr(cli, "_run", lambda *a, **k: (_ for _ in ()).throw(AssertionError("nothing to rerun")))
    out = tmp_path / "best.json"
    cli.main(["best", str(wave), "--out", str(out)])
    doc = json.loads(out.read_text())
    assert doc["candidates"] == [] and doc["chosen"] is None and doc["consistent_runs"] == 0
    assert "no run of the waves is consistent" in capsys.readouterr().out
