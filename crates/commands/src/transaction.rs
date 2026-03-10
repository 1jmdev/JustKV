use crate::dispatch::CommandId;
use crate::util::{Args, wrong_args};
use engine::store::Store;
use engine::transaction::WatchState;
use protocol::types::RespFrame;
use types::value::CompactArg;

#[derive(Default)]
pub struct TransactionState {
    in_multi: bool,
    queued: Vec<(CommandId, Vec<CompactArg>)>,
    watched: WatchState,
}

pub struct TransactionOutcome {
    pub response: RespFrame,
    pub committed_commands: Vec<(CommandId, Vec<CompactArg>)>,
}

impl TransactionState {
    pub fn cleanup(&mut self, _store: &Store) {
        self.in_multi = false;
        self.queued.clear();
        self.watched.clear();
    }

    pub fn handle_args_with<F>(
        &mut self,
        store: &Store,
        args: &mut Vec<CompactArg>,
        command: CommandId,
        mut execute: F,
    ) -> TransactionOutcome
    where
        F: FnMut(&Store, CommandId, &[CompactArg]) -> RespFrame,
    {
        let _trace = profiler::scope("commands::transaction::handle_args_with");
        if args.is_empty() {
            return TransactionOutcome {
                response: RespFrame::error_static("ERR empty command"),
                committed_commands: Vec::new(),
            };
        }

        if !self.in_multi {
            let response = match TransactionCommand::from(command) {
                TransactionCommand::Multi => {
                    multi(args.as_slice(), &mut self.in_multi, &mut self.queued)
                }
                TransactionCommand::Exec => RespFrame::error_static("ERR EXEC without MULTI"),
                TransactionCommand::Discard => RespFrame::error_static("ERR DISCARD without MULTI"),
                TransactionCommand::Watch => store
                    .with_watch_gate(|| watch(store, args.as_slice(), &mut self.watched, false)),
                TransactionCommand::Unwatch => unwatch(store, args.as_slice(), &mut self.watched),
                TransactionCommand::Other => {
                    store.with_command_gate(|| execute(store, command, args.as_slice()))
                }
            };
            return TransactionOutcome {
                response,
                committed_commands: Vec::new(),
            };
        }

        match TransactionCommand::from(command) {
            TransactionCommand::Multi => TransactionOutcome {
                response: multi(args.as_slice(), &mut self.in_multi, &mut self.queued),
                committed_commands: Vec::new(),
            },
            TransactionCommand::Exec => exec_with(
                store,
                args.as_slice(),
                &mut self.in_multi,
                &mut self.queued,
                &mut self.watched,
                execute,
            ),
            TransactionCommand::Discard => TransactionOutcome {
                response: discard(
                    store,
                    args.as_slice(),
                    &mut self.in_multi,
                    &mut self.queued,
                    &mut self.watched,
                ),
                committed_commands: Vec::new(),
            },
            TransactionCommand::Watch => TransactionOutcome {
                response: watch(store, args.as_slice(), &mut self.watched, true),
                committed_commands: Vec::new(),
            },
            TransactionCommand::Unwatch | TransactionCommand::Other => {
                self.queued.push((command, std::mem::take(args)));
                TransactionOutcome {
                    response: RespFrame::simple_static("QUEUED"),
                    committed_commands: Vec::new(),
                }
            }
        }
    }
}

pub(crate) fn multi_command(args: &Args) -> RespFrame {
    stateful_command_error(args, "MULTI")
}

pub(crate) fn exec_command(args: &Args) -> RespFrame {
    stateful_command_error(args, "EXEC")
}

pub(crate) fn discard_command(args: &Args) -> RespFrame {
    stateful_command_error(args, "DISCARD")
}

pub(crate) fn watch_command(args: &Args) -> RespFrame {
    stateful_command_error(args, "WATCH")
}

pub(crate) fn unwatch_command(args: &Args) -> RespFrame {
    stateful_command_error(args, "UNWATCH")
}

fn stateful_command_error(args: &Args, command: &str) -> RespFrame {
    if args.is_empty() {
        return RespFrame::error_static("ERR empty command");
    }

    RespFrame::Error(format!(
        "ERR {command} requires connection transaction state"
    ))
}

fn multi(
    args: &Args,
    in_multi: &mut bool,
    queued: &mut Vec<(CommandId, Vec<CompactArg>)>,
) -> RespFrame {
    let _trace = profiler::scope("commands::transaction::multi");
    if args.len() != 1 {
        return wrong_args("MULTI");
    }
    if *in_multi {
        return RespFrame::error_static("ERR MULTI calls can not be nested");
    }

    *in_multi = true;
    queued.clear();
    RespFrame::ok()
}

fn exec_with<F>(
    store: &Store,
    args: &Args,
    in_multi: &mut bool,
    queued: &mut Vec<(CommandId, Vec<CompactArg>)>,
    watched: &mut WatchState,
    mut execute: F,
) -> TransactionOutcome
where
    F: FnMut(&Store, CommandId, &[CompactArg]) -> RespFrame,
{
    let _trace = profiler::scope("commands::transaction::exec_with");
    if args.len() != 1 {
        return TransactionOutcome {
            response: wrong_args("EXEC"),
            committed_commands: Vec::new(),
        };
    }
    if !*in_multi {
        return TransactionOutcome {
            response: RespFrame::error_static("ERR EXEC without MULTI"),
            committed_commands: Vec::new(),
        };
    }

    *in_multi = false;
    store.with_transaction_gate(|| {
        if watched.is_dirty(store) {
            queued.clear();
            watched.clear();
            return TransactionOutcome {
                response: RespFrame::Array(None),
                committed_commands: Vec::new(),
            };
        }

        let queued_commands = std::mem::take(queued);
        let mut responses = Vec::with_capacity(queued_commands.len());
        let mut committed_commands = Vec::with_capacity(queued_commands.len());
        for (queued_command, queued_args) in queued_commands {
            if queued_command == CommandId::Unwatch {
                watched.clear();
                responses.push(RespFrame::ok());
                continue;
            }
            responses.push(execute(store, queued_command, queued_args.as_slice()));
            committed_commands.push((queued_command, queued_args));
        }
        watched.clear();
        TransactionOutcome {
            response: RespFrame::Array(Some(responses)),
            committed_commands,
        }
    })
}

fn discard(
    _store: &Store,
    args: &Args,
    in_multi: &mut bool,
    queued: &mut Vec<(CommandId, Vec<CompactArg>)>,
    watched: &mut WatchState,
) -> RespFrame {
    let _trace = profiler::scope("commands::transaction::discard");
    if args.len() != 1 {
        return wrong_args("DISCARD");
    }
    if !*in_multi {
        return RespFrame::error_static("ERR DISCARD without MULTI");
    }

    *in_multi = false;
    queued.clear();
    watched.clear();
    RespFrame::ok()
}

fn watch(store: &Store, args: &Args, watched: &mut WatchState, in_multi: bool) -> RespFrame {
    let _trace = profiler::scope("commands::transaction::watch");
    if args.len() < 2 {
        return wrong_args("WATCH");
    }
    if in_multi {
        return RespFrame::error_static("ERR WATCH inside MULTI is not allowed");
    }

    for key in &args[1..] {
        watched.watch(store, key.as_slice());
    }
    RespFrame::ok()
}

fn unwatch(_store: &Store, args: &Args, watched: &mut WatchState) -> RespFrame {
    let _trace = profiler::scope("commands::transaction::unwatch");
    if args.len() != 1 {
        return wrong_args("UNWATCH");
    }

    watched.clear();
    RespFrame::ok()
}

#[derive(Clone, Copy, PartialEq, Eq)]
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
