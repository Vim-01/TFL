use crate::model::CpuMetrics;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

/// Collector for CPU and system memory metrics using sysinfo.
pub struct CpuCollector {
    system: System,
}

impl Default for CpuCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuCollector {
    pub fn new() -> Self {
        let refresh_kind = RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything());
        let mut system = System::new_with_specifics(refresh_kind);
        system.refresh_cpu_all();
        system.refresh_memory();

        Self { system }
    }

    /// Refresh and collect the latest CPU and RAM metrics
    pub fn collect(&mut self) -> CpuMetrics {
        self.system.refresh_cpu_all();
        self.system.refresh_memory();

        let cpus = self.system.cpus();
        let mut core_usages = Vec::with_capacity(cpus.len());
        let mut core_frequencies_mhz = Vec::with_capacity(cpus.len());
        let mut freq_sum: u64 = 0;

        for cpu in cpus {
            let usage = cpu.cpu_usage();
            let freq = cpu.frequency();
            core_usages.push(usage);
            core_frequencies_mhz.push(freq);
            freq_sum = freq_sum.saturating_add(freq);
        }

        let avg_frequency_mhz = if !core_frequencies_mhz.is_empty() {
            freq_sum / (core_frequencies_mhz.len() as u64)
        } else {
            0
        };

        let brand = cpus.first().map(|c| c.brand().trim().to_string()).filter(|s| !s.is_empty());
        let global_usage = self.system.global_cpu_usage();
        let load_avg = System::load_average();

        let cpu_temp_c = read_cpu_temp();
        let cpu_power_w = Some(estimate_cpu_power(global_usage, avg_frequency_mhz));

        CpuMetrics {
            global_usage_percent: global_usage,
            core_usages,
            core_frequencies_mhz,
            avg_frequency_mhz,
            ram_total_bytes: self.system.total_memory(),
            ram_used_bytes: self.system.used_memory(),
            ram_available_bytes: self.system.available_memory(),
            swap_total_bytes: self.system.total_swap(),
            swap_used_bytes: self.system.used_swap(),
            uptime_seconds: System::uptime(),
            load_average: [load_avg.one, load_avg.five, load_avg.fifteen],
            brand,
            cpu_temp_c,
            cpu_power_w,
        }
    }
}

/// Read CPU temperature from hwmon (k10temp, coretemp, cpu_thermal)
fn read_cpu_temp() -> Option<f32> {
    let hwmon_dir = std::path::Path::new("/sys/class/hwmon");
    if let Ok(entries) = std::fs::read_dir(hwmon_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(name) = std::fs::read_to_string(path.join("name")) {
                let n = name.trim();
                if n == "k10temp" || n == "coretemp" || n == "cpu_thermal" {
                    for file_name in &["temp1_input", "temp3_input", "temp2_input"] {
                        if let Ok(content) = std::fs::read_to_string(path.join(file_name)) {
                            if let Ok(raw_milli) = content.trim().parse::<f32>() {
                                if raw_milli > 0.0 {
                                    return Some(raw_milli / 1000.0);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Estimate CPU package power consumption in Watts based on load, frequency, and TDP envelope
fn estimate_cpu_power(global_usage: f32, avg_frequency_mhz: u64) -> f32 {
    let usage_norm = (global_usage.clamp(0.0, 100.0) / 100.0).max(0.0);
    let freq_norm = if avg_frequency_mhz > 0 {
        (avg_frequency_mhz as f32 / 3700.0).clamp(0.7, 1.3)
    } else {
        1.0
    };
    // Base idle ~28W, dynamic scaling up to ~88W PPT
    28.0 + (usage_norm * freq_norm * 52.0)
}
