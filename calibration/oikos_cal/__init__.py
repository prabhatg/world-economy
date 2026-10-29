"""Calibration side of ManyWorlds: observation operators that map simulated worlds to what
the historical record measures (docs/05 §5.1).

Operators never change a world; they read activity recorded from it (`record`) and
score it against observation tables built by `oikos_data`. They run on the
calibration host, not in work units, so they use NumPy freely: volunteers' results are
validated by the engine's state hash, never by an operator's output.
"""
