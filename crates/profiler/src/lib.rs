#[cfg(feature = "enabled")]
mod config;
#[cfg(feature = "enabled")]
mod guard;
#[cfg(feature = "enabled")]
mod render;
#[cfg(feature = "enabled")]
mod trace;

#[cfg(feature = "enabled")]
use config::{trace_config, uppercased};
#[cfg(feature = "enabled")]
use guard::ACTIVE_TRACE;
#[cfg(feature = "enabled")]
use trace::ActiveTrace;

#[cfg(feature = "enabled")]
pub use guard::{RequestGuard, ScopeGuard};
#[cfg(feature = "enabled")]
pub use render::{fmt_time, ns_to_us, render_result_plain, render_result_pretty};
#[cfg(feature = "enabled")]
pub use trace::{CapturedNode, TraceResult};

#[cfg(feature = "enabled")]
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

/// Like `begin_request` but bypasses env-var config entirely.  Used by the
/// embedded profiling hub so that every request is traced regardless of whether
/// `BETTERKV_TRACE` is set in the environment.
#[cfg(feature = "enabled")]
pub fn begin_request_unconditional(command_hint: &[u8]) -> RequestGuard {
    let command = uppercased(command_hint);
    ACTIVE_TRACE.with(|slot| {
        *slot.borrow_mut() = Some(ActiveTrace::new(command, false));
    });
    RequestGuard { active: true }
}

#[cfg(feature = "enabled")]
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

#[cfg(feature = "enabled")]
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

#[cfg(feature = "enabled")]
pub fn run_profiled<F, R>(label: &'static str, f: F) -> (R, TraceResult)
where
    F: FnOnce() -> R,
{
    // Install a fresh active trace on this thread.
    ACTIVE_TRACE.with(|slot| {
        *slot.borrow_mut() = Some(ActiveTrace::new(label.as_bytes().to_vec(), false));
    });

    let ret = f();

    // Finalise and capture.
    let result = ACTIVE_TRACE.with(|slot| {
        let mut guard = slot.borrow_mut();
        let mut trace = guard.take().expect("trace was set above");
        trace.close_all_scopes();
        TraceResult::from_active(&trace)
    });

    (ret, result)
}

/// Finalise the currently-active thread-local trace and return it, without
/// installing a new one.  Call this *after* running the user's closure so that
/// every `scope()` call made during the request is part of the result.
#[cfg(feature = "enabled")]
pub fn capture_active_trace() -> Option<TraceResult> {
    ACTIVE_TRACE.with(|slot| {
        let mut guard = slot.borrow_mut();
        let mut active = guard.take()?;
        active.close_all_scopes();
        Some(TraceResult::from_active(&active))
    })
}

#[cfg(feature = "enabled")]
pub fn run_captured<F, R>(command_hint: &[u8], key: Option<&[u8]>, f: F) -> (R, Option<TraceResult>)
where
    F: FnOnce() -> R,
{
    let command = uppercased(command_hint);
    ACTIVE_TRACE.with(|slot| {
        *slot.borrow_mut() = Some(ActiveTrace::new(command, false));
    });

    if let Some(key) = key {
        bind_request_key(key);
    }

    let ret = f();

    let trace = ACTIVE_TRACE.with(|slot| {
        let mut guard = slot.borrow_mut();
        let mut active = guard.take()?;
        active.close_all_scopes();
        Some(TraceResult::from_active(&active))
    });

    (ret, trace)
}

// ── Disabled stubs (default)

#[cfg(not(feature = "enabled"))]
pub struct RequestGuard;

#[cfg(not(feature = "enabled"))]
impl Drop for RequestGuard {
    fn drop(&mut self) {}
}

#[cfg(not(feature = "enabled"))]
pub struct ScopeGuard;

#[cfg(not(feature = "enabled"))]
#[derive(Clone)]
pub struct TraceResult;

#[cfg(not(feature = "enabled"))]
#[derive(Clone)]
pub struct CapturedNode;

#[cfg(not(feature = "enabled"))]
impl Drop for ScopeGuard {
    fn drop(&mut self) {}
}

#[cfg(not(feature = "enabled"))]
#[inline(always)]
pub fn begin_request(_command_hint: &[u8]) -> Option<RequestGuard> {
    None
}

#[cfg(not(feature = "enabled"))]
#[inline(always)]
pub fn bind_request_key(_key: &[u8]) {}

#[cfg(not(feature = "enabled"))]
#[inline(always)]
pub fn scope(_name: &'static str) -> ScopeGuard {
    ScopeGuard
}

#[cfg(not(feature = "enabled"))]
#[inline(always)]
pub fn capture_active_trace() -> Option<TraceResult> {
    None
}

#[cfg(not(feature = "enabled"))]
#[inline(always)]
pub fn begin_request_unconditional(_command_hint: &[u8]) -> RequestGuard {
    RequestGuard
}

#[cfg(not(feature = "enabled"))]
#[inline(always)]
pub fn run_captured<F, R>(_command_hint: &[u8], _key: Option<&[u8]>, f: F) -> (R, Option<()>)
where
    F: FnOnce() -> R,
{
    (f(), None)
}
