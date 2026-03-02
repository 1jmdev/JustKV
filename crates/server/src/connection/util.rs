use protocol::types::RespFrame;

pub(super) fn wrong_args(command: &str) -> RespFrame {
    RespFrame::Error(format!(
        "ERR wrong number of arguments for '{command}' command"
    ))
}

pub(super) fn collapse_pubsub_responses(mut responses: Vec<RespFrame>) -> RespFrame {
    if responses.len() == 1 {
        responses.remove(0)
    } else {
        RespFrame::Array(Some(responses))
    }
}
