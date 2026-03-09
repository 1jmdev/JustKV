use std::path::PathBuf;
use std::time::Duration;

use regex::Regex;

#[derive(Debug, Clone)]
pub struct TestFile {
    pub path: PathBuf,
    pub metadata: FileMetadata,
    pub cases: Vec<TestCase>,
}

#[derive(Debug, Clone, Default)]
pub struct FileMetadata {
    pub name: Option<String>,
    pub group: Option<String>,
    pub since: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: String,
    pub setup: Vec<String>,
    pub run: Vec<String>,
    pub expect: ExpectedValue,
    pub cleanup: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum ExpectedValue {
    Any,
    NoReply,
    Sequence(Vec<ExpectedValue>),
    Simple(String),
    Bulk(Option<Vec<u8>>),
    IntegerAny,
    Integer(i64),
    ErrorAny,
    ErrorPrefix(String),
    EmptyArray,
    RawRegex(Regex),
    Array {
        items: Vec<ExpectedValue>,
        unordered: bool,
    },
    Capture(String),
}

#[derive(Debug, Clone)]
pub struct TestFailure {
    pub path: PathBuf,
    pub test_name: String,
    pub elapsed: Duration,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct RunSummary {
    pub discovered_total: usize,
    pub total: usize,
    pub passed: usize,
    pub skipped: usize,
    pub failed: usize,
    pub elapsed: Duration,
    pub failures: Vec<TestFailure>,
    pub warnings: Vec<String>,
}
