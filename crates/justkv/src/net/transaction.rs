use std::collections::HashMap;

use crate::engine::store::Store;
use crate::protocol::types::{BulkData, RespFrame};

#[derive(Default)]
pub struct TransactionState {
    in_multi: bool,
    queued: Vec<RespFrame>,
    watched: HashMap<Vec<u8>, Option<Vec<u8>>>,
}

impl TransactionState {
    pub fn handle_frame_with<F>(
        &mut self,
        store: &Store,
        frame: RespFrame,
        mut execute: F,
    ) -> RespFrame
    where
        F: FnMut(&Store, RespFrame) -> RespFrame,
    {
        let args = match parse_args(&frame) {
            Ok(value) => value,
            Err(err) => return RespFrame::Error(err),
        };

        if args.is_empty() {
            return RespFrame::Error("ERR empty command".to_string());
        }

        let command = args[0].as_slice();
        if command.eq_ignore_ascii_case(b"MULTI") {
            return self.multi(&args);
        }
        if command.eq_ignore_ascii_case(b"EXEC") {
            return self.exec_with(store, &args, execute);
        }
        if command.eq_ignore_ascii_case(b"DISCARD") {
            return self.discard(&args);
        }
        if command.eq_ignore_ascii_case(b"WATCH") {
            return self.watch(store, &args);
        }
        if command.eq_ignore_ascii_case(b"UNWATCH") {
            return self.unwatch(&args);
        }

        if self.in_multi {
            self.queued.push(frame);
            return RespFrame::Simple("QUEUED".to_string());
        }

        execute(store, frame)
    }

    fn multi(&mut self, args: &[Vec<u8>]) -> RespFrame {
        if args.len() != 1 {
            return wrong_args("MULTI");
        }
        if self.in_multi {
            return RespFrame::Error("ERR MULTI calls can not be nested".to_string());
        }

        self.in_multi = true;
        self.queued.clear();
        RespFrame::ok()
    }

    fn exec_with<F>(&mut self, store: &Store, args: &[Vec<u8>], mut execute: F) -> RespFrame
    where
        F: FnMut(&Store, RespFrame) -> RespFrame,
    {
        if args.len() != 1 {
            return wrong_args("EXEC");
        }
        if !self.in_multi {
            return RespFrame::Error("ERR EXEC without MULTI".to_string());
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
            out.push(execute(store, item));
        }
        self.watched.clear();
        RespFrame::Array(Some(out))
    }

    fn discard(&mut self, args: &[Vec<u8>]) -> RespFrame {
        if args.len() != 1 {
            return wrong_args("DISCARD");
        }
        if !self.in_multi {
            return RespFrame::Error("ERR DISCARD without MULTI".to_string());
        }

        self.in_multi = false;
        self.queued.clear();
        self.watched.clear();
        RespFrame::ok()
    }

    fn watch(&mut self, store: &Store, args: &[Vec<u8>]) -> RespFrame {
        if args.len() < 2 {
            return wrong_args("WATCH");
        }
        if self.in_multi {
            return RespFrame::Error("ERR WATCH inside MULTI is not allowed".to_string());
        }

        for key in &args[1..] {
            self.watched.insert(key.clone(), store.dump(key));
        }
        RespFrame::ok()
    }

    fn unwatch(&mut self, args: &[Vec<u8>]) -> RespFrame {
        if args.len() != 1 {
            return wrong_args("UNWATCH");
        }
        self.watched.clear();
        RespFrame::ok()
    }

    fn is_watch_dirty(&self, store: &Store) -> bool {
        self.watched
            .iter()
            .any(|(key, value)| store.dump(key) != *value)
    }
}

fn parse_args(frame: &RespFrame) -> Result<Vec<Vec<u8>>, String> {
    let RespFrame::Array(Some(items)) = frame else {
        return Err("ERR protocol error".to_string());
    };

    let mut args = Vec::with_capacity(items.len());
    for item in items {
        match item {
            RespFrame::Bulk(Some(BulkData::Arg(bytes))) => args.push(bytes.to_vec()),
            RespFrame::Bulk(Some(BulkData::Value(bytes))) => args.push(bytes.to_vec()),
            RespFrame::Simple(value) => args.push(value.as_bytes().to_vec()),
            _ => return Err("ERR invalid argument type".to_string()),
        }
    }

    Ok(args)
}

fn wrong_args(command: &str) -> RespFrame {
    RespFrame::Error(format!(
        "ERR wrong number of arguments for '{command}' command"
    ))
}
