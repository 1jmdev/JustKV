use crate::engine::value::CompactArg;
use crate::protocol::types::RespFrame;

pub type Args = [CompactArg];

pub fn eq_ascii(command: &[u8], expected: &[u8]) -> bool {
    command == expected || command.eq_ignore_ascii_case(expected)
}

pub fn wrong_args(command: &str) -> RespFrame {
    RespFrame::Error(format!(
        "ERR wrong number of arguments for '{command}' command"
    ))
}

pub fn int_error() -> RespFrame {
    RespFrame::Error("ERR value is not an integer or out of range".to_string())
}

pub fn wrong_type() -> RespFrame {
    RespFrame::Error(
        "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
    )
}

pub fn u64_to_bytes(value: u64) -> Vec<u8> {
    let mut buffer = itoa::Buffer::new();
    buffer.format(value).as_bytes().to_vec()
}

pub fn f64_to_bytes(value: f64) -> Vec<u8> {
    let mut buffer = ryu::Buffer::new();
    buffer.format(value).as_bytes().to_vec()
}
