"""Domestic animals and crops in Egypt's Neolithic assemblages (Linseele et al. 2014;
#51): when herds and crops reached Africa.

Each dated assemblage's identified mammal bones and how many of them come from domestic
animals (sheep, goats, cattle, pigs) rather than hunted ones, as Linseele et al. tabulate
them for the Egyptian deserts, the Fayum and the Nile valley, and whether its people grew
cultivated crops, by the article's statements (left out where it says nothing). The
counts are transcribed in the manifest; nothing is downloaded.
"""

from . import DATA, manifest as manifests

MANIFEST = DATA / "manifests" / "linseele2014-egypt.toml"


def load():
    """(assemblages, source): each assemblage's name, years [start, end), identified
    mammals and domestic ones among them, lat and lng, and `crops` (True or False) where
    known, in the manifest's order; and the source's citation."""
    m = manifests.load(MANIFEST)
    manifests.require_redistributable(m)
    assemblages = m["access"]["assemblages"]
    for a in assemblages:
        if not a["start_year"] < a["end_year"]:
            raise ValueError(f"{a['name']}: years {a['start_year']} to {a['end_year']}")
        if not 0 <= a["domestic"] <= a["mammals"]:
            raise ValueError(f"{a['name']}: {a['domestic']} domestic of {a['mammals']} mammals")
        if not isinstance(a.get("crops", False), bool):
            raise ValueError(f"{a['name']}: crops must be true or false, or left out where unknown")
    if len({a["name"] for a in assemblages}) != len(assemblages):
        raise ValueError("assemblage names repeat")
    return assemblages, {"citation": m["citation"], "doi": m["doi"], "manifest_sha256": m.sha256}
