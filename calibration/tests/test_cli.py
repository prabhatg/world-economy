"""The calibration command line (`python -m oikos_cal`)."""

import oikos_cal.__main__ as cli


def test_a_profile_holds_every_parameter_set_however_the_flags_are_written(monkeypatch):
    seen = {}
    monkeypatch.setattr(cli, "profile", lambda args: seen.update(held=args.set))
    cli.main(["profile", "herd_pressure_weight", "0", "0.8", "--set", "a=1", "--set", "b=2", "c=3", "--out", "x"])
    assert seen["held"] == ["a=1", "b=2", "c=3"], "a repeated --set adds to the held parameters"
