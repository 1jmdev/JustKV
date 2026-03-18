use std::collections::BTreeMap;

use super::command::AclCategory;
use super::user::User;

pub(super) const DEFAULT_USER: &str = "default";

#[derive(Clone, Debug)]
pub(super) struct AuthState {
    pub(super) users: BTreeMap<String, User>,
}

impl AuthState {
    pub(super) fn new() -> Self {
        let mut users = BTreeMap::new();
        users.insert(DEFAULT_USER.to_string(), User::default());
        Self { users }
    }

    pub(super) fn default_user_has_password(&self) -> bool {
        self.users
            .get(DEFAULT_USER)
            .is_some_and(|user| !user.nopass && !user.password_hashes.is_empty())
    }

    pub(super) fn default_user_auto_auth(&self) -> bool {
        self.users
            .get(DEFAULT_USER)
            .is_some_and(|user| user.enabled && user.nopass)
    }

    pub(super) fn has_passwordless_user(&self) -> bool {
        self.users.values().any(|user| user.enabled && user.nopass)
    }

    pub(super) fn set_user(&mut self, username: &str, rules: &[String]) -> Result<(), String> {
        let user = self
            .users
            .entry(username.to_string())
            .or_insert_with(User::new_restricted);

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
                user.password_hashes.clear();
                continue;
            }
            if rule.eq_ignore_ascii_case("resetpass") {
                user.nopass = false;
                user.password_hashes.clear();
                continue;
            }
            if rule.eq_ignore_ascii_case("resetkeys") {
                user.key_patterns.clear();
                continue;
            }
            if rule.eq_ignore_ascii_case("allkeys") {
                user.key_patterns.clear();
                user.key_patterns.push(b"*".to_vec());
                continue;
            }
            if rule.eq_ignore_ascii_case("resetchannels") {
                user.channel_patterns.clear();
                continue;
            }
            if rule.eq_ignore_ascii_case("allchannels") {
                user.channel_patterns.clear();
                user.channel_patterns.push(b"*".to_vec());
                continue;
            }
            if rule.eq_ignore_ascii_case("allcommands") {
                user.command_rules.set_allow_all(true);
                continue;
            }
            if rule.eq_ignore_ascii_case("nocommands") {
                user.command_rules.set_allow_all(false);
                user.command_rules.reset();
                continue;
            }
            if rule.eq_ignore_ascii_case("reset") {
                user.reset();
                continue;
            }
            if let Some(password) = rule.strip_prefix('>') {
                user.add_password(password.as_bytes());
                continue;
            }
            if let Some(password) = rule.strip_prefix('<') {
                user.remove_password(password.as_bytes());
                continue;
            }
            if let Some(hash) = rule.strip_prefix('#') {
                user.add_password_hash(hash)?;
                continue;
            }
            if let Some(hash) = rule.strip_prefix('!') {
                user.remove_password_hash(hash)?;
                continue;
            }
            if let Some(pattern) = rule.strip_prefix('~') {
                user.key_patterns.push(pattern.as_bytes().to_vec());
                continue;
            }
            if let Some(pattern) = rule.strip_prefix('&') {
                user.channel_patterns.push(pattern.as_bytes().to_vec());
                continue;
            }
            if let Some(category) = rule.strip_prefix("+@") {
                let category = AclCategory::parse(category)?;
                if category == AclCategory::All {
                    user.command_rules.set_allow_all(true);
                } else {
                    user.command_rules.allow_category(category);
                }
                continue;
            }
            if let Some(category) = rule.strip_prefix("-@") {
                let category = AclCategory::parse(category)?;
                if category == AclCategory::All {
                    user.command_rules.set_allow_all(false);
                    user.command_rules.reset();
                } else {
                    user.command_rules.deny_category(category);
                }
                continue;
            }
            if let Some(command) = rule.strip_prefix('+') {
                user.command_rules.allow_command(command);
                continue;
            }
            if let Some(command) = rule.strip_prefix('-') {
                user.command_rules.deny_command(command);
                continue;
            }

            return Err(format!("ERR Unsupported ACL rule '{rule}'"));
        }

        Ok(())
    }
}
