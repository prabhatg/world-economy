"""Wild wheats pack: where wild emmer and wild einkorn grow, from occurrence records."""

import h3
import numpy as np

from oikos_data import pack, region, wheats

BOX = region.Region(name="test-box", resolution=5, bbox=(36.0, 37.0, 38.0, 39.5), sha256="0" * 64)
MANIFEST = {"dataset": "test-wheats", "version": "2026-11-09", "citation": "Test",
            "license": {"id": "CC-BY-4.0", "attribution": "records (c) test"},
            "access": {"rings": 2, "licenses": ["CC0_1_0", "CC_BY_4_0"], "excluded_basis": ["LIVING_SPECIMEN"]}}
SHA = "1" * 64


def record(crop, lat, lng, key=1, basis="PRESERVED_SPECIMEN", license="CC_BY_4_0", dataset="d1"):
    return {"crop": crop, "key": key, "lat": lat, "lng": lng, "basis": basis, "license": license, "dataset": dataset}


def snap(*records, datasets=None):
    return wheats.snapshot(list(records), datasets or {"d1": {"title": "Wild relatives", "doi": "10.1/x"}})


def build(s):
    return pack.read(wheats.build(BOX, MANIFEST, SHA, s))


def rings_from(lat, lng):
    """Grid distance of every cell of the box from the cell at (lat, lng)."""
    at = h3.latlng_to_cell(lat, lng, 5)
    return np.array([h3.grid_distance(at, h3.int_to_str(int(c))) for c in BOX.cells()])


def test_a_wheat_ranges_within_rings_of_its_records():
    p = build(snap(record("emmer", 36.51, 38.71)))
    assert p.header["kind"] == "wild_wheats" and "time" not in p.header
    far = rings_from(36.51, 38.71)
    emmer, einkorn = p.values("emmer"), p.values("einkorn")
    assert np.all(emmer[far <= 2] == 1.0) and np.all(emmer[far > 2] == 0.0)
    assert (far <= 2).sum() == 19 and np.all(einkorn == 0.0)


def test_records_beyond_the_box_still_range_into_it():
    # A record one cell outside the box's south edge reaches the cells near it.
    p = build(snap(record("einkorn", 35.95, 38.71)))
    assert 0 < p.values("einkorn").sum() < 19


def test_living_specimens_whole_degree_coordinates_and_other_licenses_are_dropped():
    s = snap(record("emmer", 36.51, 38.71, basis="LIVING_SPECIMEN"),  # a garden or genebank plant
             record("emmer", 36.0, 38.71, key=2),  # a coordinate rounded to the degree
             record("emmer", 36.51, 39.0, key=3),
             record("emmer", 36.51, 38.71, key=4, license="CC_BY_NC_4_0"),
             record("einkorn", 36.51, 38.71, key=5))
    p = build(s)
    assert np.all(p.values("emmer") == 0.0) and p.values("einkorn").sum() == 19


def test_snapshots_are_canonical_and_fingerprinted():
    a = snap(record("emmer", 36.51, 38.71, key=2), record("einkorn", 36.6, 38.9, key=1))
    b = snap(record("einkorn", 36.6, 38.9, key=1), record("emmer", 36.51, 38.71, key=2))
    assert a == b and wheats.fingerprint(a) == wheats.fingerprint(b)
    assert wheats.fingerprint(a) != wheats.fingerprint(snap(record("emmer", 36.51, 38.71, key=2)))


def test_every_contributing_dataset_is_credited_under_its_records_licenses():
    # A dataset's own license may be stricter than the records used from it (iNaturalist's
    # CC0 records in a CC BY-NC dataset); the records' license is what applies.
    datasets = {"d1": {"title": "Wild relatives", "doi": "10.1/x"},
                "d2": {"title": "A herbarium", "doi": "10.1/y"},
                "d3": {"title": "Unused", "doi": "10.1/z"}}
    p = build(snap(record("emmer", 36.51, 38.71), record("einkorn", 36.6, 38.9, key=2, dataset="d2", license="CC0_1_0"),
                   record("einkorn", 36.61, 38.9, key=3, dataset="d1", license="CC0_1_0"),
                   record("emmer", 36.51, 38.71, key=4, dataset="d3", basis="LIVING_SPECIMEN"),
                   datasets=datasets))
    sources = p.header["sources"]
    assert sources[0]["dataset"] == "test-wheats" and sources[0]["attribution"] == "records (c) test"
    credited = {s["citation"] for s in sources[1:]}
    assert credited == {"Wild relatives. https://doi.org/10.1/x", "A herbarium. https://doi.org/10.1/y"}
    assert [s["license"] for s in sources[1:]] == ["CC-BY-4.0", "CC0-1.0"]


def test_a_snapshot_is_saved_byte_for_byte_and_read_back(tmp_path):
    # The records are kept in the repository (GBIF's search results change), so the
    # file must not change unless they do.
    s = snap(record("emmer", 36.51, 38.71), record("einkorn", 36.6, 38.9, key=2))
    a, b = tmp_path / "a.json.gz", tmp_path / "b.json.gz"
    wheats.save(s, a)
    wheats.save(s, b)
    assert a.read_bytes() == b.read_bytes()
    assert wheats.load(a) == s and wheats.fingerprint(wheats.load(a)) == wheats.fingerprint(s)


def test_a_range_reaches_as_far_at_a_finer_resolution():
    # Rings are counted at resolution 5; each finer level has cells sqrt(7) times closer.
    assert wheats.reach(2, 5) == 2 and wheats.reach(2, 6) == 5 and wheats.reach(1, 6) == 3
    fine = region.Region(name="test-box-r6", resolution=6, bbox=BOX.bbox, sha256="0" * 64)
    p = pack.read(wheats.build(fine, MANIFEST, SHA, snap(record("emmer", 36.51, 38.71))))
    at = h3.latlng_to_cell(36.51, 38.71, 6)
    far = np.array([h3.grid_distance(at, h3.int_to_str(int(c))) for c in fine.cells()])
    emmer = p.values("emmer")
    assert np.all(emmer[far <= 5] == 1.0) and np.all(emmer[far > 5] == 0.0)


def test_regions_over_one_box_share_a_snapshot():
    fine = region.Region(name="test-box-r6", resolution=6, bbox=BOX.bbox, sha256="0" * 64)
    other = region.Region(name="other", resolution=5, bbox=(30.0, 37.0, 38.0, 39.5), sha256="0" * 64)
    assert wheats.snapshot_path(fine, MANIFEST) == wheats.snapshot_path(BOX, MANIFEST)
    assert wheats.snapshot_path(other, MANIFEST) != wheats.snapshot_path(BOX, MANIFEST)
