use ahash::AHashMap;
use commands::command::CommandId;

use engine::store::Store;
use protocol::types::RespFrame;
use types::value::CompactArg;

#[derive(Default)]
pub struct TransactionState {
    in_multi: bool,
    queued: Vec<(CommandId, Vec<CompactArg>)>,
    watched: AHashMap<Vec<u8>, Option<Vec<u8>>>,
}

impl TransactionState {
    pub fn handle_args_with<F>(
        &mut self,
        store: &Store,
        args: &mut Vec<CompactArg>,
        command: CommandId,
        mut execute: F,
    ) -> RespFrame
    where
        F: FnMut(&Store, CommandId, &[CompactArg]) -> RespFrame,
    {
        let _trace = profiler::scope("server::transaction::handle_args_with");
        if args.is_empty() {
            return RespFrame::error_static("ERR empty command");
        }

        // hot path
        if !self.in_multi {
            match command {
                CommandId::Multi => return self.multi(args.as_slice()),
                CommandId::Exec => return RespFrame::error_static("ERR EXEC without MULTI"),
                CommandId::Discard => {
                    return RespFrame::error_static("ERR DISCARD without MULTI");
                }
                CommandId::Watch => return self.watch(store, args.as_slice()),
                CommandId::Unwatch => return self.unwatch(args.as_slice(), false),
                _ => {}
            }
            return execute(store, command, args.as_slice());
        }

        // cold path
        match TransactionCommand::from(command) {
            TransactionCommand::Multi => self.multi(args.as_slice()),
            TransactionCommand::Exec => self.exec_with(store, args.as_slice(), execute),
            TransactionCommand::Discard => self.discard(args.as_slice()),
            TransactionCommand::Watch => self.watch(store, args.as_slice()),
            TransactionCommand::Unwatch => self.unwatch(args.as_slice(), true),
            TransactionCommand::Other => {
                self.queued.push((command, std::mem::take(args)));
                RespFrame::simple_static("QUEUED")
            }
        }
    }

    fn multi(&mut self, args: &[CompactArg]) -> RespFrame {
        let _trace = profiler::scope("server::transaction::multi");
        if args.len() != 1 {
            return RespFrame::error_static("ERR wrong number of arguments for 'multi' command");
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
        F: FnMut(&Store, CommandId, &[CompactArg]) -> RespFrame,
    {
        let _trace = profiler::scope("server::transaction::exec_with");
        if args.len() != 1 {
            return RespFrame::error_static("ERR wrong number of arguments for 'exec' command");
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
        for (command, item) in queued {
            out.push(execute(store, command, item.as_slice()));
        }
        self.watched.clear();
        RespFrame::Array(Some(out))
    }

    fn discard(&mut self, args: &[CompactArg]) -> RespFrame {
        let _trace = profiler::scope("server::transaction::discard");
        if args.len() != 1 {
            return RespFrame::error_static("ERR wrong number of arguments for 'discard' command");
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
            return RespFrame::error_static("ERR wrong number of arguments for 'watch' command");
        }
        if self.in_multi {
            return RespFrame::error_static("ERR WATCH inside MULTI is not allowed");
        }

        for key in &args[1..] {
            self.watched.insert(key.to_vec(), store.dump(key));
        }
        RespFrame::ok()
    }

    fn unwatch(&mut self, args: &[CompactArg], queue_in_multi: bool) -> RespFrame {
        let _trace = profiler::scope("server::transaction::unwatch");
        if args.len() != 1 {
            return RespFrame::error_static("ERR wrong number of arguments for 'unwatch' command");
        }
        if queue_in_multi && self.in_multi {
            return RespFrame::simple_static("QUEUED");
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

impl From<CommandId> for TransactionCommand {
    fn from(command: CommandId) -> Self {
        match command {
            CommandId::Multi => Self::Multi,
            CommandId::Exec => Self::Exec,
            CommandId::Discard => Self::Discard,
            CommandId::Watch => Self::Watch,
            CommandId::Unwatch => Self::Unwatch,
            _ => Self::Other,
        }
    }
}
