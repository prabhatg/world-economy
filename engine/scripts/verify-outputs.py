#!/usr/bin/env python3
"""Independent check that run outputs open in standard tools (zarr-python, pyarrow)
and agree with the work-unit result. Usage: verify-outputs.py <result.json>"""
import json
import sys

import numpy as np
import pyarrow.parquet as pq
import zarr

result = json.load(open(sys.argv[1]))
root = next(f for f in result["files"] if f.endswith("summary.parquet")).rsplit("/", 1)[0]
group = zarr.open_group(f"{root}/fields.zarr", mode="r")
pop = group["population"][:]
tick = group["tick"][:]
summary = pq.read_table(f"{root}/summary.parquet").to_pydict()
panel = pq.read_table(f"{root}/panel.parquet")

assert pop.dtype == np.uint64 and pop.shape == (len(tick), group["cell_h3"].shape[0])
assert list(tick) == summary["tick"], "time coordinate matches summary table"
assert [int(x) for x in pop.sum(axis=1)] == summary["population"], "field totals match summary"
final = result["summaries"][-1]
assert summary["tick"][-1] == final["tick"] and summary["population"][-1] == final["population"]
assert panel.num_rows > 0 and set(panel.column_names) == {"tick", "id", "cell", "weight"}
print(f"outputs ok: population {pop.shape}, {panel.num_rows} panel rows, "
      f"final population {final['population']}, engine {group.attrs['engine']}")
