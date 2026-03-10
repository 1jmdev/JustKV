use super::identify::identify;
use super::list::CommandId;
use super::registry::with_command_registry;
use crate::{
    command, connection, geo, hash, json, keyspace, list, object, scripting, set, stream, string,
    ttl, zset,
};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};
use types::value::CompactArg;

pub fn dispatch(store: &Store, frame: RespFrame) -> RespFrame {
    let _trace = profiler::scope("commands::dispatcher::dispatch");
    let mut args = Vec::new();
    if let Err(err) = parse_command_into(frame, &mut args) {
        return RespFrame::error_static(err);
    }

    dispatch_args(store, &args)
}

#[inline]
pub fn dispatch_args(store: &Store, args: &[CompactArg]) -> RespFrame {
    let _trace = profiler::scope("commands::dispatcher::dispatch_args");
    if args.is_empty() {
        return RespFrame::error_static("ERR empty command");
    }

    dispatch_with_id(store, identify(args[0].as_slice()), args)
}

macro_rules! dispatch_call {
    ([unsupported], $store:ident, $args:ident) => {
        RespFrame::error_static("ERR unknown command")
    };
    ([$handler:path; store $(; $extra:expr)*], $store:ident, $args:ident) => {
        $handler($store, $args $(, $extra)*)
    };
    ([$handler:path; args $(; $extra:expr)*], $store:ident, $args:ident) => {
        $handler($args $(, $extra)*)
    };
}

macro_rules! generate_dispatch {
    (
        $(
            $len:literal => {
                $(
                    $first:expr => {
                        $(
                            {
                                variant: $variant:ident,
                                bytes: $bytes:expr,
                                dispatch: [ $($dispatch:tt)* ],
                                supported: $supported:tt,
                                group: $group:literal,
                                shape: ($arity:expr, $first_key:expr, $last_key:expr, $step:expr),
                                readonly: $readonly:tt,
                                write: $write:tt,
                                auth: $auth_kind:ident $( {
                                    categories: $categories:expr,
                                    keys: $keys:expr,
                                    channels: $channels:expr,
                                } )?,
                                notify: $notify_kind:ident $( {
                                    event: $event:expr,
                                    class: $class:expr,
                                    keys: $notify_keys:expr,
                                    response: $response:expr,
                                } )?,
                            }
                        )*
                    }
                )*
            }
        )*
    ) => {
        #[inline]
        pub fn dispatch_with_id(store: &Store, command: CommandId, args: &[CompactArg]) -> RespFrame {
            match command {
                $( $( $( CommandId::$variant => dispatch_call!([$($dispatch)*], store, args), )* )* )*
                CommandId::Unknown => RespFrame::error_static("ERR unknown command"),
            }
        }
    };
}

with_command_registry!(generate_dispatch);

pub fn parse_command(frame: RespFrame) -> Result<Vec<CompactArg>, &'static str> {
    let _trace = profiler::scope("commands::dispatcher::parse_command");
    let mut args = Vec::new();
    parse_command_into(frame, &mut args)?;
    Ok(args)
}

pub fn parse_command_into(
    frame: RespFrame,
    args: &mut Vec<CompactArg>,
) -> Result<(), &'static str> {
    let _trace = profiler::scope("commands::dispatcher::parse_command_into");
    let RespFrame::Array(Some(items)) = frame else {
        return Err("ERR protocol error");
    };

    args.clear();
    if args.capacity() < items.len() {
        args.reserve(items.len() - args.capacity());
    }

    let mut items = items.into_iter();
    if let Some(first_item) = items.next() {
        let mut first = parse_arg(first_item)?;
        first.make_ascii_uppercase();
        args.push(first);

        for item in items {
            args.push(parse_arg(item)?);
        }
    }

    Ok(())
}

#[inline]
fn parse_arg(item: RespFrame) -> Result<CompactArg, &'static str> {
    match item {
        RespFrame::Bulk(Some(BulkData::Arg(bytes))) => Ok(bytes),
        RespFrame::Bulk(Some(BulkData::Value(bytes))) => Ok(CompactArg::from_vec(bytes.into_vec())),
        RespFrame::Simple(value) => Ok(CompactArg::from_vec(value.into_bytes())),
        RespFrame::SimpleStatic(value) => Ok(CompactArg::from_slice(value.as_bytes())),
        _ => Err("ERR invalid argument type"),
    }
}
