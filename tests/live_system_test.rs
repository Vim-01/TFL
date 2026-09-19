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
