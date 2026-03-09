use crate::util::{wrong_args, wrong_type, Args};
use engine::store::Store;
use protocol::types::RespFrame;

pub(crate) fn pfadd(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::hyperlog::pfadd");
    if args.len() < 3 {
        return wrong_args("PFADD");
    }

    match store.pfadd(&args[1], &args[2..]) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn pfcount(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::hyperlog::pfcount");
    if args.len() < 2 {
        return wrong_args("PFCOUNT");
    }

    match store.pfcount(&args[1..]) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn pfmerge(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::hyperlog::pfmerge");
    if args.len() < 2 {
        return wrong_args("PFMERGE");
    }

    match store.pfmerge(&args[1], &args[2..]) {
        Ok(()) => RespFrame::ok(),
        Err(engine::store::HyperLogLogError::WrongType) => wrong_type(),
        Err(engine::store::HyperLogLogError::InvalidHyperLogLog) => {
            RespFrame::Error("WRONGTYPE Key is not a valid HyperLogLog string value.".to_string())
        }
    }
}
