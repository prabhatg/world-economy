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
paleo-DEM ocean mask per slice; inland waters (Caspian, lakes, Sea of Marmara) still
count as land until hydrology adds a lake mask.

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
