use super::list::CommandId;
use super::registry::with_command_registry;

#[inline(always)]
fn eq(a: &[u8], b: &[u8]) -> bool {
    a == b || a.eq_ignore_ascii_case(b)
}

macro_rules! generate_identify {
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
        #[inline(always)]
        pub fn dispatch_safe(input: &[u8]) -> Option<CommandId> {
            if input.is_empty() {
                return None;
            }

            let f = input[0] | 0x20;

            match input.len() {
                $(
                    $len => match f {
                        $(
                            $first => {
                                $(
                                    if eq(input, $bytes) {
                                        return Some(CommandId::$variant);
                                    }
                                )*
                            }
                        )*
                        _ => {}
                    },
                )*
                _ => {}
            }

            None
        }
    };
}

with_command_registry!(generate_identify);

#[inline(always)]
pub fn identify(command: &[u8]) -> CommandId {
    dispatch_safe(command).unwrap_or(CommandId::Unknown)
}
