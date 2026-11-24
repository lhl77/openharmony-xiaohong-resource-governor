//! Domain-agnostic capability- and resource-aware execution placement planner.
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const POLICY_VERSION: u16 = 1;

#[derive(Debug, Error)]
pub enum GovernorError {
    #[error("invalid route request: {0}")]
    InvalidRequest(String),
    #[error("no eligible execution target")]
    NoEligibleTarget,
}

pub type Result<T> = std::result::Result<T, GovernorError>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProfile {
    pub device_id: String,
    pub placement: Placement,
    pub capabilities: Vec<String>,
    pub available_memory_mb: u64,
    pub battery_percent: Option<u8>,
    pub thermal_state: ThermalState,
    pub network: NetworkState,
    pub estimated_latency_ms: u64,
    pub estimated_cost_micros: u64,
    pub available: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Placement { Local, Nearby, Cloud }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ThermalState { Nominal, Fair, Serious, Critical }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NetworkState { Offline, LocalOnly, Metered, Unmetered }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Workload {
    pub required_capabilities: Vec<String>,
    pub minimum_memory_mb: u64,
    pub maximum_latency_ms: Option<u64>,
    pub maximum_cost_micros: Option<u64>,
    pub data_class: DataClass,
    pub allow_nearby: bool,
    pub allow_cloud: bool,
    pub allowed_remote_data_classes: Vec<DataClass>,
    pub minimum_battery_percent: Option<u8>,
    pub tolerate_serious_thermal: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DataClass { Public, Private, Restricted }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RoutePolicy {
    pub version: u16,
    pub latency_weight: f64,
    pub cost_weight: f64,
    pub locality_bonus: f64,
    pub preferred_order: Vec<Placement>,
}

impl Default for RoutePolicy {
    fn default() -> Self {
        Self { version: POLICY_VERSION, latency_weight: 1.0, cost_weight: 0.3,
            locality_bonus: 20.0, preferred_order: vec![Placement::Local, Placement::Nearby, Placement::Cloud] }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CandidateScore {
    pub device_id: String,
    pub placement: Placement,
    pub score: f64,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RouteDecision {
    pub schema_version: u16,
    pub selected_device_id: String,
    pub selected_placement: Placement,
    pub ordered_candidates: Vec<CandidateScore>,
    pub rejected: Vec<RejectedCandidate>,
    pub fallback_order: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RejectedCandidate { pub device_id: String, pub reasons: Vec<String> }

pub fn choose_target(workload: &Workload, devices: &[DeviceProfile], policy: &RoutePolicy) -> Result<RouteDecision> {
    validate(workload, policy)?;
    let mut eligible = Vec::new();
    let mut rejected = Vec::new();
    for device in devices {
        let reasons = rejection_reasons(workload, device);
        if reasons.is_empty() {
            let mut reasons = vec![format!("capabilities match: {}", workload.required_capabilities.join(",")),
                format!("memory available: {} MiB", device.available_memory_mb)];
            if device.placement == Placement::Local { reasons.push("data remains on the current device".into()); }
            let score = score(device, policy);
            eligible.push(CandidateScore { device_id: device.device_id.clone(), placement: device.placement.clone(), score, reasons });
        } else {
            rejected.push(RejectedCandidate { device_id: device.device_id.clone(), reasons });
        }
    }
    if eligible.is_empty() { return Err(GovernorError::NoEligibleTarget); }
    eligible.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| a.device_id.cmp(&b.device_id)));
    rejected.sort_by(|a, b| a.device_id.cmp(&b.device_id));
    let selected = eligible[0].clone();
    let fallback_order = eligible.iter().skip(1).map(|c| c.device_id.clone()).collect();
    Ok(RouteDecision { schema_version: POLICY_VERSION, selected_device_id: selected.device_id,
        selected_placement: selected.placement, ordered_candidates: eligible, rejected, fallback_order })
}

fn validate(workload: &Workload, policy: &RoutePolicy) -> Result<()> {
    if policy.version != POLICY_VERSION { return Err(GovernorError::InvalidRequest("unsupported policy version".into())); }
    if !policy.latency_weight.is_finite() || policy.latency_weight < 0.0
        || !policy.cost_weight.is_finite() || policy.cost_weight < 0.0
        || !policy.locality_bonus.is_finite() || (policy.latency_weight == 0.0 && policy.cost_weight == 0.0) {
        return Err(GovernorError::InvalidRequest("weights must be finite and at least one score weight positive".into()));
    }
    if workload.minimum_battery_percent.is_some_and(|p| p > 100) {
        return Err(GovernorError::InvalidRequest("minimumBatteryPercent must be <= 100".into()));
    }
    Ok(())
}

fn rejection_reasons(workload: &Workload, device: &DeviceProfile) -> Vec<String> {
    let mut reasons = Vec::new();
    if !device.available { reasons.push("device unavailable".into()); }
    if device.available_memory_mb < workload.minimum_memory_mb { reasons.push("insufficient memory".into()); }
    if workload.required_capabilities.iter().any(|cap| !device.capabilities.contains(cap)) { reasons.push("required capability missing".into()); }
    if workload.maximum_latency_ms.is_some_and(|limit| device.estimated_latency_ms > limit) { reasons.push("latency limit exceeded".into()); }
    if workload.maximum_cost_micros.is_some_and(|limit| device.estimated_cost_micros > limit) { reasons.push("cost limit exceeded".into()); }
    if workload.minimum_battery_percent.is_some_and(|min| device.battery_percent.is_some_and(|b| b < min)) { reasons.push("battery below workload minimum".into()); }
    if matches!(device.thermal_state, ThermalState::Critical) || (!workload.tolerate_serious_thermal && matches!(device.thermal_state, ThermalState::Serious)) { reasons.push("thermal state disallows workload".into()); }
    match device.placement {
        Placement::Nearby if !workload.allow_nearby => reasons.push("nearby execution not allowed".into()),
        Placement::Cloud if !workload.allow_cloud => reasons.push("cloud execution not allowed".into()),
        Placement::Cloud if !workload.allowed_remote_data_classes.contains(&workload.data_class) => reasons.push("data class not allowed for remote placement".into()),
        Placement::Cloud if device.network != NetworkState::Unmetered && device.network != NetworkState::Metered => reasons.push("cloud network unavailable".into()),
        Placement::Nearby if device.network == NetworkState::Offline => reasons.push("nearby network unavailable".into()),
        _ => {}
    }
    reasons
}

fn score(device: &DeviceProfile, policy: &RoutePolicy) -> f64 {
    let rank = policy.preferred_order.iter().position(|p| p == &device.placement).unwrap_or(policy.preferred_order.len());
    let preference = (policy.preferred_order.len().saturating_sub(rank)) as f64 * 0.01;
    let local = if device.placement == Placement::Local { policy.locality_bonus } else { 0.0 };
    local + preference - policy.latency_weight * device.estimated_latency_ms as f64
        - policy.cost_weight * device.estimated_cost_micros as f64 / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    fn workload() -> Workload { Workload { required_capabilities: vec!["llm".into()], minimum_memory_mb: 512,
        maximum_latency_ms: Some(1000), maximum_cost_micros: Some(5000), data_class: DataClass::Private,
        allow_nearby: true, allow_cloud: true, allowed_remote_data_classes: vec![DataClass::Public],
        minimum_battery_percent: Some(15), tolerate_serious_thermal: false } }
    fn device(id: &str, placement: Placement, latency: u64, battery: Option<u8>) -> DeviceProfile {
        DeviceProfile { device_id:id.into(), placement, capabilities:vec!["llm".into()], available_memory_mb:2048,
            battery_percent:battery, thermal_state:ThermalState::Nominal, network:NetworkState::Unmetered,
            estimated_latency_ms:latency, estimated_cost_micros:100, available:true }
    }
    #[test]
    fn remote_data_class_allowlist_overrides_cloud_score() {
        let devices=vec![device("cloud",Placement::Cloud,1,None),device("local",Placement::Local,30,Some(70))];
        let d=choose_target(&workload(),&devices,&RoutePolicy::default()).unwrap();
        assert_eq!(d.selected_device_id,"local");
        assert!(d.rejected.iter().any(|r|r.device_id=="cloud"));
    }
    #[test]
    fn rejects_incapable_hot_or_underpowered_targets() {
        let mut d=device("phone",Placement::Local,10,Some(70));d.thermal_state=ThermalState::Critical;
        assert!(matches!(choose_target(&workload(),&[d],&RoutePolicy::default()),Err(GovernorError::NoEligibleTarget)));
    }
    #[test]
    fn latency_can_choose_nearby_and_report_fallbacks() {
        let devices=vec![device("local",Placement::Local,800,Some(70)),device("tablet",Placement::Nearby,100,Some(80))];
        let d=choose_target(&workload(),&devices,&RoutePolicy::default()).unwrap();
        assert_eq!(d.selected_device_id,"tablet");
        assert_eq!(d.fallback_order,vec!["local"]);
    }
}
