# Data contracts

## Tokens and identity
Measured total = input + cacheRead + cacheWrite + output. Reasoning overlaps output and is not added again. Null is unavailable, not measured zero. Explain incomplete component coverage when presenting a measured sum.

One prompt may own many requests, model changes and attributed subagents. Group only when logs establish the relationship. Background/review work stays distinct. Stable source IDs deduplicate imports and cannot replace or link records across providers/accounts.

Interactive request metrics use [from, to). An earlier prompt can still have requests inside that interval. Hover row tokens are lifetime totals and need not sum to the selected dock total. Advice calibration uses completed prompts separately.

## Current allowance
Each provider window is a distinct counter. Never substitute the general weekly pool for Fable or add different window percentages together. Fable requires the labeled scoped weekly allowance.

Percentage left = 100 - usedPercent only for a valid finite value in 0..100. Missing windows remain unavailable. A new UI refresh does not make an old snapshot current.

## Recent consumption
recent_usage.rs reports observed percentage-point changes inside a selected 1-minute to 30-day lookback. 91% to 87% left means 4 percentage points consumed, not a relative percentage increase.

Use the current provider/account/device, window ID and duration. Count only increasing-time pairs of connected valid readings, with matching reset boundaries, nondecreasing utilization and at most a 15-minute gap. Two comparable readings can establish zero visible change; missing readings cannot.

Do not interpolate interval edges or bridge missing reads, resets or corrections. Preserve comparable segments and mark incomplete coverage as partial. Return coverage seconds, samples and gap/reset counts. Multiple observed reset cycles can sum above 100; that is consumption, not the current balance. Snapshot values may sit on interval boundaries because they describe changes between reads, not individual request events.

Stream at most the latest 50,000 snapshots per provider/account/device in the chosen interval. Hitting that bound is partial coverage. The response omits account and device identifiers. The dock consumes the same calculation through its cached summary, with the same dockMinutes setting.

## Forecast and prompt attribution
A forecast projects actual percentage changes from fresh samples in one account/device/window/reset. Require four readings spanning 30 minutes without a gap over 15 minutes. Project only until reset or exhaustion, assuming recent pace continues. Learning, stale, steady and exhausted are different states. Tokens cannot substitute for quota readings.

Per-prompt attribution separately requires isolated completed activity and suitable surrounding snapshots. Overlap, rounding, unseen activity and gaps can leave it unknown. Recent consumption and forecasts do not identify the prompt that caused a change.

## Privacy and compatibility
Previews are bounded plain text. Strip recognized wrappers without executing markup. Enrichment cannot change accounting or ownership. Project basenames and titles remain potentially sensitive.

Credentials stay with provider-owned storage, outside history, exports and sync. Sync is immutable validated file delivery, not authenticated cloud storage or database replication. Retired browser records are compatibility data only. Browser Pro caps are dated references, never observed remaining balances.
