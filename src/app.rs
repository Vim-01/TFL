use crate::model::{CpuMetrics, GpuMetrics, HistoryRingBuffer, LlmMetrics, MetricUpdate};
use crate::theme::{BackgroundMode, Theme, ThemeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Dashboard,
    GpuDetails,
    SlotsDetails,
    GpuTop,
    Help,
}

impl ActiveTab {
    pub fn next(&self) -> Self {
        match self {
            Self::Dashboard => Self::GpuDetails,
            Self::GpuDetails => Self::SlotsDetails,
            Self::SlotsDetails => Self::GpuTop,
            Self::GpuTop => Self::Help,
            Self::Help => Self::Dashboard,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::Dashboard => Self::Help,
            Self::GpuDetails => Self::Dashboard,
            Self::SlotsDetails => Self::GpuDetails,
            Self::GpuTop => Self::SlotsDetails,
            Self::Help => Self::GpuTop,
        }
    }
}

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct App {
    pub cpu: Option<CpuMetrics>,
    pub gpus: Vec<GpuMetrics>,
    pub llm: Option<LlmMetrics>,

    // 512-step historical data ring buffers (fills ultra-wide 4K terminals cleanly without distortion)
    pub decode_tps_history: HistoryRingBuffer<f64, 512>,
    pub prefill_tps_history: HistoryRingBuffer<f64, 512>,
    pub gpu_compute_history: HistoryRingBuffer<f64, 512>,
    pub gpu_mem_controller_history: HistoryRingBuffer<f64, 512>,
    pub gpu_vram_history: HistoryRingBuffer<f64, 512>,
    pub cpu_usage_history: HistoryRingBuffer<f64, 512>,
    pub speculative_history: HistoryRingBuffer<f64, 512>,

    // Rolling CPU usage averages (1 min, 15 min, and all-time since start)
    pub cpu_samples_15m: std::collections::VecDeque<(std::time::Instant, f64)>,
    pub cpu_all_time_sum: f64,
    pub cpu_all_time_count: u64,

    // Rolling LLM Decode TPS averages & peak
    pub llm_decode_samples_15m: std::collections::VecDeque<(std::time::Instant, f64)>,
    pub llm_decode_all_time_sum: f64,
    pub llm_decode_all_time_count: u64,
    pub llm_decode_peak: f64,

    // Rolling LLM Prefill TPS averages & peak
    pub llm_prefill_samples_15m: std::collections::VecDeque<(std::time::Instant, f64)>,
    pub llm_prefill_all_time_sum: f64,
    pub llm_prefill_all_time_count: u64,
    pub llm_prefill_peak: f64,

    // Rolling LLM MTP Acceptance Rate averages & peak
    pub llm_mtp_samples_15m: std::collections::VecDeque<(std::time::Instant, f64)>,
    pub llm_mtp_all_time_sum: f64,
    pub llm_mtp_all_time_count: u64,
    pub llm_mtp_peak: f64,

    pub active_tab: ActiveTab,
    pub is_paused: bool,
    pub should_quit: bool,
    pub selected_gpu_index: usize,
    pub selected_slot_index: usize,
    pub selected_process_index: usize,

    // Theming and Options State
    pub theme_id: ThemeId,
    pub theme: Theme,
    pub bg_mode: BackgroundMode,
    pub options_menu_open: bool,
    pub options_menu_index: usize,
    pub poll_interval: Arc<AtomicU64>,
}

impl App {
    pub const OPTIONS_COUNT: usize = 4;

    pub fn new() -> Self {
        Self::with_poll_interval(Arc::new(AtomicU64::new(1000)))
    }

    pub fn with_poll_interval(poll_interval: Arc<AtomicU64>) -> Self {
        let theme_id = ThemeId::BtopNeon;
        // Default to transparent background: preserves host terminal's transparency / blur
        let bg_mode = BackgroundMode::Transparent;
        let theme = Theme::get(theme_id, bg_mode);

        Self {
            cpu: None,
            gpus: Vec::new(),
            llm: None,
            decode_tps_history: HistoryRingBuffer::with_initial(0.0),
            prefill_tps_history: HistoryRingBuffer::with_initial(0.0),
            gpu_compute_history: HistoryRingBuffer::with_initial(0.0),
            gpu_mem_controller_history: HistoryRingBuffer::with_initial(0.0),
            gpu_vram_history: HistoryRingBuffer::with_initial(0.0),
            cpu_usage_history: HistoryRingBuffer::with_initial(0.0),
            speculative_history: HistoryRingBuffer::with_initial(0.0),
            cpu_samples_15m: std::collections::VecDeque::new(),
            cpu_all_time_sum: 0.0,
            cpu_all_time_count: 0,
            llm_decode_samples_15m: std::collections::VecDeque::new(),
            llm_decode_all_time_sum: 0.0,
            llm_decode_all_time_count: 0,
            llm_decode_peak: 0.0,
            llm_prefill_samples_15m: std::collections::VecDeque::new(),
            llm_prefill_all_time_sum: 0.0,
            llm_prefill_all_time_count: 0,
            llm_prefill_peak: 0.0,
            llm_mtp_samples_15m: std::collections::VecDeque::new(),
            llm_mtp_all_time_sum: 0.0,
            llm_mtp_all_time_count: 0,
            llm_mtp_peak: 0.0,
            active_tab: ActiveTab::Dashboard,
            is_paused: false,
            should_quit: false,
            selected_gpu_index: 0,
            selected_slot_index: 0,
            selected_process_index: 0,
            theme_id,
            theme,
            bg_mode,
            options_menu_open: false,
            options_menu_index: 0,
            poll_interval,
        }
    }

    pub fn poll_interval_ms(&self) -> u64 {
        self.poll_interval.load(Ordering::Relaxed)
    }

    pub fn set_poll_interval(&self, ms: u64) {
        self.poll_interval.store(ms, Ordering::Relaxed);
    }

    pub fn cpu_avg_1m(&self) -> f64 {
        if self.cpu_samples_15m.is_empty() {
            return self.cpu.as_ref().map(|c| c.global_usage_percent as f64).unwrap_or(0.0);
        }
        let now = std::time::Instant::now();
        let cutoff_1m = now.checked_sub(std::time::Duration::from_secs(60)).unwrap_or(now);
        let mut sum = 0.0;
        let mut count = 0;
        for (t, val) in self.cpu_samples_15m.iter().rev() {
            if *t >= cutoff_1m {
                sum += *val;
                count += 1;
            } else {
                break;
            }
        }
        if count > 0 { sum / count as f64 } else { 0.0 }
    }

    pub fn cpu_avg_15m(&self) -> f64 {
        if self.cpu_samples_15m.is_empty() {
            return self.cpu.as_ref().map(|c| c.global_usage_percent as f64).unwrap_or(0.0);
        }
        let sum: f64 = self.cpu_samples_15m.iter().map(|(_, v)| *v).sum();
        sum / self.cpu_samples_15m.len() as f64
    }

    pub fn cpu_avg_all_time(&self) -> f64 {
        if self.cpu_all_time_count > 0 {
            self.cpu_all_time_sum / self.cpu_all_time_count as f64
        } else {
            self.cpu.as_ref().map(|c| c.global_usage_percent as f64).unwrap_or(0.0)
        }
    }

    // --- Decode TPS Statistics ---
    pub fn llm_decode_avg_1m(&self) -> f64 {
        let now = std::time::Instant::now();
        let cutoff_1m = now.checked_sub(std::time::Duration::from_secs(60)).unwrap_or(now);
        let mut sum = 0.0;
        let mut count = 0;
        for (t, val) in self.llm_decode_samples_15m.iter().rev() {
            if *t >= cutoff_1m {
                sum += *val;
                count += 1;
            } else {
                break;
            }
        }
        if count > 0 {
            sum / count as f64
        } else {
            self.llm.as_ref().map(|l| l.current_decode_tps as f64).unwrap_or(0.0)
        }
    }

    pub fn llm_decode_avg_15m(&self) -> f64 {
        if self.llm_decode_samples_15m.is_empty() {
            return self.llm.as_ref().map(|l| l.current_decode_tps as f64).unwrap_or(0.0);
        }
        let sum: f64 = self.llm_decode_samples_15m.iter().map(|(_, v)| *v).sum();
        sum / self.llm_decode_samples_15m.len() as f64
    }

    pub fn llm_decode_avg_all(&self) -> f64 {
        if self.llm_decode_all_time_count > 0 {
            self.llm_decode_all_time_sum / self.llm_decode_all_time_count as f64
        } else {
            self.llm.as_ref().map(|l| l.current_decode_tps as f64).unwrap_or(0.0)
        }
    }

    pub fn llm_decode_peak(&self) -> f64 {
        let col_peak = self.llm.as_ref().map(|l| l.peak_decode_tps as f64).unwrap_or(0.0);
        self.llm_decode_peak.max(col_peak).max(self.decode_tps_history.max())
    }

    // --- Prefill TPS Statistics ---
    pub fn llm_prefill_avg_1m(&self) -> f64 {
        let now = std::time::Instant::now();
        let cutoff_1m = now.checked_sub(std::time::Duration::from_secs(60)).unwrap_or(now);
        let mut sum = 0.0;
        let mut count = 0;
        for (t, val) in self.llm_prefill_samples_15m.iter().rev() {
            if *t >= cutoff_1m {
                sum += *val;
                count += 1;
            } else {
                break;
            }
        }
        if count > 0 {
            sum / count as f64
        } else {
            self.llm.as_ref().map(|l| l.current_prefill_tps as f64).unwrap_or(0.0)
        }
    }

    pub fn llm_prefill_avg_15m(&self) -> f64 {
        if self.llm_prefill_samples_15m.is_empty() {
            return self.llm.as_ref().map(|l| l.current_prefill_tps as f64).unwrap_or(0.0);
        }
        let sum: f64 = self.llm_prefill_samples_15m.iter().map(|(_, v)| *v).sum();
        sum / self.llm_prefill_samples_15m.len() as f64
    }

    pub fn llm_prefill_avg_all(&self) -> f64 {
        if self.llm_prefill_all_time_count > 0 {
            self.llm_prefill_all_time_sum / self.llm_prefill_all_time_count as f64
        } else {
            self.llm.as_ref().map(|l| l.current_prefill_tps as f64).unwrap_or(0.0)
        }
    }

    pub fn llm_prefill_peak(&self) -> f64 {
        self.llm_prefill_peak.max(self.prefill_tps_history.max())
    }

    // --- MTP Acceptance Rate Statistics (%) ---
    pub fn llm_mtp_avg_1m(&self) -> f64 {
        let now = std::time::Instant::now();
        let cutoff_1m = now.checked_sub(std::time::Duration::from_secs(60)).unwrap_or(now);
        let mut sum = 0.0;
        let mut count = 0;
        for (t, val) in self.llm_mtp_samples_15m.iter().rev() {
            if *t >= cutoff_1m {
                sum += *val;
                count += 1;
            } else {
                break;
            }
        }
        if count > 0 {
            sum / count as f64
        } else {
            self.llm.as_ref().and_then(|l| l.speculative_acceptance_rate).map(|r| r as f64).unwrap_or(0.0)
        }
    }

    pub fn llm_mtp_avg_15m(&self) -> f64 {
        if self.llm_mtp_samples_15m.is_empty() {
            return self.llm.as_ref().and_then(|l| l.speculative_acceptance_rate).map(|r| r as f64).unwrap_or(0.0);
        }
        let sum: f64 = self.llm_mtp_samples_15m.iter().map(|(_, v)| *v).sum();
        sum / self.llm_mtp_samples_15m.len() as f64
    }

    pub fn llm_mtp_avg_all(&self) -> f64 {
        if self.llm_mtp_all_time_count > 0 {
            self.llm_mtp_all_time_sum / self.llm_mtp_all_time_count as f64
        } else {
            self.llm.as_ref().and_then(|l| l.speculative_acceptance_rate).map(|r| r as f64).unwrap_or(0.0)
        }
    }

    pub fn llm_mtp_peak(&self) -> f64 {
        self.llm_mtp_peak.max(self.speculative_history.max())
    }

    // Backward compatibility aliases
    pub fn llm_tps_avg_1m(&self) -> f64 {
        self.llm_decode_avg_1m()
    }

    pub fn llm_tps_avg_15m(&self) -> f64 {
        self.llm_decode_avg_15m()
    }

    pub fn llm_tps_avg_all_time(&self) -> f64 {
        self.llm_decode_avg_all()
    }

    pub fn handle_metric_update(&mut self, update: MetricUpdate) {
        if self.is_paused {
            return;
        }

        match update {
            MetricUpdate::Cpu(cpu) => {
                let usage = cpu.global_usage_percent as f64;
                if usage.is_finite() {
                    let now = std::time::Instant::now();
                    self.cpu_usage_history.push(usage);
                    self.cpu_all_time_sum += usage;
                    self.cpu_all_time_count += 1;
                    self.cpu_samples_15m.push_back((now, usage));

                    // Prune samples older than 15 minutes (900 seconds)
                    let cutoff_15m = now.checked_sub(std::time::Duration::from_secs(900)).unwrap_or(now);
                    while let Some((t, _)) = self.cpu_samples_15m.front() {
                        if *t < cutoff_15m {
                            self.cpu_samples_15m.pop_front();
                        } else {
                            break;
                        }
                    }
                }
                self.cpu = Some(cpu);
            }
            MetricUpdate::Gpu(gpus) => {
                if gpus.is_empty() {
                    self.selected_gpu_index = 0;
                } else if self.selected_gpu_index >= gpus.len() {
                    self.selected_gpu_index = gpus.len() - 1;
                }

                if let Some(target_gpu) = gpus.get(self.selected_gpu_index).or_else(|| gpus.first()) {
                    let compute = target_gpu.compute_percent as f64;
                    let mem = target_gpu.mem_utilization_percent as f64;
                    let vram_pct = if target_gpu.vram_total_bytes > 0 {
                        (target_gpu.vram_used_bytes as f64 / target_gpu.vram_total_bytes as f64) * 100.0
                    } else {
                        0.0
                    };
                    if compute.is_finite() {
                        self.gpu_compute_history.push(compute);
                    }
                    if mem.is_finite() {
                        self.gpu_mem_controller_history.push(mem);
                    }
                    if vram_pct.is_finite() {
                        self.gpu_vram_history.push(vram_pct);
                    }
                }
                self.gpus = gpus;
            }
            MetricUpdate::Llm(llm) => {
                let now = std::time::Instant::now();
                let dec_tps = llm.instant_decode_tps as f64;
                let prf_tps = llm.current_prefill_tps as f64;
                if dec_tps.is_finite() {
                    self.decode_tps_history.push(dec_tps);
                }
                if prf_tps.is_finite() {
                    self.prefill_tps_history.push(prf_tps);
                }

                // 1. Decode TPS Sampling & Peak Tracking
                let active_dec_tps = if llm.active_slots > 0 && dec_tps > 0.0 {
                    dec_tps
                } else if llm.active_slots > 0 && llm.current_decode_tps > 0.0 {
                    llm.current_decode_tps as f64
                } else {
                    0.0
                };

                if active_dec_tps > 0.0 {
                    self.llm_decode_peak = self.llm_decode_peak.max(active_dec_tps);
                    self.llm_decode_samples_15m.push_back((now, active_dec_tps));
                    self.llm_decode_all_time_sum += active_dec_tps;
                    self.llm_decode_all_time_count += 1;

                    let cutoff_15m = now.checked_sub(std::time::Duration::from_secs(900)).unwrap_or(now);
                    while let Some((t, _)) = self.llm_decode_samples_15m.front() {
                        if *t < cutoff_15m {
                            self.llm_decode_samples_15m.pop_front();
                        } else {
                            break;
                        }
                    }
                }
                if llm.peak_decode_tps > 0.0 {
                    self.llm_decode_peak = self.llm_decode_peak.max(llm.peak_decode_tps as f64);
                }

                // 2. Prefill TPS Sampling & Peak Tracking
                if prf_tps > 0.0 {
                    self.llm_prefill_peak = self.llm_prefill_peak.max(prf_tps);
                    self.llm_prefill_samples_15m.push_back((now, prf_tps));
                    self.llm_prefill_all_time_sum += prf_tps;
                    self.llm_prefill_all_time_count += 1;

                    let cutoff_15m = now.checked_sub(std::time::Duration::from_secs(900)).unwrap_or(now);
                    while let Some((t, _)) = self.llm_prefill_samples_15m.front() {
                        if *t < cutoff_15m {
                            self.llm_prefill_samples_15m.pop_front();
                        } else {
                            break;
                        }
                    }
                }

                // 3. MTP Acceptance Rate Sampling & Peak Tracking
                if let Some(rate) = llm.speculative_acceptance_rate {
                    let r = rate as f64;
                    if r.is_finite() {
                        self.speculative_history.push(r);
                        if r > 0.0 || llm.mtp_draft_generated > 0 {
                            self.llm_mtp_peak = self.llm_mtp_peak.max(r);
                            self.llm_mtp_samples_15m.push_back((now, r));
                            self.llm_mtp_all_time_sum += r;
                            self.llm_mtp_all_time_count += 1;

                            let cutoff_15m = now.checked_sub(std::time::Duration::from_secs(900)).unwrap_or(now);
                            while let Some((t, _)) = self.llm_mtp_samples_15m.front() {
                                if *t < cutoff_15m {
                                    self.llm_mtp_samples_15m.pop_front();
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                }

                if llm.slots.is_empty() {
                    self.selected_slot_index = 0;
                } else if self.selected_slot_index >= llm.slots.len() {
                    self.selected_slot_index = llm.slots.len() - 1;
                }

                self.llm = Some(*llm);
            }
        }
    }

    pub fn set_theme(&mut self, id: ThemeId) {
        self.theme_id = id;
        self.theme = Theme::get(id, self.bg_mode);
    }

    pub fn cycle_theme_next(&mut self) {
        self.set_theme(self.theme_id.next());
    }

    pub fn cycle_theme_prev(&mut self) {
        self.set_theme(self.theme_id.prev());
    }

    pub fn cycle_bg_mode_next(&mut self) {
        self.bg_mode = self.bg_mode.next();
        self.theme = Theme::get(self.theme_id, self.bg_mode);
    }

    pub fn cycle_bg_mode_prev(&mut self) {
        self.bg_mode = self.bg_mode.prev();
        self.theme = Theme::get(self.theme_id, self.bg_mode);
    }

    pub fn toggle_solid_background(&mut self) {
        self.cycle_bg_mode_next();
    }

    pub fn toggle_options_menu(&mut self) {
        self.options_menu_open = !self.options_menu_open;
    }

    pub fn on_menu_up(&mut self) {
        if self.options_menu_index == 0 {
            self.options_menu_index = Self::OPTIONS_COUNT.saturating_sub(1);
        } else {
            self.options_menu_index -= 1;
        }
    }

    pub fn on_menu_down(&mut self) {
        self.options_menu_index = (self.options_menu_index + 1) % Self::OPTIONS_COUNT;
    }

    pub fn on_menu_left(&mut self) {
        match self.options_menu_index {
            0 => self.cycle_theme_prev(),
            1 => self.cycle_bg_mode_prev(),
            2 => {
                let current = self.poll_interval_ms();
                const PRESETS: [u64; 10] = [100, 200, 250, 500, 750, 1000, 1500, 2000, 3000, 5000];
                let next = if let Some(idx) = PRESETS.iter().position(|&x| x == current) {
                    if idx == 0 {
                        PRESETS[PRESETS.len() - 1]
                    } else {
                        PRESETS[idx - 1]
                    }
                } else {
                    current.saturating_sub(100).clamp(100, 5000)
                };
                self.set_poll_interval(next);
            }
            3 => self.active_tab = self.active_tab.prev(),
            _ => {}
        }
    }

    pub fn on_menu_right_or_enter(&mut self) {
        match self.options_menu_index {
            0 => self.cycle_theme_next(),
            1 => self.cycle_bg_mode_next(),
            2 => {
                let current = self.poll_interval_ms();
                const PRESETS: [u64; 10] = [100, 200, 250, 500, 750, 1000, 1500, 2000, 3000, 5000];
                let next = if let Some(idx) = PRESETS.iter().position(|&x| x == current) {
                    if idx + 1 >= PRESETS.len() {
                        PRESETS[0]
                    } else {
                        PRESETS[idx + 1]
                    }
                } else {
                    (current + 100).clamp(100, 5000)
                };
                self.set_poll_interval(next);
            }
            3 => self.active_tab = self.active_tab.next(),
            _ => {}
        }
    }

    pub fn on_tab_pressed(&mut self) {
        self.active_tab = self.active_tab.next();
    }

    pub fn on_backtab_pressed(&mut self) {
        self.active_tab = self.active_tab.prev();
    }

    pub fn toggle_pause(&mut self) {
        self.is_paused = !self.is_paused;
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn next_item(&mut self) {
        if self.options_menu_open {
            self.on_menu_down();
            return;
        }

        match self.active_tab {
            ActiveTab::Dashboard | ActiveTab::SlotsDetails => {
                if let Some(ref llm) = self.llm {
                    if !llm.slots.is_empty() {
                        self.selected_slot_index = (self.selected_slot_index + 1) % llm.slots.len();
                    }
                }
            }
            ActiveTab::GpuDetails if !self.gpus.is_empty() => {
                self.selected_gpu_index = (self.selected_gpu_index + 1) % self.gpus.len();
            }
            ActiveTab::GpuTop => {
                if let Some(gpu) = self.gpus.first() {
                    if !gpu.processes.is_empty() {
                        self.selected_process_index = (self.selected_process_index + 1) % gpu.processes.len();
                    }
                }
            }
            _ => {}
        }
    }

    pub fn prev_item(&mut self) {
        if self.options_menu_open {
            self.on_menu_up();
            return;
        }

        match self.active_tab {
            ActiveTab::Dashboard | ActiveTab::SlotsDetails => {
                if let Some(ref llm) = self.llm {
                    if !llm.slots.is_empty() {
                        if self.selected_slot_index == 0 {
                            self.selected_slot_index = llm.slots.len().saturating_sub(1);
                        } else {
                            self.selected_slot_index -= 1;
                        }
                    }
                }
            }
            ActiveTab::GpuDetails if !self.gpus.is_empty() => {
                if self.selected_gpu_index == 0 {
                    self.selected_gpu_index = self.gpus.len().saturating_sub(1);
                } else {
                    self.selected_gpu_index -= 1;
                }
            }
            ActiveTab::GpuTop => {
                if let Some(gpu) = self.gpus.first() {
                    if !gpu.processes.is_empty() {
                        if self.selected_process_index == 0 {
                            self.selected_process_index = gpu.processes.len().saturating_sub(1);
                        } else {
                            self.selected_process_index -= 1;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
