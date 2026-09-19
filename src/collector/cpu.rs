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
        }
    }
}
