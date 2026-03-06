use ahash::AHashMap;

use engine::store::Store;
use protocol::types::RespFrame;
use types::value::CompactArg;

#[derive(Default)]
pub struct TransactionState {
    in_multi: bool,
    queued: Vec<Vec<CompactArg>>,
    watched: AHashMap<Vec<u8>, Option<Vec<u8>>>,
}

impl TransactionState {
    pub fn handle_args_with<F>(
        &mut self,
        store: &Store,
        args: &mut Vec<CompactArg>,
        mut execute: F,
    ) -> RespFrame
    where
        F: FnMut(&Store, &[CompactArg]) -> RespFrame,
    {
        let _trace = profiler::scope("server::transaction::handle_args_with");
        if args.is_empty() {
            return RespFrame::error_static("ERR empty command");
        }

        // hot path
        if !self.in_multi {
            let cmd = args[0].as_slice();
            match cmd.first().copied() {
                Some(b'M') if cmd == b"MULTI" => return self.multi(args.as_slice()),
                Some(b'E') if cmd == b"EXEC" => {
                    return RespFrame::error_static("ERR EXEC without MULTI");
                }
                Some(b'D') if cmd == b"DISCARD" => {
                    return RespFrame::error_static("ERR DISCARD without MULTI");
                }
                Some(b'W') if cmd == b"WATCH" => return self.watch(store, args.as_slice()),
                Some(b'U') if cmd == b"UNWATCH" => return self.unwatch(args.as_slice()),
                _ => {}
            }
            return execute(store, args.as_slice());
        }

        // cold path
        match classify_transaction_command(args[0].as_slice()) {
            TransactionCommand::Multi => self.multi(args.as_slice()),
            TransactionCommand::Exec => self.exec_with(store, args.as_slice(), execute),
            TransactionCommand::Discard => self.discard(args.as_slice()),
            TransactionCommand::Watch => self.watch(store, args.as_slice()),
            TransactionCommand::Unwatch => self.unwatch(args.as_slice()),
            TransactionCommand::Other => {
                self.queued.push(std::mem::take(args));
                RespFrame::simple_static("QUEUED")
            }
        }
    }

    fn multi(&mut self, args: &[CompactArg]) -> RespFrame {
        let _trace = profiler::scope("server::transaction::multi");
        if args.len() != 1 {
            return RespFrame::error_static("ERR wrong number of arguments for 'MULTI' command");
        }
        if self.in_multi {
            return RespFrame::error_static("ERR MULTI calls can not be nested");
        }

        self.in_multi = true;
        self.queued.clear();
        RespFrame::ok()
    }

    fn exec_with<F>(&mut self, store: &Store, args: &[CompactArg], mut execute: F) -> RespFrame
    where
        F: FnMut(&Store, &[CompactArg]) -> RespFrame,
    {
        let _trace = profiler::scope("server::transaction::exec_with");
        if args.len() != 1 {
            return RespFrame::error_static("ERR wrong number of arguments for 'EXEC' command");
        }
        if !self.in_multi {
            return RespFrame::error_static("ERR EXEC without MULTI");
        }

        self.in_multi = false;
        if self.is_watch_dirty(store) {
            self.queued.clear();
            self.watched.clear();
            return RespFrame::Array(None);
        }

        let queued = std::mem::take(&mut self.queued);
        let mut out = Vec::with_capacity(queued.len());
        for item in queued {
            out.push(execute(store, item.as_slice()));
        }
        self.watched.clear();
        RespFrame::Array(Some(out))
    }

    fn discard(&mut self, args: &[CompactArg]) -> RespFrame {
        let _trace = profiler::scope("server::transaction::discard");
        if args.len() != 1 {
            return RespFrame::error_static("ERR wrong number of arguments for 'DISCARD' command");
        }
        if !self.in_multi {
            return RespFrame::error_static("ERR DISCARD without MULTI");
        }

        self.in_multi = false;
        self.queued.clear();
        self.watched.clear();
        RespFrame::ok()
    }

    fn watch(&mut self, store: &Store, args: &[CompactArg]) -> RespFrame {
        let _trace = profiler::scope("server::transaction::watch");
        if args.len() < 2 {
            return RespFrame::error_static("ERR wrong number of arguments for 'WATCH' command");
        }
        if self.in_multi {
            return RespFrame::error_static("ERR WATCH inside MULTI is not allowed");
        }

        for key in &args[1..] {
            self.watched.insert(key.to_vec(), store.dump(key));
        }
        RespFrame::ok()
    }

    fn unwatch(&mut self, args: &[CompactArg]) -> RespFrame {
        let _trace = profiler::scope("server::transaction::unwatch");
        if args.len() != 1 {
            return RespFrame::error_static("ERR wrong number of arguments for 'UNWATCH' command");
        }
        self.watched.clear();
        RespFrame::ok()
    }

    fn is_watch_dirty(&self, store: &Store) -> bool {
        let _trace = profiler::scope("server::transaction::is_watch_dirty");
        self.watched
            .iter()
            .any(|(key, value)| store.dump(key) != *value)
    }
}

#[derive(PartialEq, Eq)]
enum TransactionCommand {
    Multi,
    Exec,
    Discard,
    Watch,
    Unwatch,
    Other,
}

#[inline]
fn classify_transaction_command(command: &[u8]) -> TransactionCommand {
    match command.first().copied() {
        Some(b'M') if command == b"MULTI" => TransactionCommand::Multi,
        Some(b'E') if command == b"EXEC" => TransactionCommand::Exec,
        Some(b'D') if command == b"DISCARD" => TransactionCommand::Discard,
        Some(b'W') if command == b"WATCH" => TransactionCommand::Watch,
        Some(b'U') if command == b"UNWATCH" => TransactionCommand::Unwatch,
        _ => TransactionCommand::Other,
    }
}
