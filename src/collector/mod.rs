use crate::model::MetricUpdate;
use cpu::CpuCollector;
use gpu::CompositeGpuCollector;
use llm::LlmCollector;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

pub mod cpu;
pub mod gpu;
pub mod llm;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Spawns background worker tasks that continuously stream metrics to the App.
pub struct CollectorEngine {
    handles: Vec<JoinHandle<()>>,
}

impl CollectorEngine {
    pub fn spawn(
        tx: Sender<MetricUpdate>,
        llm_endpoint: String,
        poll_interval: Arc<AtomicU64>,
    ) -> Self {
        let mut handles = Vec::new();

        // 1. CPU & Memory collector task (runs blocking syscalls via spawn_blocking)
        let tx_cpu = tx.clone();
        let interval_cpu = poll_interval.clone();
        let cpu_handle = tokio::spawn(async move {
            let collector_arc = Arc::new(std::sync::Mutex::new(CpuCollector::new()));
            loop {
                let ms = interval_cpu.load(Ordering::Relaxed).max(50);
                tokio::time::sleep(Duration::from_millis(ms)).await;

                let col = collector_arc.clone();
                let metrics_res = tokio::task::spawn_blocking(move || {
                    let mut guard = col.lock().unwrap();
                    guard.collect()
                })
                .await;

                if let Ok(metrics) = metrics_res {
                    if tx_cpu.send(MetricUpdate::Cpu(metrics)).await.is_err() {
                        break;
                    }
                } else {
                    break;
                }
            }
        });
        handles.push(cpu_handle);

        // 2. GPU collector task (runs hardware sysfs / nvidia-smi via spawn_blocking)
        let tx_gpu = tx.clone();
        let interval_gpu = poll_interval.clone();
        let gpu_handle = tokio::spawn(async move {
            let collector_arc = Arc::new(std::sync::Mutex::new(CompositeGpuCollector::new()));
            loop {
                let ms = interval_gpu.load(Ordering::Relaxed).max(50);
                tokio::time::sleep(Duration::from_millis(ms)).await;

                let col = collector_arc.clone();
                let metrics_res = tokio::task::spawn_blocking(move || {
                    let mut guard = col.lock().unwrap();
                    guard.collect()
                })
                .await;

                if let Ok(metrics) = metrics_res {
                    if tx_gpu.send(MetricUpdate::Gpu(metrics)).await.is_err() {
                        break;
                    }
                } else {
                    break;
                }
            }
        });
        handles.push(gpu_handle);

        // 3. LLM backend collector task (pure async HTTP I/O)
        let tx_llm = tx;
        let interval_llm = poll_interval;
        let llm_handle = tokio::spawn(async move {
            let mut collector = LlmCollector::new(llm_endpoint);
            loop {
                let ms = interval_llm.load(Ordering::Relaxed).max(100);
                tokio::time::sleep(Duration::from_millis(ms)).await;

                let metrics = collector.collect().await;
                if tx_llm.send(MetricUpdate::Llm(Box::new(metrics))).await.is_err() {
                    break;
                }
            }
        });
        handles.push(llm_handle);

        Self { handles }
    }
}

impl Drop for CollectorEngine {
    fn drop(&mut self) {
        for handle in &self.handles {
            handle.abort();
        }
    }
}
