use protocol::types::RespFrame;

#[derive(Debug, Clone, Copy)]
pub enum AuthError {
    WrongPass,
    NoPasswordSet,
}

pub fn no_auth() -> RespFrame {
    let _trace = profiler::scope("server::auth::no_auth");
    RespFrame::error_static("NOAUTH Authentication required.")
}
