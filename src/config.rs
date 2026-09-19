use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "tfl",
    version = "0.1.0",
    about = "Top For LLM (TFL) — Modern TUI monitor for local LLM inference, GPU, and CPU"
)]
pub struct Config {
    /// LLM inference backend endpoint URL (llama.cpp, vLLM, Ollama)
    #[arg(short = 'u', long = "url", default_value = "http://127.0.0.1:8080")]
    pub endpoint_url: String,

    /// Metrics update polling interval in milliseconds
    #[arg(short = 'i', long = "interval", default_value_t = 1000)]
    pub interval_ms: u64,

    /// Optional log file path (to avoid interfering with the TUI)
    #[arg(long = "log", default_value = "/tmp/tfl.log")]
    pub log_path: String,
}
