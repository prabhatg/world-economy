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
- **GDAL** (via rasterio and pyogrio) reads raster data in the pipelines.

## Volunteers, reviewers, contributors, institutions
*Your name here.*
