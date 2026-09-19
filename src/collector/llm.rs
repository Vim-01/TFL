use crate::model::{LlmMetrics, SlotInfo};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

/// Collector for llama.cpp / vLLM inference server
pub struct LlmCollector {
    client: Client,
    endpoint_url: String,
    last_poll_instant: Option<Instant>,
    slot_trackers: HashMap<u32, SlotTracker>,
    cached_proc_info: Option<ProcLlamaInfo>,
    last_proc_scan: Option<Instant>,
}

#[derive(Debug, Clone, Default)]
struct SlotTracker {
    last_task_id: Option<i64>,
    last_decoded: u64,
    last_prompt_processed: u64,
}

#[derive(Debug, Clone, Default)]
struct ProcLlamaInfo {
    cache_type_k: String,
    cache_type_v: String,
    model_draft_path: Option<String>,
    model_draft_name: Option<String>,
    model_draft_quant: Option<String>,
    _flash_attention: bool,
}

// JSON schema for llama.cpp /props
#[derive(Debug, Deserialize)]
struct LlamaProps {
    #[serde(default)]
    model_alias: Option<String>,
    #[serde(default)]
    model_path: Option<String>,
    #[serde(default)]
    model_ftype: Option<String>,
    #[serde(default)]
    total_slots: Option<u32>,
    #[serde(default)]
    default_generation_settings: Option<DefaultGenSettings>,
}

#[derive(Debug, Deserialize)]
struct DefaultGenSettings {
    #[serde(default)]
    n_ctx: Option<u64>,
    #[serde(default)]
    _params: Option<GenParams>,
}

#[derive(Debug, Deserialize)]
struct GenParams {
    #[serde(rename = "speculative.types", default)]
    _speculative_types: Option<String>,
}

// Handle both Single Object and Array representations of next_token across llama.cpp versions
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NextTokenRepresentation {
    Single(SlotNextTokenRaw),
    List(Vec<SlotNextTokenRaw>),
}

impl NextTokenRepresentation {
    fn n_decoded(&self) -> u64 {
        match self {
            Self::Single(token) => token.n_decoded.unwrap_or(0),
            Self::List(tokens) => tokens.first().and_then(|t| t.n_decoded).unwrap_or(0),
        }
    }
}

// JSON schema for llama.cpp /slots
#[derive(Debug, Deserialize)]
struct LlamaSlotRaw {
    id: u32,
    #[serde(default)]
    id_task: Option<i64>,
    #[serde(default)]
    is_processing: bool,
    #[serde(default)]
    n_ctx: Option<u64>,
    #[serde(default)]
    n_prompt_tokens: Option<u64>,
    #[serde(default)]
    n_prompt_tokens_processed: Option<u64>,
    #[serde(default)]
    n_prompt_tokens_cache: Option<u64>,
    #[serde(default)]
    speculative: Option<bool>,
    #[serde(default)]
    params: Option<SlotParamsRaw>,
    #[serde(default)]
    next_token: Option<NextTokenRepresentation>,
}

#[derive(Debug, Deserialize)]
struct SlotParamsRaw {
    #[serde(rename = "speculative.types", default)]
    speculative_types: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SlotNextTokenRaw {
    #[serde(default)]
    n_decoded: Option<u64>,
}

impl LlmCollector {
    pub fn new(endpoint_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(800))
            .build()
            .unwrap_or_default();

        Self {
            client,
            endpoint_url,
            last_poll_instant: None,
            slot_trackers: HashMap::new(),
            cached_proc_info: None,
            last_proc_scan: None,
        }
    }

    pub async fn collect(&mut self) -> LlmMetrics {
        let base_url = self.endpoint_url.trim_end_matches('/');

        // Periodically scan /proc (every 5 seconds) to extract CLI flags
        let now = Instant::now();
        if self.last_proc_scan.is_none()
            || now.duration_since(self.last_proc_scan.unwrap()) > Duration::from_secs(5)
        {
            self.cached_proc_info = scan_proc_for_llama_flags();
            self.last_proc_scan = Some(now);
        }

        // Fetch /props
        let props_url = format!("{base_url}/props");
        let props_res = self.client.get(&props_url).send().await;

        let props: Option<LlamaProps> = match props_res {
            Ok(resp) if resp.status().is_success() => resp.json().await.ok(),
            _ => None,
        };

        // Fetch /slots
        let slots_url = format!("{base_url}/slots");
        let slots_res = self.client.get(&slots_url).send().await;

        let raw_slots: Vec<LlamaSlotRaw> = match slots_res {
            Ok(resp) if resp.status().is_success() => resp.json().await.unwrap_or_default(),
            _ => Vec::new(),
        };

        let is_connected = props.is_some() || !raw_slots.is_empty();

        if !is_connected {
            return LlmMetrics {
                is_connected: false,
                engine_name: "llama.cpp (Disconnected)".to_string(),
                endpoint_url: self.endpoint_url.clone(),
                ..Default::default()
            };
        }

        // Parse Model information
        let model_alias = props
            .as_ref()
            .and_then(|p| p.model_alias.clone())
            .unwrap_or_else(|| "Unknown Model".to_string());

        let model_path = props
            .as_ref()
            .and_then(|p| p.model_path.clone())
            .unwrap_or_default();

        let model_ftype = props
            .as_ref()
            .and_then(|p| p.model_ftype.clone())
            .unwrap_or_else(|| "f16".to_string());

        let context_window_max = props
            .as_ref()
            .and_then(|p| p.default_generation_settings.as_ref())
            .and_then(|s| s.n_ctx)
            .unwrap_or(8192)
            .max(1);

        let total_slots = props
            .as_ref()
            .and_then(|p| p.total_slots)
            .unwrap_or(raw_slots.len().max(1) as u32);

        // Extract cache quantization and draft model from /proc flags if available
        let proc_info = self.cached_proc_info.clone().unwrap_or_default();
        let cache_type_k = if !proc_info.cache_type_k.is_empty() {
            proc_info.cache_type_k
        } else {
            "f16".to_string()
        };
        let cache_type_v = if !proc_info.cache_type_v.is_empty() {
            proc_info.cache_type_v
        } else {
            "f16".to_string()
        };

        // Parse slots & aggregate tokens
        let mut active_slots = 0;
        let mut total_prompt_tokens = 0_u64;
        let mut total_cache_hit_tokens = 0_u64;
        let mut total_context_tokens_used = 0_u64;
        let mut has_speculative = false;
        let mut speculative_type_found = None;

        let elapsed_secs = self
            .last_poll_instant
            .map(|t| now.duration_since(t).as_secs_f32())
            .unwrap_or(0.0);

        let mut d_prompt_tokens = 0_u64;
        let mut d_decoded_tokens = 0_u64;
        let mut slot_infos = Vec::with_capacity(raw_slots.len());

        for raw in raw_slots {
            if raw.is_processing {
                active_slots += 1;
            }

            let prompt_processed = raw.n_prompt_tokens_processed.unwrap_or(0);
            let prompt_total = raw.n_prompt_tokens.unwrap_or(0);
            let prompt_cache = raw.n_prompt_tokens_cache.unwrap_or(0);
            let decoded = raw.next_token.as_ref().map(|nt| nt.n_decoded()).unwrap_or(0);

            total_prompt_tokens = total_prompt_tokens.saturating_add(prompt_total);
            total_cache_hit_tokens = total_cache_hit_tokens.saturating_add(prompt_cache);

            // Total context occupied in KV cache pool is the sum of tokens across active slots
            let slot_used = prompt_total.saturating_add(decoded);
            total_context_tokens_used = total_context_tokens_used.saturating_add(slot_used);

            // Calculate per-slot delta token throughput safely
            let tracker = self.slot_trackers.entry(raw.id).or_default();
            if raw.id_task == tracker.last_task_id {
                let d_dec = decoded.saturating_sub(tracker.last_decoded);
                let d_prm = prompt_processed.saturating_sub(tracker.last_prompt_processed);
                d_decoded_tokens = d_decoded_tokens.saturating_add(d_dec);
                d_prompt_tokens = d_prompt_tokens.saturating_add(d_prm);
            } else {
                // Task changed / new request started
                d_decoded_tokens = d_decoded_tokens.saturating_add(decoded);
                d_prompt_tokens = d_prompt_tokens.saturating_add(prompt_processed);
            }

            tracker.last_task_id = raw.id_task;
            tracker.last_decoded = decoded;
            tracker.last_prompt_processed = prompt_processed;

            let spec = raw.speculative.unwrap_or(false);
            let spec_type = raw
                .params
                .as_ref()
                .and_then(|p| p.speculative_types.clone());

            if spec || spec_type.is_some() {
                has_speculative = true;
                if spec_type.is_some() {
                    speculative_type_found = spec_type.clone();
                }
            }

            slot_infos.push(SlotInfo {
                id: raw.id,
                id_task: raw.id_task,
                is_processing: raw.is_processing,
                n_ctx: raw.n_ctx.unwrap_or(context_window_max),
                n_prompt_tokens: prompt_total,
                n_prompt_tokens_processed: prompt_processed,
                n_prompt_tokens_cache: prompt_cache,
                n_decoded: decoded,
                speculative: spec,
                speculative_type: spec_type,
                decode_tokens_per_sec: 0.0,
            });
        }

        // Compute throughput speeds
        let (current_prefill_tps, current_decode_tps) = if elapsed_secs > 0.05 {
            let prefill_speed = d_prompt_tokens as f32 / elapsed_secs;
            let decode_speed = d_decoded_tokens as f32 / elapsed_secs;
            (prefill_speed, decode_speed)
        } else {
            (0.0, 0.0)
        };

        self.last_poll_instant = Some(now);

        // KV Cache Pool calculation: context tokens used across all slots vs context limit
        let kv_cache_pool_percent = if context_window_max > 0 {
            ((total_context_tokens_used as f32 / context_window_max as f32) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

        // Cache hit rate
        let cache_hit_rate_percent = if total_prompt_tokens > 0 {
            ((total_cache_hit_tokens as f32 / total_prompt_tokens as f32) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

        LlmMetrics {
            is_connected: true,
            engine_name: "llama.cpp".to_string(),
            endpoint_url: self.endpoint_url.clone(),
            model_alias,
            model_path,
            model_ftype,
            model_param_count: None,
            cache_type_k,
            cache_type_v,
            speculative_enabled: has_speculative || proc_info.model_draft_path.is_some(),
            speculative_type: speculative_type_found
                .or_else(|| proc_info.model_draft_name.clone()),
            speculative_draft_model: proc_info.model_draft_name,
            speculative_draft_quant: proc_info.model_draft_quant,
            // Only report speculative acceptance rate when actually measured, never fabricate
            speculative_acceptance_rate: None,
            total_slots,
            active_slots,
            pending_requests: 0,
            evicted_blocks: 0,
            kv_cache_pool_percent,
            context_tokens_used: total_context_tokens_used,
            context_window_max,
            cache_hit_rate_percent,
            current_prefill_tps,
            current_decode_tps,
            time_to_first_token_ms: None,
            inter_token_latency_ms: None,
            slots: slot_infos,
        }
    }
}

/// Parse /proc to extract active llama-server command line arguments
fn scan_proc_for_llama_flags() -> Option<ProcLlamaInfo> {
    let proc_dir = Path::new("/proc");
    let entries = fs::read_dir(proc_dir).ok()?;

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let pid_str = file_name.to_string_lossy();
        if !pid_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        let cmdline_path = entry.path().join("cmdline");
        if let Ok(bytes) = fs::read(&cmdline_path) {
            let cmdline = String::from_utf8_lossy(&bytes);
            if !cmdline.contains("llama-server") {
                continue;
            }

            // Args are separated by \0 in /proc/cmdline
            let args: Vec<&str> = cmdline.split('\0').filter(|s| !s.is_empty()).collect();
            let mut info = ProcLlamaInfo::default();

            let mut i = 0;
            while i < args.len() {
                let arg = args[i];
                if (arg == "-ctk" || arg == "--cache-type-k") && i + 1 < args.len() {
                    info.cache_type_k = args[i + 1].to_string();
                    i += 1;
                } else if (arg == "-ctv" || arg == "--cache-type-v") && i + 1 < args.len() {
                    info.cache_type_v = args[i + 1].to_string();
                    i += 1;
                } else if (arg == "-md" || arg == "--model-draft") && i + 1 < args.len() {
                    let draft_path = args[i + 1].to_string();
                    let name = Path::new(&draft_path)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| draft_path.clone());

                    let quant = extract_quant_from_name(&name);
                    info.model_draft_path = Some(draft_path);
                    info.model_draft_name = Some(name);
                    info.model_draft_quant = quant;
                    i += 1;
                } else if arg == "-fa" || arg == "--flash-attn" {
                    info._flash_attention = true;
                }
                i += 1;
            }

            return Some(info);
        }
    }

    None
}

fn extract_quant_from_name(name: &str) -> Option<String> {
    let upper = name.to_uppercase();
    for keyword in &["BF16", "F16", "F32"] {
        if upper.contains(keyword) {
            return Some((*keyword).to_string());
        }
    }

    let clean = upper.trim_end_matches(".GGUF");

    // Look for occurrences of quant prefixes with boundary markers
    for prefix in &["-IQ", "_IQ", ".IQ", "/IQ", "-Q", "_Q", ".Q", "/Q"] {
        if let Some(pos) = clean.rfind(prefix) {
            let quant_part = &clean[pos + 1..];
            let end = quant_part
                .find(|c: char| c == '-' || c == '.' || c == '/' || c.is_whitespace())
                .unwrap_or(quant_part.len());
            let candidate = &quant_part[..end];
            if is_quant_string(candidate) {
                return Some(candidate.to_string());
            }
        }
    }

    // Direct check if any component starts directly with quant
    for part in clean.split(|c: char| c == '-' || c == '.' || c == '/' || c.is_whitespace()) {
        if is_quant_string(part) {
            return Some(part.to_string());
        }
    }
    None
}

fn is_quant_string(s: &str) -> bool {
    if s.starts_with("IQ") && s.len() >= 3 {
        s.chars().nth(2).is_some_and(|c| c.is_ascii_digit())
    } else if s.starts_with('Q') && s.len() >= 2 {
        s.chars().nth(1).is_some_and(|c| c.is_ascii_digit())
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_quant_from_name() {
        assert_eq!(
            extract_quant_from_name("Qwen3.8-27B-GSQ-RCO-IQ3_S.gguf"),
            Some("IQ3_S".to_string())
        );
        assert_eq!(
            extract_quant_from_name("Qwen3.8-27B-DSpark-Q8_0.gguf"),
            Some("Q8_0".to_string())
        );
        assert_eq!(
            extract_quant_from_name("model-q4_k_m.gguf"),
            Some("Q4_K_M".to_string())
        );
        assert_eq!(
            extract_quant_from_name("DeepSeek-Coder_Q4_K_M.gguf"),
            Some("Q4_K_M".to_string())
        );
        assert_eq!(
            extract_quant_from_name("mmproj-Qwen3.8-27B-BF16.gguf"),
            Some("BF16".to_string())
        );
        assert_eq!(extract_quant_from_name("unquantized-model.bin"), None);
    }
}
