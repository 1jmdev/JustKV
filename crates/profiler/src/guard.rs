use std::cell::RefCell;

use crate::trace::ActiveTrace;

thread_local! {
    pub(crate) static ACTIVE_TRACE: RefCell<Option<ActiveTrace>> = const { RefCell::new(None) };
}

pub struct RequestGuard {
    pub(crate) active: bool,
}

impl Drop for RequestGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        ACTIVE_TRACE.with(|slot| {
            let mut guard = slot.borrow_mut();
            let Some(mut trace) = guard.take() else {
                return;
            };
            trace.close_all_scopes();
            if trace.emit {
                trace.emit();
            }
        });
    }
}

pub struct ScopeGuard {
    pub(crate) active: bool,
}

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        ACTIVE_TRACE.with(|slot| {
            let mut guard = slot.borrow_mut();
            if let Some(trace) = guard.as_mut() {
                trace.exit_scope();
            }
        });
    }
}
