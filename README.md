# Xiaohong M5 Resource Governor

A domain-agnostic Rust decision core for selecting an execution placement and producing an explainable fallback order. It is not tied to notes, images, or a specific model provider.

## Decision inputs

- Workload hard constraints: capabilities, memory, latency/cost bounds, data classification, allowed nearby/cloud placement, battery and thermal tolerance.
- Device/placement profiles: availability, capabilities, free memory, battery, thermal state, network, estimated latency and cost.
- Versioned policy weights and placement preference.

Privacy, capability, and resource checks are hard filters. Remote execution requires both `allowCloud` and an explicit `allowedRemoteDataClasses` entry for the workload's data class. Only eligible candidates are scored; the response contains reasons, rejected candidates, and ordered fallback targets.

## Scope

This crate returns a decision only. It does not collect telemetry, migrate tasks, download/load models, authorize data access, invoke network services, or implement OpenHarmony APIs. The host must revalidate changing conditions and permissions immediately before dispatch. Cost/latency estimates and profiles are supplied by the host and may be stale. See [API.md](API.md).

## Verify

`cargo test --locked`, `cargo check --locked`, and `cargo run --locked --example route_demo`.

License: MIT.
