"""House-size Gini coefficients: wealth inequality among the households of dated
archaeological sites (Kohler et al. 2017), the record of when inequality rose (ADR-0074)."""

import io

import pyarrow.parquet as pq
import pytest

from oikos_data import inequality, raster, region

BOX = region.Region(name="test-box", resolution=5, bbox=(34.0, 39.0, 31.0, 45.0), sha256="0" * 64)
SOURCES = [{"dataset": "test-kohler", "version": "1", "license": "Europe-PMC-author-manuscript",
            "manifest_sha256": "1" * 64, "citation": "Test", "attribution": "Test data"}]


def row(site, year, gini, n, lower="", upper="", adaptation="horticulture", period="Neolithic"):
    return {"Site": site, "Period": period, "Date_AD": float(year), "Gini": gini, "Lower_B": lower, "Upper_B": upper,
            "Basis": "HH area", "Households_in_sample": float(n), "Adaptation": adaptation, "World": "OW"}


def rows():
    """The region's five sites, a site of the Old World elsewhere and one of the New; names
    outside the region and every value are made up (the published ones are not
    redistributed)."""
    return [row("Jerf el Ahmar", -9100, 0.11, 3, 0.1, 0.12, period="PPNA"),
            row("Çatalhöyük", -6400, 0.21, 12),
            row("Tell Sabi Abyad Burnt Village", -6100, 0.31, 5, 0.2, 0.4),
            row("Tepe Gawra", -4100, 0.41, 6, 0.3, 0.5),
            row("Tell Brak Level 16", -3700, 0.51, 8, 0.4, 0.6, adaptation="agriculture"),
            row("Somewhere far", 500, 0.6, 30, adaptation="agriculture"),
            {**row("Somewhere else", 900, 0.25, 40), "World": "NW"}]


def table(sites=rows()):
    return pq.read_table(io.BytesIO(inequality.build(BOX, SOURCES, sites))).to_pydict()


def test_sites_in_the_region_keep_their_gini_houses_date_and_cell():
    t = table()
    assert t["site"] == ["Jerf el Ahmar", "Çatalhöyük", "Tell Sabi Abyad Burnt Village", "Tepe Gawra",
                         "Tell Brak Level 16"]
    assert t["year"] == [-9100, -6400, -6100, -4100, -3700]
    assert t["gini"] == [0.11, 0.21, 0.31, 0.41, 0.51]
    assert t["houses"] == [3, 12, 5, 6, 8]
    assert t["lower"][1] is None and t["upper"][0] == 0.12, "intervals only where the authors computed them"
    cells = BOX.cells()
    assert all(0 <= c < len(cells) for c in t["cell"])
    assert t["lat"][1] == pytest.approx(37.666) and t["lng"][1] == pytest.approx(32.826)


def test_a_known_site_missing_from_the_table_means_it_changed():
    with pytest.raises(ValueError, match="Tepe Gawra"):
        table([r for r in rows() if r["Site"] != "Tepe Gawra"])


def test_house_ginis_are_never_packed():
    with pytest.raises(ValueError, match="redistribut"):
        inequality.require_local()


def test_the_published_spreadsheet_has_the_sites_and_columns_expected():
    try:
        sources, sheet = inequality.inputs(offline=True)
    except raster.MissingInputs:
        pytest.skip("Kohler et al. 2017 supplement not downloaded (oikos_data.inequality.inputs() fetches it)")
    t = pq.read_table(io.BytesIO(inequality.build(BOX, sources, sheet))).to_pydict()
    assert t["site"] == ["Jerf el Ahmar", "Çatalhöyük", "Tell Sabi Abyad Burnt Village", "Tepe Gawra",
                         "Tell Brak Level 16"]
    assert all(n >= 2 for n in t["houses"]) and all(0 < g < 1 for g in t["gini"])
    assert t["year"] == sorted(t["year"]) and -9_500 < t["year"][0] and t["year"][-1] < -3_000
