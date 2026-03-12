use commands::dispatch::CommandId;
use commands::dispatch::dispatch_with_context;
use commands::pubsub::DispatchContext;
use engine::pubsub::PubSubHub;
use protocol::types::{BulkData, RespFrame};
use types::value::CompactArg;

use super::ConnectionShared;
use super::PubSubSession;
use super::notifications::emit_command_notifications;
use super::util::wrong_args;
use crate::auth::{self, AuthError, AuthService, SessionAuth, no_perm};

#[derive(Default)]
pub(super) struct ClientState {
    name: Option<CompactArg>,
    suppress_current_reply: bool,
}

impl ClientState {
    pub(super) fn take_suppress_current_reply(&mut self) -> bool {
        std::mem::take(&mut self.suppress_current_reply)
    }
}

pub(super) fn execute_regular_command(
    shared: &ConnectionShared,
    pubsub: &mut PubSubSession,
    client_state: &mut ClientState,
    auth_state: &mut SessionAuth,
    command: CommandId,
    args: &[CompactArg],
) -> RespFrame {
    if args.is_empty() {
        return RespFrame::error_static("ERR empty command");
    }

    if command == CommandId::Auth {
        return auth_command(&shared.auth, auth_state, args);
    }

    if command == CommandId::Acl {
        if !auth_state.authorized() {
            return auth::no_auth();
        }
        return shared.auth.acl_command(auth_state, args);
    }

    if command == CommandId::Hello
        && let Some(response) = hello_with_auth(&shared.auth, auth_state, args)
    {
        return response;
    }

    if shared.auth.fast_path() {
        return dispatch_authorized(shared, pubsub, client_state, command, args);
    }

    // Hot path: already authorized
    if auth_state.authorized() {
        let acl_epoch = shared.auth.acl_epoch();
        if auth_state.acl_epoch() != acl_epoch
            && !shared.auth.refresh_session(auth_state, acl_epoch)
        {
            return auth::no_auth();
        }
        if auth_state.acl_check_required()
            && let Err(error) =
                shared
                    .auth
                    .dry_run(auth_state.user().unwrap_or_default(), command, args)
        {
            return no_perm(error);
        }
        return dispatch_authorized(shared, pubsub, client_state, command, args);
    }

    if !is_allowed_without_auth(command) {
        return auth::no_auth();
    }

    dispatch_authorized(shared, pubsub, client_state, command, args)
}

#[inline]
fn dispatch_authorized(
    shared: &ConnectionShared,
    pubsub: &mut PubSubSession,
    client_state: &mut ClientState,
    command: CommandId,
    args: &[CompactArg],
) -> RespFrame {
    if command == CommandId::Client {
        return client_command(client_state, args);
    }

    let mut context = ServerDispatchContext {
        hub: &shared.pubsub_hub,
        pubsub,
    };
    let response = dispatch_with_context(&shared.store, &mut context, command, args);
    if shared.pubsub_hub.keyspace_notifications_enabled() {
        emit_command_notifications(&shared.pubsub_hub, command, args, &response);
    }
    response
}

fn client_command(client_state: &mut ClientState, args: &[CompactArg]) -> RespFrame {
    if args.len() < 2 {
        return RespFrame::error_static("ERR wrong number of arguments for 'client' command");
    }

    let sub = args[1].as_slice();
    if sub.eq_ignore_ascii_case(b"SETNAME") {
        if args.len() != 3 {
            return wrong_args("client|setname");
        }
        client_state.name = Some(args[2].clone());
        return RespFrame::ok();
    }
    if sub.eq_ignore_ascii_case(b"GETNAME") {
        if args.len() != 2 {
            return wrong_args("client|getname");
        }
        return RespFrame::Bulk(client_state.name.clone().map(BulkData::Arg));
    }
    if sub.eq_ignore_ascii_case(b"SETINFO") {
        return RespFrame::ok();
    }
    if sub.eq_ignore_ascii_case(b"ID") {
        if args.len() != 2 {
            return wrong_args("client|id");
        }
        return RespFrame::Integer(1);
    }
    if sub.eq_ignore_ascii_case(b"LIST") || sub.eq_ignore_ascii_case(b"INFO") {
        return RespFrame::Bulk(Some(BulkData::from_vec(Vec::new())));
    }
    if sub.eq_ignore_ascii_case(b"PAUSE") {
        if args.len() != 3 {
            return RespFrame::error_static("ERR wrong number of arguments for 'client' command");
        }
        return RespFrame::ok();
    }
    if sub.eq_ignore_ascii_case(b"UNPAUSE") {
        if args.len() != 2 {
            return RespFrame::error_static("ERR wrong number of arguments for 'client' command");
        }
        return RespFrame::ok();
    }
    if sub.eq_ignore_ascii_case(b"TRACKING") {
        if args.len() < 3 {
            return RespFrame::error_static("ERR wrong number of arguments for 'client' command");
        }
        return RespFrame::ok();
    }
    if sub.eq_ignore_ascii_case(b"TRACKINGINFO") {
        return RespFrame::Array(Some(vec![]));
    }
    if sub.eq_ignore_ascii_case(b"REPLY") {
        if args.len() != 3 {
            return RespFrame::error_static("ERR wrong number of arguments for 'client' command");
        }
        if args[2].eq_ignore_ascii_case(b"ON") {
            return RespFrame::ok();
        }
        if args[2].eq_ignore_ascii_case(b"OFF") || args[2].eq_ignore_ascii_case(b"SKIP") {
            client_state.suppress_current_reply = true;
            return RespFrame::ok();
        }
        return RespFrame::error_static("ERR syntax error");
    }

    unknown_subcommand_error(sub)
}

fn auth_command(
    auth: &AuthService,
    auth_state: &mut SessionAuth,
    args: &[CompactArg],
) -> RespFrame {
    if args.len() == 2 {
        if !auth.default_user_has_password() {
            return RespFrame::error_static(
                "ERR AUTH <password> called without any password configured for the default user. Are you sure your configuration is correct?",
            );
        }
        return authenticate_with(auth, auth_state, b"default", args[1].as_slice());
    }
    if args.len() == 3 {
        return authenticate_with(auth, auth_state, args[1].as_slice(), args[2].as_slice());
    }
    wrong_args("AUTH")
}

fn authenticate_with(
    auth: &AuthService,
    auth_state: &mut SessionAuth,
    username: &[u8],
    password: &[u8],
) -> RespFrame {
    match auth.authenticate(username, password) {
        Ok(user) => {
            auth_state.set_user(user.username);
            auth_state.set_acl_state(user.acl_check_required, user.acl_epoch);
            RespFrame::ok()
        }
        Err(AuthError::WrongPass) => {
            RespFrame::error_static("WRONGPASS invalid username-password pair or user is disabled.")
        }
        Err(AuthError::NoPasswordSet) => {
            RespFrame::error_static("ERR AUTH called without any password configured for the user.")
        }
    }
}

fn hello_with_auth(
    auth: &AuthService,
    auth_state: &mut SessionAuth,
    args: &[CompactArg],
) -> Option<RespFrame> {
    if args.len() < 4 {
        return None;
    }

    let mut index = 2;
    while index < args.len() {
        let token = args[index].as_slice();
        if token.eq_ignore_ascii_case(b"AUTH") {
            if index + 2 >= args.len() {
                return Some(RespFrame::error_static(
                    "ERR Syntax error in HELLO option AUTH",
                ));
            }
            let response = authenticate_with(
                auth,
                auth_state,
                args[index + 1].as_slice(),
                args[index + 2].as_slice(),
            );
            if response_is_ok(&response) {
                return None;
            }
            return Some(response);
        }
        index += 1;
    }
    None
}

fn is_allowed_without_auth(command: CommandId) -> bool {
    matches!(
        command,
        CommandId::Auth | CommandId::Hello | CommandId::Quit
    )
}

fn response_is_ok(response: &RespFrame) -> bool {
    match response {
        RespFrame::SimpleStatic(value) => *value == "OK",
        RespFrame::Simple(value) => value == "OK",
        _ => false,
    }
}

fn unknown_subcommand_error(subcommand: &[u8]) -> RespFrame {
    RespFrame::Error(format!(
        "ERR unknown subcommand '{}'.",
        String::from_utf8_lossy(subcommand)
    ))
}

struct ServerDispatchContext<'a> {
    hub: &'a PubSubHub,
    pubsub: &'a mut PubSubSession,
}

impl DispatchContext for ServerDispatchContext<'_> {
    fn publish(&mut self, channel: &[u8], payload: &[u8]) -> i64 {
        self.hub.publish(channel, payload)
    }

    fn spublish(&mut self, channel: &[u8], payload: &[u8]) -> i64 {
        self.hub.spublish(channel, payload)
    }

    fn subscribe(&mut self, channel: &[u8]) -> i64 {
        self.pubsub.subscribe(self.hub, channel)
    }

    fn unsubscribe(&mut self, channel: &[u8]) -> i64 {
        self.pubsub.unsubscribe(self.hub, channel)
    }

    fn unsubscribe_all(&mut self) -> Vec<Vec<u8>> {
        self.pubsub.unsubscribe_all(self.hub)
    }

    fn psubscribe(&mut self, pattern: &[u8]) -> i64 {
        self.pubsub.psubscribe(self.hub, pattern)
    }

    fn punsubscribe(&mut self, pattern: &[u8]) -> i64 {
        self.pubsub.punsubscribe(self.hub, pattern)
    }

    fn punsubscribe_all(&mut self) -> Vec<Vec<u8>> {
        self.pubsub.punsubscribe_all(self.hub)
    }

    fn ssubscribe(&mut self, channel: &[u8]) -> i64 {
        self.pubsub.ssubscribe(self.hub, channel)
    }

    fn sunsubscribe(&mut self, channel: &[u8]) -> i64 {
        self.pubsub.sunsubscribe(self.hub, channel)
    }

    fn sunsubscribe_all(&mut self) -> Vec<Vec<u8>> {
        self.pubsub.sunsubscribe_all(self.hub)
    }

    fn subscription_count(&self) -> i64 {
        self.pubsub.subscription_count()
    }

    fn pubsub_channels(&self, pattern: Option<&[u8]>) -> Vec<Vec<u8>> {
        self.hub.pubsub_channels(pattern)
    }

    fn pubsub_numsub(&self, channels: &[Vec<u8>]) -> Vec<(Vec<u8>, i64)> {
        self.hub.pubsub_numsub(channels)
    }

    fn pubsub_numpat(&self) -> i64 {
        self.hub.pubsub_numpat()
    }

    fn pubsub_shardchannels(&self, pattern: Option<&[u8]>) -> Vec<Vec<u8>> {
        self.hub.pubsub_shardchannels(pattern)
    }

    fn pubsub_shardnumsub(&self, channels: &[Vec<u8>]) -> Vec<(Vec<u8>, i64)> {
        self.hub.pubsub_shardnumsub(channels)
    }

    fn set_notify_flags(&mut self, flags: &[u8]) -> Result<(), ()> {
        self.hub.set_notify_flags(flags)
    }

    fn get_notify_flags(&self) -> Vec<u8> {
        self.hub.get_notify_flags()
    }
}
