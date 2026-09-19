use crate::model::{GpuMetrics, GpuVendor, VramBreakdown};
use std::fs;
use std::path::{Path, PathBuf};

/// Trait for platform/vendor GPU collectors
pub trait GpuBackend: Send + Sync {
    fn collect(&mut self) -> Vec<GpuMetrics>;
}

/// Linux native sysfs collector for AMD Radeon GPUs (amdgpu driver)
pub struct AmdSysfsBackend {
    cards: Vec<AmdCardPaths>,
}

#[derive(Clone, Debug)]
struct AmdCardPaths {
    index: u32,
    name: String,
    pci_id: String,
    temp_edge_file: Option<PathBuf>,
    temp_junction_file: Option<PathBuf>,
    temp_mem_file: Option<PathBuf>,
    power_file: Option<PathBuf>,
    power_cap_file: Option<PathBuf>,
    fan_pwm_file: Option<PathBuf>,
    vram_total_file: PathBuf,
    vram_used_file: PathBuf,
    gtt_total_file: PathBuf,
    gtt_used_file: PathBuf,
    gpu_busy_file: PathBuf,
    mem_busy_file: Option<PathBuf>,
    mem_vendor_file: Option<PathBuf>,
    sclk_file: Option<PathBuf>,
    mclk_file: Option<PathBuf>,
}

impl AmdSysfsBackend {
    pub fn probe() -> Option<Self> {
        let drm_dir = Path::new("/sys/class/drm");
        if !drm_dir.exists() {
            return None;
        }

        let mut cards = Vec::new();
        let entries = match fs::read_dir(drm_dir) {
            Ok(e) => e,
            Err(_) => return None,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();

            // Match card0, card1, etc. but not connectors like card1-DP-1
            if !name_str.starts_with("card") || name_str.contains('-') {
                continue;
            }

            let card_index = name_str.trim_start_matches("card").parse::<u32>().unwrap_or(0);
            let device_dir = path.join("device");

            // Verify amdgpu driver
            let driver_link = device_dir.join("driver");
            if let Ok(driver_target) = fs::read_link(&driver_link) {
                if !driver_target.to_string_lossy().contains("amdgpu") {
                    continue;
                }
            } else {
                continue;
            }

            // Find hwmon directory for sensors
            let hwmon_base = device_dir.join("hwmon");
            let mut hwmon_dir = None;
            if let Ok(hw_entries) = fs::read_dir(&hwmon_base) {
                for hw_entry in hw_entries.flatten() {
                    if hw_entry.file_name().to_string_lossy().starts_with("hwmon") {
                        hwmon_dir = Some(hw_entry.path());
                        break;
                    }
                }
            }

            let mut temp_edge_file = None;
            let mut temp_junction_file = None;
            let mut temp_mem_file = None;
            let mut power_file = None;
            let mut power_cap_file = None;
            let mut fan_pwm_file = None;

            if let Some(ref hw_dir) = hwmon_dir {
                // Discover temperature files
                for i in 1..=5 {
                    let label_file = hw_dir.join(format!("temp{i}_label"));
                    let input_file = hw_dir.join(format!("temp{i}_input"));
                    if let Ok(label) = fs::read_to_string(&label_file) {
                        let label_clean = label.trim().to_lowercase();
                        if label_clean.contains("edge") {
                            temp_edge_file = Some(input_file);
                        } else if label_clean.contains("junction") || label_clean.contains("hotspot") {
                            temp_junction_file = Some(input_file);
                        } else if label_clean.contains("mem") {
                            temp_mem_file = Some(input_file);
                        }
                    } else if i == 1 && input_file.exists() && temp_edge_file.is_none() {
                        temp_edge_file = Some(input_file);
                    }
                }

                // Discover power files (check power1_average, then power1_input)
                let p1_avg = hw_dir.join("power1_average");
                let p1_inp = hw_dir.join("power1_input");
                if p1_avg.exists() {
                    power_file = Some(p1_avg);
                } else if p1_inp.exists() {
                    power_file = Some(p1_inp);
                }

                let p1_cap = hw_dir.join("power1_cap");
                if p1_cap.exists() {
                    power_cap_file = Some(p1_cap);
                }

                let pwm1 = hw_dir.join("pwm1");
                if pwm1.exists() {
                    fan_pwm_file = Some(pwm1);
                }
            }

            // Read PCI bus / device ID
            let pci_id = match fs::read_link(&path) {
                Ok(link) => {
                    let s = link.to_string_lossy();
                    s.split('/')
                        .find(|part| part.starts_with("0000:"))
                        .unwrap_or("0000:00:00.0")
                        .to_string()
                }
                Err(_) => "Unknown PCI".to_string(),
            };

            let name = detect_amd_model_name(&device_dir, &pci_id);

            let vram_total_file = device_dir.join("mem_info_vram_total");
            let vram_used_file = device_dir.join("mem_info_vram_used");
            let gtt_total_file = device_dir.join("mem_info_gtt_total");
            let gtt_used_file = device_dir.join("mem_info_gtt_used");
            let gpu_busy_file = device_dir.join("gpu_busy_percent");
            let mem_busy_file = {
                let p = device_dir.join("mem_busy_percent");
                if p.exists() { Some(p) } else { None }
            };
            let mem_vendor_file = {
                let p = device_dir.join("mem_info_vram_vendor");
                if p.exists() { Some(p) } else { None }
            };
            let sclk_file = {
                let p = device_dir.join("current_gfxclk");
                if p.exists() { Some(p) } else { None }
            };
            let mclk_file = {
                let p = device_dir.join("pp_dpm_mclk");
                if p.exists() { Some(p) } else { None }
            };

            cards.push(AmdCardPaths {
                index: card_index,
                name,
                pci_id,
                temp_edge_file,
                temp_junction_file,
                temp_mem_file,
                power_file,
                power_cap_file,
                fan_pwm_file,
                vram_total_file,
                vram_used_file,
                gtt_total_file,
                gtt_used_file,
                gpu_busy_file,
                mem_busy_file,
                mem_vendor_file,
                sclk_file,
                mclk_file,
            });
        }

        if cards.is_empty() {
            None
        } else {
            Some(Self { cards })
        }
    }
}

impl GpuBackend for AmdSysfsBackend {
    fn collect(&mut self) -> Vec<GpuMetrics> {
        let mut results = Vec::with_capacity(self.cards.len());

        for card in &self.cards {
            let compute_percent = read_u32_from_file(&card.gpu_busy_file)
                .map(|val| (val as f32).clamp(0.0, 100.0))
                .unwrap_or(0.0);

            // True memory controller load from mem_busy_percent (or fallback to VRAM load)
            let mem_utilization_percent = card
                .mem_busy_file
                .as_ref()
                .and_then(|p| read_u32_from_file(p))
                .map(|val| (val as f32).clamp(0.0, 100.0))
                .unwrap_or_else(|| {
                    let total = read_u64_from_file(&card.vram_total_file).unwrap_or(0);
                    let used = read_u64_from_file(&card.vram_used_file).unwrap_or(0);
                    if total > 0 {
                        ((used as f32 / total as f32) * 100.0).clamp(0.0, 100.0)
                    } else {
                        0.0
                    }
                });

            let vram_total_bytes = read_u64_from_file(&card.vram_total_file).unwrap_or(0);
            let vram_used_bytes = read_u64_from_file(&card.vram_used_file).unwrap_or(0);
            let gtt_total_bytes = read_u64_from_file(&card.gtt_total_file).unwrap_or(0);
            let gtt_used_bytes = read_u64_from_file(&card.gtt_used_file).unwrap_or(0);

            // Read signed integer millidegrees
            let temp_edge_c = card
                .temp_edge_file
                .as_ref()
                .and_then(|p| read_i32_from_file(p))
                .map(|milli_c| milli_c as f32 / 1000.0);

            let temp_hotspot_c = card
                .temp_junction_file
                .as_ref()
                .and_then(|p| read_i32_from_file(p))
                .map(|milli_c| milli_c as f32 / 1000.0);

            let temp_mem_c = card
                .temp_mem_file
                .as_ref()
                .and_then(|p| read_i32_from_file(p))
                .map(|milli_c| milli_c as f32 / 1000.0);

            let power_current_w = card
                .power_file
                .as_ref()
                .and_then(|p| read_u64_from_file(p))
                .map(|micro_w| micro_w as f32 / 1_000_000.0);

            let power_cap_w = card
                .power_cap_file
                .as_ref()
                .and_then(|p| read_u64_from_file(p))
                .map(|micro_w| micro_w as f32 / 1_000_000.0);

            let fan_percent = card
                .fan_pwm_file
                .as_ref()
                .and_then(|p| read_u32_from_file(p))
                .map(|pwm| ((pwm as f32 / 255.0) * 100.0).clamp(0.0, 100.0));

            let mem_vendor = card
                .mem_vendor_file
                .as_ref()
                .and_then(|p| fs::read_to_string(p).ok())
                .map(|s| s.trim().to_string());

            let sclk_mhz = card
                .sclk_file
                .as_ref()
                .and_then(|p| read_u32_from_file(p));

            let mclk_mhz = card
                .mclk_file
                .as_ref()
                .and_then(|p| parse_amd_dpm_mclk(p));

            let free_bytes = vram_total_bytes.saturating_sub(vram_used_bytes);
            let vram_breakdown = VramBreakdown {
                weights_bytes: 0,
                kv_cache_bytes: 0,
                context_bytes: vram_used_bytes,
                free_bytes,
            };

            results.push(GpuMetrics {
                index: card.index,
                name: card.name.clone(),
                vendor: GpuVendor::Amd,
                driver_version: "amdgpu".to_string(),
                pci_bus_id: card.pci_id.clone(),
                compute_percent,
                mem_utilization_percent,
                vram_total_bytes,
                vram_used_bytes,
                gtt_total_bytes,
                gtt_used_bytes,
                vram_breakdown,
                temp_edge_c,
                temp_hotspot_c,
                temp_mem_c,
                power_current_w,
                power_cap_w,
                fan_percent,
                sclk_mhz,
                mclk_mhz,
                mem_vendor,
            });
        }

        results
    }
}

/// Fallback or NVIDIA backend using nvidia-smi if available
pub struct NvidiaBackend {
    has_nvidia_smi: bool,
}

impl NvidiaBackend {
    pub fn probe() -> Option<Self> {
        let has_smi = std::process::Command::new("which")
            .arg("nvidia-smi")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if has_smi {
            Some(Self { has_nvidia_smi: true })
        } else {
            None
        }
    }
}

impl GpuBackend for NvidiaBackend {
    fn collect(&mut self) -> Vec<GpuMetrics> {
        if !self.has_nvidia_smi {
            return Vec::new();
        }

        let output = match std::process::Command::new("nvidia-smi")
            .args([
                "--query-gpu=index,name,driver_version,utilization.gpu,utilization.memory,memory.total,memory.used,temperature.gpu,power.draw,power.limit",
                "--format=csv,noheader,nounits",
            ])
            .output()
        {
            Ok(o) => o,
            Err(_) => return Vec::new(),
        };

        if !output.status.success() {
            return Vec::new();
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut results = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() < 10 {
                continue;
            }

            let index: u32 = parts[0].parse().unwrap_or(0);
            let name = parts[1].to_string();
            let driver_version = parts[2].to_string();
            let compute_percent: f32 = parts[3].parse().unwrap_or(0.0);
            let mem_utilization_percent: f32 = parts[4].parse().unwrap_or(0.0);
            let total_mb: u64 = parts[5].parse().unwrap_or(0);
            let used_mb: u64 = parts[6].parse().unwrap_or(0);
            let temp_edge_c: Option<f32> = parts[7].parse().ok();
            let power_current_w: Option<f32> = parts[8].parse().ok();
            let power_cap_w: Option<f32> = parts[9].parse().ok();

            let vram_total_bytes = total_mb * 1024 * 1024;
            let vram_used_bytes = used_mb * 1024 * 1024;
            let free_bytes = vram_total_bytes.saturating_sub(vram_used_bytes);

            results.push(GpuMetrics {
                index,
                name,
                vendor: GpuVendor::Nvidia,
                driver_version,
                pci_bus_id: format!("GPU:{index}"),
                compute_percent,
                mem_utilization_percent,
                vram_total_bytes,
                vram_used_bytes,
                gtt_total_bytes: 0,
                gtt_used_bytes: 0,
                vram_breakdown: VramBreakdown {
                    weights_bytes: 0,
                    kv_cache_bytes: 0,
                    context_bytes: vram_used_bytes,
                    free_bytes,
                },
                temp_edge_c,
                temp_hotspot_c: None,
                temp_mem_c: None,
                power_current_w,
                power_cap_w,
                fan_percent: None,
                sclk_mhz: None,
                mclk_mhz: None,
                mem_vendor: None,
            });
        }

        results
    }
}

/// Composite GPU Collector that supports multiple simultaneous GPU backends
pub struct CompositeGpuCollector {
    backends: Vec<Box<dyn GpuBackend>>,
}

impl CompositeGpuCollector {
    pub fn new() -> Self {
        let mut backends: Vec<Box<dyn GpuBackend>> = Vec::new();

        // 1. Probe native Linux AMD sysfs backend
        if let Some(amd) = AmdSysfsBackend::probe() {
            backends.push(Box::new(amd));
        }

        // 2. Probe NVIDIA backend
        if let Some(nvidia) = NvidiaBackend::probe() {
            backends.push(Box::new(nvidia));
        }

        Self { backends }
    }

    pub fn collect(&mut self) -> Vec<GpuMetrics> {
        let mut all_gpus = Vec::new();
        for backend in &mut self.backends {
            all_gpus.extend(backend.collect());
        }
        all_gpus
    }
}

impl Default for CompositeGpuCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------- Helper Parsing Functions ----------------

fn read_u32_from_file(path: &Path) -> Option<u32> {
    let content = fs::read_to_string(path).ok()?;
    content.trim().parse::<u32>().ok()
}

fn read_i32_from_file(path: &Path) -> Option<i32> {
    let content = fs::read_to_string(path).ok()?;
    content.trim().parse::<i32>().ok()
}

fn read_u64_from_file(path: &Path) -> Option<u64> {
    let content = fs::read_to_string(path).ok()?;
    content.trim().parse::<u64>().ok()
}

fn parse_amd_dpm_mclk(path: &Path) -> Option<u32> {
    let content = fs::read_to_string(path).ok()?;
    for line in content.lines() {
        if line.contains('*') {
            if let Some(mhz_part) = line.split_whitespace().find(|p| p.ends_with("Mhz") || p.ends_with("MHz")) {
                let num_str = mhz_part.trim_end_matches("Mhz").trim_end_matches("MHz");
                if let Ok(mhz) = num_str.parse::<u32>() {
                    return Some(mhz);
                }
            }
        }
    }
    None
}

fn detect_amd_model_name(device_dir: &Path, pci_id: &str) -> String {
    let device_id = fs::read_to_string(device_dir.join("device"))
        .ok()
        .map(|s| s.trim().to_lowercase())
        .unwrap_or_default();

    if device_id == "0x744c" {
        return "AMD Radeon RX 7900 XTX".to_string();
    }
    if device_id == "0x7448" {
        return "AMD Radeon RX 7900 XT".to_string();
    }
    if device_id == "0x7449" {
        return "AMD Radeon RX 7900 GRE".to_string();
    }
    if device_id == "0x73bf" {
        return "AMD Radeon RX 6900 XT / 6950 XT".to_string();
    }
    if device_id == "0x73df" {
        return "AMD Radeon RX 6700 XT / 6750 XT".to_string();
    }
    if device_id == "0x743f" {
        return "AMD Radeon RX 6400 / 6500 XT".to_string();
    }

    let product_name = device_dir.join("product_name");
    if let Ok(prod) = fs::read_to_string(product_name) {
        let clean = prod.trim();
        if !clean.is_empty() {
            return clean.to_string();
        }
    }

    format!("AMD Radeon ({pci_id})")
}
