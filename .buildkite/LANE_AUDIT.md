# `.rayci.yml` Lane Audit for Fork Infrastructure

> Generated from issue #96. This document tracks which CI lanes are
> feasible on the fork's current infrastructure and their enablement order.

## Lane Classification

| File | Tier | CPU-Only Steps | GPU Steps | ARM64 | Windows | macOS | Key Dependencies |
|---|---|---|---|---|---|---|---|
| `_forge.rayci.yml` | 1 | 2 | — | — | — | — | None |
| `lint.rayci.yml` | 1 | 2 (×11 matrix) | — | — | — | — | forge |
| `cicd.rayci.yml` | 1 | 5 | — | — | — | — | forge, base images |
| `dependencies.rayci.yml` | 1–2 | 2 | — | — | — | — | forge, base test image |
| `base.rayci.yml` | 2 | 5 | — | 2 | — | — | None (root images) |
| `_wheel-build.rayci.yml` | 2 | 4 (×12) | — | — | — | — | forge, manylinux |
| `build.rayci.yml` | 2–3 | 17 (×80+) | — | — | — | — | wheel-build, base images, Docker registry |
| `core.rayci.yml` | 3 | 24 | 2 | — | — | — | forge, wheel-build |
| `doc.rayci.yml` | 4 | 6 | — | — | — | — | base build, wheel-build |
| `others.rayci.yml` | 4 | 6 | — | — | — | — | base build, forge, FOSSA |
| `data.rayci.yml` | 4 | 22 | 2 | — | — | — | base ML, wheel-build, Snowflake |
| `serve.rayci.yml` | 4 | 19 | 1 | — | — | — | base build, wheel-build |
| `rllib.rayci.yml` | 4 | 8 | 4 | — | — | — | base ML/GPU, wheel-build |
| `ml.rayci.yml` | 4 | 18 | 6 | — | — | — | base ML/GPU, wheel-build, WandB, Comet |
| `llm.rayci.yml` | 4–5 | 1 | 2 | — | — | — | base build/cu128, wheel-build |
| `_images.rayci.yml` | 2–5 | 12 (×132) | — | — | — | — | Docker registry |
| `kuberay.rayci.yml` | 5 | 3 | — | — | — | — | base build, K8s tooling |
| `macos.rayci.yml` | 5 | — | — | — | — | 9 | macOS-arm64 runners |
| `windows.rayci.yml` | 5 | — | — | — | 10 | — | Windows runners |
| `linux_aarch64.rayci.yml` | 5 | — | — | 22 (×170) | — | — | ARM64 runners |

## Instance Types Required

| Instance Type | Standard x86_64? | Files Using It |
|---|---|---|
| `default` (unspecified) | ✅ | forge, lint, base (x86), wheel-build, images |
| `small` | ✅ | cicd, dependencies, core, build, others, aarch64 (uploads) |
| `medium` | ✅ | core, build, data, serve, rllib, ml, llm, doc, others, kuberay |
| `large` | ✅ | core, serve, rllib, ml, others, kuberay |
| `gpu` | ❌ GPU | serve, rllib, ml |
| `gpu-large` | ❌ GPU | core, data, rllib, ml, llm |
| `g6-large` | ❌ GPU (newer gen) | llm |
| `builder-arm64` | ❌ ARM64 | base (2 steps), linux_aarch64 |
| `medium-arm64` | ❌ ARM64 | linux_aarch64 |
| `builder-windows` | ❌ Windows | windows |
| `windows` | ❌ Windows | windows |
| `macos-arm64` | ❌ macOS | macos |

## Recommended Enablement Order

### Phase 1 — Prove rayci bootstrap (Tier 1)
- `_forge.rayci.yml` — zero dependencies, builds forge image
- `lint.rayci.yml` — depends on forge only, always runs
- `cicd.rayci.yml` — CI tooling tests, small instances
- `dependencies.rayci.yml` — dependency checks

### Phase 2 — Base images and wheels (Tier 2)
- `base.rayci.yml` (x86_64 steps only, skip 2 ARM64 steps)
- `_wheel-build.rayci.yml`

### Phase 3 — Core C++/Python tests (Tier 3)
- `core.rayci.yml` (CPU steps only, skip 2 GPU steps)

### Phase 4 — Domain test lanes (Tier 4)
1. `doc.rayci.yml` (CPU only)
2. `others.rayci.yml` (skip FOSSA if no API key)
3. `data.rayci.yml` (CPU only, skip Snowflake auth step)
4. `serve.rayci.yml` (CPU only)
5. `rllib.rayci.yml` (CPU only)
6. `ml.rayci.yml` (CPU only, skip WandB/Comet auth steps)
7. `llm.rayci.yml` (CPU only)
8. `_images.rayci.yml`
9. `build.rayci.yml`

### Phase 5 — Special infrastructure (Tier 5, needs human approval)
- GPU lanes (17 steps across 6 files)
- `kuberay.rayci.yml`
- `linux_aarch64.rayci.yml`
- `windows.rayci.yml`
- `macos.rayci.yml`

## Lanes Needing Human Approval to Disable/Defer

| Lane Category | Blocked By | Steps Affected |
|---|---|---|
| GPU test steps | No gpu/gpu-large/g6-large runners | 17 steps across core, data, serve, rllib, ml, llm |
| ARM64 image builds | No builder-arm64/medium-arm64 runners | 2 steps in base + 22 in linux_aarch64 |
| Windows tests | No builder-windows/windows runners | 10 steps |
| macOS tests | No macos-arm64 runners | 9 steps |
| K8s chaos tests | K8s infra + host networking | 9 matrix combos in kuberay |
| Credential-gated steps | Missing API keys | Snowflake, WandB, Comet, FOSSA, Docker registry |
