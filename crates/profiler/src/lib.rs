mod config;
mod guard;
mod render;
mod trace;

use config::{trace_config, uppercased};
use guard::ACTIVE_TRACE;
use trace::ActiveTrace;

pub use guard::{RequestGuard, ScopeGuard};

/// Call at the start of handling a request. Returns a guard that, when
/// dropped, finalises and prints the full call tree.
pub fn begin_request(command_hint: &[u8]) -> Option<RequestGuard> {
    let config = trace_config()?;

    if !config.command_allowed(command_hint) {
        return None;
    }
    if !config.try_acquire_slot() {
        return None;
    }

    let command = uppercased(command_hint);
    ACTIVE_TRACE.with(|slot| {
        *slot.borrow_mut() = Some(ActiveTrace::new(command, config.pretty));
    });
    Some(RequestGuard { active: true })
}

/// Call once the key is known so the trace header shows the key and optional
/// key-based filtering can suppress the trace.
pub fn bind_request_key(key: &[u8]) {
    ACTIVE_TRACE.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(trace) = guard.as_mut() else {
            return;
        };
        trace.key = Some(key.to_vec());

        if let Some(config) = trace_config() {
            if let Some(filter_key) = config.key_filter.as_ref() {
                if filter_key.as_slice() != key {
                    trace.emit = false;
                }
            }
        }
    });
}

/// Wrap a function body to record it as a named node in the call tree.
/// Typically called as `let _t = profiler::scope("module::function");`.
pub fn scope(name: &'static str) -> ScopeGuard {
    let mut entered = false;
    ACTIVE_TRACE.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(trace) = guard.as_mut() else {
            return;
        };
        trace.enter_scope(name);
        entered = true;
    });
    ScopeGuard { active: entered }
}
