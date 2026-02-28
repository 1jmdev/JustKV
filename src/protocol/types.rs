#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RespFrame {
    Simple(String),
    Error(String),
    Integer(i64),
    Bulk(Option<Vec<u8>>),
    Array(Option<Vec<RespFrame>>),
    Map(Vec<(RespFrame, RespFrame)>),
}

impl RespFrame {
    pub fn ok() -> Self {
        Self::Simple("OK".to_string())
    }
}
