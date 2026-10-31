"""Archaeobotany: crop items among the charred plant remains of dated site phases (the
ORIGINS database, Wallace et al. 2018), the record that dates when people began to
plant (#29)."""

import hashlib
import io

import pyarrow.parquet as pq
import pytest

from oikos_data import archaeobotany, raster, region

BOX = region.Region(name="test-box", resolution=5, bbox=(30.0, 40.0, 33.0, 43.0), sha256="0" * 64)
SOURCES = [{"dataset": "test-origins", "version": "1", "license": "ADS-Terms", "manifest_sha256": "1" * 64,
            "citation": "Test", "attribution": "Test data"}]


def tables():
    """Two sites in the box (one with two phases) and one outside it."""
    return {
        "1_Site.txt": [
            {"SiteName": "Abu Hureyra", "SiteCode": "ABHU", "Region-Coarse": "N Levant", "Latitude": "35.87",
             "Longitude": "38.40"},
            {"SiteName": "Jerf el Ahmar", "SiteCode": "JEAH", "Region-Coarse": "N Levant", "Latitude": "36.40",
             "Longitude": "38.22"},
            {"SiteName": "Far away", "SiteCode": "FARA", "Region-Coarse": "Elsewhere", "Latitude": "10.0",
             "Longitude": "10.0"},
        ],
        "2_SitePhase.txt": [
            {"SiteName (SPh)": "Abu Hureyra", "SitePhase-ID": "ABHU Epip1", "PhaseName": "Epip1",
             "PhaseDescription": "", "ApproxStartBC": "-11146", "ApproxEndBC": "-11034", "MainPeriod": "Pre-PPNA"},
            {"SiteName (SPh)": "Abu Hureyra", "SitePhase-ID": "ABHU PN", "PhaseName": "PN", "PhaseDescription": "",
             "ApproxStartBC": "-5600", "ApproxEndBC": "-5500", "MainPeriod": "PPNC/PN"},
            {"SiteName (SPh)": "Jerf el Ahmar", "SitePhase-ID": "JEAH PPNA", "PhaseName": "PPNA",
             "PhaseDescription": "", "ApproxStartBC": "-9500", "ApproxEndBC": "-8700", "MainPeriod": "PPNA"},
            {"SiteName (SPh)": "Far away", "SitePhase-ID": "FARA X", "PhaseName": "X", "PhaseDescription": "",
             "ApproxStartBC": "-9000", "ApproxEndBC": "-8000", "MainPeriod": "PPNB"},
        ],
        "3_Records-Samples.txt": [
            {"SitePhase-ID": "ABHU Epip1", "RecordID": "r1", "RecordType": "sample", "Exclusion notes (if any)": ""},
            {"SitePhase-ID": "ABHU Epip1", "RecordID": "r2", "RecordType": "sample",
             "Exclusion notes (if any)": "mixed context"},
            {"SitePhase-ID": "ABHU PN", "RecordID": "r3", "RecordType": "sample", "Exclusion notes (if any)": ""},
            {"SitePhase-ID": "JEAH PPNA", "RecordID": "r4", "RecordType": "sample", "Exclusion notes (if any)": ""},
            {"SitePhase-ID": "JEAH PPNA", "RecordID": "r5", "RecordType": "phase", "Exclusion notes (if any)": ""},
            {"SitePhase-ID": "FARA X", "RecordID": "r6", "RecordType": "sample", "Exclusion notes (if any)": ""},
        ],
        "5_Taxa_Entries.txt": [
            # r1: 10 wild barley grains (progenitor) and 90 small grass seeds, charred
            {"RecordID": "r1", "OriginalTaxon": "Hordeum spontaneum", "PlantPart": "grain", "Preservation": "c",
             "MNPP": "10"},
            {"RecordID": "r1", "OriginalTaxon": "Stipa", "PlantPart": "grain", "Preservation": "c", "MNPP": "90"},
            # ... rachis (chaff) and a desiccated seed do not count, nor does an ambiguous taxon
            {"RecordID": "r1", "OriginalTaxon": "Hordeum spontaneum", "PlantPart": "rachis", "Preservation": "c",
             "MNPP": "500"},
            {"RecordID": "r1", "OriginalTaxon": "Stipa", "PlantPart": "grain", "Preservation": "d", "MNPP": "50"},
            {"RecordID": "r1", "OriginalTaxon": "Hordeum spontaneum/bulbosum", "PlantPart": "grain",
             "Preservation": "c", "MNPP": "40"},
            {"RecordID": "r2", "OriginalTaxon": "Hordeum spontaneum", "PlantPart": "grain", "Preservation": "c",
             "MNPP": "1000"},
            {"RecordID": "r3", "OriginalTaxon": "Triticum dicoccum", "PlantPart": "grain", "Preservation": "c",
             "MNPP": "30"},
            {"RecordID": "r3", "OriginalTaxon": "Lens culinaris", "PlantPart": "seed", "Preservation": "c",
             "MNPP": "30"},
            {"RecordID": "r4", "OriginalTaxon": "Secale", "PlantPart": "grain", "Preservation": "c", "MNPP": "60"},
            {"RecordID": "r4", "OriginalTaxon": "Stipa", "PlantPart": "grain", "Preservation": "", "MNPP": "40"},
            {"RecordID": "r5", "OriginalTaxon": "Secale", "PlantPart": "grain", "Preservation": "c", "MNPP": "900"},
            {"RecordID": "r6", "OriginalTaxon": "Secale", "PlantPart": "grain", "Preservation": "c", "MNPP": "9"},
        ],
        "6_PlantPart_Categories.txt": [
            {"PlantPart": "grain", "PlantPart_Cat": "Reproductive organ"},
            {"PlantPart": "seed", "PlantPart_Cat": "Reproductive organ"},
            {"PlantPart": "rachis", "PlantPart_Cat": "Dense/Durable Chaff"},
        ],
        "7_Taxa_Standardisation.txt": [
            {"OriginalTaxon": t, "ProjectTaxon": t} for t in
            ("Hordeum spontaneum", "Stipa", "Hordeum spontaneum/bulbosum", "Triticum dicoccum", "Lens culinaris",
             "Secale")
        ],
        "8_Taxa_Groups.txt": [
            {"ProjectTaxon": "Hordeum spontaneum", "DomProgWild": "Crop Progenitor"},
            {"ProjectTaxon": "Stipa", "DomProgWild": "Wild"},
            {"ProjectTaxon": "Hordeum spontaneum/bulbosum", "DomProgWild": "Progenitor/Wild"},
            {"ProjectTaxon": "Triticum dicoccum", "DomProgWild": "Domesticated crop"},
            {"ProjectTaxon": "Lens culinaris", "DomProgWild": "Domesticated crop"},
            {"ProjectTaxon": "Secale", "DomProgWild": "Presumed Crop Domesticate/Progenitor"},
        ],
    }


def test_crop_items_are_counted_per_dated_phase_as_wallace_et_al_select_them():
    t = pq.read_table(io.BytesIO(archaeobotany.build_table(BOX, SOURCES, tables()))).to_pydict()
    rows = {p: i for i, p in enumerate(t["phase"])}
    assert set(rows) == {"ABHU Epip1", "ABHU PN", "JEAH PPNA"}, "phases in the region only"
    i = rows["ABHU Epip1"]
    assert (t["crop"][i], t["items"][i]) == (10, 100), "charred grains and seeds of samples kept for analysis"
    assert (t["start_year"][i], t["end_year"][i], t["period"][i]) == (-11146, -11034, "Pre-PPNA")
    assert t["site"][i] == "Abu Hureyra" and t["lat"][i] == 35.87 and t["samples"][i] == 1
    assert (t["crop"][rows["ABHU PN"]], t["items"][rows["ABHU PN"]]) == (60, 60), "domesticates count as crops"
    assert (t["crop"][rows["JEAH PPNA"]], t["items"][rows["JEAH PPNA"]]) == (60, 100), \
        "unrecorded preservation counts as charred; phase-level amalgamations do not count"
    assert pq.read_metadata(io.BytesIO(archaeobotany.build_table(BOX, SOURCES, tables()))).metadata[b"sources"]


def test_the_tables_come_from_a_manual_download_checked_file_by_file(tmp_path, monkeypatch):
    monkeypatch.setenv("OIKOS_DATA_CACHE", str(tmp_path))
    with pytest.raises(raster.MissingInputs, match="archaeologydataservice.ac.uk"):
        archaeobotany.read_tables({"1_Site.txt": "0" * 64}, "ads-test")
    d = tmp_path / "manual" / "ads-test"
    d.mkdir(parents=True)
    (d / "1_Site.txt").write_text('"SiteName","Latitude"\n"A","1.5"\n')
    good = hashlib.sha256((d / "1_Site.txt").read_bytes()).hexdigest()
    assert archaeobotany.read_tables({"1_Site.txt": good}, "ads-test") == {"1_Site.txt": [{"SiteName": "A",
                                                                                           "Latitude": "1.5"}]}
    with pytest.raises(ValueError, match="1_Site.txt: SHA-256"):
        archaeobotany.read_tables({"1_Site.txt": "0" * 64}, "ads-test")
