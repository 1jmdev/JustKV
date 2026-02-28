use crate::engine::value::CompactArg;
use crate::protocol::types::RespFrame;

pub type Args = [CompactArg];

pub fn eq_ascii(command: &[u8], expected: &[u8]) -> bool {
    command.eq_ignore_ascii_case(expected)
}

pub fn wrong_args(command: &str) -> RespFrame {
    RespFrame::Error(format!(
        "ERR wrong number of arguments for '{command}' command"
    ))
}

pub fn int_error() -> RespFrame {
    RespFrame::Error("ERR value is not an integer or out of range".to_string())
}
