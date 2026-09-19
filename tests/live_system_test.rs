use tfl::collector::cpu::CpuCollector;
use tfl::collector::gpu::CompositeGpuCollector;
use tfl::collector::llm::LlmCollector;

#[tokio::test]
async fn test_live_cpu_collector() {
    let mut cpu = CpuCollector::new();
    let metrics = cpu.collect();

    assert!(!metrics.core_usages.is_empty(), "Should detect at least 1 CPU core");
    println!("Detected {} CPU cores, global load: {:.1}%", metrics.core_usages.len(), metrics.global_usage_percent);
    println!("RAM Total: {:.2} GB, Used: {:.2} GB",
        metrics.ram_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
        metrics.ram_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    );
}

#[tokio::test]
async fn test_live_gpu_collector() {
    let mut gpu = CompositeGpuCollector::new();
    let gpus = gpu.collect();

    for g in &gpus {
        println!("GPU {}: {} ({})", g.index, g.name, g.vendor.as_str());
        println!("VRAM: {:.2} GB / {:.2} GB, Compute: {:.1}%",
            g.vram_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            g.vram_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            g.compute_percent
        );
        if let Some(t) = g.temp_edge_c {
            println!("Edge Temp: {:.1}°C", t);
        }
        if let Some(h) = g.temp_hotspot_c {
            println!("Hotspot Temp: {:.1}°C", h);
        }
        if let Some(m) = g.temp_mem_c {
            println!("VRAM Temp: {:.1}°C", m);
        }
        if let Some(p) = g.power_current_w {
            println!("Power: {:.1} W", p);
        }
        if let Some(rpm) = g.fan_rpm {
            println!("Fan RPM: {}", rpm);
        }
        if let Some(mv) = g.voltage_mv {
            println!("Voltage: {} mV", mv);
        }
        if let Some(ref pcie) = g.pcie_link {
            println!("PCIe Link: {}", pcie);
        }
        println!("GPU Processes: {} found", g.processes.len());
        for p in g.processes.iter().take(5) {
            println!("  - PID {}: {} | VRAM: {:.1} MiB ({:.0}%) | GTT: {:.1} MiB",
                p.pid, p.name, p.vram_bytes as f64 / (1024.0 * 1024.0), p.vram_percent, p.gtt_bytes as f64 / (1024.0 * 1024.0));
        }
    }
}

#[tokio::test]
async fn test_live_llm_collector() {
    let mut llm = LlmCollector::new("http://127.0.0.1:8080".to_string());
    let metrics = llm.collect().await;

    println!("LLM Connected: {}", metrics.is_connected);
    if metrics.is_connected {
        println!("Model: {}", metrics.model_alias);
        println!("Quant: {}", metrics.model_ftype);
        println!("Cache K: {}, Cache V: {}", metrics.cache_type_k, metrics.cache_type_v);
        println!("Speculative Draft: {:?}", metrics.speculative_draft_model);
        println!("Slots: {}/{}", metrics.active_slots, metrics.total_slots);
        if let Some(r) = metrics.speculative_acceptance_rate {
            println!("MTP Acceptance: {:.1}%", r);
        }
    }
}

#[tokio::test]
async fn test_live_dashboard_render() {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::widgets::Widget;
    use tfl::app::App;
    use tfl::model::MetricUpdate;
    use tfl::ui::DashboardView;

    let mut app = App::new();

    // Collect live metrics
    let mut cpu = CpuCollector::new();
    let cpu_metrics = cpu.collect();
    app.handle_metric_update(MetricUpdate::Cpu(cpu_metrics));

    let mut gpu = CompositeGpuCollector::new();
    let gpu_metrics = gpu.collect();
    app.handle_metric_update(MetricUpdate::Gpu(gpu_metrics));

    let mut llm = LlmCollector::new("http://127.0.0.1:8080".to_string());
    let llm_metrics = llm.collect().await;
    app.handle_metric_update(MetricUpdate::Llm(Box::new(llm_metrics)));

    // Render Dashboard to in-memory terminal buffer (160x45)
    let area = Rect::new(0, 0, 160, 45);
    let mut buf = Buffer::empty(area);

    let view = DashboardView::new(&app);
    view.render(area, &mut buf);

    // Verify buffer was written and non-empty
    let mut non_empty_cells = 0;
    for y in 0..area.height {
        for x in 0..area.width {
            if buf[(x, y)].symbol() != " " {
                non_empty_cells += 1;
            }
        }
    }
    println!("Dashboard successfully rendered: {non_empty_cells} active glyph cells");
    assert!(non_empty_cells > 100, "Dashboard should contain rendered glyphs");
}

#[tokio::test]
async fn test_live_gpu_top_render() {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::widgets::Widget;
    use tfl::app::{ActiveTab, App};
    use tfl::model::MetricUpdate;
    use tfl::ui::gpu_top_view::GpuTopView;

    let mut app = App::new();
    let mut gpu = CompositeGpuCollector::new();
    let gpu_metrics = gpu.collect();
    app.handle_metric_update(MetricUpdate::Gpu(gpu_metrics));

    // Tab transitions
    assert_eq!(ActiveTab::Dashboard.next(), ActiveTab::GpuDetails);
    assert_eq!(ActiveTab::GpuDetails.next(), ActiveTab::SlotsDetails);
    assert_eq!(ActiveTab::SlotsDetails.next(), ActiveTab::GpuTop);
    assert_eq!(ActiveTab::GpuTop.next(), ActiveTab::Help);
    assert_eq!(ActiveTab::Help.next(), ActiveTab::Dashboard);

    assert_eq!(ActiveTab::Dashboard.prev(), ActiveTab::Help);
    assert_eq!(ActiveTab::Help.prev(), ActiveTab::GpuTop);
    assert_eq!(ActiveTab::GpuTop.prev(), ActiveTab::SlotsDetails);

    // Render GpuTopView
    let area = Rect::new(0, 0, 160, 45);
    let mut buf = Buffer::empty(area);
    let view = GpuTopView::new(&app);
    view.render(area, &mut buf);

    let mut non_empty_cells = 0;
    for y in 0..area.height {
        for x in 0..area.width {
            if buf[(x, y)].symbol() != " " {
                non_empty_cells += 1;
            }
        }
    }
    println!("GpuTopView rendered: {non_empty_cells} active glyph cells");
    assert!(non_empty_cells > 50, "GpuTopView should render borders and process info");
}

#[tokio::test]
async fn test_dynamic_slots_scaling() {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::widgets::Widget;
    use tfl::app::App;
    use tfl::model::{LlmMetrics, MetricUpdate, SlotInfo};
    use tfl::ui::DashboardView;

    let mut app = App::new();

    // Create mock LLM with 4 slots
    let mut slots = Vec::new();
    for id in 0..4 {
        slots.push(SlotInfo {
            id,
            id_task: Some(1000 + id as i64),
            is_processing: id == 0 || id == 2,
            n_ctx: 131072,
            n_prompt_tokens: 1024,
            n_prompt_tokens_processed: 1024,
            n_prompt_tokens_cache: 512,
            n_decoded: 42 * (id as u64 + 1),
            speculative: true,
            speculative_type: Some("draft-dspark".to_string()),
            decode_tokens_per_sec: 35.0,
            draft_acceptance_rate: Some(25.0),
        });
    }

    let llm_metrics = LlmMetrics {
        is_connected: true,
        engine_name: "llama.cpp".to_string(),
        endpoint_url: "http://127.0.0.1:8080".to_string(),
        model_alias: "Qwen3.8-27B".to_string(),
        model_path: "/models/qwen.gguf".to_string(),
        model_ftype: "IQ3_S".to_string(),
        context_window_max: 131072,
        total_slots: 4,
        active_slots: 2,
        context_tokens_used: 4096,
        cache_hit_rate_percent: 50.0,
        kv_cache_pool_percent: 25.0,
        cache_type_k: "q8_0".to_string(),
        cache_type_v: "q8_0".to_string(),
        speculative_draft_model: Some("DSpark".to_string()),
        speculative_draft_quant: Some("Q8_0".to_string()),
        speculative_type: Some("draft-dspark".to_string()),
        speculative_acceptance_rate: Some(25.0),
        instant_decode_tps: 35.0,
        current_decode_tps: 35.0,
        current_prefill_tps: 450.0,
        peak_decode_tps: 65.0,
        slots,
        ..Default::default()
    };

    app.handle_metric_update(MetricUpdate::Llm(Box::new(llm_metrics)));

    // Render dashboard with 4 slots on standard terminal (160x45)
    let area = Rect::new(0, 0, 160, 45);
    let mut buf = Buffer::empty(area);
    let view = DashboardView::new(&app);
    view.render(area, &mut buf);

    // Check that Slot #0, #1, #2, #3 are rendered in the buffer
    let mut found_slot_0 = false;
    let mut found_slot_1 = false;
    let mut found_slot_2 = false;
    let mut found_slot_3 = false;

    for y in 0..area.height {
        let mut line_str = String::new();
        for x in 0..area.width {
            line_str.push_str(buf[(x, y)].symbol());
        }
        if line_str.contains("#0") { found_slot_0 = true; }
        if line_str.contains("#1") { found_slot_1 = true; }
        if line_str.contains("#2") { found_slot_2 = true; }
        if line_str.contains("#3") { found_slot_3 = true; }
    }

    assert!(found_slot_0, "Slot #0 must be visible");
    assert!(found_slot_1, "Slot #1 must be visible when scaled");
    assert!(found_slot_2, "Slot #2 must be visible when scaled");
    assert!(found_slot_3, "Slot #3 must be visible when scaled");
    println!("Dynamic slots scaling verified: all 4 slots rendered successfully!");
}
