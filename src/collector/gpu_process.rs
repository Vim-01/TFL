use crate::model::GpuProcessInfo;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Collect list of processes currently utilizing GPU memory via Linux DRM fdinfo or nvidia-smi
pub fn collect_drm_gpu_processes(total_vram_bytes: u64) -> Vec<GpuProcessInfo> {
    let mut processes = Vec::new();
    let proc_dir = Path::new("/proc");

    let entries = match fs::read_dir(proc_dir) {
        Ok(e) => e,
        Err(_) => return processes,
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();

        // Must be numeric PID
        let Ok(pid) = name_str.parse::<u32>() else {
            continue;
        };

        let pid_path = entry.path();
        let fdinfo_dir = pid_path.join("fdinfo");
        if !fdinfo_dir.is_dir() {
            continue;
        }

        let fd_entries = match fs::read_dir(&fdinfo_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        let mut total_vram_kib = 0_u64;
        let mut total_gtt_kib = 0_u64;
        let mut seen_clients = HashSet::new();
        let mut has_drm = false;

        for fd_entry in fd_entries.flatten() {
            let fd_path = fd_entry.path();
            let Ok(content) = fs::read_to_string(&fd_path) else {
                continue;
            };

            let mut client_id: Option<String> = None;
            let mut fd_vram_kib = 0_u64;
            let mut fd_gtt_kib = 0_u64;
            let mut is_drm_fd = false;

            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("drm-client-id:") {
                    client_id = Some(rest.trim().to_string());
                } else if let Some(rest) = line.strip_prefix("drm-memory-vram:") {
                    is_drm_fd = true;
                    fd_vram_kib = parse_drm_memory_kib(rest.trim());
                } else if let Some(rest) = line.strip_prefix("drm-memory-gtt:") {
                    is_drm_fd = true;
                    fd_gtt_kib = parse_drm_memory_kib(rest.trim());
                }
            }

            if is_drm_fd {
                has_drm = true;
                if let Some(ref cid) = client_id {
                    if seen_clients.contains(cid) {
                        continue;
                    }
                    seen_clients.insert(cid.clone());
                }
                total_vram_kib = total_vram_kib.saturating_add(fd_vram_kib);
                total_gtt_kib = total_gtt_kib.saturating_add(fd_gtt_kib);
            }
        }

        if has_drm && (total_vram_kib > 0 || total_gtt_kib > 0) {
            // Read process name only for verified GPU clients (avoids hundreds of idle daemon reads)
            let comm = fs::read_to_string(pid_path.join("comm"))
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| format!("PID {pid}"));

            let vram_bytes = total_vram_kib.saturating_mul(1024);
            let gtt_bytes = total_gtt_kib.saturating_mul(1024);
            let vram_percent = if total_vram_bytes > 0 {
                ((vram_bytes as f32 / total_vram_bytes as f32) * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            };

            processes.push(GpuProcessInfo {
                pid,
                name: comm,
                vram_bytes,
                gtt_bytes,
                vram_percent,
            });
        }
    }

    // Sort descending by VRAM usage
    processes.sort_by_key(|a| std::cmp::Reverse(a.vram_bytes));
    processes
}

fn parse_drm_memory_kib(line: &str) -> u64 {
    let mut parts = line.split_whitespace();
    let val: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let unit = parts.next().unwrap_or("KiB");
    match unit {
        "B" => val / 1024,
        "KiB" => val,
        "MiB" => val.saturating_mul(1024),
        "GiB" => val.saturating_mul(1024 * 1024),
        _ => val,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_drm_gpu_processes_does_not_panic() {
        let procs = collect_drm_gpu_processes(24 * 1024 * 1024 * 1024);
        // Should execute smoothly without panic
        for p in procs.iter().take(5) {
            assert!(p.pid > 0);
            assert!(!p.name.is_empty());
        }
    }
}
