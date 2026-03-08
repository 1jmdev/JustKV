use std::collections::BTreeMap;

pub(super) const DEFAULT_USER: &str = "default";

#[derive(Clone, Debug)]
pub(super) struct AuthState {
    pub(super) users: BTreeMap<String, User>,
}

#[derive(Clone, Debug)]
pub(super) struct User {
    pub(super) enabled: bool,
    pub(super) nopass: bool,
    pub(super) passwords: Vec<Vec<u8>>,
}

impl Default for User {
    fn default() -> Self {
        let _trace = profiler::scope("server::auth::default_user");
        Self {
            enabled: true,
            nopass: true,
            passwords: Vec::new(),
        }
    }
}

impl AuthState {
    pub(super) fn default_user_has_password(&self) -> bool {
        self.users
            .get(DEFAULT_USER)
            .is_some_and(|default_user| !default_user.nopass && !default_user.passwords.is_empty())
    }

    pub(super) fn default_user_auto_auth(&self) -> bool {
        self.users
            .get(DEFAULT_USER)
            .is_some_and(|user| user.enabled && user.nopass)
    }

    pub(super) fn set_user(&mut self, username: &str, rules: &[String]) -> Result<(), String> {
        let _trace = profiler::scope("server::auth::set_user");
        let user = self.users.entry(username.to_string()).or_insert(User {
            enabled: false,
            nopass: false,
            passwords: Vec::new(),
        });
        for rule in rules {
            if rule.eq_ignore_ascii_case("on") {
                user.enabled = true;
                continue;
            }
            if rule.eq_ignore_ascii_case("off") {
                user.enabled = false;
                continue;
            }
            if rule.eq_ignore_ascii_case("nopass") {
                user.nopass = true;
                user.passwords.clear();
                continue;
            }
            if rule.eq_ignore_ascii_case("resetpass") {
                user.nopass = false;
                user.passwords.clear();
                continue;
            }
            if rule.eq_ignore_ascii_case("reset") {
                *user = User {
                    enabled: false,
                    nopass: false,
                    passwords: Vec::new(),
                };
                continue;
            }
            if let Some(password) = rule.strip_prefix('>') {
                user.nopass = false;
                user.passwords.push(password.as_bytes().to_vec());
                continue;
            }
            if let Some(password) = rule.strip_prefix('<') {
                user.passwords.retain(|item| item != password.as_bytes());
                continue;
            }

            if is_ignored_acl_rule(rule) {
                continue;
            }
            return Err(format!("ERR Unsupported ACL rule '{rule}'"));
        }
        Ok(())
    }
}

fn is_ignored_acl_rule(rule: &str) -> bool {
    let _trace = profiler::scope("server::auth::is_ignored_acl_rule");
    if rule.starts_with('~') || rule.starts_with('&') {
        return true;
    }
    if rule.starts_with('+') || rule.starts_with('-') {
        return true;
    }
    matches!(
        rule.to_ascii_lowercase().as_str(),
        "allkeys" | "resetkeys" | "allchannels" | "resetchannels" | "allcommands" | "nocommands"
    )
}

pub(super) fn user_acl_line(name: &str, user: &User) -> String {
    let _trace = profiler::scope("server::auth::user_acl_line");
    let mut parts = vec![format!("user {name}")];
    parts.push(if user.enabled {
        "on".to_string()
    } else {
        "off".to_string()
    });

    if user.nopass {
        parts.push("nopass".to_string());
    } else if user.passwords.is_empty() {
        parts.push("resetpass".to_string());
    } else {
        for password in &user.passwords {
            parts.push(format!(">{}", String::from_utf8_lossy(password)));
        }
    }

    parts.push("~*".to_string());
    parts.push("+@all".to_string());
    parts.join(" ")
}
