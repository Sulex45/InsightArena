p# TODO - Pause/Resume per-market suspension

- [ ] Inspect predictions flow to enforce `market.is_paused` prevents creating predictions/stakes.
- [ ] Update `backend/src/markets/entities/market.entity.ts` with `is_paused` (and optionally `paused_at`).
- [ ] Add Soroban contract integration methods for `pause_market` and `resume_market`.
- [ ] Add `MarketsService.pauseMarket` and `MarketsService.resumeMarket` with admin-only auth + validations.
- [ ] Add controller endpoints: `POST /markets/:id/pause` and `POST /markets/:id/resume`.
- [ ] Ensure market listing/response DTO includes pause state (if not already included via entity serialization).
- [ ] Update frontend to disable prediction UI when paused and add admin controls.
- [ ] Add backend tests (unit + e2e if available) covering authorization and behavior when paused.
- [ ] Run backend tests and frontend typecheck/build.
