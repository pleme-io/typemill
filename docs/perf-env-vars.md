# Performance Environment Variables

This document is the shared source of truth for performance-related environment variables used by:

- Rust runtime checks (`crates/mill-services/src/services/perf_env.rs`)
- E2E matrix artifact writing (`tests/e2e/src/test_refactoring_matrix.rs`)
- CI workflow configuration (`.github/workflows/perf-gate.yml`)
- Perf history aggregation script (`.github/scripts/update_perf_history.py`)

## Core controls

- `TYPEMILL_PERF_ASSERT_STRICT`
- `TYPEMILL_MATRIX_PROFILE`
- `TYPEMILL_MATRIX_VERIFY_EVERY` (set per lane in perf CI; TypeScript lane uses a higher value)
- `TYPEMILL_MATRIX_VERIFY_FORCE_HIGH_RISK`
- `TYPEMILL_MATRIX_ARTIFACT_DIR`
- `TYPEMILL_MATRIX_TS_INCREMENTAL`
- `TYPEMILL_LSP_MODE` (set to `off` for pure matrix perf timing lanes)

## Scan cache / matrix acceleration

- `TYPEMILL_FILELIST_CACHE_TTL_MS`

## Directory move thresholds

- `TYPEMILL_PERF_MAX_DIRECTORY_MOVE_DETECTOR_MS`
- `TYPEMILL_PERF_MAX_DIRECTORY_MOVE_DOC_REWRITE_MS`
- `TYPEMILL_PERF_MAX_DIRECTORY_MOVE_CONVERT_MS`

## Prune stress thresholds

- `TYPEMILL_PRUNE_TIER_120_MAX_MS`
- `TYPEMILL_PRUNE_TIER_500_MAX_MS`
- `TYPEMILL_PRUNE_TIER_1000_MAX_MS`

## Artifact schemas

- Matrix artifact schema version: `1`
- Perf history schema version: `1`
