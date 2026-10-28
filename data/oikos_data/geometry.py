"""Geometry pack: H3 index, centroid and area of every cell of a region.

Runs never compute geometry (platform trigonometry differs); they read it from this
pack. Centroids are stored in micro-degrees and areas in square meters as integers,
so last-bit differences between hosts' math libraries do not change the pack id.
"""

import h3
import numpy as np

from . import pack


def build(region):
    cells = region.cells()
    names = [h3.int_to_str(int(c)) for c in cells]
    lat, lng = np.array([h3.cell_to_latlng(c) for c in names]).T
    area_m2 = np.array([h3.cell_area(c, "m^2") for c in names])
    b = pack.Builder("geometry", pack.region(region.name, region.resolution, cells))
    b.source(
        dataset="h3",
        version=h3.__version__,
        license="Apache-2.0",
        citation="Uber Technologies. H3: a hexagonal hierarchical geospatial indexing system. https://h3geo.org",
        manifest_sha256=region.sha256,
    )
    b.field("h3", "h3 index", "H3 cell index (ascending; defines the cell order)", cells)
    micro = 1e-6
    b.field("lat", "degrees_north", "Cell center latitude", np.round(lat / micro).astype(np.int32), scale=micro)
    b.field("lng", "degrees_east", "Cell center longitude", np.round(lng / micro).astype(np.int32), scale=micro)
    b.field("area_km2", "km2", "Cell area on the WGS84 authalic sphere", np.round(area_m2).astype(np.uint32), scale=micro)
    return b.build()
