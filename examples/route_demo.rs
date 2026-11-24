use xiaohong_resource_governor::{choose_target, DataClass, DeviceProfile, NetworkState, Placement, RoutePolicy, ThermalState, Workload};

fn main() {
    let workload = Workload {
        required_capabilities: vec!["text-generation".into()], minimum_memory_mb: 2048,
        maximum_latency_ms: Some(1500), maximum_cost_micros: Some(8000),
        data_class: DataClass::Private, allow_nearby: true, allow_cloud: false,
        allowed_remote_data_classes: vec![DataClass::Public],
        minimum_battery_percent: Some(10), tolerate_serious_thermal: false,
    };
    let devices = vec![
        DeviceProfile { device_id: "phone".into(), placement: Placement::Local,
            capabilities: vec!["text-generation".into()], available_memory_mb: 4096,
            battery_percent: Some(72), thermal_state: ThermalState::Fair, network: NetworkState::Offline,
            estimated_latency_ms: 900, estimated_cost_micros: 0, available: true },
        DeviceProfile { device_id: "tablet".into(), placement: Placement::Nearby,
            capabilities: vec!["text-generation".into()], available_memory_mb: 8192,
            battery_percent: Some(80), thermal_state: ThermalState::Nominal, network: NetworkState::LocalOnly,
            estimated_latency_ms: 260, estimated_cost_micros: 0, available: true },
        DeviceProfile { device_id: "cloud".into(), placement: Placement::Cloud,
            capabilities: vec!["text-generation".into()], available_memory_mb: 65536,
            battery_percent: None, thermal_state: ThermalState::Nominal, network: NetworkState::Unmetered,
            estimated_latency_ms: 120, estimated_cost_micros: 3000, available: true },
    ];
    let decision = choose_target(&workload, &devices, &RoutePolicy::default()).expect("eligible target");
    println!("{}", serde_json::to_string_pretty(&decision).expect("serialize decision"));
}
