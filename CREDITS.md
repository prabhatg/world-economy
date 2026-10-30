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
- **RiverATLAS and LakeATLAS v1.0 / HydroATLAS** (hydrology pack): Linke, S. et al.
  (2019), Scientific Data 6: 283; Lehner, B., Messager, M.L., Korver, M.C., Linke, S.
  (2022), Scientific Data 9: 351; river reaches from HydroRIVERS (Lehner & Grill 2013,
  Hydrological Processes 27: 2171–2186) and lakes from HydroLAKES (Messager et al. 2016,
  Nature Communications 7: 13603); inundation from GIEMS-D15 (Fluet-Chouinard et al.
  2015). CC BY 4.0, www.hydrosheds.org.
- **SoilGrids 2.0** (soils pack): ISRIC – World Soil Information; Poggio, L. et al. (2021),
  SOIL 7, 217–240, doi:10.5194/soil-7-217-2021. CC BY 4.0.
- **p3k14c** (radiocarbon observations): Bird, D. et al. (2022), Scientific Data 9: 27,
  doi:10.1038/s41597-022-01118-7; data doi:10.5281/zenodo.5597795 (MIT), and the
  databases it compiles (in M1 mostly CalPal, CONTEXT, Flohr et al. 2016, Vermeersch
  2019, Manning & Timpson 2014, 14SEA).
- **IntCal20** (radiocarbon calibration): Reimer, P.J. et al. (2020), Radiocarbon 62:
  725–757, doi:10.1017/RDC.2020.41. CC BY 4.0.
- **Barley rachis assemblages** (domestication timing): Allaby, R.G., Stevens, C.,
  Lucas, L., Maeda, O. & Fuller, D.Q. (2017), Phil. Trans. R. Soc. B 372: 20160429,
  doi:10.1098/rstb.2016.0429; supplement doi:10.6084/m9.figshare.5436709.v1 (CC BY 4.0),
  and the archaeobotanical reports it compiles (Colledge, Kislev, Tanno & Willcox,
  van Zeist & Bakker-Heeres, de Moulins, Fairbairn et al., and others).
- **Survey settlements** (site counts): Palmisano, A., Lawrence, D., de Gruchy, M.W.,
  Bevan, A. & Shennan, S. (2021), Quaternary Science Reviews 252: 106739,
  doi:10.1016/j.quascirev.2020.106739; data doi:10.5281/zenodo.4322979 (CC BY 4.0), and
  the surface surveys it compiles (listed in the archive's References.txt).
- **PROJ** (via rasterio) projects SoilGrids' Homolosine grid.
- **GDAL** (via rasterio and pyogrio) reads raster data in the pipelines.

### Methods in our observation operators
- **Taphonomic correction:** Surovell, T.A. et al. (2009), Journal of Archaeological
  Science 36: 1715–1724, doi:10.1016/j.jas.2009.03.029; validated by Bluhm, L.E. &
  Surovell, T.A. (2019), Quaternary Research 91: 325–328, doi:10.1017/qua.2018.78.
- **Site-phase binning and simulation-based SPD comparison:** Shennan, S. et al. (2013),
  Nature Communications 4: 2486, doi:10.1038/ncomms3486; Timpson, A. et al. (2014),
  Journal of Archaeological Science 52: 549–557, doi:10.1016/j.jas.2014.08.011;
  rcarbon, Crema, E.R. & Bevan, A. (2021), Radiocarbon 63: 23–39, doi:10.1017/RDC.2020.95.
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

## Volunteers, reviewers, contributors, institutions
*Your name here.*
