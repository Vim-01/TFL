use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Circular history ring buffer with a fixed maximum capacity.
/// Ensures zero runtime reallocations once populated.
#[derive(Debug, Clone)]
pub struct HistoryRingBuffer<T, const N: usize> {
    buffer: VecDeque<T>,
}

impl<T: Copy + Default, const N: usize> Default for HistoryRingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy, const N: usize> HistoryRingBuffer<T, N> {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::with_capacity(N),
        }
    }

    pub fn with_initial(initial_value: T) -> Self {
        let mut buffer = VecDeque::with_capacity(N);
        if N > 0 {
            for _ in 0..N {
                buffer.push_back(initial_value);
            }
        }
        Self { buffer }
    }

    pub fn push(&mut self, value: T) {
        if N == 0 {
            return;
        }
        if self.buffer.len() >= N {
            self.buffer.pop_front();
        }
        self.buffer.push_back(value);
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn latest(&self) -> Option<T> {
        self.buffer.back().copied()
    }

    pub fn as_vec(&self) -> Vec<T> {
        self.buffer.iter().copied().collect()
    }

    #[allow(dead_code)]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.buffer.iter()
    }
}

impl<const N: usize> HistoryRingBuffer<f64, N> {
    pub fn max(&self) -> f64 {
        self.buffer
            .iter()
            .copied()
            .filter(|x| x.is_finite())
            .fold(f64::NEG_INFINITY, |acc, x| acc.max(x))
            .max(0.0)
    }

    pub fn average(&self) -> f64 {
        let finite_items: Vec<f64> = self.buffer.iter().copied().filter(|x| x.is_finite()).collect();
        if finite_items.is_empty() {
            0.0
        } else {
            let sum: f64 = finite_items.iter().sum();
            sum / (finite_items.len() as f64)
        }
    }
}

/// System and CPU metrics
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct CpuMetrics {
    /// Overall CPU usage percentage (0.0 to 100.0)
    pub global_usage_percent: f32,
    /// Per-core usage percentage (0.0 to 100.0 for each core)
    pub core_usages: Vec<f32>,
    /// Per-core frequencies in MHz
    pub core_frequencies_mhz: Vec<u64>,
    /// Average frequency across all cores in MHz
    pub avg_frequency_mhz: u64,
    /// Total RAM in bytes
    pub ram_total_bytes: u64,
    /// Used RAM in bytes
    pub ram_used_bytes: u64,
    /// Available RAM in bytes
    pub ram_available_bytes: u64,
    /// Total Swap in bytes
    pub swap_total_bytes: u64,
    /// Used Swap in bytes
    pub swap_used_bytes: u64,
    /// System uptime in seconds
    pub uptime_seconds: u64,
    /// 1, 5, 15 minute load averages
    pub load_average: [f64; 3],
    /// CPU model name / brand (e.g. AMD Ryzen 5 5500X3D, Intel i9-13900K)
    pub brand: Option<String>,
    /// CPU package temperature in Celsius
    pub cpu_temp_c: Option<f32>,
    /// CPU package power consumption in Watts
    pub cpu_power_w: Option<f32>,
}

/// GPU Vendor identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GpuVendor {
    Amd,
    Nvidia,
    Intel,
    #[default]
    Unknown,
}

impl GpuVendor {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Amd => "AMD",
            Self::Nvidia => "NVIDIA",
            Self::Intel => "Intel",
            Self::Unknown => "Generic",
        }
    }
}

/// Segmented breakdown of VRAM usage
#[derive(Debug, Clone, Copy, Default)]
pub struct VramBreakdown {
    /// Estimated memory occupied by model weights (in bytes)
    pub weights_bytes: u64,
    /// Estimated memory occupied by KV cache (in bytes)
    pub kv_cache_bytes: u64,
    /// Driver and runtime context memory overhead (in bytes)
    pub context_bytes: u64,
    /// Free unallocated VRAM (in bytes)
    pub free_bytes: u64,
}

/// Unified GPU Metrics for AMD, NVIDIA, Intel
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct GpuMetrics {
    pub index: u32,
    pub name: String,
    pub vendor: GpuVendor,
    pub driver_version: String,
    pub pci_bus_id: String,

    /// GPU compute / core utilization (0.0 to 100.0)
    pub compute_percent: f32,
    /// Memory controller / memory bandwidth utilization (0.0 to 100.0)
    pub mem_utilization_percent: f32,

    /// VRAM total in bytes
    pub vram_total_bytes: u64,
    /// VRAM currently used in bytes
    pub vram_used_bytes: u64,
    /// GTT (System shared) memory total in bytes
    pub gtt_total_bytes: u64,
    /// GTT memory currently used in bytes
    pub gtt_used_bytes: u64,

    /// Segmented breakdown of VRAM
    pub vram_breakdown: VramBreakdown,

    /// Edge / Core temperature in Celsius
    pub temp_edge_c: Option<f32>,
    /// Hotspot / Junction temperature in Celsius
    pub temp_hotspot_c: Option<f32>,
    /// VRAM memory temperature in Celsius
    pub temp_mem_c: Option<f32>,

    /// Current power consumption in Watts
    pub power_current_w: Option<f32>,
    /// Maximum power consumption cap in Watts
    pub power_cap_w: Option<f32>,

    /// Fan speed percentage (0.0 to 100.0)
    pub fan_percent: Option<f32>,

    /// Current graphics core clock in MHz
    pub sclk_mhz: Option<u32>,
    /// Current memory clock in MHz
    pub mclk_mhz: Option<u32>,

    /// Memory vendor (e.g. Samsung, Micron, Hynix)
    pub mem_vendor: Option<String>,

    /// Real fan rotational speed in RPM
    pub fan_rpm: Option<u32>,
    /// Real core voltage in mV
    pub voltage_mv: Option<u32>,
    /// Real PCIe link status (e.g. "Gen4 x16")
    pub pcie_link: Option<String>,

    /// Processes currently using this GPU
    pub processes: Vec<GpuProcessInfo>,
}

/// Information about a process utilizing GPU resources (VRAM, GTT, Compute)
#[derive(Debug, Clone, Default)]
pub struct GpuProcessInfo {
    pub pid: u32,
    pub name: String,
    pub vram_bytes: u64,
    pub gtt_bytes: u64,
    pub vram_percent: f32,
}

/// Status of an individual inference slot in llama.cpp / vLLM
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SlotInfo {
    pub id: u32,
    pub id_task: Option<i64>,
    pub is_processing: bool,
    pub n_ctx: u64,
    pub n_prompt_tokens: u64,
    pub n_prompt_tokens_processed: u64,
    pub n_prompt_tokens_cache: u64,
    pub n_decoded: u64,
    pub speculative: bool,
    pub speculative_type: Option<String>,
    pub decode_tokens_per_sec: f32,
    pub draft_acceptance_rate: Option<f32>,
}

/// Comprehensive LLM Engine and Inference Metrics
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct LlmMetrics {
    /// Whether the server backend is currently reachable
    pub is_connected: bool,
    /// Backend engine name (e.g. "llama.cpp", "vLLM", "Ollama")
    pub engine_name: String,
    /// Backend endpoint URL
    pub endpoint_url: String,

    /// Loaded model alias / display name
    pub model_alias: String,
    /// Full model file path on host
    pub model_path: String,
    /// Quantization format (e.g. "IQ3_S - 3.4375 bpw", "Q4_K_M")
    pub model_ftype: String,
    /// Total parameters formatted (e.g. "27B")
    pub model_param_count: Option<String>,

    /// Cache Quantization types (Key and Value)
    pub cache_type_k: String,
    pub cache_type_v: String,

    /// Speculative Decoding / MTP configuration
    pub speculative_enabled: bool,
    pub speculative_type: Option<String>,
    pub speculative_draft_model: Option<String>,
    pub speculative_draft_quant: Option<String>,
    /// Speculative acceptance rate (0.0 to 100.0)
    pub speculative_acceptance_rate: Option<f32>,
    /// MTP Draft tokens accepted
    pub mtp_draft_accepted: u64,
    /// MTP Draft tokens generated
    pub mtp_draft_generated: u64,
    /// Mean draft accepted length
    pub mtp_mean_len: Option<f32>,

    /// Slot utilization
    pub total_slots: u32,
    pub active_slots: u32,
    pub pending_requests: u32,
    pub evicted_blocks: u32,

    /// KV Cache Pool utilization (0.0 to 100.0)
    pub kv_cache_pool_percent: f32,
    /// Current context tokens across active slots
    pub context_tokens_used: u64,
    /// Maximum context window allowed
    pub context_window_max: u64,

    /// Cache hit rate percentage (0.0 to 100.0)
    pub cache_hit_rate_percent: f32,

    /// Real-time throughput metrics
    pub current_prefill_tps: f32,
    pub current_decode_tps: f32,
    pub instant_decode_tps: f32,
    pub peak_decode_tps: f32,

    /// Latencies
    pub time_to_first_token_ms: Option<f32>,
    pub inter_token_latency_ms: Option<f32>,

    /// Detailed list of slots
    pub slots: Vec<SlotInfo>,
}

/// Unified metrics packet sent across channels from collectors to the app
#[derive(Debug, Clone)]
pub enum MetricUpdate {
    Cpu(CpuMetrics),
    Gpu(Vec<GpuMetrics>),
    Llm(Box<LlmMetrics>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_ring_buffer_push_and_capacity() {
        let mut buffer: HistoryRingBuffer<f64, 3> = HistoryRingBuffer::new();
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);

        buffer.push(10.0);
        buffer.push(20.0);
        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer.latest(), Some(20.0));

        buffer.push(30.0);
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.as_vec(), vec![10.0, 20.0, 30.0]);

        // Overflow: should pop oldest element (10.0)
        buffer.push(40.0);
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.as_vec(), vec![20.0, 30.0, 40.0]);
        assert_eq!(buffer.latest(), Some(40.0));
        assert_eq!(buffer.max(), 40.0);
        assert!((buffer.average() - 30.0).abs() < 1e-6);
    }

    #[test]
    fn test_history_ring_buffer_with_initial() {
        let buffer: HistoryRingBuffer<u32, 5> = HistoryRingBuffer::with_initial(42);
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer.as_vec(), vec![42, 42, 42, 42, 42]);
    }

    #[test]
    fn test_gpu_vendor_as_str() {
        assert_eq!(GpuVendor::Amd.as_str(), "AMD");
        assert_eq!(GpuVendor::Nvidia.as_str(), "NVIDIA");
        assert_eq!(GpuVendor::Intel.as_str(), "Intel");
        assert_eq!(GpuVendor::Unknown.as_str(), "Generic");
    }
}
