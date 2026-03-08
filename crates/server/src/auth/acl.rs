use parking_lot::RwLock;
use protocol::types::{BulkData, RespFrame};
use types::value::CompactArg;

use super::error::no_auth;
use super::session::SessionAuth;
use super::state::{user_acl_line, AuthState, DEFAULT_USER};

/// Handles the ACL command and its subcommands. Returns the response frame.
pub(super) fn handle_acl_command(
    inner: &RwLock<AuthState>,
    session: &SessionAuth,
    args: &[CompactArg],
) -> RespFrame {
    let _trace = profiler::scope("server::auth::acl_command");
    if !session.is_authorized() {
        return no_auth();
    }
    if args.len() < 2 {
        return RespFrame::error_static("ERR wrong number of arguments for 'ACL' command");
    }

    let sub = args[1].as_slice();

    if sub.eq_ignore_ascii_case(b"WHOAMI") {
        if args.len() != 2 {
            return RespFrame::error_static(
                "ERR wrong number of arguments for 'ACL WHOAMI' command",
            );
        }
        let current = session
            .user
            .clone()
            .unwrap_or_else(|| DEFAULT_USER.to_string());
        return RespFrame::Bulk(Some(BulkData::from_vec(current.into_bytes())));
    }

    if sub.eq_ignore_ascii_case(b"USERS") {
        if args.len() != 2 {
            return RespFrame::error_static(
                "ERR wrong number of arguments for 'ACL USERS' command",
            );
        }
        let users = inner
            .read()
            .users
            .keys()
            .map(|name| RespFrame::Bulk(Some(BulkData::from_vec(name.as_bytes().to_vec()))))
            .collect();
        return RespFrame::Array(Some(users));
    }

    if sub.eq_ignore_ascii_case(b"LIST") {
        if args.len() != 2 {
            return RespFrame::error_static("ERR wrong number of arguments for 'ACL LIST' command");
        }
        let lines = inner
            .read()
            .users
            .iter()
            .map(|(name, user)| user_acl_line(name, user))
            .map(|line| RespFrame::Bulk(Some(BulkData::from_vec(line.into_bytes()))))
            .collect();
        return RespFrame::Array(Some(lines));
    }

    if sub.eq_ignore_ascii_case(b"SETUSER") {
        if args.len() < 3 {
            return RespFrame::error_static(
                "ERR wrong number of arguments for 'ACL SETUSER' command",
            );
        }
        let username = match std::str::from_utf8(args[2].as_slice()) {
            Ok(name) => name.to_ascii_lowercase(),
            Err(_) => return RespFrame::error_static("ERR invalid ACL username"),
        };
        let rules = args[3..]
            .iter()
            .map(|arg| String::from_utf8_lossy(arg.as_slice()).to_string())
            .collect::<Vec<_>>();

        let mut state = inner.write();
        match state.set_user(&username, &rules) {
            Ok(()) => RespFrame::ok(),
            Err(err) => RespFrame::Error(err),
        }
    } else if sub.eq_ignore_ascii_case(b"DELUSER") {
        if args.len() < 3 {
            return RespFrame::error_static(
                "ERR wrong number of arguments for 'ACL DELUSER' command",
            );
        }
        let mut state = inner.write();
        let mut removed = 0;
        for name in &args[2..] {
            if let Ok(username) = std::str::from_utf8(name.as_slice()) {
                if username.eq_ignore_ascii_case(DEFAULT_USER) {
                    continue;
                }
                if state.users.remove(&username.to_ascii_lowercase()).is_some() {
                    removed += 1;
                }
            }
        }
        RespFrame::Integer(removed)
    } else {
        RespFrame::error_static("ERR Unknown ACL subcommand")
    }
}
