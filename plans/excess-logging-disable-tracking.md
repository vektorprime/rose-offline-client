# Excess Logging Disable Tracking

## Issue
Repeated INFO logs from map-editor model refresh and blood overlay diagnostics were flooding the client log.

## Affected Systems
- Map editor loaded-zone model catalog refresh.
- Blood overlay texture generation and diagnostic query system.

## Attempts
- Removed the per-run BloodOverlay config INFO log from `blood_overlay_generate_system`.
- Removed the unused `blood_overlay_debug_query_system` diagnostic logger.
- Removed the repeated `[UPDATE MODELS] Updated models from loaded zone` INFO summary.

## Result
- Code edited. Build intentionally not run per user request.
