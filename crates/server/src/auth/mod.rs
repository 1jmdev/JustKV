mod acl;
mod error;
mod session;
mod state;

use std::collections::BTreeMap;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::config::Config;

use self::state::{AuthState, User, DEFAULT_USER};

pub use self::error::{no_auth, AuthError};
pub use self::session::SessionAuth;

#[derive(Clone, Debug)]
pub struct UserDirectiveConfig {
    pub name: String,
    pub rules: Vec<String>,
}

#[derive(Clone)]
pub struct AuthService {
    pub(self) inner: Arc<RwLock<AuthState>>,
}

impl AuthService {
    pub fn from_config(config: &Config) -> Result<Self, String> {
        let _trace = profiler::scope("server::auth::from_config");
        let mut users = BTreeMap::new();
        users.insert(DEFAULT_USER.to_string(), User::default());

        if let Some(requirepass) = &config.requirepass {
            let default = users
                .get_mut(DEFAULT_USER)
                .expect("default user is always present");
            default.enabled = true;
            default.nopass = false;
            default.passwords.clear();
            default.passwords.push(requirepass.as_bytes().to_vec());
        }

        let mut state = AuthState { users };
        for user_directive in &config.user_directives {
            state.set_user(&user_directive.name, &user_directive.rules)?;
        }

        Ok(Self {
            inner: Arc::new(RwLock::new(state)),
        })
    }

    pub fn new_session(&self) -> SessionAuth {
        let _trace = profiler::scope("server::auth::new_session");
        let state = self.inner.read();
        if state.default_user_auto_auth() {
            SessionAuth::auto_authorized()
        } else {
            SessionAuth::unauthenticated()
        }
    }

    pub fn is_authorized(&self, session: &SessionAuth) -> bool {
        let _trace = profiler::scope("server::auth::is_authorized");
        session.is_authorized()
    }

    pub fn authenticate(&self, username: &[u8], password: &[u8]) -> Result<String, AuthError> {
        let _trace = profiler::scope("server::auth::authenticate");
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
        if user.nopass {
            return Ok(username);
        }
        if user.passwords.is_empty() {
            return Err(AuthError::NoPasswordSet);
        }
        if user.passwords.iter().any(|candidate| candidate == password) {
            Ok(username)
        } else {
            Err(AuthError::WrongPass)
        }
    }

    pub fn acl_command(
        &self,
        session: &SessionAuth,
        args: &[types::value::CompactArg],
    ) -> protocol::types::RespFrame {
        let _trace = profiler::scope("server::auth::acl_command");
        acl::handle_acl_command(&self.inner, session, args)
    }

    pub fn default_user_has_password(&self) -> bool {
        let _trace = profiler::scope("server::auth::default_user_has_password");
        let state = self.inner.read();
        state.default_user_has_password()
    }
}

pub fn parse_user_directive(values: &[String]) -> Result<UserDirectiveConfig, String> {
    let _trace = profiler::scope("server::auth::parse_user_directive");
    if values.is_empty() {
        return Err("directive 'user' requires at least a username".to_string());
    }

    Ok(UserDirectiveConfig {
        name: values[0].to_ascii_lowercase(),
        rules: values[1..].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirepass_enforces_auth() {
        let _trace = profiler::scope("server::auth::tests::requirepass_enforces_auth");
        let config = Config {
            requirepass: Some("secret".to_string()),
            ..Config::default()
        };
        let auth = AuthService::from_config(&config).expect("auth service");
        let session = auth.new_session();
        assert!(!auth.is_authorized(&session));
        assert!(matches!(
            auth.authenticate(b"default", b"secret"),
            Ok(user) if user == "default"
        ));
    }

    #[test]
    fn user_directive_supports_custom_user() {
        let _trace = profiler::scope("server::auth::tests::user_directive_supports_custom_user");
        let config = Config {
            user_directives: vec![UserDirectiveConfig {
                name: "alice".to_string(),
                rules: vec!["on".to_string(), ">wonderland".to_string()],
            }],
            ..Config::default()
        };
        let auth = AuthService::from_config(&config).expect("auth service");
        assert!(matches!(
            auth.authenticate(b"alice", b"wonderland"),
            Ok(user) if user == "alice"
        ));
        assert!(matches!(
            auth.authenticate(b"alice", b"wrong"),
            Err(AuthError::WrongPass)
        ));
    }

    #[test]
    fn default_config_auto_authorizes_default_user() {
        let _trace =
            profiler::scope("server::auth::tests::default_config_auto_authorizes_default_user");
        let auth = AuthService::from_config(&Config::default()).expect("auth service");
        let session = auth.new_session();
        assert!(session.is_authorized());
        assert_eq!(session.user(), Some("default"));
    }
}
