macro_rules! with_command_registry {
    ($callback:ident) => {
        $callback! {
            3 => {
                b'a' => {
                    {
                        variant: Acl,
                        bytes: b"ACL",
                        dispatch: [unsupported],
                        supported: false,
                        group: "",
                        shape: (0, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Admin, AclCategory::Slow],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'd' => {
                    {
                        variant: Del,
                        bytes: b"DEL",
                        dispatch: [keyspace::del; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, -1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Keyspace],
                            keys: KeyExtraction::AllFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"del",
                            class: b'g',
                            keys: NotificationKeyArguments::AllFrom(1),
                            response: NotificationResponsePolicy::PositiveInteger,
                        },
                    }
                }
                b'g' => {
                    {
                        variant: Get,
                        bytes: b"GET",
                        dispatch: [string::get; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'l' => {
                    {
                        variant: Lcs,
                        bytes: b"LCS",
                        dispatch: [string::lcs; store],
                        supported: true,
                        group: "string",
                        shape: (-3, 1, 2, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::String],
                            keys: KeyExtraction::Pair,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b's' => {
                    {
                        variant: Set,
                        bytes: b"SET",
                        dispatch: [string::set; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"set",
                            class: b'$',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                }
                b't' => {
                    {
                        variant: Ttl,
                        bytes: b"TTL",
                        dispatch: [ttl::ttl; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::Keyspace],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
            }
            4 => {
                b'a' => {
                    {
                        variant: Auth,
                        bytes: b"AUTH",
                        dispatch: [connection::auth; args],
                        supported: true,
                        group: "connection",
                        shape: (-1, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Connection, AclCategory::Fast],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'c' => {
                    {
                        variant: Copy,
                        bytes: b"COPY",
                        dispatch: [keyspace::copy; store],
                        supported: true,
                        group: "generic",
                        shape: (3, 1, 2, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Keyspace],
                            keys: KeyExtraction::Pair,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'd' => {
                    {
                        variant: Dump,
                        bytes: b"DUMP",
                        dispatch: [keyspace::dump; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Keyspace],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: Decr,
                        bytes: b"DECR",
                        dispatch: [string::decr; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"set",
                            class: b'$',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                }
                b'e' => {
                    {
                        variant: Echo,
                        bytes: b"ECHO",
                        dispatch: [connection::echo; args],
                        supported: true,
                        group: "connection",
                        shape: (-1, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Connection, AclCategory::Fast],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: Eval,
                        bytes: b"EVAL",
                        dispatch: [scripting::eval; store],
                        supported: true,
                        group: "scripting",
                        shape: (-3, 3, 3, 1),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Scripting, AclCategory::Slow],
                            keys: KeyExtraction::EvalStyle,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: Exec,
                        bytes: b"EXEC",
                        dispatch: [transaction::exec_command; args],
                        supported: true,
                        group: "transaction",
                        shape: (1, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Slow, AclCategory::Transaction],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'h' => {
                    {
                        variant: HSet,
                        bytes: b"HSET",
                        dispatch: [hash::hset; store],
                        supported: true,
                        group: "hash",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::Hash],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"hset",
                            class: b'h',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: HGet,
                        bytes: b"HGET",
                        dispatch: [hash::hget; store],
                        supported: true,
                        group: "hash",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::Hash],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: HDel,
                        bytes: b"HDEL",
                        dispatch: [hash::hdel; store],
                        supported: true,
                        group: "hash",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::Hash],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"hset",
                            class: b'h',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: HLen,
                        bytes: b"HLEN",
                        dispatch: [hash::hlen; store],
                        supported: true,
                        group: "hash",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::Hash],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'i' => {
                    {
                        variant: Incr,
                        bytes: b"INCR",
                        dispatch: [string::incr; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"set",
                            class: b'$',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                }
                b'k' => {
                    {
                        variant: Keys,
                        bytes: b"KEYS",
                        dispatch: [keyspace::keys; store],
                        supported: true,
                        group: "generic",
                        shape: (-1, 0, 0, 0),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Keyspace, AclCategory::Dangerous],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'l' => {
                    {
                        variant: LPop,
                        bytes: b"LPOP",
                        dispatch: [list::lpop; store],
                        supported: true,
                        group: "list",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::List],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"lset",
                            class: b'l',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: LRem,
                        bytes: b"LREM",
                        dispatch: [list::lrem; store],
                        supported: true,
                        group: "list",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: LLen,
                        bytes: b"LLEN",
                        dispatch: [list::llen; store],
                        supported: true,
                        group: "list",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::List],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: LSet,
                        bytes: b"LSET",
                        dispatch: [list::lset; store],
                        supported: true,
                        group: "list",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::List],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"lset",
                            class: b'l',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: LPos,
                        bytes: b"LPOS",
                        dispatch: [list::lpos; store],
                        supported: true,
                        group: "list",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::List],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'm' => {
                    {
                        variant: Move,
                        bytes: b"MOVE",
                        dispatch: [keyspace::move_key; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Keyspace],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: MGet,
                        bytes: b"MGET",
                        dispatch: [string::mget; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, -1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::String],
                            keys: KeyExtraction::AllFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: MSet,
                        bytes: b"MSET",
                        dispatch: [string::mset; store],
                        supported: true,
                        group: "string",
                        shape: (-3, 1, -1, 2),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::String],
                            keys: KeyExtraction::EveryOtherFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"set",
                            class: b'$',
                            keys: NotificationKeyArguments::EveryOtherFrom(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                }
                b'p' => {
                    {
                        variant: Ping,
                        bytes: b"PING",
                        dispatch: [connection::ping; args],
                        supported: true,
                        group: "connection",
                        shape: (-1, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Connection, AclCategory::Fast],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: PTtl,
                        bytes: b"PTTL",
                        dispatch: [ttl::pttl; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::Keyspace],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'q' => {
                    {
                        variant: Quit,
                        bytes: b"QUIT",
                        dispatch: [connection::quit; args],
                        supported: true,
                        group: "connection",
                        shape: (-1, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Connection, AclCategory::Fast],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'r' => {
                    {
                        variant: RPop,
                        bytes: b"RPOP",
                        dispatch: [list::rpop; store],
                        supported: true,
                        group: "list",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::List],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"lset",
                            class: b'l',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                }
                b's' => {
                    {
                        variant: Scan,
                        bytes: b"SCAN",
                        dispatch: [keyspace::scan; store],
                        supported: true,
                        group: "generic",
                        shape: (-1, 0, 0, 0),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Keyspace],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: Sort,
                        bytes: b"SORT",
                        dispatch: [keyspace::sort; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Keyspace],
                            keys: KeyExtraction::SortStore,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: SAdd,
                        bytes: b"SADD",
                        dispatch: [set::sadd; store],
                        supported: true,
                        group: "set",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::Set],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"sadd",
                            class: b's',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: SRem,
                        bytes: b"SREM",
                        dispatch: [set::srem; store],
                        supported: true,
                        group: "set",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::Set],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"sadd",
                            class: b's',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: SPop,
                        bytes: b"SPOP",
                        dispatch: [set::spop; store],
                        supported: true,
                        group: "set",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Set],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"sadd",
                            class: b's',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                }
                b't' => {
                    {
                        variant: Type,
                        bytes: b"TYPE",
                        dispatch: [keyspace::key_type; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::Keyspace],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'x' => {
                    {
                        variant: XAdd,
                        bytes: b"XADD",
                        dispatch: [stream::xadd; store],
                        supported: true,
                        group: "stream",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Stream],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: XLen,
                        bytes: b"XLEN",
                        dispatch: [stream::xlen; store],
                        supported: true,
                        group: "stream",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::Stream],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: XDel,
                        bytes: b"XDEL",
                        dispatch: [stream::xdel; store],
                        supported: true,
                        group: "stream",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::Stream],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: XAck,
                        bytes: b"XACK",
                        dispatch: [stream::xack; store],
                        supported: true,
                        group: "stream",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::Stream],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'z' => {
                    {
                        variant: ZAdd,
                        bytes: b"ZADD",
                        dispatch: [zset::zadd; store],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::SortedSet],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"zadd",
                            class: b'z',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: ZRem,
                        bytes: b"ZREM",
                        dispatch: [zset::zrem; store],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::SortedSet],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"zadd",
                            class: b'z',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                }
            }
            5 => {
                b'b' => {
                    {
                        variant: BitOp,
                        bytes: b"BITOP",
                        dispatch: [string::bitop; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::String, AclCategory::Bitmap],
                            keys: KeyExtraction::AllFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: BLPop,
                        bytes: b"BLPOP",
                        dispatch: [list::blpop; store],
                        supported: true,
                        group: "list",
                        shape: (-3, 1, -2, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Blocking, AclCategory::List],
                            keys: KeyExtraction::AllExceptLastFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: BRPop,
                        bytes: b"BRPOP",
                        dispatch: [list::brpop; store],
                        supported: true,
                        group: "list",
                        shape: (-3, 1, -2, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Blocking, AclCategory::List],
                            keys: KeyExtraction::AllExceptLastFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'd' => {
                    {
                        variant: Delex,
                        bytes: b"DELEX",
                        dispatch: [string::delex; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"del",
                            class: b'g',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::IntegerOne,
                        },
                    }
                }
                b'g' => {
                    {
                        variant: GetEx,
                        bytes: b"GETEX",
                        dispatch: [string::getex; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'h' => {
                    {
                        variant: Hello,
                        bytes: b"HELLO",
                        dispatch: [connection::hello; args],
                        supported: true,
                        group: "connection",
                        shape: (-1, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Connection, AclCategory::Fast],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: HMSet,
                        bytes: b"HMSET",
                        dispatch: [hash::hmset; store],
                        supported: true,
                        group: "hash",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::Hash],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: HMGet,
                        bytes: b"HMGET",
                        dispatch: [hash::hmget; store],
                        supported: true,
                        group: "hash",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::Hash],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: HKeys,
                        bytes: b"HKEYS",
                        dispatch: [hash::hkeys; store],
                        supported: true,
                        group: "hash",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Hash],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: HVals,
                        bytes: b"HVALS",
                        dispatch: [hash::hvals; store],
                        supported: true,
                        group: "hash",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Hash],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: HScan,
                        bytes: b"HSCAN",
                        dispatch: [hash::hscan; store],
                        supported: true,
                        group: "hash",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Hash],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'l' => {
                    {
                        variant: LPush,
                        bytes: b"LPUSH",
                        dispatch: [list::lpush; store],
                        supported: true,
                        group: "list",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::List],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"lset",
                            class: b'l',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: LTrim,
                        bytes: b"LTRIM",
                        dispatch: [list::ltrim; store],
                        supported: true,
                        group: "list",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::List],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"lset",
                            class: b'l',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: LMove,
                        bytes: b"LMOVE",
                        dispatch: [list::lmove; store],
                        supported: true,
                        group: "list",
                        shape: (3, 1, 2, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::List],
                            keys: KeyExtraction::Pair,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"lset",
                            class: b'l',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: LMPop,
                        bytes: b"LMPOP",
                        dispatch: [list::lmpop; store],
                        supported: true,
                        group: "list",
                        shape: (-3, 2, -1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::List],
                            keys: KeyExtraction::Counted {
                count_index: 1,
                first_key: 2,
            },
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'm' => {
                    {
                        variant: Multi,
                        bytes: b"MULTI",
                        dispatch: [transaction::multi_command; args],
                        supported: true,
                        group: "transaction",
                        shape: (1, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Fast, AclCategory::Transaction],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'p' => {
                    {
                        variant: PfAdd,
                        bytes: b"PFADD",
                        dispatch: [string::pfadd; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::String, AclCategory::HyperLogLog],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'r' => {
                    {
                        variant: RPush,
                        bytes: b"RPUSH",
                        dispatch: [list::rpush; store],
                        supported: true,
                        group: "list",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::List],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"lset",
                            class: b'l',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                }
                b's' => {
                    {
                        variant: SetNx,
                        bytes: b"SETNX",
                        dispatch: [string::setnx; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: SetEx,
                        bytes: b"SETEX",
                        dispatch: [string::setex; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"set",
                            class: b'$',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: SCard,
                        bytes: b"SCARD",
                        dispatch: [set::scard; store],
                        supported: true,
                        group: "set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::Set],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: SMove,
                        bytes: b"SMOVE",
                        dispatch: [set::smove; store],
                        supported: true,
                        group: "set",
                        shape: (3, 1, 2, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Set],
                            keys: KeyExtraction::Pair,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"sadd",
                            class: b's',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: SDiff,
                        bytes: b"SDIFF",
                        dispatch: [set::sdiff; store],
                        supported: true,
                        group: "set",
                        shape: (-2, 1, -1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Set],
                            keys: KeyExtraction::AllFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: SScan,
                        bytes: b"SSCAN",
                        dispatch: [set::sscan; store],
                        supported: true,
                        group: "set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Set],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b't' => {
                    {
                        variant: Touch,
                        bytes: b"TOUCH",
                        dispatch: [keyspace::touch; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, -1, 1),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::Keyspace],
                            keys: KeyExtraction::AllFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'w' => {
                    {
                        variant: Watch,
                        bytes: b"WATCH",
                        dispatch: [transaction::watch_command; args],
                        supported: true,
                        group: "transaction",
                        shape: (-2, 1, -1, 1),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Fast, AclCategory::Transaction],
                            keys: KeyExtraction::AllFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'x' => {
                    {
                        variant: XTrim,
                        bytes: b"XTRIM",
                        dispatch: [stream::xtrim; store],
                        supported: true,
                        group: "stream",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Stream],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: XRead,
                        bytes: b"XREAD",
                        dispatch: [stream::xread; store],
                        supported: true,
                        group: "stream",
                        shape: (-4, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Blocking, AclCategory::Stream],
                            keys: KeyExtraction::XReadStyle,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'z' => {
                    {
                        variant: ZCard,
                        bytes: b"ZCARD",
                        dispatch: [zset::zcard; store],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::SortedSet],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: ZRank,
                        bytes: b"ZRANK",
                        dispatch: [zset::zrank; store; false],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::SortedSet],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: ZMPop,
                        bytes: b"ZMPOP",
                        dispatch: [zset::zmpop; store],
                        supported: true,
                        group: "sorted-set",
                        shape: (-3, 2, -1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::SortedSet],
                            keys: KeyExtraction::Counted {
                count_index: 1,
                first_key: 2,
            },
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: ZDiff,
                        bytes: b"ZDIFF",
                        dispatch: [zset::zop; store; "ZDIFF"],
                        supported: true,
                        group: "sorted-set",
                        shape: (-3, 2, -1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::SortedSet],
                            keys: KeyExtraction::Counted {
                count_index: 1,
                first_key: 2,
            },
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: ZScan,
                        bytes: b"ZSCAN",
                        dispatch: [zset::zscan; store],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::SortedSet],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
            }
            6 => {
                b'a' => {
                    {
                        variant: Append,
                        bytes: b"APPEND",
                        dispatch: [string::append; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"set",
                            class: b'$',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                }
                b'b' => {
                    {
                        variant: BitPos,
                        bytes: b"BITPOS",
                        dispatch: [string::bitpos; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::String, AclCategory::Bitmap],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: BLMPop,
                        bytes: b"BLMPOP",
                        dispatch: [list::blmpop; store],
                        supported: true,
                        group: "list",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Blocking, AclCategory::List],
                            keys: KeyExtraction::Counted {
                count_index: 2,
                first_key: 3,
            },
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: BZMPop,
                        bytes: b"BZMPOP",
                        dispatch: [zset::bzmpop; store],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Blocking, AclCategory::SortedSet],
                            keys: KeyExtraction::Counted {
                count_index: 2,
                first_key: 3,
            },
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'c' => {
                    {
                        variant: Client,
                        bytes: b"CLIENT",
                        dispatch: [connection::client; args],
                        supported: true,
                        group: "connection",
                        shape: (-1, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Connection, AclCategory::Fast],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: Config,
                        bytes: b"CONFIG",
                        dispatch: [unsupported],
                        supported: true,
                        group: "server",
                        shape: (-2, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Admin, AclCategory::Slow],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'd' => {
                    {
                        variant: DbSize,
                        bytes: b"DBSIZE",
                        dispatch: [keyspace::dbsize; store],
                        supported: true,
                        group: "server",
                        shape: (-1, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::Keyspace],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: Digest,
                        bytes: b"DIGEST",
                        dispatch: [string::digest; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: DecrBy,
                        bytes: b"DECRBY",
                        dispatch: [string::decrby; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"set",
                            class: b'$',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                }
                b'e' => {
                    {
                        variant: Exists,
                        bytes: b"EXISTS",
                        dispatch: [keyspace::exists; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, -1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::Keyspace],
                            keys: KeyExtraction::AllFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: Expire,
                        bytes: b"EXPIRE",
                        dispatch: [ttl::expire; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::Keyspace],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"expire",
                            class: b'g',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::IntegerOne,
                        },
                    }
                }
                b'g' => {
                    {
                        variant: GetSet,
                        bytes: b"GETSET",
                        dispatch: [string::getset; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"set",
                            class: b'$',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: GetDel,
                        bytes: b"GETDEL",
                        dispatch: [string::getdel; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: GetBit,
                        bytes: b"GETBIT",
                        dispatch: [string::getbit; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::String, AclCategory::Bitmap],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: GeoAdd,
                        bytes: b"GEOADD",
                        dispatch: [geo::geoadd; store],
                        supported: true,
                        group: "geo",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Geo],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: GeoPos,
                        bytes: b"GEOPOS",
                        dispatch: [geo::geopos; store],
                        supported: true,
                        group: "geo",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Geo],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'h' => {
                    {
                        variant: HSetNx,
                        bytes: b"HSETNX",
                        dispatch: [hash::hsetnx; store],
                        supported: true,
                        group: "hash",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::Hash],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"hset",
                            class: b'h',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                }
                b'i' => {
                    {
                        variant: IncrBy,
                        bytes: b"INCRBY",
                        dispatch: [string::incrby; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"set",
                            class: b'$',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                }
                b'l' => {
                    {
                        variant: LPushX,
                        bytes: b"LPUSHX",
                        dispatch: [list::lpushx; store],
                        supported: true,
                        group: "list",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: LIndex,
                        bytes: b"LINDEX",
                        dispatch: [list::lindex; store],
                        supported: true,
                        group: "list",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::List],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: LRange,
                        bytes: b"LRANGE",
                        dispatch: [list::lrange; store],
                        supported: true,
                        group: "list",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::List],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'm' => {
                    {
                        variant: MSetEx,
                        bytes: b"MSETEX",
                        dispatch: [string::msetex; store],
                        supported: true,
                        group: "string",
                        shape: (-4, 2, -1, 2),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::String],
                            keys: KeyExtraction::Counted {
                                count_index: 1,
                                first_key: 2,
                            },
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: MSetNx,
                        bytes: b"MSETNX",
                        dispatch: [string::msetnx; store],
                        supported: true,
                        group: "string",
                        shape: (-3, 1, -1, 2),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::String],
                            keys: KeyExtraction::EveryOtherFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"set",
                            class: b'$',
                            keys: NotificationKeyArguments::EveryOtherFrom(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                }
                b'o' => {
                    {
                        variant: Object,
                        bytes: b"OBJECT",
                        dispatch: [object::object; store],
                        supported: true,
                        group: "generic",
                        shape: (-3, 0, 0, 0),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Keyspace, AclCategory::Read, AclCategory::Slow],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'p' => {
                    {
                        variant: PSetEx,
                        bytes: b"PSETEX",
                        dispatch: [string::psetex; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"set",
                            class: b'$',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: PubSub,
                        bytes: b"PUBSUB",
                        dispatch: [unsupported],
                        supported: true,
                        group: "pubsub",
                        shape: (-2, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::PubSub, AclCategory::Read, AclCategory::Fast],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'r' => {
                    {
                        variant: Rename,
                        bytes: b"RENAME",
                        dispatch: [keyspace::rename; store],
                        supported: true,
                        group: "generic",
                        shape: (3, 1, 2, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Keyspace],
                            keys: KeyExtraction::Pair,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"rename",
                            class: b'g',
                            keys: NotificationKeyArguments::Argument(2),
                            response: NotificationResponsePolicy::OkOrIntegerOne,
                        },
                    }
                    {
                        variant: RPushX,
                        bytes: b"RPUSHX",
                        dispatch: [list::rpushx; store],
                        supported: true,
                        group: "list",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                }
                b's' => {
                    {
                        variant: Select,
                        bytes: b"SELECT",
                        dispatch: [connection::select_db; args],
                        supported: true,
                        group: "connection",
                        shape: (-1, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Connection, AclCategory::Fast],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: Script,
                        bytes: b"SCRIPT",
                        dispatch: [scripting::script; store],
                        supported: true,
                        group: "scripting",
                        shape: (-1, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Scripting, AclCategory::Slow],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: SubStr,
                        bytes: b"SUBSTR",
                        dispatch: [string::substr; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: StrLen,
                        bytes: b"STRLEN",
                        dispatch: [string::strlen; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: SetBit,
                        bytes: b"SETBIT",
                        dispatch: [string::setbit; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::String, AclCategory::Bitmap],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: SInter,
                        bytes: b"SINTER",
                        dispatch: [set::sinter; store],
                        supported: true,
                        group: "set",
                        shape: (-2, 1, -1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Set],
                            keys: KeyExtraction::AllFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: SUnion,
                        bytes: b"SUNION",
                        dispatch: [set::sunion; store],
                        supported: true,
                        group: "set",
                        shape: (-2, 1, -1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Set],
                            keys: KeyExtraction::AllFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'u' => {
                    {
                        variant: Unlink,
                        bytes: b"UNLINK",
                        dispatch: [keyspace::unlink; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, -1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::Keyspace],
                            keys: KeyExtraction::AllFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"del",
                            class: b'g',
                            keys: NotificationKeyArguments::AllFrom(1),
                            response: NotificationResponsePolicy::PositiveInteger,
                        },
                    }
                }
                b'x' => {
                    {
                        variant: XDelex,
                        bytes: b"XDELEX",
                        dispatch: [stream::xdelex; store],
                        supported: true,
                        group: "stream",
                        shape: (-5, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::Stream],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: XRange,
                        bytes: b"XRANGE",
                        dispatch: [stream::xrange; store],
                        supported: true,
                        group: "stream",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Stream],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: XGroup,
                        bytes: b"XGROUP",
                        dispatch: [stream::xgroup; store],
                        supported: true,
                        group: "stream",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Stream],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: XClaim,
                        bytes: b"XCLAIM",
                        dispatch: [stream::xclaim; store],
                        supported: true,
                        group: "stream",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Stream],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'z' => {
                    {
                        variant: ZCount,
                        bytes: b"ZCOUNT",
                        dispatch: [zset::zcount; store],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::SortedSet],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: ZScore,
                        bytes: b"ZSCORE",
                        dispatch: [zset::zscore; store],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::SortedSet],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: ZRange,
                        bytes: b"ZRANGE",
                        dispatch: [zset::zrange; store; false],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::SortedSet],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: ZInter,
                        bytes: b"ZINTER",
                        dispatch: [zset::zop; store; "ZINTER"],
                        supported: true,
                        group: "sorted-set",
                        shape: (-3, 2, -1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::SortedSet],
                            keys: KeyExtraction::Counted {
                count_index: 1,
                first_key: 2,
            },
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: ZUnion,
                        bytes: b"ZUNION",
                        dispatch: [zset::zop; store; "ZUNION"],
                        supported: true,
                        group: "sorted-set",
                        shape: (-3, 2, -1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::SortedSet],
                            keys: KeyExtraction::Counted {
                count_index: 1,
                first_key: 2,
            },
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
            }
            7 => {
                b'c' => {
                    {
                        variant: Command,
                        bytes: b"COMMAND",
                        dispatch: [command::command; args],
                        supported: true,
                        group: "server",
                        shape: (-1, 0, 0, 0),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Connection, AclCategory::Fast],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'd' => {
                    {
                        variant: Discard,
                        bytes: b"DISCARD",
                        dispatch: [transaction::discard_command; args],
                        supported: true,
                        group: "transaction",
                        shape: (1, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Fast, AclCategory::Transaction],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'e' => {
                    {
                        variant: EvalRo,
                        bytes: b"EVAL_RO",
                        dispatch: [scripting::eval_ro; store],
                        supported: true,
                        group: "scripting",
                        shape: (-3, 3, 3, 1),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Scripting, AclCategory::Slow, AclCategory::Read],
                            keys: KeyExtraction::EvalStyle,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: EvalSha,
                        bytes: b"EVALSHA",
                        dispatch: [scripting::evalsha; store],
                        supported: true,
                        group: "scripting",
                        shape: (-3, 3, 3, 1),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Scripting, AclCategory::Slow],
                            keys: KeyExtraction::EvalStyle,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'f' => {
                    {
                        variant: FlushDb,
                        bytes: b"FLUSHDB",
                        dispatch: [keyspace::flushdb; store],
                        supported: true,
                        group: "server",
                        shape: (-1, 0, 0, 0),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Admin, AclCategory::Dangerous, AclCategory::Slow],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'g' => {
                    {
                        variant: GeoDist,
                        bytes: b"GEODIST",
                        dispatch: [geo::geodist; store],
                        supported: true,
                        group: "geo",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Geo],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: GeoHash,
                        bytes: b"GEOHASH",
                        dispatch: [geo::geohash; store],
                        supported: true,
                        group: "geo",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Geo],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'h' => {
                    {
                        variant: HGetAll,
                        bytes: b"HGETALL",
                        dispatch: [hash::hgetall; store],
                        supported: true,
                        group: "hash",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Hash],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: HGetDel,
                        bytes: b"HGETDEL",
                        dispatch: [hash::hgetdel; store],
                        supported: true,
                        group: "hash",
                        shape: (-5, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::Hash],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"hdel",
                            class: b'h',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: HExists,
                        bytes: b"HEXISTS",
                        dispatch: [hash::hexists; store],
                        supported: true,
                        group: "hash",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::Hash],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: HStrLen,
                        bytes: b"HSTRLEN",
                        dispatch: [hash::hstrlen; store],
                        supported: true,
                        group: "hash",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::Hash],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: HIncrBy,
                        bytes: b"HINCRBY",
                        dispatch: [hash::hincrby; store],
                        supported: true,
                        group: "hash",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::Hash],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"hset",
                            class: b'h',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                }
                b'l' => {
                    {
                        variant: LInsert,
                        bytes: b"LINSERT",
                        dispatch: [list::linsert; store],
                        supported: true,
                        group: "list",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::List],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"lset",
                            class: b'l',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                }
                b'p' => {
                    {
                        variant: PExpire,
                        bytes: b"PEXPIRE",
                        dispatch: [ttl::pexpire; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::Keyspace],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"expire",
                            class: b'g',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::IntegerOne,
                        },
                    }
                    {
                        variant: Persist,
                        bytes: b"PERSIST",
                        dispatch: [ttl::persist; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::Keyspace],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"persist",
                            class: b'g',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::IntegerOne,
                        },
                    }
                    {
                        variant: PfCount,
                        bytes: b"PFCOUNT",
                        dispatch: [string::pfcount; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::String, AclCategory::HyperLogLog],
                            keys: KeyExtraction::AllFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: PfMerge,
                        bytes: b"PFMERGE",
                        dispatch: [string::pfmerge; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::String, AclCategory::HyperLogLog],
                            keys: KeyExtraction::AllFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: Publish,
                        bytes: b"PUBLISH",
                        dispatch: [unsupported],
                        supported: true,
                        group: "pubsub",
                        shape: (3, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::PubSub, AclCategory::Write, AclCategory::Fast],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::First,
                        },
                        notify: none,
                    }
                }
                b'r' => {
                    {
                        variant: Restore,
                        bytes: b"RESTORE",
                        dispatch: [keyspace::restore; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Keyspace, AclCategory::Dangerous],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'u' => {
                    {
                        variant: Unwatch,
                        bytes: b"UNWATCH",
                        dispatch: [transaction::unwatch_command; args],
                        supported: true,
                        group: "transaction",
                        shape: (1, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Fast, AclCategory::Transaction],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'z' => {
                    {
                        variant: ZIncrBy,
                        bytes: b"ZINCRBY",
                        dispatch: [zset::zincrby; store],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::SortedSet],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"zadd",
                            class: b'z',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: ZMScore,
                        bytes: b"ZMSCORE",
                        dispatch: [zset::zmscore; store],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::SortedSet],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: ZPopMin,
                        bytes: b"ZPOPMIN",
                        dispatch: [zset::zpop; store; false],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::SortedSet],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"zadd",
                            class: b'z',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: ZPopMax,
                        bytes: b"ZPOPMAX",
                        dispatch: [zset::zpop; store; true],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::SortedSet],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"zadd",
                            class: b'z',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                }
            }
            8 => {
                b'b' => {
                    {
                        variant: BitCount,
                        bytes: b"BITCOUNT",
                        dispatch: [string::bitcount; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::String, AclCategory::Bitmap],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: BitField,
                        bytes: b"BITFIELD",
                        dispatch: [string::bitfield; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::String, AclCategory::Bitmap],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: BZPopMin,
                        bytes: b"BZPOPMIN",
                        dispatch: [zset::bzpop; store; false],
                        supported: true,
                        group: "sorted-set",
                        shape: (-3, 1, -2, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Blocking, AclCategory::SortedSet],
                            keys: KeyExtraction::AllExceptLastFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: BZPopMax,
                        bytes: b"BZPOPMAX",
                        dispatch: [zset::bzpop; store; true],
                        supported: true,
                        group: "sorted-set",
                        shape: (-3, 1, -2, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Blocking, AclCategory::SortedSet],
                            keys: KeyExtraction::AllExceptLastFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'e' => {
                    {
                        variant: ExpireAt,
                        bytes: b"EXPIREAT",
                        dispatch: [ttl::expireat; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::Keyspace],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"expire",
                            class: b'g',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::IntegerOne,
                        },
                    }
                }
                b'f' => {
                    {
                        variant: FlushAll,
                        bytes: b"FLUSHALL",
                        dispatch: [keyspace::flushall; store],
                        supported: true,
                        group: "server",
                        shape: (-1, 0, 0, 0),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Admin, AclCategory::Dangerous, AclCategory::Slow],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'g' => {
                    {
                        variant: GetRange,
                        bytes: b"GETRANGE",
                        dispatch: [string::getrange; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'j' => {
                    {
                        variant: JsonDel,
                        bytes: b"JSON.DEL",
                        dispatch: [json::json_del; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: JsonGet,
                        bytes: b"JSON.GET",
                        dispatch: [json::json_get; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: JsonSet,
                        bytes: b"JSON.SET",
                        dispatch: [json::json_set; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                }
                b'r' => {
                    {
                        variant: RenameNx,
                        bytes: b"RENAMENX",
                        dispatch: [keyspace::renamenx; store],
                        supported: true,
                        group: "generic",
                        shape: (3, 1, 2, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Keyspace],
                            keys: KeyExtraction::Pair,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"rename",
                            class: b'g',
                            keys: NotificationKeyArguments::Argument(2),
                            response: NotificationResponsePolicy::OkOrIntegerOne,
                        },
                    }
                }
                b's' => {
                    {
                        variant: SPublish,
                        bytes: b"SPUBLISH",
                        dispatch: [unsupported],
                        supported: true,
                        group: "pubsub",
                        shape: (3, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::PubSub, AclCategory::Write, AclCategory::Fast],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::First,
                        },
                        notify: none,
                    }
                    {
                        variant: SetRange,
                        bytes: b"SETRANGE",
                        dispatch: [string::setrange; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::String],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"set",
                            class: b'$',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: SMembers,
                        bytes: b"SMEMBERS",
                        dispatch: [set::smembers; store],
                        supported: true,
                        group: "set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Set],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'x' => {
                    {
                        variant: XPending,
                        bytes: b"XPENDING",
                        dispatch: [stream::xpending; store],
                        supported: true,
                        group: "stream",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Stream],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'z' => {
                    {
                        variant: ZRevRank,
                        bytes: b"ZREVRANK",
                        dispatch: [zset::zrank; store; true],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::SortedSet],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
            }
            9 => {
                b'g' => {
                    {
                        variant: GeoRadius,
                        bytes: b"GEORADIUS",
                        dispatch: [geo::georadius; store],
                        supported: true,
                        group: "geo",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Geo],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: GeoSearch,
                        bytes: b"GEOSEARCH",
                        dispatch: [geo::geosearch; store],
                        supported: true,
                        group: "geo",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Geo],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'j' => {
                    {
                        variant: JsonMGet,
                        bytes: b"JSON.MGET",
                        dispatch: [json::json_mget; store],
                        supported: true,
                        group: "generic",
                        shape: (-3, 1, -2, 1),
                        readonly: true,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: JsonMSet,
                        bytes: b"JSON.MSET",
                        dispatch: [json::json_mset; store],
                        supported: true,
                        group: "generic",
                        shape: (-4, 1, -1, 3),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: JsonResp,
                        bytes: b"JSON.RESP",
                        dispatch: [json::json_resp; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: JsonType,
                        bytes: b"JSON.TYPE",
                        dispatch: [json::json_type; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                }
                b'p' => {
                    {
                        variant: PExpireAt,
                        bytes: b"PEXPIREAT",
                        dispatch: [ttl::pexpireat; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::Keyspace],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"expire",
                            class: b'g',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::IntegerOne,
                        },
                    }
                }
                b'r' => {
                    {
                        variant: RPopLPush,
                        bytes: b"RPOPLPUSH",
                        dispatch: [list::rpoplpush; store],
                        supported: true,
                        group: "list",
                        shape: (3, 1, 2, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: some {
                            event: b"lset",
                            class: b'l',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                    {
                        variant: RandomKey,
                        bytes: b"RANDOMKEY",
                        dispatch: [keyspace::randomkey; store],
                        supported: true,
                        group: "generic",
                        shape: (1, 0, 0, 0),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Keyspace],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b's' => {
                    {
                        variant: SIsMember,
                        bytes: b"SISMEMBER",
                        dispatch: [set::sismember; store],
                        supported: true,
                        group: "set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Fast, AclCategory::Set],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: Subscribe,
                        bytes: b"SUBSCRIBE",
                        dispatch: [unsupported],
                        supported: true,
                        group: "pubsub",
                        shape: (-2, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::PubSub, AclCategory::Read, AclCategory::Fast],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::AllFrom(1),
                        },
                        notify: none,
                    }
                }
                b'x' => {
                    {
                        variant: XRevRange,
                        bytes: b"XREVRANGE",
                        dispatch: [stream::xrevrange; store],
                        supported: true,
                        group: "stream",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Stream],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'z' => {
                    {
                        variant: ZRevRange,
                        bytes: b"ZREVRANGE",
                        dispatch: [zset::zrange; store; true],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: ZLexCount,
                        bytes: b"ZLEXCOUNT",
                        dispatch: [zset::zlexcount; store],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                }
            }
            10 => {
                b'b' => {
                    {
                        variant: BRPopLPush,
                        bytes: b"BRPOPLPUSH",
                        dispatch: [list::brpoplpush; store],
                        supported: true,
                        group: "list",
                        shape: (3, 1, 2, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Blocking, AclCategory::List],
                            keys: KeyExtraction::Pair,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'e' => {
                    {
                        variant: EvalShaRo,
                        bytes: b"EVALSHA_RO",
                        dispatch: [scripting::evalsha_ro; store],
                        supported: true,
                        group: "scripting",
                        shape: (-3, 3, 3, 1),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Scripting, AclCategory::Slow, AclCategory::Read],
                            keys: KeyExtraction::EvalStyle,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'h' => {
                    {
                        variant: HRandField,
                        bytes: b"HRANDFIELD",
                        dispatch: [hash::hrandfield; store],
                        supported: true,
                        group: "hash",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Hash],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'j' => {
                    {
                        variant: JsonClear,
                        bytes: b"JSON.CLEAR",
                        dispatch: [json::json_clear; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: JsonDebug,
                        bytes: b"JSON.DEBUG",
                        dispatch: [json::json_debug; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: JsonMerge,
                        bytes: b"JSON.MERGE",
                        dispatch: [json::json_merge; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                }
                b'p' => {
                    {
                        variant: PSubscribe,
                        bytes: b"PSUBSCRIBE",
                        dispatch: [unsupported],
                        supported: true,
                        group: "pubsub",
                        shape: (-2, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::PubSub, AclCategory::Read, AclCategory::Fast],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::AllFrom(1),
                        },
                        notify: none,
                    }
                }
                b's' => {
                    {
                        variant: SSubscribe,
                        bytes: b"SSUBSCRIBE",
                        dispatch: [unsupported],
                        supported: true,
                        group: "pubsub",
                        shape: (-2, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::PubSub, AclCategory::Read, AclCategory::Fast],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::AllFrom(1),
                        },
                        notify: none,
                    }
                    {
                        variant: SMIsMember,
                        bytes: b"SMISMEMBER",
                        dispatch: [set::smismember; store],
                        supported: true,
                        group: "set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: SDiffStore,
                        bytes: b"SDIFFSTORE",
                        dispatch: [set::sdiffstore; store],
                        supported: true,
                        group: "set",
                        shape: (-4, 1, -1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Set],
                            keys: KeyExtraction::AllFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: SInterCard,
                        bytes: b"SINTERCARD",
                        dispatch: [set::sintercard; store],
                        supported: true,
                        group: "set",
                        shape: (-3, 2, -1, 1),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Set],
                            keys: KeyExtraction::Counted {
                count_index: 1,
                first_key: 2,
            },
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'x' => {
                    {
                        variant: XReadGroup,
                        bytes: b"XREADGROUP",
                        dispatch: [stream::xreadgroup; store],
                        supported: true,
                        group: "stream",
                        shape: (-4, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Blocking, AclCategory::Stream],
                            keys: KeyExtraction::XReadStyle,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: XAutoClaim,
                        bytes: b"XAUTOCLAIM",
                        dispatch: [stream::xautoclaim; store],
                        supported: true,
                        group: "stream",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Stream],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'z' => {
                    {
                        variant: ZDiffStore,
                        bytes: b"ZDIFFSTORE",
                        dispatch: [zset::zop_store; store; "ZDIFFSTORE"],
                        supported: true,
                        group: "sorted-set",
                        shape: (-4, 1, -1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                }
            }
            11 => {
                b'b' => {
                    {
                        variant: BitFieldRo,
                        bytes: b"BITFIELD_RO",
                        dispatch: [string::bitfield_ro; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::String, AclCategory::Bitmap],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'i' => {
                    {
                        variant: IncrByFloat,
                        bytes: b"INCRBYFLOAT",
                        dispatch: [string::incrbyfloat; store],
                        supported: true,
                        group: "string",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                }
                b'j' => {
                    {
                        variant: JsonArrLen,
                        bytes: b"JSON.ARRLEN",
                        dispatch: [json::json_arrlen; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: JsonArrPop,
                        bytes: b"JSON.ARRPOP",
                        dispatch: [json::json_arrpop; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: JsonForget,
                        bytes: b"JSON.FORGET",
                        dispatch: [json::json_forget; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: JsonObjLen,
                        bytes: b"JSON.OBJLEN",
                        dispatch: [json::json_objlen; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: JsonStrLen,
                        bytes: b"JSON.STRLEN",
                        dispatch: [json::json_strlen; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: JsonToggle,
                        bytes: b"JSON.TOGGLE",
                        dispatch: [json::json_toggle; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                }
                b's' => {
                    {
                        variant: SInterStore,
                        bytes: b"SINTERSTORE",
                        dispatch: [set::sinterstore; store],
                        supported: true,
                        group: "set",
                        shape: (-4, 1, -1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Set],
                            keys: KeyExtraction::AllFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: SUnionStore,
                        bytes: b"SUNIONSTORE",
                        dispatch: [set::sunionstore; store],
                        supported: true,
                        group: "set",
                        shape: (-4, 1, -1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Set],
                            keys: KeyExtraction::AllFrom(1),
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: SRandMember,
                        bytes: b"SRANDMEMBER",
                        dispatch: [set::srandmember; store],
                        supported: true,
                        group: "set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Set],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'u' => {
                    {
                        variant: Unsubscribe,
                        bytes: b"UNSUBSCRIBE",
                        dispatch: [unsupported],
                        supported: true,
                        group: "pubsub",
                        shape: (-1, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::PubSub, AclCategory::Read, AclCategory::Fast],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::AllFrom(1),
                        },
                        notify: none,
                    }
                }
                b'z' => {
                    {
                        variant: ZInterStore,
                        bytes: b"ZINTERSTORE",
                        dispatch: [zset::zop_store; store; "ZINTERSTORE"],
                        supported: true,
                        group: "sorted-set",
                        shape: (-4, 1, -1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: ZUnionStore,
                        bytes: b"ZUNIONSTORE",
                        dispatch: [zset::zop_store; store; "ZUNIONSTORE"],
                        supported: true,
                        group: "sorted-set",
                        shape: (-4, 1, -1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: ZRandMember,
                        bytes: b"ZRANDMEMBER",
                        dispatch: [zset::zrandmember; store],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::SortedSet],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: ZRangeStore,
                        bytes: b"ZRANGESTORE",
                        dispatch: [zset::zrangestore; store],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: ZRangeByLex,
                        bytes: b"ZRANGEBYLEX",
                        dispatch: [zset::zrange_by_lex; store; false],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                }
            }
            12 => {
                b'g' => {
                    {
                        variant: GeoRadiusRo,
                        bytes: b"GEORADIUS_RO",
                        dispatch: [geo::georadius_ro; store],
                        supported: true,
                        group: "geo",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Geo],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'h' => {
                    {
                        variant: HIncrByFloat,
                        bytes: b"HINCRBYFLOAT",
                        dispatch: [hash::hincrbyfloat; store],
                        supported: true,
                        group: "hash",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Fast, AclCategory::Hash],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: some {
                            event: b"hset",
                            class: b'h',
                            keys: NotificationKeyArguments::Argument(1),
                            response: NotificationResponsePolicy::AnySuccess,
                        },
                    }
                }
                b'j' => {
                    {
                        variant: JsonArrTrim,
                        bytes: b"JSON.ARRTRIM",
                        dispatch: [json::json_arrtrim; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: JsonObjKeys,
                        bytes: b"JSON.OBJKEYS",
                        dispatch: [json::json_objkeys; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                }
                b'p' => {
                    {
                        variant: PUnsubscribe,
                        bytes: b"PUNSUBSCRIBE",
                        dispatch: [unsupported],
                        supported: true,
                        group: "pubsub",
                        shape: (-1, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::PubSub, AclCategory::Read, AclCategory::Fast],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::AllFrom(1),
                        },
                        notify: none,
                    }
                }
                b's' => {
                    {
                        variant: SUnsubscribe,
                        bytes: b"SUNSUBSCRIBE",
                        dispatch: [unsupported],
                        supported: true,
                        group: "pubsub",
                        shape: (-1, 0, 0, 0),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::PubSub, AclCategory::Read, AclCategory::Fast],
                            keys: KeyExtraction::None,
                            channels: ChannelExtraction::AllFrom(1),
                        },
                        notify: none,
                    }
                }
            }
            13 => {
                b'j' => {
                    {
                        variant: JsonArrIndex,
                        bytes: b"JSON.ARRINDEX",
                        dispatch: [json::json_arrindex; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                }
                b'z' => {
                    {
                        variant: ZRangeByScore,
                        bytes: b"ZRANGEBYSCORE",
                        dispatch: [zset::zrange_by_score; store; false],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::SortedSet],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
            }
            14 => {
                b'g' => {
                    {
                        variant: GeoSearchStore,
                        bytes: b"GEOSEARCHSTORE",
                        dispatch: [geo::geosearchstore; store],
                        supported: true,
                        group: "geo",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::Geo],
                            keys: KeyExtraction::Pair,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
                b'j' => {
                    {
                        variant: JsonArrAppend,
                        bytes: b"JSON.ARRAPPEND",
                        dispatch: [json::json_arrappend; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: JsonArrInsert,
                        bytes: b"JSON.ARRINSERT",
                        dispatch: [json::json_arrinsert; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: JsonNumIncrBy,
                        bytes: b"JSON.NUMINCRBY",
                        dispatch: [json::json_numincrby; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: JsonNumMultBy,
                        bytes: b"JSON.NUMMULTBY",
                        dispatch: [json::json_nummultby; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: JsonStrAppend,
                        bytes: b"JSON.STRAPPEND",
                        dispatch: [json::json_strappend; store],
                        supported: true,
                        group: "generic",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                }
                b'z' => {
                    {
                        variant: ZRevRangeByLex,
                        bytes: b"ZREVRANGEBYLEX",
                        dispatch: [zset::zrange_by_lex; store; true],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                    {
                        variant: ZRemRangeByLex,
                        bytes: b"ZREMRANGEBYLEX",
                        dispatch: [zset::zremrangebylex; store],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                }
            }
            15 => {
                b'z' => {
                    {
                        variant: ZRemRangeByRank,
                        bytes: b"ZREMRANGEBYRANK",
                        dispatch: [zset::zremrangebyrank; store],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: true,
                        auth: some {
                            categories: &[AclCategory::Write, AclCategory::Slow, AclCategory::SortedSet],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
            }
            16 => {
                b'z' => {
                    {
                        variant: ZRevRangeByScore,
                        bytes: b"ZREVRANGEBYSCORE",
                        dispatch: [zset::zrange_by_score; store; true],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::SortedSet],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                    {
                        variant: ZRemRangeByScore,
                        bytes: b"ZREMRANGEBYSCORE",
                        dispatch: [zset::zremrangebyscore; store],
                        supported: true,
                        group: "sorted-set",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: none,
                        notify: none,
                    }
                }
            }
            17 => {
                b'g' => {
                    {
                        variant: GeoRadiusByMember,
                        bytes: b"GEORADIUSBYMEMBER",
                        dispatch: [geo::georadiusbymember; store],
                        supported: true,
                        group: "geo",
                        shape: (-2, 1, 1, 1),
                        readonly: false,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Geo],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
            }
            20 => {
                b'g' => {
                    {
                        variant: GeoRadiusByMemberRo,
                        bytes: b"GEORADIUSBYMEMBER_RO",
                        dispatch: [geo::georadiusbymember_ro; store],
                        supported: true,
                        group: "geo",
                        shape: (-2, 1, 1, 1),
                        readonly: true,
                        write: false,
                        auth: some {
                            categories: &[AclCategory::Read, AclCategory::Slow, AclCategory::Geo],
                            keys: KeyExtraction::Single,
                            channels: ChannelExtraction::None,
                        },
                        notify: none,
                    }
                }
            }
        }
    };
}

pub(crate) use with_command_registry;
