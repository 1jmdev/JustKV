use tokio::sync::mpsc::UnboundedSender;

use commands::dispatcher::dispatch_args;
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};
use types::value::CompactArg;

use super::super::pubsub::{ConnectionPubSub, PubSubHub};
use super::notifications::emit_command_notifications;
use super::util::{collapse_pubsub_responses, wrong_args};
use crate::auth::{self, AuthError, AuthService, SessionAuth};
use crate::profile::ProfileHub;

#[inline]
fn bulk_static(value: &'static [u8]) -> RespFrame {
    RespFrame::Bulk(Some(BulkData::Arg(CompactArg::from_slice(value))))
}

pub(super) fn execute_regular_command(
    store: &Store,
    hub: &PubSubHub,
    push_tx: &UnboundedSender<RespFrame>,
    pubsub_state: &mut ConnectionPubSub,
    auth: &AuthService,
    auth_state: &mut SessionAuth,
    profiler: &ProfileHub,
    args: &[CompactArg],
) -> RespFrame {
    let _trace = profiler::scope("server::connection::dispatch::execute_regular_command");
    if args.is_empty() {
        return RespFrame::error_static("ERR empty command");
    }

    let command = args[0].as_slice();

    if command == b"AUTH" {
        return auth_command(auth, auth_state, args);
    }

    if command == b"ACL" {
        return auth.acl_command(auth_state, args);
    }

    if command == b"HELLO" {
        if let Some(response) = hello_with_auth(auth, auth_state, args) {
            return response;
        }
    }

    if !auth.is_authorized(auth_state) && !is_allowed_without_auth(command) {
        return auth::no_auth();
    }

    if let Some(response) =
        handle_pubsub_or_config_command(hub, push_tx, pubsub_state, command, args)
    {
        return response;
    }

    let key = args.get(1).map(CompactArg::as_slice);
    profiler.run_command(key, || {
        let response = dispatch_args(store, args);
        if hub.keyspace_notifications_enabled() {
            emit_command_notifications(hub, command, args, &response);
        }
        response
    })
}

fn auth_command(
    auth: &AuthService,
    auth_state: &mut SessionAuth,
    args: &[CompactArg],
) -> RespFrame {
    let _trace = profiler::scope("server::connection::dispatch::auth_command");
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
    let _trace = profiler::scope("server::connection::dispatch::authenticate_with");
    match auth.authenticate(username, password) {
        Ok(user) => {
            auth_state.set_user(user);
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
    let _trace = profiler::scope("server::connection::dispatch::hello_with_auth");
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

fn is_allowed_without_auth(command: &[u8]) -> bool {
    let _trace = profiler::scope("server::connection::dispatch::is_allowed_without_auth");
    matches!(command, b"AUTH" | b"HELLO" | b"QUIT")
}

fn response_is_ok(response: &RespFrame) -> bool {
    let _trace = profiler::scope("server::connection::dispatch::response_is_ok");
    match response {
        RespFrame::SimpleStatic(value) => *value == "OK",
        RespFrame::Simple(value) => value == "OK",
        _ => false,
    }
}

fn handle_pubsub_or_config_command(
    hub: &PubSubHub,
    push_tx: &UnboundedSender<RespFrame>,
    pubsub_state: &mut ConnectionPubSub,
    command: &[u8],
    args: &[CompactArg],
) -> Option<RespFrame> {
    let _trace = profiler::scope("server::connection::dispatch::handle_pubsub_or_config_command");

    // parse_command_into uppercases args[0], so exact-byte matches are enough here.
    match command.first().copied() {
        Some(b'P') => {
            if command == b"PUBLISH" {
                return Some(publish_command(hub, args));
            }
            if command == b"PSUBSCRIBE" {
                return Some(psubscribe_command(hub, push_tx, pubsub_state, args));
            }
            if command == b"PUNSUBSCRIBE" {
                return Some(punsubscribe_command(hub, pubsub_state, args));
            }
            if command == b"PUBSUB" {
                return Some(pubsub_command(hub, args));
            }
            None
        }
        Some(b'S') if command == b"SUBSCRIBE" => {
            Some(subscribe_command(hub, push_tx, pubsub_state, args))
        }
        Some(b'U') if command == b"UNSUBSCRIBE" => {
            Some(unsubscribe_command(hub, pubsub_state, args))
        }
        Some(b'C') if command == b"CONFIG" => Some(config_command(hub, args)),
        _ => None,
    }
}

fn publish_command(hub: &PubSubHub, args: &[CompactArg]) -> RespFrame {
    let _trace = profiler::scope("server::connection::dispatch::publish_command");
    if args.len() != 3 {
        return wrong_args("PUBLISH");
    }
    RespFrame::Integer(hub.publish(&args[1], &args[2]))
}

fn subscribe_command(
    hub: &PubSubHub,
    push_tx: &UnboundedSender<RespFrame>,
    pubsub_state: &mut ConnectionPubSub,
    args: &[CompactArg],
) -> RespFrame {
    let _trace = profiler::scope("server::connection::dispatch::subscribe_command");
    if args.len() < 2 {
        return wrong_args("SUBSCRIBE");
    }

    let mut responses = Vec::with_capacity(args.len() - 1);
    for channel in &args[1..] {
        pubsub_state.subscribe(hub, channel, push_tx);
        responses.push(RespFrame::Array(Some(vec![
            bulk_static(b"subscribe"),
            RespFrame::Bulk(Some(BulkData::Arg(channel.clone()))),
            RespFrame::Integer(pubsub_state.subscription_count()),
        ])));
    }
    collapse_pubsub_responses(responses)
}

fn unsubscribe_command(
    hub: &PubSubHub,
    pubsub_state: &mut ConnectionPubSub,
    args: &[CompactArg],
) -> RespFrame {
    let _trace = profiler::scope("server::connection::dispatch::unsubscribe_command");
    let channels = if args.len() == 1 {
        let existing = pubsub_state.unsubscribe_all(hub);
        if existing.is_empty() {
            vec![CompactArg::from_vec(Vec::new())]
        } else {
            existing.into_iter().map(CompactArg::from_vec).collect()
        }
    } else {
        let mut out = Vec::with_capacity(args.len() - 1);
        for channel in &args[1..] {
            let _ = pubsub_state.unsubscribe(hub, channel);
            out.push(channel.clone());
        }
        out
    };

    let mut responses = Vec::with_capacity(channels.len());
    for channel in channels {
        responses.push(RespFrame::Array(Some(vec![
            bulk_static(b"unsubscribe"),
            if channel.is_empty() {
                RespFrame::Bulk(None)
            } else {
                RespFrame::Bulk(Some(BulkData::Arg(channel)))
            },
            RespFrame::Integer(pubsub_state.subscription_count()),
        ])));
    }
    collapse_pubsub_responses(responses)
}

fn psubscribe_command(
    hub: &PubSubHub,
    push_tx: &UnboundedSender<RespFrame>,
    pubsub_state: &mut ConnectionPubSub,
    args: &[CompactArg],
) -> RespFrame {
    let _trace = profiler::scope("server::connection::dispatch::psubscribe_command");
    if args.len() < 2 {
        return wrong_args("PSUBSCRIBE");
    }

    let mut responses = Vec::with_capacity(args.len() - 1);
    for pattern in &args[1..] {
        pubsub_state.psubscribe(hub, pattern, push_tx);
        responses.push(RespFrame::Array(Some(vec![
            bulk_static(b"psubscribe"),
            RespFrame::Bulk(Some(BulkData::Arg(pattern.clone()))),
            RespFrame::Integer(pubsub_state.subscription_count()),
        ])));
    }
    collapse_pubsub_responses(responses)
}

fn punsubscribe_command(
    hub: &PubSubHub,
    pubsub_state: &mut ConnectionPubSub,
    args: &[CompactArg],
) -> RespFrame {
    let _trace = profiler::scope("server::connection::dispatch::punsubscribe_command");
    let patterns = if args.len() == 1 {
        let existing = pubsub_state.punsubscribe_all(hub);
        if existing.is_empty() {
            vec![CompactArg::from_vec(Vec::new())]
        } else {
            existing.into_iter().map(CompactArg::from_vec).collect()
        }
    } else {
        let mut out = Vec::with_capacity(args.len() - 1);
        for pattern in &args[1..] {
            let _ = pubsub_state.punsubscribe(hub, pattern);
            out.push(pattern.clone());
        }
        out
    };

    let mut responses = Vec::with_capacity(patterns.len());
    for pattern in patterns {
        responses.push(RespFrame::Array(Some(vec![
            bulk_static(b"punsubscribe"),
            if pattern.is_empty() {
                RespFrame::Bulk(None)
            } else {
                RespFrame::Bulk(Some(BulkData::Arg(pattern)))
            },
            RespFrame::Integer(pubsub_state.subscription_count()),
        ])));
    }
    collapse_pubsub_responses(responses)
}

fn pubsub_command(hub: &PubSubHub, args: &[CompactArg]) -> RespFrame {
    let _trace = profiler::scope("server::connection::dispatch::pubsub_command");
    if args.len() < 2 {
        return wrong_args("PUBSUB");
    }

    let subcommand = args[1].as_slice();
    if subcommand.eq_ignore_ascii_case(b"CHANNELS") {
        let pattern = if args.len() == 3 {
            Some(args[2].as_slice())
        } else if args.len() == 2 {
            None
        } else {
            return wrong_args("PUBSUB");
        };
        let channels = hub.pubsub_channels(pattern);
        return RespFrame::Array(Some(
            channels
                .into_iter()
                .map(|channel| RespFrame::Bulk(Some(BulkData::from_vec(channel))))
                .collect(),
        ));
    }

    if subcommand.eq_ignore_ascii_case(b"NUMSUB") {
        let channels = args[2..]
            .iter()
            .map(|channel| channel.to_vec())
            .collect::<Vec<_>>();
        let counts = hub.pubsub_numsub(&channels);
        let mut response = Vec::with_capacity(counts.len() * 2);
        for (channel, count) in counts {
            response.push(RespFrame::Bulk(Some(BulkData::from_vec(channel))));
            response.push(RespFrame::Integer(count));
        }
        return RespFrame::Array(Some(response));
    }

    if subcommand.eq_ignore_ascii_case(b"NUMPAT") {
        if args.len() != 2 {
            return wrong_args("PUBSUB");
        }
        return RespFrame::Integer(hub.pubsub_numpat());
    }

    RespFrame::Error("ERR Unknown PUBSUB subcommand".to_string())
}

fn config_command(hub: &PubSubHub, args: &[CompactArg]) -> RespFrame {
    let _trace = profiler::scope("server::connection::dispatch::config_command");
    if args.len() < 2 {
        return wrong_args("CONFIG");
    }

    let subcommand = args[1].as_slice();
    if subcommand.eq_ignore_ascii_case(b"GET") {
        if args.len() != 3 {
            return wrong_args("CONFIG");
        }
        if !args[2]
            .as_slice()
            .eq_ignore_ascii_case(b"notify-keyspace-events")
        {
            return RespFrame::Array(Some(vec![]));
        }
        return RespFrame::Array(Some(vec![
            bulk_static(b"notify-keyspace-events"),
            RespFrame::Bulk(Some(BulkData::from_vec(hub.get_notify_flags()))),
        ]));
    }

    if subcommand.eq_ignore_ascii_case(b"SET") {
        if args.len() != 4 {
            return wrong_args("CONFIG");
        }
        if !args[2]
            .as_slice()
            .eq_ignore_ascii_case(b"notify-keyspace-events")
        {
            return RespFrame::error_static("ERR Unsupported CONFIG parameter");
        }
        return match hub.set_notify_flags(&args[3]) {
            Ok(()) => RespFrame::ok(),
            Err(()) => {
                RespFrame::error_static("ERR CONFIG SET failed (possibly related to argument)")
            }
        };
    }

    RespFrame::error_static("ERR Unknown subcommand or wrong number of arguments for CONFIG")
}
