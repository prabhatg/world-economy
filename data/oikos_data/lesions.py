"""Lesions of infection before and after farming (Eshed et al. 2010; #42): how many of the
Natufian and Neolithic skeletons of the southern Levant show them, and where the skeletons
were buried. The counts are transcribed in the manifest; nothing is downloaded.
"""

from . import DATA, manifest as manifests

MANIFEST = DATA / "manifests" / "eshed2010-lesions.toml"


def load():
    """(periods, sites, source): each period's name, years [start, end), skeletons examined
    and affected, in time order; each site's name, period, individuals, lat and lng; and the
    source's citation. Checks that the sites add up to each period's sample."""
    m = manifests.load(MANIFEST)
    manifests.require_redistributable(m)
    periods, sites = m["access"]["periods"], m["access"]["sites"]
    for a, b in zip(periods, periods[1:]):
        if a["end_year"] > b["start_year"]:
            raise ValueError(f"periods {a['name']} and {b['name']} overlap or are out of order")
    for p in periods:
        if not 0 <= p["affected"] <= p["examined"]:
            raise ValueError(f"{p['name']}: {p['affected']} affected of {p['examined']} examined")
        n = sum(s["individuals"] for s in sites if s["period"] == p["name"])
        if n != p["examined"]:
            raise ValueError(f"{p['name']}: its sites hold {n} individuals, the period {p['examined']}")
    unknown = {s["period"] for s in sites} - {p["name"] for p in periods}
    if unknown:
        raise ValueError(f"sites of unknown periods {sorted(unknown)}")
    return periods, sites, {"citation": m["citation"], "doi": m["doi"], "manifest_sha256": m.sha256}
