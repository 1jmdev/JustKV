use super::state::DEFAULT_USER;

#[derive(Clone, Debug)]
pub struct SessionAuth {
    pub(super) user: Option<String>,
    pub(super) authorized: bool,
}

impl SessionAuth {
    pub(super) fn auto_authorized() -> Self {
        Self {
            user: Some(DEFAULT_USER.to_string()),
            authorized: true,
        }
    }

    pub(super) fn unauthenticated() -> Self {
        Self {
            user: None,
            authorized: false,
        }
    }

    pub fn user(&self) -> Option<&str> {
        let _trace = profiler::scope("server::auth::session_user");
        self.user.as_deref()
    }

    pub fn set_user(&mut self, user: String) {
        let _trace = profiler::scope("server::auth::session_set_user");
        self.user = Some(user);
        self.authorized = true;
    }

    pub fn is_authorized(&self) -> bool {
        let _trace = profiler::scope("server::auth::session_is_authorized");
        self.authorized
    }

    #[inline(always)]
    pub fn authorized(&self) -> bool {
        self.authorized
    }
}
