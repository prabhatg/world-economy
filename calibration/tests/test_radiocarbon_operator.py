"""The radiocarbon-density observation operator on synthetic curves and worlds whose true
histories are known (docs/05 §5.1, ADR-0050)."""

import math

import h3
import numpy as np
import pytest

from oikos_cal import radiocarbon as rc
from oikos_data import radiocarbon as dates_pipeline

TRUE = rc.Window(-10_000, -6_000)
OBSERVED = rc.Window(-9_500, -6_500)
SETTINGS = rc.Settings(true=TRUE, observed=OBSERVED, error_edges=(0, 50, 100), block_resolution=2)


def linear_curve():
    """14C age equals cal BP, no curve error."""
    bp = np.arange(0, 20_001, 5, dtype=float)
    return dates_pipeline.Curve(bp, bp.copy(), np.zeros_like(bp))


def plateau_curve():
    """Like the linear curve, but flat at 9,000 14C BP from 9,900 to 10,300 cal BP (a
    radiocarbon plateau), with a curve error of 15 years everywhere."""
    bp = np.arange(0, 20_001, 5, dtype=float)
    c14 = np.where(bp < 9_900, bp - 900, np.where(bp < 10_300, 9_000, bp - 1_300))
    return dates_pipeline.Curve(bp, c14.astype(float), np.full_like(bp, 15.0))


def cells_in_two_blocks():
    """Two cells in each of two resolution-2 blocks (Levant, Zagros)."""
    out = []
    for lat, lng in ((32.0, 35.5), (33.0, 47.0)):
        c = h3.latlng_to_cell(lat, lng, 5)
        out += [h3.str_to_int(c), h3.str_to_int(next(n for n in h3.grid_ring(c, 1)
                                                       if h3.cell_to_parent(n, 2) == h3.cell_to_parent(c, 2)))]
    return np.array(out, dtype=np.uint64)


def test_taphonomic_survival_is_surovell_2009():
    t = rc.SUROVELL_2009
    assert t.survival(np.array([1950]))[0] == pytest.approx(1.0)
    ten_kya = t.survival(np.array([1950 - 10_000]))[0]
    assert ten_kya == pytest.approx(((10_000 + 2176.4) / 2176.4) ** -1.3925309)
    assert ten_kya == pytest.approx(0.0909, abs=1e-3), "about a tenth of 10,000-year-old dates survive"
    years = np.arange(-12_000, 0, 100)
    assert (np.diff(t.survival(years)) > 0).all(), "older deposits are lost more"
    assert (rc.NO_TAPHONOMY.survival(years) == 1).all()


def test_on_a_linear_curve_the_kernel_is_a_gaussian_blur():
    # True date uniform in a century, measured with error s and calibrated on a linear
    # curve: the calibrated date is the true date plus N(0, 2 s^2).
    s = 60.0
    k = rc.kernel(linear_curve(), TRUE, OBSERVED, [s])[0]
    sd = math.sqrt(2) * s

    def stays(lo, hi):  # P(calibrated date in [lo, hi) | true date uniform in [0, 100))
        ys = np.arange(0.5, 100)
        cdf = lambda x: 0.5 * (1 + np.vectorize(math.erf)((x - ys) / (sd * math.sqrt(2))))
        return float(np.mean(cdf(hi) - cdf(lo)))

    t = 20  # true century starting at -8,000
    o = t - (OBSERVED.first - TRUE.first) // 100
    assert k[t, o] == pytest.approx(stays(0, 100), abs=2e-3)
    assert k[t, o + 1] == pytest.approx(stays(100, 200), abs=2e-3)
    assert k[t, o - 2] == pytest.approx(stays(-200, -100), abs=2e-3)
    assert k[t].sum() == pytest.approx(1.0, abs=1e-6), "interior centuries keep all their mass"
    assert k[0].sum() < 0.01, "a true date far outside the observed window does not reach it"


def test_the_kernel_is_what_calibrating_simulated_dates_gives():
    # A plateau spreads calibrated probability over the whole flat stretch: the kernel
    # must reproduce the calibration pipeline's own output, plateau included.
    curve, s = plateau_curve(), 40.0
    k = rc.kernel(curve, TRUE, OBSERVED, [s])[0]
    rng = np.random.default_rng(7)
    t = 17  # -8,300 to -8,200: on the plateau
    years = TRUE.first + 100 * t + rng.uniform(0, 100, 4000)
    mu = np.interp(1950 - years, curve.cal_bp, curve.c14)
    ages = rng.normal(mu, math.sqrt(s * s + 15.0**2))
    mc = np.zeros(OBSERVED.centuries)
    for a in ages:
        bp, p = dates_pipeline.calibrate(a, s, curve)
        cal = 1950 - bp
        inside = (cal >= OBSERVED.first) & (cal < OBSERVED.end)
        mc += np.bincount(((cal[inside] - OBSERVED.first) // 100).astype(int), weights=p[inside], minlength=OBSERVED.centuries)
    mc /= len(ages)
    assert np.abs(k[t] - mc).max() < 0.02, np.round(np.c_[k[t], mc][15:30], 3)
    assert k[t, 11:16].sum() > 0.8 and k[t].max() < 0.4, "spread over the plateau, -8,350 to -7,950"


def table(rows):
    """A dates table in the pipeline's schema: (h3, site, age, error[, loc_accuracy])."""
    cols = {"h3": [], "site_name": [], "c14_age": [], "c14_error": [], "loc_accuracy": [], "lab_id": []}
    for i, r in enumerate(rows):
        h, site, age, err, acc = (*r, 3) if len(r) == 4 else r
        for k, v in zip(cols, (int(h), site, float(age), float(err), acc, f"L-{i}")):
            cols[k].append(v)
    return cols


def test_dates_from_one_site_and_phase_count_once():
    a, b = (int(c) for c in cells_in_two_blocks()[:2])
    bp = lambda year: 1950 - year  # linear curve: 14C age = cal BP
    rows = [(a, "Tell X", bp(y), 30) for y in (-9_000, -8_950, -8_850, -8_500)]
    rows += [(b, "Tell X", bp(-9_000), 30)]  # same name, different cell: another site
    rows += [(a, "Tell Y", bp(-8_000), 30, 1)]  # located only to a province: not used
    rows += [(a, "Tell Z", bp(-8_000), 150)]  # error above the largest class: not used
    obs = rc.observe(table(rows), linear_curve(), SETTINGS)
    weights = sorted(round(w, 6) for w in obs.weights)
    assert weights == [round(1 / 3, 6)] * 3 + [1.0, 1.0], "a 150-year phase of three dates, then one date"
    assert obs.mass.sum() == pytest.approx(3.0, abs=1e-6), "each phase contributes one date's mass"
    assert len(obs.blocks) == 1, "both cells are in one block"
    assert obs.mass[0, 0].sum() == pytest.approx(3.0, abs=1e-6), "errors of 30 years are the first class"


def test_research_intensity_per_block_and_overall_scale_do_not_matter():
    cells = cells_in_two_blocks()
    rng = np.random.default_rng(3)
    history = np.zeros((4, TRUE.centuries))
    history[:2, 20:] = 1.0  # Levant from -8,000
    history[2:, 10:] = 1.0  # Zagros from -9,000
    data = rc.synthetic_dates(cells, history, TRUE, linear_curve(), 400, [30.0, 80.0], rc.SUROVELL_2009, rng)
    op = rc.Operator(data, linear_curve(), cells, SETTINGS)
    base = op.log_likelihood(history)
    research = history.copy()
    research[2:] *= 25.0  # Zagros excavated 25 times more intensively
    assert op.log_likelihood(research) == pytest.approx(base)
    assert op.log_likelihood(1e6 * history) == pytest.approx(base)


@pytest.mark.parametrize("curve", [linear_curve(), plateau_curve()], ids=["linear", "plateau"])
def test_the_true_history_of_a_synthetic_world_scores_best(curve):
    # A Levant rising exponentially from -8,500 and a Zagros peaking at -7,500: dates
    # drawn from that history, lost to taphonomy, measured and calibrated.
    cells = cells_in_two_blocks()
    t = TRUE.midpoints()
    def history(levant_onset=-8_500, zagros_peak=-7_500):
        h = np.zeros((4, TRUE.centuries))
        h[:2] = np.where(t >= levant_onset, np.exp((t - levant_onset) / 800), 0)
        h[2:] = np.exp(-0.5 * ((t - zagros_peak) / 400) ** 2)
        return h
    data = rc.synthetic_dates(cells, history(), TRUE, curve, 1000, [30.0, 70.0], rc.SUROVELL_2009,
                              np.random.default_rng(11))
    op = rc.Operator(data, curve, cells, SETTINGS)
    best = op.log_likelihood(history())
    for name, alt in {"onset 500 years early": history(levant_onset=-9_000),
                      "onset 500 years late": history(levant_onset=-8_000),
                      "peak 500 years early": history(zagros_peak=-8_000),
                      "flat": np.ones((4, TRUE.centuries))}.items():
        assert op.log_likelihood(alt) < best - 5, name


def test_ignoring_taphonomic_loss_fits_worse():
    # Over 9,000 years survival falls threefold, enough to tell with 1,500 dates.
    settings = rc.Settings(error_edges=(0, 50, 100))
    cells = cells_in_two_blocks()
    flat = np.ones((4, settings.true.centuries))
    data = rc.synthetic_dates(cells, flat, settings.true, linear_curve(), 1500, [30.0, 70.0], rc.SUROVELL_2009,
                              np.random.default_rng(2))
    op = rc.Operator(data, linear_curve(), cells, settings)
    assert op.log_likelihood(flat, rc.NO_TAPHONOMY) < op.log_likelihood(flat) - 20


def test_dates_where_the_model_has_nobody_cost_but_do_not_break_the_score():
    cells = cells_in_two_blocks()
    h = np.ones((4, TRUE.centuries))
    data = rc.synthetic_dates(cells, h, TRUE, linear_curve(), 200, [30.0], rc.SUROVELL_2009, np.random.default_rng(5))
    op = rc.Operator(data, linear_curve(), cells, SETTINGS)
    empty = h.copy()
    empty[2:] = 0
    assert np.isfinite(op.log_likelihood(empty)) and op.log_likelihood(empty) < op.log_likelihood(h)


def test_expected_mass_matches_the_observed_totals_per_block_and_class():
    cells = cells_in_two_blocks()
    h = np.ones((4, TRUE.centuries))
    data = rc.synthetic_dates(cells, h, TRUE, linear_curve(), 300, [30.0, 80.0], rc.SUROVELL_2009,
                              np.random.default_rng(9))
    op = rc.Operator(data, linear_curve(), cells, SETTINGS)
    e = op.expected(h)
    assert e.shape == op.obs.mass.shape
    np.testing.assert_allclose(e.sum(axis=2), op.obs.mass.sum(axis=2), rtol=1e-9)


def test_a_table_without_usable_dates_is_refused():
    a = int(cells_in_two_blocks()[0])
    with pytest.raises(ValueError, match="no usable dates"):
        rc.Operator(table([(a, "Tell Y", 10_000, 30, 1)]), linear_curve(), cells_in_two_blocks(), SETTINGS)
