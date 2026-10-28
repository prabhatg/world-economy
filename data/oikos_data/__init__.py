"""Ingestion pipelines: dataset manifests in, content-addressed forcing packs out.

The repository stores manifests and code, never raw data (docs/09). Pipelines fetch
only what the region needs into a cache outside the repository, aggregate it to H3
cells and write forcing packs (docs/03 §3.8a) whose ids are recorded in
data/packs.lock.json, so a rebuild anywhere can be checked against the expected bytes.
"""

from pathlib import Path

DATA = Path(__file__).resolve().parents[1]
