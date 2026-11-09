# 9. Data sources (initial survey)

Licenses must be checked per dataset before redistribution; the repo stores
**manifests and ingestion code**, not raw data.

**Implemented:** [`data/`](../data/README.md) holds regions, dataset manifests (license,
including whether derived data may be redistributed and when that was checked;
citation; access; coverage; variables and units) and the Python pipelines that build
forcing packs (doc 3 §3.8a), with every pack id locked in `data/packs.lock.json`.
Packs so far: M1 geometry (22,198 res-5 cells, ADR-0040) and M1 terrain from
Copernicus DEM GLO-90 (licence checked 2026-10-28: the GLO-90 "Full, Free & Open"
licence allows redistributing modified data with its attribution and liability notice,
which the pack carries), read as cloud-optimized GeoTIFF overviews with every input
tile fingerprinted in `data/locks/`. M1 climate comes from CHELSA-TraCE21k v1.0
(ADR-0042): annual mean temperature, temperature seasonality, annual precipitation and
precipitation seasonality per cell for 92 century slices (12,050–2,950 BCE midpoints),
read as row windows of the strip-stored global GeoTIFFs (licence checked 2026-10-28:
EnviDat lists CC BY-SA 4.0 in its licence field and CC BY 2.0 in its description; we
follow the stricter, so the climate pack is shared under CC BY-SA 4.0 with its
attribution; open question 23). The coastline pack (ADR-0043) uses the same dataset's
paleo-DEM ocean mask per slice; the hydrology pack (ADR-0044) supplies the lakes
(Caspian included) that it counts as land, so only the Sea of Marmara remains as a
known gap. Hydrology uses RiverATLAS and LakeATLAS (HydroATLAS, CC BY 4.0, checked
2026-10-28) rather than the standalone HydroRIVERS, whose HydroSHEDS licence requires
end-user agreements and forbids stand-alone redistribution; they are fetched through
the figshare API and checked against its published MD5. Soils come from SoilGrids 2.0
(ADR-0045; CC BY 4.0 per Poggio et al. 2021, checked 2026-10-28), 1 km aggregates.
SoilGrids 2.0 has no depth layer, so depth to bedrock within 2 m comes from SoilGrids250m
2017 (BDRICM; Shangguan et al. 2017), whose layers are under the Open Database License
v1.0 (ISRIC's metadata record and Hengl et al. 2017, checked 2026-11-09): its share-alike
clause covers the derived bedrock pack, which is shared under ODbL v1.0 (ADR-0106).
Decadal climate anomalies come from the NGRIP 20-year δ18O means on GICC05 (Andersen et
al. 2007), PANGAEA's export, whose record states CC BY 3.0 (checked 2026-11-09;
ADR-0107). The same speleothems on NOAA's paleo archive carry no licence; SISALv3's are
CC BY 4.0, for regional records later (open question 41).
Where wild emmer and wild einkorn grow comes from GBIF's occurrence records (ADR-0108),
requested through its search API under CC0 1.0 and CC BY 4.0 only (a record's own
license governs it, which may be looser than its dataset's: iNaturalist's CC0 records sit
in a CC BY-NC dataset); CC BY-NC records are left out. Every contributing dataset is
credited in the pack's sources. The search's results change as publishers update them,
so the snapshot is fingerprinted in a lock like the raster tiles.
The African Humid Period, which CHELSA-TraCE21k's climate model misses, comes from the
mid-Holocene precipitation reconstructions of Bartlein et al. (2011; NOAA WDS-Paleo,
CC0, checked 2026-11-04; ADR-0084), fitted together with published estimates for
Egypt's Western Desert from the charcoal and fauna of its Holocene sites (Neumann 1989;
Wendorf and Schild 1980, 1998; Bubenzer and Riemer 2007; ADR-0093).
Radiocarbon observations come from p3k14c (ADR-0046): the public, location-fuzzed
release in the authors' MIT-licensed compendium on Zenodo (the canonical tDAR copy needs
a browser), calibrated with IntCal20 (CC BY 4.0); both checked 2026-10-28.
Domestication timing comes from the barley rachis assemblages (shattering vs
non-shattering ears) of Allaby et al. 2017's supplement on figshare (CC BY 4.0, checked
2026-10-29; ADR-0051); its emmer and einkorn tables are EMF pictures that keep their
text, read from it since ADR-0101.
Settlement counts come from the survey compilation of Palmisano et al. 2021 on Zenodo
(CC BY 4.0, checked 2026-10-29; ADR-0059): dated site phases from surface surveys in
South-Central Anatolia and Upper Mesopotamia. Its Southern Levant settlements (DAAHL)
are not in the archive.
Obsidian sourcing comes from DObsiSS 2.0 (Batist 2016) on Zenodo (CC BY-SA 4.0, checked
2026-10-30; ADR-0060): artefacts assigned to volcanic sources by chemical
characterization in 176 dated site contexts of the region, and the volcanoes'
locations. Derived packs and tables share its license.
The onset of cultivation comes from the Origins of agriculture archaeobotanical database
(Wallace, Livarda, Charles and Jones 2018, Archaeology Data Service doi:10.5284/1046750;
ADR-0064): plant remains per sample of dated site phases in southwest Asia, central
Anatolia and Cyprus, from the Epipalaeolithic to the Pottery Neolithic. It is under the ADS Terms of Use
and Access (checked 2026-10-31): research use with acknowledgement, no sale, and the
data or anything derived from it given only to registered users. ADS serves it to
browsers only, so it is downloaded by hand, and its phase table is built in memory and
never committed or packed.
House-size Gini coefficients come from Supplementary Table 2 of Kohler et al. (2017,
Nature 551: 619-622, doi:10.1038/nature24646; ADR-0074): 63 sites worldwide, five of them
in the M1 region. The author manuscript's supplement at Europe PMC (PMC5714260) carries no
licence (checked 2026-11-02), so it is used for research only: it is fetched through the
Europe PMC API, its spreadsheet checked by SHA-256, and the site table built in memory,
never committed or packed. Sites are located by their radiocarbon samples in p3k14c.
Sheep and goat body size and juvenile shares come from the supporting information of
Arbuckle et al. (2014, PLoS ONE 9: e99845, CC BY, checked 2026-11-04; #37): 42 dated
faunal assemblages of Anatolia and its neighbours, through Europe PMC's supplement
archive, each member checked by SHA-256; the full databases are in Open Context.
Lesions of infection come from Eshed et al. (2010, American Journal of Physical
Anthropology 143: 121-133, doi:10.1002/ajpa.21301; ADR-0086): 2 of 200 Natufian and 9 of
205 Neolithic skeletons from 13 sites of the southern Levant. The article is under
Wiley's copyright and is not stored; its counts and sample by site are facts it reports,
transcribed with its citation into the manifest by the maintainer's decision
(2026-11-04, #42).

## Forcing (inputs)

| Domain | Candidates |
|---|---|
| Paleoclimate (transient, downscaled) | CHELSA-TraCE21k (1 km, 21 ka→present), TraCE-21ka, PaleoView, PAGES2k (last 2 ka), CRU/ERA5 (modern) |
| Sea level / ice | ICE-6G_C, other GIA reconstructions |
| Volcanic & solar forcing | eVolv2k, ice-core sulfate records, solar reconstructions |
| Terrain & hydrology | SRTM/GMTED, HydroSHEDS |
| Soils & suitability | SoilGrids, HWSD, FAO GAEZ (crop suitability) |
| Biomes / vegetation | BIOME reconstructions, pollen-based (Neotoma) |
| Domesticates | archaeobotanical/zooarchaeological syntheses (origin centers, dates) |

## Calibration & validation (observations)

| Era | Candidates |
|---|---|
| Prehistory | p3k14c radiocarbon database, regional site gazetteers, Neotoma, Allen Ancient DNA Resource (AADR) |
| Social complexity | Seshat Global History Databank, D-PLACE (ethnography), Ethnographic Atlas |
| Population & land use | HYDE 3.x, historical censuses, IPUMS International |
| Economy | Maddison Project, Clio-Infra, Allen-Unger prices & wages, Penn World Table, WDI |
| Polities & conflict | historical borders (e.g. Cliopatria / GeaCron-like datasets), Correlates of War, UCDP |
| Health | Human Mortality Database, Global Burden of Disease (IHME) |
| Governance & freedom | V-Dem, Polity, World Governance Indicators |
| Modern micro / synthetic pops | IPUMS, DHS, LSMS, satellite night lights, WorldPop |

## Literature (priors & mechanisms)

Behavioral economics meta-analyses, ethnographic syntheses, cliodynamics, economic
history, epidemiology, development RCT databases (e.g. J-PAL/IPA evaluations, 3ie),
used via the knowledge-ingestion pipeline (doc 7).
