use commands::dispatch::CommandId;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use parking_lot::RwLock;
use protocol::types::RespFrame;
use types::value::CompactArg;

use crate::config::Config;

use super::acl;
use super::error::{AuthError, PermissionError};
use super::password::password_hash;
use super::session::SessionAuth;
use super::state::{AuthState, DEFAULT_USER};

pub struct AuthenticatedUser {
    pub username: String,
    pub acl_check_required: bool,
    pub acl_epoch: u64,
}

#[derive(Clone)]
pub struct AuthService {
    pub(super) inner: Arc<RwLock<AuthState>>,
    pub(super) acl_epoch: Arc<AtomicU64>,
    pub(super) fast_path: bool,
}

impl AuthService {
    pub fn from_config(config: &Config) -> Result<Self, String> {
        let mut state = AuthState::new();

        if let Some(requirepass) = &config.requirepass {
            let Some(default) = state.users.get_mut(DEFAULT_USER) else {
                return Err("default user is missing from auth state".to_string());
            };
            default.enabled = true;
            default.nopass = false;
            default.password_hashes.clear();
            default
                .password_hashes
                .push(password_hash(requirepass.as_bytes()));
        }

        for user_directive in &config.user_directives {
            state.set_user(&user_directive.name, &user_directive.rules)?;
        }

        Ok(Self {
            inner: Arc::new(RwLock::new(state)),
            acl_epoch: Arc::new(AtomicU64::new(0)),
            fast_path: config.requirepass.is_none() && config.user_directives.is_empty(),
        })
    }

    #[inline(always)]
    pub fn fast_path(&self) -> bool {
        self.fast_path
    }

    pub fn new_session(&self) -> SessionAuth {
        let state = self.inner.read();
        if state.default_user_auto_auth() {
            let mut session = SessionAuth::auto_authorized();
            if let Some(user) = state.users.get(DEFAULT_USER) {
                session.set_acl_state(user.acl_check_required(), 0);
            }
            session
        } else {
            SessionAuth::unauthenticated()
        }
    }

    pub fn is_authorized(&self, session: &SessionAuth) -> bool {
        session.is_authorized()
    }

    pub fn authenticate(
        &self,
        username: &[u8],
        password: &[u8],
    ) -> Result<AuthenticatedUser, AuthError> {
        let username = std::str::from_utf8(username)
            .map_err(|_| AuthError::WrongPass)?
            .to_ascii_lowercase();

        let state = self.inner.read();
        let Some(user) = state.users.get(&username) else {
            return Err(AuthError::WrongPass);
        };

        if !user.enabled {
            return Err(AuthError::WrongPass);
        }
        if !user.nopass && user.password_hashes.is_empty() {
            return Err(AuthError::NoPasswordSet);
        }
        if user.check_password(password) {
            Ok(AuthenticatedUser {
                username,
                acl_check_required: user.acl_check_required(),
                acl_epoch: self.acl_epoch.load(Ordering::Relaxed),
            })
        } else {
            Err(AuthError::WrongPass)
        }
    }

    pub fn acl_command(&self, session: &SessionAuth, args: &[CompactArg]) -> RespFrame {
        acl::handle_acl_command(&self.inner, &self.acl_epoch, session, args)
    }

    pub fn default_user_has_password(&self) -> bool {
        self.inner.read().default_user_has_password()
    }

    pub fn has_passwordless_user(&self) -> bool {
        self.inner.read().has_passwordless_user()
    }

    #[inline(always)]
    pub fn acl_epoch(&self) -> u64 {
        self.acl_epoch.load(Ordering::Relaxed)
    }

    pub fn refresh_session(&self, session: &mut SessionAuth, acl_epoch: u64) -> bool {
        let Some(username) = session.user() else {
            session.revoke();
            return false;
        };

        let state = self.inner.read();
        let Some(user) = state.users.get(username) else {
            session.revoke();
            return false;
        };
        if !user.enabled {
            session.revoke();
            return false;
        }
        session.set_acl_state(user.acl_check_required(), acl_epoch);
        true
    }

    pub fn dry_run(
        &self,
        username: &str,
        command: CommandId,
        args: &[CompactArg],
    ) -> Result<(), PermissionError> {
        let state = self.inner.read();
        let Some(user) = state.users.get(username) else {
            return Err(PermissionError::Command("ACL DRYRUN".to_string()));
        };
        if args.is_empty() {
            return Ok(());
        }
        user.check_permissions(command, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::UserDirectiveConfig;

    fn arg(value: &str) -> CompactArg {
        CompactArg::from_vec(value.as_bytes().to_vec())
    }

    #[test]
    fn requirepass_enforces_auth() {
        let config = Config {
            requirepass: Some("secret".to_string()),
            ..Config::default()
        };
        let auth = AuthService::from_config(&config).expect("auth service");
        let session = auth.new_session();
        assert!(!auth.is_authorized(&session));
        assert!(matches!(
            auth.authenticate(b"default", b"secret"),
            Ok(user) if user.username == "default"
        ));
    }

    #[test]
    fn default_config_auto_authorizes_default_user() {
        let auth = AuthService::from_config(&Config::default()).expect("auth service");
        let session = auth.new_session();
        assert!(session.is_authorized());
        assert_eq!(session.user(), Some("default"));
        assert!(auth.has_passwordless_user());
    }

    #[test]
    fn requirepass_disables_passwordless_access() {
        let config = Config {
            requirepass: Some("secret".to_string()),
            ..Config::default()
        };
        let auth = AuthService::from_config(&config).expect("auth service");

        assert!(!auth.has_passwordless_user());
    }

    #[test]
    fn passwordless_acl_user_is_detected() {
        let config = Config {
            user_directives: vec![
                UserDirectiveConfig {
                    name: "default".to_string(),
                    rules: vec!["reset".to_string()],
                },
                UserDirectiveConfig {
                    name: "guest".to_string(),
                    rules: vec![
                        "on".to_string(),
                        "nopass".to_string(),
                        "+@all".to_string(),
                        "allkeys".to_string(),
                        "allchannels".to_string(),
                    ],
                },
            ],
            ..Config::default()
        };
        let auth = AuthService::from_config(&config).expect("auth service");

        assert!(auth.has_passwordless_user());
    }

    #[test]
    fn acl_dry_run_checks_command_and_key_access() {
        let config = Config {
            user_directives: vec![UserDirectiveConfig {
                name: "alice".to_string(),
                rules: vec![
                    "on".to_string(),
                    ">wonderland".to_string(),
                    "+GET".to_string(),
                    "~cache:*".to_string(),
                ],
            }],
            ..Config::default()
        };
        let auth = AuthService::from_config(&config).expect("auth service");

        assert!(
            auth.dry_run("alice", CommandId::Get, &[arg("GET"), arg("cache:1")])
                .is_ok()
        );
        assert!(matches!(
            auth.dry_run(
                "alice",
                CommandId::Set,
                &[arg("SET"), arg("cache:1"), arg("v")]
            ),
            Err(PermissionError::Command(_))
        ));
        assert_eq!(
            auth.dry_run("alice", CommandId::Get, &[arg("GET"), arg("other:1")]),
            Err(PermissionError::Key)
        );
    }
}
