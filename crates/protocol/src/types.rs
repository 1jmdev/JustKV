use bytes::Bytes;

use engine::value::{CompactArg, CompactValue};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BulkData {
    Arg(CompactArg),
    Value(CompactValue),
}

impl BulkData {
    pub fn from_vec(value: Vec<u8>) -> Self {
        let _trace = profiler::scope("protocol::types::from_vec");
        Self::Arg(CompactArg::from_vec(value))
    }

    pub fn as_slice(&self) -> &[u8] {
        let _trace = profiler::scope("protocol::types::as_slice");
        match self {
            Self::Arg(value) => value.as_slice(),
            Self::Value(value) => value.as_slice(),
        }
    }

    pub fn into_vec(self) -> Vec<u8> {
        let _trace = profiler::scope("protocol::types::into_vec");
        match self {
            Self::Arg(value) => value.into_vec(),
            Self::Value(value) => value.into_vec(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RespFrame {
    Simple(String),
    SimpleStatic(&'static str),
    Error(String),
    ErrorStatic(&'static str),
    Integer(i64),
    Bulk(Option<BulkData>),
    BulkValues(Vec<CompactValue>),
    /// Pre-encoded RESP bytes, written directly to the output buffer.
    PreEncoded(Bytes),
    Array(Option<Vec<RespFrame>>),
    Map(Vec<(RespFrame, RespFrame)>),
}

impl RespFrame {
    pub fn ok() -> Self {
        let _trace = profiler::scope("protocol::types::ok");
        Self::SimpleStatic("OK")
    }

    pub fn simple_static(value: &'static str) -> Self {
        let _trace = profiler::scope("protocol::types::simple_static");
        Self::SimpleStatic(value)
    }

    pub fn error_static(value: &'static str) -> Self {
        let _trace = profiler::scope("protocol::types::error_static");
        Self::ErrorStatic(value)
    }
}
