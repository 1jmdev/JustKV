use protocol::types::RespFrame;

pub(super) fn wrong_args(command: &str) -> RespFrame {
    let _trace = profiler::scope("server::connection::util::wrong_args");
    RespFrame::Error(format!(
        "ERR wrong number of arguments for '{command}' command"
    ))
}

pub(super) fn collapse_pubsub_responses(mut responses: Vec<RespFrame>) -> RespFrame {
    let _trace = profiler::scope("server::connection::util::collapse_pubsub_responses");
    if responses.len() == 1 {
        responses.remove(0)
    } else {
        RespFrame::Array(Some(responses))
    }
}
