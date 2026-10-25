# 11. Volunteer computing

Modeled on SETI@home / Folding@home / climateprediction.net, with a modern coordinator
of our own. Volunteers supply the **ensemble** capacity (calibration, sensitivity
analysis, emulator training, scenario ensembles); a central cluster runs the
tightly coupled work (modern full-resolution runs, assimilation, the 2026 twin).

## 11.1 What runs where

| Workload | Volunteers | Central (cloud/HPC) |
|---|---|---|
| Deep-past & regional calibration ensembles | ✅ primary | overflow |
| Sensitivity analysis, emulator training runs | ✅ primary | overflow |
| Scenario ensembles at coarse/regional resolution | ✅ primary | overflow |
| Volunteer "explore" runs (their own world) | ✅ | — |
| Particle-filter assimilation (synchronous) | — | ✅ |
| Modern full-resolution runs (TB state, MPI) | — | ✅ |
| Analysis, emulator fitting, SBI | — | ✅ |

## 11.2 Work unit (backend-neutral)

```
WorkUnit {
  id, experiment_id, queue (science | explore | validation),
  engine_version (content hash of signed binary),
  start_state: initial-config | checkpoint ref (content hash),
  inputs: [forcing-pack refs (content hash)],
  params, policy, seed, segment: { from_tick, to_tick },
  outputs_requested: summaries | obs-operator outputs | checkpoint,
  resources: { est_flops, peak_mem, disk, gpu_optional },
  deadline, replication_policy
}
Result {
  wu_id, host_id, canonical_output_hash,
  outputs (summaries, obs-operator arrays, optional checkpoint ref),
  runtime stats, client/platform class
}
```

- The same unit runs on a browser, a phone, a desktop, a cloud spot VM or an HPC node.
- **Segmented runs:** long runs are chains of segments (e.g. 500 sim-years each) with
  checkpoints between them, so interrupted work resumes and long histories can hop
  between hosts.
- **Target size:** 10–60 minutes on a typical laptop core; inputs ≤ ~50 MB (cached),
  outputs ≤ ~1 MB except optional checkpoints.
- **Small inputs:** climate ships as regional statistics + a seeded stochastic weather
  generator (reconstructed on client), not raw multi-GB fields. Forcing packs are
  versioned, content-addressed and cached across work units.

## 11.3 Coordinator (our own, cloud-native)

Rust services on GCP (portable to any cloud):

```
 Experiment manager (Python: calibration waves, scenario designs)
          │ creates experiments
          ▼
 Work generator ──► Work store (Postgres, partitioned) ◄── Validator ◄── Result ingest
          │                     │                          (quorum)      (signed-URL uploads
          │                     ▼                                         to object storage)
          │              Dispatcher (pull API: HTTPS/gRPC-web/WebSocket)
          │              - leases with deadlines, capability matching
          │              - host reliability scoring, adaptive replication
          ▼
 CDN / object storage: signed binaries (WASM + native), forcing packs, checkpoints
          │
 Accounts · teams · credits · leaderboards · "my worlds" API · telemetry (minimal)
          │
 Analysis store (Parquet on GCS + DuckDB/BigQuery) ──► calibration, emulators, viewer
```

- **Pull-based, stateless dispatch**: clients ask for N units matching their
  capabilities (memory, GPU class, expected uptime, bandwidth); lease expires at
  deadline → re-issued.
- **Over-provisioning & straggler handling**: issue a few % extra; first valid quorum
  wins; late results still accepted for credit.
- **Asynchronous science**: calibration algorithms consume results as they arrive
  (asynchronous batch history matching / sequential SBI); nothing blocks on a full wave.
- **Scale target**: millions of work units/day on a modest Postgres + object storage
  footprint; horizontal dispatcher replicas; Spanner/AlloyDB only if needed.
- **Lesson from SETI@home**: the central result pipeline (ingest, validate, analyze)
  was the bottleneck — it is designed and load-tested before public launch.

## 11.4 Result validation & trust

- **Bitwise determinism** (doc 3.5) makes validation a hash comparison of canonical
  outputs.
- **Adaptive replication**: new hosts get 2× replication; established reliable hosts
  1× with random spot-check replicas; any mismatch drops reliability and re-issues.
- **Homogeneous redundancy** for GPU-float kernels that are not bit-exact across
  vendors: replicas compared only within the same platform class.
- **Plausibility checks**: conservation laws and invariant checks recomputed server-side
  from summaries; outliers re-run centrally.
- **Code integrity**: only signed engine binaries are accepted; clients verify
  signatures; WASM runs sandboxed by construction; native clients run the engine in a
  sandboxed child process with no network access.
- **Privacy**: work units contain no personal data (synthetic populations only);
  accounts optional; minimal, documented telemetry; GDPR-compliant.

## 11.5 Clients (all types)

Shared Rust engine everywhere; thin platform shells.

| Client | Tech | Behavior |
|---|---|---|
| **Browser** | engine → WASM in Web Workers; WebGPU later | runs while tab open; throttles on battery; zero install — the main on-ramp |
| **Desktop** (Win/macOS/Linux) | native Rust engine + UI shell; GPU via CUDA/Metal/Vulkan | background on idle, tray app, screensaver showing your world, respects thermals |
| **Mobile** (Android, iOS) | native Rust engine + UI shell | Android: while charging + Wi-Fi (WorkManager). iOS: limited by background rules — mainly explore/visualize, short compute windows |
| **Headless** | CLI / Docker / Apptainer | servers, university clusters, data-center and cloud-credit donors |

Native client architecture (thin Flutter shell over a Rust daemon that is also the
headless client, sandboxed engine workers, embedded web globe) is in
[doc 13](13-native-clients.md).

## 11.6 Volunteer types (all)

| Type | What they do | Queue / system |
|---|---|---|
| **Compute donors** | passive compute | `science` queue (designed sampling) |
| **Explorers** | watch "their" world; pick from curated scenarios; name & share worlds | `explore` queue — results flagged, never mixed into designed statistics |
| **Citizen scientists** | verify LLM-extracted parameters/mechanisms against sources; classify archaeological/historical records (Zooniverse-style) | knowledge-base review tool |
| **Domain experts** | review mechanisms, priors and results | expert review workflow, credited authorship |
| **Developers** | contribute mechanisms, clients, tooling | open-source repo, mechanism test harness |
| **Institutions** | donate cluster time or cloud credits | headless client, institutional accounts |

Engagement: every volunteer machine runs an alternative history of humanity. Show it:
timelines, maps, "how your world compares to real history", milestones (first village,
first city, first state), team leaderboards, credits for compute and for verified
knowledge contributions, published acknowledgments.

## 11.7 Responsible operation

- Default limits on CPU/GPU share, thermal and battery-aware throttling.
- Optional "green mode": compute when local grid carbon intensity is low.
- Clear consent; easy pause/uninstall; no ads, no data monetization.

## 11.8 Nonprofit, open-source governance

- Foundation-style nonprofit (fiscal sponsor initially, e.g. an open-source foundation),
  with a public charter: mission = evidence for improving lives of the majority.
- Steering committee + scientific advisory board (economics, archaeology, climate,
  development, ethics); ethics review for policy-facing results.
- Licenses (proposed): code Apache-2.0; results and docs CC-BY 4.0; datasets per
  source license, with ingestion code open.
- Funding: grants, donations, cloud/HPC credits; transparent finances.
- Open roadmap, public RFCs for mechanisms, contributor and volunteer codes of conduct.
