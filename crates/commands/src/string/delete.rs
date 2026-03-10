use crate::util::{wrong_args, wrong_type, Args};
use engine::store::{Store, StringDigestCondition};
use protocol::types::RespFrame;

pub(crate) fn delex(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::delete::delex");
    if args.len() < 2 {
        return wrong_args("DELEX");
    }

    if args.len() == 3 {
        return wrong_args("DELEX");
    }

    if args.len() != 2 && args.len() != 4 {
        return crate::util::syntax_error();
    }

    let condition = if args.len() == 2 {
        None
    } else if args[2].eq_ignore_ascii_case(b"IFEQ") {
        Some(StringDigestCondition::Eq(args[3].as_slice()))
    } else if args[2].eq_ignore_ascii_case(b"IFNE") {
        Some(StringDigestCondition::Ne(args[3].as_slice()))
    } else if args[2].eq_ignore_ascii_case(b"IFDEQ") {
        Some(StringDigestCondition::DigestEq(args[3].as_slice()))
    } else if args[2].eq_ignore_ascii_case(b"IFDNE") {
        Some(StringDigestCondition::DigestNe(args[3].as_slice()))
    } else {
        return crate::util::syntax_error();
    };

    match store.delex(&args[1], condition) {
        Ok(deleted) => RespFrame::Integer(i64::from(deleted)),
        Err(()) => wrong_type(),
    }
}
