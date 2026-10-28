"""Dataset manifests: every input declares license, citation, variables and coverage."""

import pytest

from oikos_data import manifest

VALID = """
dataset = "test-dem"
title = "Test DEM"
version = "1"
homepage = "https://example.org/dem"
citation = "Example (2026). Test DEM."
doi = "10.0000/example"

[license]
id = "CC-BY-4.0"
url = "https://creativecommons.org/licenses/by/4.0/"
derived_redistribution = true
attribution = "Contains test data (c) Example."
verified = "2026-10-28"

[access]
kind = "cog_tiles"
url_template = "https://example.org/{tile}.tif"

[coverage]
spatial = "global"
temporal = "present"

[[variables]]
name = "elevation"
unit = "m"
description = "Height above the EGM2008 geoid."
"""


def test_valid_manifest_loads(tmp_path):
    f = tmp_path / "m.toml"
    f.write_text(VALID)
    m = manifest.load(f)
    assert m["dataset"] == "test-dem" and m["license"]["derived_redistribution"] is True
    assert m.sha256 and len(m.sha256) == 64


@pytest.mark.parametrize(
    "edit, message",
    [
        (lambda s: s.replace('citation = "Example (2026). Test DEM."\n', ""), "citation"),
        (lambda s: s.replace('verified = "2026-10-28"\n', ""), "verified"),
        (lambda s: s.replace('unit = "m"\n', ""), "unit"),
        (lambda s: 'surprise = 1\n' + s, "surprise"),
    ],
)
def test_invalid_manifests_are_rejected(tmp_path, edit, message):
    f = tmp_path / "m.toml"
    f.write_text(edit(VALID))
    with pytest.raises(ValueError, match=message):
        manifest.load(f)


def test_packs_need_datasets_that_allow_derived_redistribution(tmp_path):
    f = tmp_path / "m.toml"
    f.write_text(VALID.replace("derived_redistribution = true", "derived_redistribution = false"))
    m = manifest.load(f)
    with pytest.raises(ValueError, match="test-dem.*redistribut"):
        manifest.require_redistributable(m)
    f.write_text(VALID)
    manifest.require_redistributable(manifest.load(f))


def test_repository_manifests_are_valid():
    for m in manifest.all_manifests():
        assert m["dataset"]
