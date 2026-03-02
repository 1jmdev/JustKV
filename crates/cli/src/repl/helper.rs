use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper, Result};

const COMMANDS: &[&str] = &[
    "APPEND",
    "AUTH",
    "CLIENT",
    "COMMAND",
    "COPY",
    "DBSIZE",
    "DECR",
    "DECRBY",
    "DEL",
    "DISCARD",
    "ECHO",
    "EXEC",
    "EXISTS",
    "EXPIRE",
    "EXPIREAT",
    "FLUSHDB",
    "GET",
    "GETDEL",
    "GETSET",
    "HELLO",
    "INCR",
    "INCRBY",
    "KEYS",
    "MOVE",
    "MGET",
    "MSET",
    "MSETNX",
    "MULTI",
    "PEXPIRE",
    "PEXPIREAT",
    "PERSIST",
    "PING",
    "PTTL",
    "PUBLISH",
    "PUBSUB",
    "PSUBSCRIBE",
    "PUNSUBSCRIBE",
    "QUIT",
    "RENAME",
    "RENAMENX",
    "RESTORE",
    "SCAN",
    "SELECT",
    "SET",
    "SETNX",
    "STRLEN",
    "SORT",
    "TOUCH",
    "TTL",
    "TYPE",
    "DUMP",
    "SUBSCRIBE",
    "UNLINK",
    "UNSUBSCRIBE",
    "UNWATCH",
    "WATCH",
];

const META_COMMANDS: &[&str] = &[":clear", ":help", ":history", ":raw", ":quit"];

pub struct ReplHelper;

impl ReplHelper {
    pub fn new() -> Self {
        let _trace = profiler::scope("cli::repl::helper::new");
        Self
    }
}

impl Completer for ReplHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _: &Context<'_>) -> Result<(usize, Vec<Pair>)> {
        let _trace = profiler::scope("cli::repl::helper::complete");
        let pos = pos.min(line.len());
        let prefix = &line[..pos];
        let start = prefix.rfind(char::is_whitespace).map_or(0, |idx| idx + 1);
        let token = &prefix[start..];

        if token.is_empty() {
            return Ok((start, vec![]));
        }

        let head_only = prefix[..start].trim().is_empty();
        let mut candidates = Vec::new();

        if head_only {
            candidates.extend(COMMANDS.iter().copied());
            candidates.extend(META_COMMANDS.iter().copied());
        }

        let needle = token.to_ascii_lowercase();
        let mut pairs = candidates
            .into_iter()
            .filter(|item| item.to_ascii_lowercase().starts_with(needle.as_str()))
            .map(|item| Pair {
                display: item.to_string(),
                replacement: item.to_string(),
            })
            .collect::<Vec<Pair>>();
        pairs.sort_unstable_by(|left, right| left.display.cmp(&right.display));

        Ok((start, pairs))
    }
}

impl Hinter for ReplHelper {
    type Hint = String;
}

impl Highlighter for ReplHelper {}

impl Validator for ReplHelper {}

impl Helper for ReplHelper {}
