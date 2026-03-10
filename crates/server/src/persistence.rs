use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use bytes::BytesMut;
use commands::dispatch::{CommandId, dispatch_args, identify, is_write_command_id};
use commands::transaction::TransactionState;
use engine::store::Store;
use protocol::parser::parse_command_into;
use protocol::types::RespFrame;
use types::value::CompactArg;

use crate::backup::{self, RestoreStats};
use crate::config::{AppendFsync, Config, SaveRule};

#[derive(Clone)]
pub struct PersistenceHandle {
    sender: Option<Sender<Request>>,
}

pub struct RestoreOutcome {
    pub snapshot: Option<RestoreStats>,
    pub aof_commands_replayed: u64,
    pub aof_tail_truncated: bool,
}

enum Request {
    Append { payload: Vec<u8>, dirty: u64 },
    Shutdown { reply: mpsc::Sender<Result<(), String>> },
}

struct PersistenceThread {
    config: Config,
    store: Store,
    receiver: Receiver<Request>,
    snapshot_path: PathBuf,
    aof_path: PathBuf,
    aof_file: Option<std::fs::File>,
    aof_size: u64,
    dirty_changes: u64,
    dirty_since: Option<Instant>,
    last_fsync: Instant,
}

impl PersistenceHandle {
    pub fn spawn(store: Store, config: Config) -> Self {
        if !persistence_enabled(&config) {
            return Self { sender: None };
        }

        let (sender, receiver) = mpsc::channel();
        let handle = Self {
            sender: Some(sender),
        };
        let thread_handle = handle.clone();
        let snapshot_path = config.snapshot_path();
        let aof_path = config.appendonly_path();
        thread::Builder::new()
            .name("betterkv-persistence".to_string())
            .spawn(move || {
                let thread = PersistenceThread::new(config, store, receiver, snapshot_path, aof_path);
                if let Err(err) = thread.run() {
                    tracing::error!(error = %err, "persistence thread exited with error");
                }
            })
            .map_err(|err| tracing::error!(error = %err, "failed to spawn persistence thread"))
            .ok();
        thread_handle
    }

    pub fn record_command(&self, command: CommandId, args: &[CompactArg], response: &RespFrame) {
        if self.sender.is_none() {
            return;
        }
        if !should_log_command(command, response) {
            return;
        }
        self.record_encoded_command(args, 1);
    }

    pub fn record_transaction(&self, commands: &[(CommandId, Vec<CompactArg>)]) {
        if self.sender.is_none() {
            return;
        }
        if commands.is_empty() {
            return;
        }

        let mut loggable = Vec::new();
        for (command, args) in commands {
            if is_aof_command(*command) {
                loggable.push((*command, args));
            }
        }
        if loggable.is_empty() {
            return;
        }

        let mut payload = encode_resp_command(&[CompactArg::from_slice(b"MULTI")]);
        for (_, args) in &loggable {
            payload.extend_from_slice(&encode_resp_command(args));
        }
        payload.extend_from_slice(&encode_resp_command(&[CompactArg::from_slice(b"EXEC")]));
        self.send_request(Request::Append {
            payload,
            dirty: loggable.len() as u64,
        });
    }

    pub fn shutdown(&self) -> Result<(), String> {
        let Some(sender) = &self.sender else {
            return Ok(());
        };
        let (reply_tx, reply_rx) = mpsc::channel();
        sender
            .send(Request::Shutdown { reply: reply_tx })
            .map_err(|err| format!("failed to send shutdown request: {err}"))?;
        reply_rx
            .recv()
            .map_err(|err| format!("failed to receive shutdown reply: {err}"))?
    }

    fn record_encoded_command(&self, args: &[CompactArg], dirty: u64) {
        self.send_request(Request::Append {
            payload: encode_resp_command(args),
            dirty,
        });
    }

    fn send_request(&self, request: Request) {
        let Some(sender) = &self.sender else {
            return;
        };
        if let Err(err) = sender.send(request) {
            tracing::error!(error = %err, "failed to send persistence request");
        }
    }
}

fn persistence_enabled(config: &Config) -> bool {
    config.appendonly || config.snapshot_on_shutdown || !config.save_rules.is_empty()
}

impl PersistenceThread {
    fn new(
        config: Config,
        store: Store,
        receiver: Receiver<Request>,
        snapshot_path: PathBuf,
        aof_path: PathBuf,
    ) -> Self {
        Self {
            config,
            store,
            receiver,
            snapshot_path,
            aof_path,
            aof_file: None,
            aof_size: 0,
            dirty_changes: 0,
            dirty_since: None,
            last_fsync: Instant::now(),
        }
    }

    fn run(mut self) -> Result<(), String> {
        self.open_aof_if_enabled()?;
        loop {
            match self.receiver.recv_timeout(Duration::from_secs(1)) {
                Ok(Request::Append { payload, dirty }) => {
                    self.append_payload(&payload, dirty)?;
                    self.drain_append_queue()?;
                    self.maybe_fsync()?;
                    self.maybe_snapshot_or_rewrite()?;
                }
                Ok(Request::Shutdown { reply }) => {
                    let result = self.handle_shutdown();
                    let _ = reply.send(result.clone());
                    return result;
                }
                Err(RecvTimeoutError::Timeout) => {
                    self.maybe_fsync()?;
                    self.maybe_snapshot_or_rewrite()?;
                }
                Err(RecvTimeoutError::Disconnected) => return Ok(()),
            }
        }
    }

    fn drain_append_queue(&mut self) -> Result<(), String> {
        let mut buffered = Vec::new();
        let mut dirty = 0u64;
        loop {
            match self.receiver.try_recv() {
                Ok(Request::Append {
                    payload,
                    dirty: more_dirty,
                }) => {
                    buffered.extend_from_slice(&payload);
                    dirty = dirty.saturating_add(more_dirty);
                }
                Ok(Request::Shutdown { reply }) => {
                    let result = self.handle_shutdown();
                    let _ = reply.send(result.clone());
                    return result;
                }
                Err(_) => break,
            }
        }

        if !buffered.is_empty() {
            self.append_payload(&buffered, dirty)?;
        }
        Ok(())
    }

    fn append_payload(&mut self, payload: &[u8], dirty: u64) -> Result<(), String> {
        if self.config.appendonly {
            let aof_path = self.aof_path.display().to_string();
            let next_size = self.aof_size.saturating_add(payload.len() as u64);
            let appendfsync = self.config.appendfsync;
            let file = self.ensure_aof_file()?;
            file.write_all(payload).map_err(|err| {
                format!("failed to append to appendonly file {aof_path}: {err}")
            })?;
            if appendfsync == AppendFsync::Always {
                file.sync_data().map_err(|err| {
                    format!("failed to fsync appendonly file {aof_path}: {err}")
                })?;
                self.last_fsync = Instant::now();
            }
            self.aof_size = next_size;
        }

        if dirty > 0 {
            self.dirty_changes = self.dirty_changes.saturating_add(dirty);
            if self.dirty_since.is_none() {
                self.dirty_since = Some(Instant::now());
            }
        }
        Ok(())
    }

    fn maybe_fsync(&mut self) -> Result<(), String> {
        if self.config.appendfsync != AppendFsync::EverySec || !self.config.appendonly {
            return Ok(());
        }
        if self.last_fsync.elapsed() < Duration::from_secs(1) {
            return Ok(());
        }

        if let Some(file) = &mut self.aof_file {
            file.sync_data().map_err(|err| {
                format!("failed to fsync appendonly file {}: {err}", self.aof_path.display())
            })?;
            self.last_fsync = Instant::now();
        }
        Ok(())
    }

    fn maybe_snapshot_or_rewrite(&mut self) -> Result<(), String> {
        if self.should_snapshot() || self.should_rewrite_aof() {
            self.write_snapshot("background")?;
        }
        Ok(())
    }

    fn should_snapshot(&self) -> bool {
        let Some(dirty_since) = self.dirty_since else {
            return false;
        };
        for SaveRule { seconds, changes } in &self.config.save_rules {
            if *seconds == 0 {
                continue;
            }
            if dirty_since.elapsed() >= Duration::from_secs(*seconds)
                && self.dirty_changes >= *changes
            {
                return true;
            }
        }
        false
    }

    fn should_rewrite_aof(&self) -> bool {
        self.config.appendonly
            && self.config.auto_aof_rewrite_percentage > 0
            && self.aof_size >= self.config.auto_aof_rewrite_min_size
            && self.dirty_changes > 0
    }

    fn handle_shutdown(&mut self) -> Result<(), String> {
        if self.config.snapshot_on_shutdown && self.dirty_changes > 0 {
            self.write_snapshot("shutdown")?;
        }
        if self.config.appendonly {
            if let Some(file) = &mut self.aof_file {
                file.sync_all().map_err(|err| {
                    format!("failed to sync appendonly file {}: {err}", self.aof_path.display())
                })?;
            }
        }
        Ok(())
    }

    fn write_snapshot(&mut self, reason: &str) -> Result<(), String> {
        let stats = backup::write_snapshot(
            &self.store,
            &self.snapshot_path,
            self.config.snapshot_compression,
        )?;
        tracing::info!(
            reason,
            keys_written = stats.keys_written,
            bytes_written = stats.bytes_written,
            uncompressed_bytes = stats.uncompressed_bytes,
            path = %self.snapshot_path.display(),
            "snapshot completed"
        );
        self.dirty_changes = 0;
        self.dirty_since = None;
        if self.config.appendonly {
            self.reset_aof()?;
        }
        Ok(())
    }

    fn open_aof_if_enabled(&mut self) -> Result<(), String> {
        if !self.config.appendonly {
            return Ok(());
        }
        if let Some(parent) = self.aof_path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "failed to create appendonly directory {}: {err}",
                    parent.display()
                )
            })?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&self.aof_path)
            .map_err(|err| {
                format!("failed to open appendonly file {}: {err}", self.aof_path.display())
            })?;
        let size = file
            .metadata()
            .map_err(|err| {
                format!("failed to stat appendonly file {}: {err}", self.aof_path.display())
            })?
            .len();
        self.aof_size = size;
        self.aof_file = Some(file);
        Ok(())
    }

    fn ensure_aof_file(&mut self) -> Result<&mut std::fs::File, String> {
        if self.aof_file.is_none() {
            self.open_aof_if_enabled()?;
        }
        self.aof_file
            .as_mut()
            .ok_or_else(|| "appendonly file is not available".to_string())
    }

    fn reset_aof(&mut self) -> Result<(), String> {
        let aof_path = self.aof_path.display().to_string();
        let file = self.ensure_aof_file()?;
        file.set_len(0).map_err(|err| {
            format!("failed to truncate appendonly file {aof_path}: {err}")
        })?;
        file.seek(SeekFrom::Start(0)).map_err(|err| {
            format!("failed to seek appendonly file {aof_path}: {err}")
        })?;
        file.sync_all().map_err(|err| {
            format!("failed to sync appendonly file {aof_path}: {err}")
        })?;
        self.aof_size = 0;
        self.last_fsync = Instant::now();
        Ok(())
    }
}

pub async fn restore(store: &Store, config: &Config) -> Result<RestoreOutcome, String> {
    let snapshot_path = config.snapshot_path();
    let aof_path = config.appendonly_path();
    let snapshot = if snapshot_path.exists() {
        Some(backup::load_snapshot(store, &snapshot_path).await?)
    } else {
        None
    };

    let mut aof_commands_replayed = 0u64;
    let mut aof_tail_truncated = false;
    if config.appendonly && aof_path.exists() {
        let bytes = tokio::fs::read(&aof_path)
            .await
            .map_err(|err| format!("failed to read appendonly file {}: {err}", aof_path.display()))?;
        let (replayed, truncated) = replay_aof(store, &bytes)?;
        aof_commands_replayed = replayed;
        aof_tail_truncated = truncated;
    }

    Ok(RestoreOutcome {
        snapshot,
        aof_commands_replayed,
        aof_tail_truncated,
    })
}

fn replay_aof(store: &Store, bytes: &[u8]) -> Result<(u64, bool), String> {
    let _trace = profiler::scope("server::persistence::replay_aof");
    let mut buffer = BytesMut::from(bytes);
    let mut args = Vec::with_capacity(16);
    let mut transaction_state = TransactionState::default();
    let mut replayed = 0u64;

    while parse_command_into(&mut buffer, &mut args)
        .map_err(|err| format!("failed to parse appendonly file: {err}"))?
        .is_some()
    {
        let command = identify(args[0].as_slice());
        let outcome = transaction_state.handle_args_with(store, &mut args, command, |inner_store, _, cmd_args| {
            dispatch_args(inner_store, cmd_args)
        });
        if response_is_error(&outcome.response) {
            return Err("appendonly replay failed due to command error".to_string());
        }
        replayed = replayed.saturating_add(1);
    }

    Ok((replayed, !buffer.is_empty()))
}

pub fn should_log_command(command: CommandId, response: &RespFrame) -> bool {
    !response_is_error(response) && !response_is_queued(response) && is_aof_command(command)
}

fn is_aof_command(command: CommandId) -> bool {
    is_write_command_id(command) || matches!(command, CommandId::Eval | CommandId::EvalSha)
}

fn response_is_error(response: &RespFrame) -> bool {
    matches!(response, RespFrame::Error(_) | RespFrame::ErrorStatic(_))
}

fn response_is_queued(response: &RespFrame) -> bool {
    matches!(response, RespFrame::SimpleStatic("QUEUED"))
        || matches!(response, RespFrame::Simple(value) if value == "QUEUED")
}

fn encode_resp_command(args: &[CompactArg]) -> Vec<u8> {
    let mut capacity = 16;
    for arg in args {
        capacity += 16 + arg.len();
    }
    let mut out = Vec::with_capacity(capacity);
    push_decimal_prefixed(&mut out, b'*', args.len() as u64);
    for arg in args {
        push_decimal_prefixed(&mut out, b'$', arg.len() as u64);
        out.extend_from_slice(arg.as_slice());
        out.extend_from_slice(b"\r\n");
    }
    out
}

fn push_decimal_prefixed(out: &mut Vec<u8>, prefix: u8, value: u64) {
    out.push(prefix);
    out.extend_from_slice(value.to_string().as_bytes());
    out.extend_from_slice(b"\r\n");
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use engine::store::Store;

    use super::*;
    use crate::config::SnapshotCompression;

    #[tokio::test]
    async fn snapshot_roundtrip_restores_data() {
        let store = Store::new(1);
        store.set(b"alpha", b"beta", None);

        let path = unique_temp_path("snapshot-roundtrip.rdb");
        let parent = path.parent().map(|value| value.to_path_buf());
        let stats = backup::write_snapshot(&store, &path, SnapshotCompression::Lz4)
            .expect("snapshot write should succeed");
        assert!(stats.keys_written >= 1);

        let restored = Store::new(1);
        let loaded = backup::load_snapshot(&restored, &path)
            .await
            .expect("snapshot load should succeed");
        assert_eq!(loaded.keys_loaded, 1);
        assert_eq!(
            restored.get(b"alpha").ok().flatten().as_deref(),
            Some(b"beta" as &[u8])
        );

        let _ = std::fs::remove_file(&path);
        if let Some(parent) = parent {
            let _ = std::fs::remove_dir(&parent);
        }
    }

    #[test]
    fn replay_aof_restores_transaction() {
        let store = Store::new(1);
        let multi = encode_resp_command(&[CompactArg::from_slice(b"MULTI")]);
        let set = encode_resp_command(&[
            CompactArg::from_slice(b"SET"),
            CompactArg::from_slice(b"tx:key"),
            CompactArg::from_slice(b"value"),
        ]);
        let exec = encode_resp_command(&[CompactArg::from_slice(b"EXEC")]);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&multi);
        bytes.extend_from_slice(&set);
        bytes.extend_from_slice(&exec);

        let (replayed, truncated) = replay_aof(&store, &bytes).expect("aof replay should succeed");
        assert_eq!(replayed, 3);
        assert!(!truncated);
        assert_eq!(
            store.get(b"tx:key").ok().flatten().as_deref(),
            Some(b"value" as &[u8])
        );
    }

    fn unique_temp_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_nanos();
        path.push(format!("betterkv-{unique}"));
        let _ = std::fs::create_dir_all(&path);
        path.push(name);
        path
    }
}
