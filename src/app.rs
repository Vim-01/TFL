use crate::model::{CpuMetrics, GpuMetrics, HistoryRingBuffer, LlmMetrics, MetricUpdate};
use crate::theme::{Theme, ThemeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Dashboard,
    GpuDetails,
    SlotsDetails,
    Help,
}

impl ActiveTab {
    pub fn next(&self) -> Self {
        match self {
            Self::Dashboard => Self::GpuDetails,
            Self::GpuDetails => Self::SlotsDetails,
            Self::SlotsDetails => Self::Help,
            Self::Help => Self::Dashboard,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::Dashboard => Self::Help,
            Self::GpuDetails => Self::Dashboard,
            Self::SlotsDetails => Self::GpuDetails,
            Self::Help => Self::SlotsDetails,
        }
    }
}

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct App {
    pub cpu: Option<CpuMetrics>,
    pub gpus: Vec<GpuMetrics>,
    pub llm: Option<LlmMetrics>,

    // 60-second historical data ring buffers
    pub decode_tps_history: HistoryRingBuffer<f64, 60>,
    pub prefill_tps_history: HistoryRingBuffer<f64, 60>,
    pub gpu_compute_history: HistoryRingBuffer<f64, 60>,
    pub gpu_vram_history: HistoryRingBuffer<f64, 60>,
    pub cpu_usage_history: HistoryRingBuffer<f64, 60>,
    pub speculative_history: HistoryRingBuffer<f64, 60>,

    pub active_tab: ActiveTab,
    pub is_paused: bool,
    pub should_quit: bool,
    pub selected_gpu_index: usize,
    pub selected_slot_index: usize,

    // Theming and Options State
    pub theme_id: ThemeId,
    pub theme: Theme,
    pub solid_background: bool,
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
        let solid_background = false;
        let theme = Theme::get(theme_id, solid_background);

        Self {
            cpu: None,
            gpus: Vec::new(),
            llm: None,
            decode_tps_history: HistoryRingBuffer::with_initial(0.0),
            prefill_tps_history: HistoryRingBuffer::with_initial(0.0),
            gpu_compute_history: HistoryRingBuffer::with_initial(0.0),
            gpu_vram_history: HistoryRingBuffer::with_initial(0.0),
            cpu_usage_history: HistoryRingBuffer::with_initial(0.0),
            speculative_history: HistoryRingBuffer::with_initial(0.0),
            active_tab: ActiveTab::Dashboard,
            is_paused: false,
            should_quit: false,
            selected_gpu_index: 0,
            selected_slot_index: 0,
            theme_id,
            theme,
            solid_background,
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

    pub fn handle_metric_update(&mut self, update: MetricUpdate) {
        if self.is_paused {
            return;
        }

        match update {
            MetricUpdate::Cpu(cpu) => {
                let usage = cpu.global_usage_percent as f64;
                if usage.is_finite() {
                    self.cpu_usage_history.push(usage);
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
                    if compute.is_finite() {
                        self.gpu_compute_history.push(compute);
                    }
                    if mem.is_finite() {
                        self.gpu_vram_history.push(mem);
                    }
                }
                self.gpus = gpus;
            }
            MetricUpdate::Llm(llm) => {
                let dec_tps = llm.current_decode_tps as f64;
                let prf_tps = llm.current_prefill_tps as f64;
                if dec_tps.is_finite() {
                    self.decode_tps_history.push(dec_tps);
                }
                if prf_tps.is_finite() {
                    self.prefill_tps_history.push(prf_tps);
                }
                if let Some(rate) = llm.speculative_acceptance_rate {
                    let r = rate as f64;
                    if r.is_finite() {
                        self.speculative_history.push(r);
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
        self.theme = Theme::get(id, self.solid_background);
    }

    pub fn cycle_theme_next(&mut self) {
        self.set_theme(self.theme_id.next());
    }

    pub fn cycle_theme_prev(&mut self) {
        self.set_theme(self.theme_id.prev());
    }

    pub fn toggle_solid_background(&mut self) {
        self.solid_background = !self.solid_background;
        self.theme = Theme::get(self.theme_id, self.solid_background);
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
            1 => self.toggle_solid_background(),
            2 => {
                let current = self.poll_interval_ms();
                let next = match current {
                    2000 => 1000,
                    1000 => 500,
                    500 => 250,
                    _ => 2000,
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
            1 => self.toggle_solid_background(),
            2 => {
                let current = self.poll_interval_ms();
                let next = match current {
                    250 => 500,
                    500 => 1000,
                    1000 => 2000,
                    _ => 250,
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
            _ => {}
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
