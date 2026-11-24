# M5 Resource Governor API

## Rust API

- `DeviceProfile` describes a candidate target and its host-provided capabilities/resource/network estimates.
- `Workload` declares required capabilities and hard resource, latency, cost, privacy, and placement constraints.
- `RoutePolicy` provides versioned scoring weights and preferred placement ordering.
- `choose_target(workload, devices, policy)` filters ineligible targets, ranks the rest, and returns `RouteDecision` with explanations, rejected reasons, and fallback IDs.

## Decision semantics

1. Hard constraints are filters, not score penalties. Remote execution requires both `allowCloud` and explicit inclusion of the workload's `dataClass` in `allowedRemoteDataClasses`; an absent/empty allowlist denies remote execution.
2. Missing required capabilities, inadequate memory, exceeded workload bounds, critical thermal state, or unavailable device reject that candidate.
3. Among remaining targets, the score combines locality bonus, placement preference, estimated latency, and estimated cost. Lower latency/cost is favored according to caller weights.
4. The output is a recommendation, not a dispatch authorization. The host must recheck privacy consent, system permissions, device availability, and current resources immediately before executing.

## Limitations

Profiles and estimates are supplied by the host and may become stale. There is no telemetry collector, scheduler, model loader, migration engine, network client, or OpenHarmony-specific adapter in this core. No eligible candidate returns `NoEligibleTarget`; callers decide whether to wait, ask the user, or use an allowed degraded capability.
