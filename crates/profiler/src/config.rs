use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

static TRACE_CONFIG: OnceLock<Option<TraceConfig>> = OnceLock::new();

pub(crate) struct TraceConfig {
    pub(crate) command_filter: Option<Vec<u8>>,
    pub(crate) key_filter: Option<Vec<u8>>,
    remaining: AtomicU64,
}

impl TraceConfig {
    fn from_env() -> Option<Self> {
        if !env_flag_enabled("JUSTKV_TRACE") {
            return None;
        }

        let max_traces = std::env::var("JUSTKV_TRACE_MAX")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(1);

        let command_filter = std::env::var("JUSTKV_TRACE_COMMAND")
            .ok()
            .map(|v| uppercased(v.as_bytes()));

        let key_filter = std::env::var("JUSTKV_TRACE_KEY")
            .ok()
            .map(|v| v.into_bytes());

        Some(Self {
            command_filter,
            key_filter,
            remaining: AtomicU64::new(max_traces),
        })
    }

    pub(crate) fn try_acquire_slot(&self) -> bool {
        self.remaining
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |cur| {
                if cur == 0 {
                    None
                } else {
                    Some(cur - 1)
                }
            })
            .is_ok()
    }

    pub(crate) fn command_allowed(&self, command: &[u8]) -> bool {
        match self.command_filter.as_ref() {
            None => true,
            Some(expected) => expected.as_slice() == uppercased(command).as_slice(),
        }
    }
}

pub(crate) fn trace_config() -> Option<&'static TraceConfig> {
    TRACE_CONFIG.get_or_init(TraceConfig::from_env).as_ref()
}

pub(crate) fn uppercased(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    out.make_ascii_uppercase();
    out
}

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name)
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}
