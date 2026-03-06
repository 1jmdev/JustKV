#[cfg(feature = "profiling")]
mod inner {
    use std::sync::Arc;

    use parking_lot::Mutex;

    #[derive(Clone, Copy)]
    pub enum ReportKind {
        All,
        Avg,
        Best,
        Worst,
    }

    #[derive(Default)]
    pub(super) struct ProfileState {
        pub enabled: bool,
        pub runs: Vec<profiler::TraceResult>,
    }

    #[derive(Clone)]
    pub struct ProfileHub {
        pub(super) inner: Arc<Mutex<ProfileState>>,
    }

    impl ProfileHub {
        pub fn disabled() -> Self {
            Self {
                inner: Arc::new(Mutex::new(ProfileState::default())),
            }
        }

        pub fn capturing() -> Self {
            Self {
                inner: Arc::new(Mutex::new(ProfileState {
                    enabled: true,
                    runs: Vec::new(),
                })),
            }
        }

        pub fn reset(&self) {
            self.inner.lock().runs.clear();
        }

        pub fn set_enabled(&self, enabled: bool) {
            self.inner.lock().enabled = enabled;
        }

        #[inline]
        pub fn is_enabled(&self) -> bool {
            self.inner.lock().enabled
        }

        #[inline]
        pub fn run_command<F, R>(&self, key: Option<&[u8]>, f: F) -> R
        where
            F: FnOnce() -> R,
        {
            if self.is_enabled() {
                if let Some(key) = key {
                    profiler::bind_request_key(key);
                }
                let ret = f();
                if let Some(trace) = profiler::capture_active_trace() {
                    self.inner.lock().runs.push(trace);
                }
                return ret;
            }
            f()
        }

        pub fn selected_runs(
            &self,
            kind: ReportKind,
        ) -> Result<Vec<profiler::TraceResult>, String> {
            let state = self.inner.lock();
            if state.runs.is_empty() {
                return Ok(Vec::new());
            }
            Ok(choose_runs(&state.runs, kind)
                .into_iter()
                .cloned()
                .collect())
        }
    }

    pub(super) fn choose_runs(
        runs: &[profiler::TraceResult],
        kind: ReportKind,
    ) -> Vec<&profiler::TraceResult> {
        match kind {
            ReportKind::All => runs.iter().collect(),
            ReportKind::Best => runs
                .iter()
                .min_by_key(|run| run.total_ns)
                .into_iter()
                .collect(),
            ReportKind::Worst => runs
                .iter()
                .max_by_key(|run| run.total_ns)
                .into_iter()
                .collect(),
            ReportKind::Avg => {
                let avg = runs.iter().map(|run| run.total_ns).sum::<u64>() / runs.len() as u64;
                runs.iter()
                    .min_by_key(|run| run.total_ns.abs_diff(avg))
                    .into_iter()
                    .collect()
            }
        }
    }
}

#[cfg(not(feature = "profiling"))]
mod inner {
    #[derive(Clone, Copy)]
    pub struct ProfileHub;

    #[derive(Clone, Copy)]
    pub enum ReportKind {
        All,
        Avg,
        Best,
        Worst,
    }

    impl ProfileHub {
        #[inline(always)]
        pub fn disabled() -> Self {
            Self
        }

        #[inline(always)]
        pub fn capturing() -> Self {
            Self
        }

        #[inline(always)]
        pub fn reset(&self) {}

        #[inline(always)]
        pub fn set_enabled(&self, _enabled: bool) {}

        #[inline(always)]
        pub fn is_enabled(&self) -> bool {
            false
        }

        #[inline(always)]
        pub fn run_command<F, R>(&self, _key: Option<&[u8]>, f: F) -> R
        where
            F: FnOnce() -> R,
        {
            f()
        }

        pub fn selected_runs(
            &self,
            _kind: ReportKind,
        ) -> Result<Vec<std::convert::Infallible>, String> {
            Err("profiling support is not compiled into this server build".to_string())
        }
    }
}

pub use inner::{ProfileHub, ReportKind};
