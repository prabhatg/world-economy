# Credits

This project exists because of many contributors. See
[docs/12-identity-and-credit.md](docs/12-identity-and-credit.md) for how credit works.
Once volunteers join, this file will be complemented by the public credits ledger.

## Founder
- Prabhat Gupta — vision, design direction

## Design & development tools
- Claude (Anthropic) — AI assistant for design documents and code
- Fable (Anthropic) — AI assistant for native-client design review

## Knowledge we build on
The mechanisms and data are built on the work of researchers across economics,
history, archaeology, climate science, ecology, demography, epidemiology, psychology,
neuroscience and the social sciences. Every source is cited in the knowledge base and
in the credit manifest of each result. Datasets surveyed so far are listed in
[docs/09-data-sources.md](docs/09-data-sources.md).

### Data and libraries in our forcing packs
- **H3** (Uber Technologies, Apache-2.0, https://h3geo.org): the grid, and the cell
  centroids and areas of the geometry pack (via h3-py and h3o).
- **Copernicus DEM GLO-90** (terrain pack): produced using Copernicus WorldDEM-90
  © DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under
  COPERNICUS by the European Union and ESA; all rights reserved. The organisations in
  charge of the Copernicus programme by law or by delegation do not incur any liability
  for any use of the Copernicus WorldDEM-90. (doi:10.5270/ESA-c5d3d65; read from AWS
  Open Data.)
- **CHELSA-TraCE21k v1.0** (climate pack): Karger, D.N., Nobis, M.P., Normand, S.,
  Graham, C.H., Zimmermann, N.E. (2023), Climate of the Past 19, 439–456; data
  doi:10.16904/envidat.211 (WSL, EnviDat), licensed under CC BY-SA 4.0. It downscales the
  CCSM3 TraCE-21ka simulation (Liu et al. 2009, Science 325, 310–314; He 2011). The
  climate and coastline packs are derived from it (aggregated to H3 cells; the
  coastline from its paleo-orography, whose ice comes from ICE-6G_C, Peltier, Argus &
  Drummond 2015, JGR Solid Earth 120, 450–487) and shared under the same
  license.
- **Mid-Holocene precipitation reconstructions** (humid-period pack): Bartlein, P.J.,
  Harrison, S.P., Brewer, S., et al. (2011), Pollen-based continental climate
  reconstructions at 6 and 21 ka: a global synthesis, Climate Dynamics 37, 775–802,
  doi:10.1007/s00382-010-0904-1; data from NOAA WDS-Paleo (contribution 2010-127),
  dedicated to the public domain (CC0), and the pollen and plant-macrofossil records it
  synthesizes. Holocene rainfall in Egypt's Western Desert (the same pack's fit,
  ADR-0093): Neumann, K. (1989), Holocene vegetation of the Eastern Sahara: charcoal
  from prehistoric sites, African Archaeological Review 7, 97–116, as cited by
  Bubenzer, O. & Riemer, H. (2007), Geoarchaeology 22, 607–620,
  doi:10.1002/gea.20176; and the Nabta Playa estimates of Neumann (1989), Barakat
  (1995) and Wendorf & Schild (1980), as cited by Wendorf, F. & Schild, R. (1998),
  Nabta Playa and its role in northeastern African prehistory, Journal of
  Anthropological Archaeology 17, 97–123.
- **RiverATLAS and LakeATLAS v1.0 / HydroATLAS** (hydrology pack): Linke, S. et al.
  (2019), Scientific Data 6: 283; Lehner, B., Messager, M.L., Korver, M.C., Linke, S.
  (2022), Scientific Data 9: 351; river reaches from HydroRIVERS (Lehner & Grill 2013,
  Hydrological Processes 27: 2171–2186) and lakes from HydroLAKES (Messager et al. 2016,
  Nature Communications 7: 13603); inundation from GIEMS-D15 (Fluet-Chouinard et al.
  2015). CC BY 4.0, www.hydrosheds.org. Lakes a modern work filled, left out: the
  Bitter Lakes (Por, F.D. (1978), Lessepsian Migration, Springer, Ecological Studies 23)
  and the Toshka lakes (Chipman, J.W. & Lillesand, T.M. (2007), International Journal of
  Remote Sensing 28, 4365–4379).
- **SoilGrids 2.0** (soils pack): ISRIC – World Soil Information; Poggio, L. et al. (2021),
  SOIL 7, 217–240, doi:10.5194/soil-7-217-2021. CC BY 4.0.
- **SoilGrids250m 2017 depth to bedrock** (bedrock pack): ISRIC – World Soil Information;
  Shangguan, W. et al. (2017), Journal of Advances in Modeling Earth Systems 9, 65–88,
  doi:10.1002/2016MS000686; Hengl, T. et al. (2017), PLoS ONE 12(2): e0169748,
  doi:10.1371/journal.pone.0169748. ODbL v1.0; the derived pack is shared under ODbL v1.0.
- **NGRIP GICC05 20-year δ18O** (climate_anomaly pack): Andersen, K.K. et al. (2007),
  PANGAEA, doi:10.1594/PANGAEA.586838; time scale Vinther, B.M. et al. (2006), Journal of
  Geophysical Research 111, D13102, and Rasmussen, S.O. et al. (2006), Journal of
  Geophysical Research 111, D06102. CC BY 3.0.
- **GBIF occurrence records of wild emmer and wild einkorn** (wild_wheats pack): GBIF.org
  occurrence search (CC0 1.0 and CC BY 4.0 records, retrieved 2026-11-09), chiefly the
  Crop Wild Relatives Occurrence data consortia (2024), A global database for the
  distributions of crop wild relatives, version 1.13, CIAT, doi:10.15468/jyrthk
  (Castañeda-Álvarez, N.P. et al. 2016, Nature Plants 2: 16022,
  doi:10.1038/nplants.2016.22), with genebank passport data (USDA NPGS, SINGER,
  EURISCO) and herbaria; each contributing dataset is credited in the pack's sources.
- **p3k14c** (radiocarbon observations): Bird, D. et al. (2022), Scientific Data 9: 27,
  doi:10.1038/s41597-022-01118-7; data doi:10.5281/zenodo.5597795 (MIT), and the
  databases it compiles (in M1 mostly CalPal, CONTEXT, Flohr et al. 2016, Vermeersch
  2019, Manning & Timpson 2014, 14SEA).
- **IntCal20** (radiocarbon calibration): Reimer, P.J. et al. (2020), Radiocarbon 62:
  725–757, doi:10.1017/RDC.2020.41. CC BY 4.0.
- **Barley, emmer and einkorn rachis assemblages** (domestication timing, and each
  wheat's selection relative to barley's, #38): Allaby, R.G., Stevens, C.,
  Lucas, L., Maeda, O. & Fuller, D.Q. (2017), Phil. Trans. R. Soc. B 372: 20160429,
  doi:10.1098/rstb.2016.0429; supplement doi:10.6084/m9.figshare.5436709.v1 (CC BY 4.0),
  and the archaeobotanical reports it compiles (Colledge, Kislev, Tanno & Willcox,
  van Zeist & Bakker-Heeres, de Moulins, Fairbairn et al., Weide et al., White &
  Makarewicz, Arranz-Otaegui et al., and others).
- **Survey settlements** (site counts): Palmisano, A., Lawrence, D., de Gruchy, M.W.,
  Bevan, A. & Shennan, S. (2021), Quaternary Science Reviews 252: 106739,
  doi:10.1016/j.quascirev.2020.106739; data doi:10.5281/zenodo.4322979 (CC BY 4.0), and
  the surface surveys it compiles (listed in the archive's References.txt).
- **DObsiSS 2.0** (obsidian pack and sourced assemblages): Batist, Z. (2016), The
  Database of Obsidian Sourcing Studies, doi:10.5281/zenodo.55848, licensed under CC
  BY-SA 4.0, and the sourcing studies it compiles (biblio.bib in the archive); derived
  data are shared under the same license.
- **Caprine size and age** (faunal table): Arbuckle, B.S., Kansa, S.W., Kansa, E., et al.
  (2014), Data sharing reveals complexity in the westward spread of domestic animals
  across Neolithic Turkey, PLoS ONE 9(6): e99845, doi:10.1371/journal.pone.0099845 (CC BY),
  and the faunal studies and Open Context datasets it compiles.
- **Domestic animals and crops in Egypt** (domestic-stock and crops records): Linseele, V., Van Neer, W.,
  Thys, S., Phillipps, R., Cappers, R., Wendrich, W. & Holdaway, S. (2014), New
  archaeozoological data from the Fayum 'Neolithic' with a critical assessment of the
  evidence for early stock keeping in Egypt, PLoS ONE 9(10): e108517,
  doi:10.1371/journal.pone.0108517 (CC BY 4.0), and the faunal studies it compiles.
- **Lesions of infection** (infection-lesions record): Eshed, V., Gopher, A., Pinhasi, R.
  & Hershkovitz, I. (2010), Paleopathology and the origin of agriculture in the Levant,
  American Journal of Physical Anthropology 143: 121-133, doi:10.1002/ajpa.21301, and the
  excavators of the 13 sites whose skeletons it studies.
- **Origins of agriculture archaeobotanical database** (cultivation-onset record):
  Wallace, M., Livarda, A., Charles, M. & Jones, G. (2018), York: Archaeology Data Service
  (distributor), doi:10.5284/1046750, used under the ADS Terms of Use and Access; funded
  by the ERC ('The Evolutionary Origins of Agriculture', 269830-EOA) and NERC ('Origins
  of Agriculture', NE/H022716/1); building on Sue Colledge's database for the AHRB
  project 'The Origin and Spread of Neolithic Plant Economies in the Near East and
  Europe' and the NERC 'Domestication of Europe' project; and the archaeobotanical
  publications it compiles (its tables 3 and 4). Analysis: Wallace, M. et al. (2019),
  Vegetation History and Archaeobotany 28: 449–463, doi:10.1007/s00334-018-0702-y. Those
  who collected and compiled the data bear no responsibility for our analysis or
  interpretation of it.
- **House-size Gini coefficients** (inequality record): Kohler, T.A., Smith, M.E.,
  Bogaard, A. et al. (2017), Nature 551: 619–622, doi:10.1038/nature24646, Supplementary
  Table 2 (author manuscript, Europe PMC PMC5714260), and the analyses it compiles
  (Bogaard; Demirergi et al.), from Kohler, T.A. & Smith, M.E. (eds.) (2018), Ten Thousand
  Years of Inequality. Used for research only, never redistributed.
- **PROJ** (via rasterio) projects SoilGrids' Homolosine grid.
- **GDAL** (via rasterio and pyogrio) reads raster data in the pipelines.

### Methods in our observation operators
- **Taphonomic correction:** Surovell, T.A. et al. (2009), Journal of Archaeological
  Science 36: 1715–1724, doi:10.1016/j.jas.2009.03.029; validated by Bluhm, L.E. &
  Surovell, T.A. (2019), Quaternary Research 91: 325–328, doi:10.1017/qua.2018.78.
- **Dates out of sight (drowned land, buried lowlands):** Stanley, D.J. & Warne, A.G.
  (1994), Worldwide initiation of Holocene marine deltas by deceleration of sea-level
  rise, Science 265: 228–231, doi:10.1126/science.265.5169.228; Stanley, D.J. &
  Toscano, M.A. (2009), Ancient archaeological sites buried and submerged along Egypt's
  Nile delta coast, Journal of Coastal Research 25: 158–170, doi:10.2112/08-0013.1;
  Altaweel, M. et al. (2019), New insights on the role of environmental dynamics shaping
  southern Mesopotamia, Iraq 81: 23–46, doi:10.1017/irq.2019.2.
- **Site-phase binning and simulation-based SPD comparison:** Shennan, S. et al. (2013),
  Nature Communications 4: 2486, doi:10.1038/ncomms3486; Timpson, A. et al. (2014),
  Journal of Archaeological Science 52: 549–557, doi:10.1016/j.jas.2014.08.011;
  rcarbon, Crema, E.R. & Bevan, A. (2021), Radiocarbon 63: 23–39, doi:10.1017/RDC.2020.95.
- **Juvenile index:** Bocquet-Appel, J.-P. (2002), Current Anthropology 43: 637–650,
  doi:10.1086/342429; read through a stable population with the Siler mortality of
  Gurven, M. & Kaplan, H. (2007), Population and Development Review 33: 321–365,
  doi:10.1111/j.1728-4457.2007.00171.x (Siler, W. (1979), Ecology 60: 750–757, doi:10.2307/1936612).
- **House-size Gini:** the small-sample Gini of Dixon, P.M., Weiner, J.,
  Mitchell-Olds, T. & Woodley, R. (1987), Ecology 68: 1548–1551, doi:10.2307/1939238, as
  used by Kohler et al. (2017); wealth dynamics after Borgerhoff Mulder, M. et al.
  (2009), Science 326: 682–688, doi:10.1126/science.1178336.
- **Aoristic site counts:** Crema, E.R. (2012), Journal of Archaeological Method and
  Theory 19: 440–461, doi:10.1007/s10816-011-9122-3; as applied to these surveys by
  Palmisano et al. (2021).

### Methods in our calibration
- **Screening:** Morris, M.D. (1991), Technometrics 33: 161–174,
  doi:10.1080/00401706.1991.10484804; μ* of Campolongo, F., Cariboni, J. & Saltelli, A.
  (2007), Environmental Modelling & Software 22: 1509–1518,
  doi:10.1016/j.envsoft.2006.10.004.
- **Emulators:** Kennedy, M.C. & O'Hagan, A. (2001), JRSS B 63: 425–464,
  doi:10.1111/1467-9868.00294; Rasmussen, C.E. & Williams, C.K.I. (2006), Gaussian
  Processes for Machine Learning (MIT Press); design size from Loeppky, J.L., Sacks, J. &
  Welch, W.J. (2009), Technometrics 51: 366–376, doi:10.1198/TECH.2009.08040.
- **History matching:** Craig, P.S. et al. (1997), Case Studies in Bayesian Statistics
  III: 37–93; Vernon, I., Goldstein, M. & Bower, R.G. (2010), Bayesian Analysis 5:
  619–669, doi:10.1214/10-BA524; Williamson, D. et al. (2013), Climate Dynamics 41:
  1703–1729, doi:10.1007/s00382-013-1896-4; three-sigma rule, Pukelsheim, F. (1994), The
  American Statistician 48: 88–91, doi:10.1080/00031305.1994.10476030.
- **SciPy** (optimization and linear algebra for the emulators).

### Software in our viewer
- **deck.gl**, **luma.gl** and **loaders.gl** (vis.gl, OpenJS Foundation; MIT): the map.
- **h3-js** (Uber Technologies, Apache-2.0): cell boundaries.
- **browser_wasi_shim** (bjorn3; MIT or Apache-2.0): runs the engine in volunteers' browsers.
- **noble-hashes** (Paul Miller; MIT): BLAKE3, so the browser checks every file it downloads.
- **Vite** and **Vitest** (MIT), **TypeScript** (Microsoft, Apache-2.0) and
  **Playwright** (Microsoft, Apache-2.0): building, testing and rendering checks.

## Volunteers, reviewers, contributors, institutions
*Your name here.*
