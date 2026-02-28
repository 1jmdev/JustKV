use crate::protocol::types::RespFrame;

pub type Args = [Vec<u8>];

pub fn upper(value: &[u8]) -> Vec<u8> {
    value.iter().map(u8::to_ascii_uppercase).collect()
}

pub fn wrong_args(command: &str) -> RespFrame {
    RespFrame::Error(format!(
        "ERR wrong number of arguments for '{command}' command"
    ))
}

pub fn int_error() -> RespFrame {
    RespFrame::Error("ERR value is not an integer or out of range".to_string())
}
