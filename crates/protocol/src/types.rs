use bytes::Bytes;
use types::value::{CompactArg, CompactValue};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BulkData {
    Arg(CompactArg),
    Value(CompactValue),
}

impl BulkData {
    #[inline(always)]
    pub fn from_vec(value: Vec<u8>) -> Self {
        if value.len() <= 15 {
            Self::Arg(CompactArg::from_vec(value))
        } else {
            Self::Value(CompactValue::from_vec(value))
        }
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::Arg(value) => value.as_slice(),
            Self::Value(value) => value.as_slice(),
        }
    }

    #[inline(always)]
    pub fn into_vec(self) -> Vec<u8> {
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
    BulkOptions(Vec<Option<CompactValue>>),
    BulkValues(Vec<CompactValue>),
    PreEncoded(Bytes),
    Array(Option<Vec<RespFrame>>),
    Map(Vec<(RespFrame, RespFrame)>),
}

impl RespFrame {
    #[inline(always)]
    pub fn ok() -> Self {
        Self::SimpleStatic("OK")
    }

    #[inline(always)]
    pub fn simple_static(value: &'static str) -> Self {
        Self::SimpleStatic(value)
    }

    #[inline(always)]
    pub fn error_static(value: &'static str) -> Self {
        Self::ErrorStatic(value)
    }
}