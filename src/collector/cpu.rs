use crate::model::CpuMetrics;
use std::path::{Path, PathBuf};
use std::time::Instant;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

/// Collector for CPU and system memory metrics using sysinfo.
pub struct CpuCollector {
    system: System,
    cached_temp_file: Option<PathBuf>,
    cached_power_file: Option<PathBuf>,
    last_rapl_energy: Option<(u64, Instant)>,
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

        let cached_temp_file = find_cpu_temp_file();
        let cached_power_file = find_cpu_power_file();

        Self {
            system,
            cached_temp_file,
            cached_power_file,
            last_rapl_energy: None,
        }
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

        let core_count = cpus.len();
        let brand = cpus.first().map(|c| c.brand().trim().to_string()).filter(|s| !s.is_empty());
        let global_usage = self.system.global_cpu_usage();
        let load_avg = System::load_average();

        // 1. Read temperature (cached path with on-demand fallback)
        let cpu_temp_c = self.read_temp();

        // 2. Read power consumption: sysfs RAPL / hwmon with core-scaled fallback
        let cpu_power_w = self.read_or_estimate_power(core_count, global_usage, avg_frequency_mhz);

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

    fn read_temp(&mut self) -> Option<f32> {
        if let Some(ref path) = self.cached_temp_file {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(milli) = content.trim().parse::<f32>() {
                    if milli > 0.0 {
                        return Some(milli / 1000.0);
                    }
                }
            }
        }
        // Cache miss or read error: re-discover once
        self.cached_temp_file = find_cpu_temp_file();
        if let Some(ref path) = self.cached_temp_file {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(milli) = content.trim().parse::<f32>() {
                    if milli > 0.0 {
                        return Some(milli / 1000.0);
                    }
                }
            }
        }
        None
    }

    fn read_or_estimate_power(&mut self, core_count: usize, global_usage: f32, avg_frequency_mhz: u64) -> Option<f32> {
        let now = Instant::now();

        // Try reading energy counter from powercap RAPL
        if let Some(ref path) = self.cached_power_file {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(uj) = content.trim().parse::<u64>() {
                    if let Some((prev_uj, prev_time)) = self.last_rapl_energy {
                        let elapsed_secs = (now - prev_time).as_secs_f32();
                        if elapsed_secs > 0.05 && uj >= prev_uj {
                            let delta_uj = uj - prev_uj;
                            let watts = (delta_uj as f32 / 1_000_000.0) / elapsed_secs;
                            if (1.0..=1000.0).contains(&watts) {
                                self.last_rapl_energy = Some((uj, now));
                                return Some(watts);
                            }
                        }
                    }
                    self.last_rapl_energy = Some((uj, now));
                }
            }
        }

        // Adaptive core-scaled fallback (idle ~2.2W/core, dynamic scaling proportional to core count)
        Some(estimate_cpu_power_scaled(core_count, global_usage, avg_frequency_mhz))
    }
}

/// Discover CPU temperature sensor path in `/sys/class/hwmon`
/// Prioritizes junction/Tdie before Tctl (to eliminate AMD artificial temperature offsets)
fn find_cpu_temp_file() -> Option<PathBuf> {
    let hwmon_dir = Path::new("/sys/class/hwmon");
    if let Ok(entries) = std::fs::read_dir(hwmon_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(name) = std::fs::read_to_string(path.join("name")) {
                let n = name.trim();
                if n == "k10temp" || n == "coretemp" || n == "cpu_thermal" {
                    // Check for labeled inputs first (prioritize Tdie / Package id 0)
                    for i in 1..=8 {
                        let label_file = path.join(format!("temp{i}_label"));
                        let input_file = path.join(format!("temp{i}_input"));
                        if let Ok(label) = std::fs::read_to_string(&label_file) {
                            let l = label.trim().to_lowercase();
                            if (l.contains("tdie") || l.contains("package id 0")) && input_file.exists() {
                                return Some(input_file);
                            }
                        }
                    }

                    // Priority order: temp3_input (Tdie on AMD) -> temp1_input (Tctl) -> temp2_input
                    for file_name in &["temp3_input", "temp1_input", "temp2_input"] {
                        let f = path.join(file_name);
                        if f.exists() {
                            return Some(f);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Discover CPU power sensor path in `/sys/class/powercap` or `/sys/class/hwmon`
fn find_cpu_power_file() -> Option<PathBuf> {
    let rapl_candidates = [
        "/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj",
        "/sys/class/powercap/intel-rapl:0/energy_uj",
        "/sys/class/powercap/intel-rapl:0:0/energy_uj",
    ];

    for candidate in &rapl_candidates {
        let p = PathBuf::from(candidate);
        if p.exists() && std::fs::read_to_string(&p).is_ok() {
            return Some(p);
        }
    }

    // Check hwmon power inputs
    let hwmon_dir = Path::new("/sys/class/hwmon");
    if let Ok(entries) = std::fs::read_dir(hwmon_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(name) = std::fs::read_to_string(path.join("name")) {
                let n = name.trim();
                if n == "k10temp" || n == "coretemp" || n == "amdgpu" {
                    continue;
                }
                for file_name in &["power1_average", "power1_input"] {
                    let f = path.join(file_name);
                    if f.exists() && std::fs::read_to_string(&f).is_ok() {
                        return Some(f);
                    }
                }
            }
        }
    }

    None
}

/// Dynamically estimate CPU package power consumption based on detected core count, usage, and clock speed
fn estimate_cpu_power_scaled(core_count: usize, global_usage: f32, avg_frequency_mhz: u64) -> f32 {
    let cores = (core_count as f32).clamp(1.0, 128.0);
    let usage_norm = (global_usage.clamp(0.0, 100.0) / 100.0).max(0.0);
    let freq_norm = if avg_frequency_mhz > 0 {
        (avg_frequency_mhz as f32 / 3600.0).clamp(0.6, 1.4)
    } else {
        1.0
    };

    // Scaled baseline idle power + dynamic power envelope based on active core count
    let base_idle = (cores * 2.2 + 5.0).clamp(4.0, 75.0);
    let dynamic_range = (cores * 5.2).clamp(15.0, 350.0);

    base_idle + (usage_norm * freq_norm * dynamic_range)
}
