use std::fs::OpenOptions;
use std::io::{IoSlice, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use bytes::BytesMut;
use commands::dispatch::{CommandId, dispatch_args, identify, is_write_command_id};
use commands::transaction::TransactionState;
use engine::store::Store;
use itoa::Buffer as ItoaBuffer;
use parking_lot::{Condvar, Mutex};
use protocol::parser::parse_command_into;
use protocol::types::RespFrame;
use smallvec::SmallVec;
use types::value::CompactArg;

use crate::backup::{self, RestoreStats};
use crate::config::{AppendFsync, Config, SaveRule};

const AOF_NOTIFY_BYTES: usize = 64 * 1024;
const PERSISTENCE_TICK: Duration = Duration::from_millis(200);
const VECTORED_WRITE_BATCH: usize = 32;
const SPARE_BUFFER_POOL_LIMIT: usize = 128;
const MAX_RECYCLED_BUFFER_CAPACITY: usize = 256 * 1024;

#[derive(Clone)]
pub struct PersistenceHandle {
    state: Option<Arc<SharedState>>,
    appendonly_enabled: bool,
}

pub struct RestoreOutcome {
    pub snapshot: Option<RestoreStats>,
    pub aof_commands_replayed: u64,
    pub aof_tail_truncated: bool,
}

struct SharedState {
    mutex: Mutex<ProducerState>,
    condvar: Condvar,
}

struct ProducerState {
    pending_chunks: Vec<Vec<u8>>,
    pending_bytes_len: usize,
    pending_dirty: u64,
    spare_chunks: Vec<Vec<u8>>,
    shutdown_requested: bool,
    shutdown_complete: bool,
    shutdown_error: Option<String>,
}

struct PersistenceThread {
    config: Config,
    store: Store,
    snapshot_path: PathBuf,
    aof_path: PathBuf,
    aof_file: Option<std::fs::File>,
    aof_size: u64,
    dirty_changes: u64,
    dirty_since: Option<Instant>,
    last_fsync: Instant,
    shared: Arc<SharedState>,
    drained_chunks: Vec<Vec<u8>>,
}

impl PersistenceHandle {
    pub fn spawn(store: Store, config: Config) -> Self {
        if !persistence_enabled(&config) {
            return Self {
                state: None,
                appendonly_enabled: false,
            };
        }

        let shared = Arc::new(SharedState {
            mutex: Mutex::new(ProducerState {
                pending_chunks: Vec::new(),
                pending_bytes_len: 0,
                pending_dirty: 0,
                spare_chunks: Vec::new(),
                shutdown_requested: false,
                shutdown_complete: false,
                shutdown_error: None,
            }),
            condvar: Condvar::new(),
        });

        let thread_shared = Arc::clone(&shared);
        let snapshot_path = config.snapshot_path();
        let aof_path = config.appendonly_path();
        let appendonly_enabled = config.appendonly;
        thread::Builder::new()
            .name("betterkv-persistence".to_string())
            .spawn(move || {
                let thread =
                    PersistenceThread::new(config, store, snapshot_path, aof_path, thread_shared);
                if let Err(err) = thread.run() {
                    tracing::error!(error = %err, "persistence thread exited with error");
                }
            })
            .map_err(|err| tracing::error!(error = %err, "failed to spawn persistence thread"))
            .ok();

        Self {
            state: Some(shared),
            appendonly_enabled,
        }
    }

    pub fn record_command(&self, command: CommandId, args: &[CompactArg], response: &RespFrame) {
        let mut local_bytes = Vec::with_capacity(512);
        let mut local_dirty = 0;
        self.record_command_to_buffer(command, args, response, &mut local_bytes, &mut local_dirty);
        self.flush_buffer(&mut local_bytes, &mut local_dirty);
    }

    #[inline(always)]
    pub fn is_enabled(&self) -> bool {
        self.state.is_some()
    }

    pub fn record_command_to_buffer(
        &self,
        command: CommandId,
        args: &[CompactArg],
        response: &RespFrame,
        local_bytes: &mut Vec<u8>,
        local_dirty: &mut u64,
    ) {
        if self.state.is_none() {
            return;
        }
        if !should_track_dirty(command, response) {
            return;
        }
        *local_dirty = local_dirty.saturating_add(1);
        if !self.appendonly_enabled {
            return;
        }
        encode_resp_command_into(local_bytes, args);
    }

    pub fn record_transaction(&self, commands: &[(CommandId, Vec<CompactArg>)]) {
        let mut local_bytes = Vec::with_capacity(512);
        let mut local_dirty = 0;
        self.record_transaction_to_buffer(commands, &mut local_bytes, &mut local_dirty);
        self.flush_buffer(&mut local_bytes, &mut local_dirty);
    }

    pub fn record_transaction_to_buffer(
        &self,
        commands: &[(CommandId, Vec<CompactArg>)],
        local_bytes: &mut Vec<u8>,
        local_dirty: &mut u64,
    ) {
        if self.state.is_none() || commands.is_empty() {
            return;
        }

        let dirty_increment = commands
            .iter()
            .filter(|(command, _)| is_aof_command(*command))
            .count() as u64;
        if dirty_increment == 0 {
            return;
        }
        *local_dirty = local_dirty.saturating_add(dirty_increment);
        if !self.appendonly_enabled {
            return;
        }

        let start_len = local_bytes.len();
        let start_dirty = *local_dirty;
        let mut has_logged_command = false;
        encode_resp_command_into(local_bytes, &[CompactArg::from_slice(b"MULTI")]);
        for (command, args) in commands {
            if !is_aof_command(*command) {
                continue;
            }
            has_logged_command = true;
            encode_resp_command_into(local_bytes, args);
        }
        if !has_logged_command {
            local_bytes.truncate(start_len);
            *local_dirty = start_dirty;
            return;
        }
        encode_resp_command_into(local_bytes, &[CompactArg::from_slice(b"EXEC")]);
    }

    pub fn shutdown(&self) -> Result<(), String> {
        let Some(shared) = &self.state else {
            return Ok(());
        };

        let mut guard = shared.mutex.lock();
        guard.shutdown_requested = true;
        shared.condvar.notify_one();
        while !guard.shutdown_complete {
            shared.condvar.wait(&mut guard);
        }
        match &guard.shutdown_error {
            Some(err) => Err(err.clone()),
            None => Ok(()),
        }
    }

    pub fn flush_buffer(&self, local_bytes: &mut Vec<u8>, local_dirty: &mut u64) {
        if self.state.is_none() {
            return;
        }
        if local_bytes.is_empty() && *local_dirty == 0 {
            return;
        }

        self.append_chunk(local_bytes, *local_dirty);
        *local_dirty = 0;
    }

    fn append_chunk(&self, local_bytes: &mut Vec<u8>, dirty: u64) {
        let Some(shared) = &self.state else {
            return;
        };
        if local_bytes.is_empty() && dirty == 0 {
            return;
        }

        let target_capacity = local_bytes.capacity().max(1024);
        let mut guard = shared.mutex.lock();
        let was_idle = guard.pending_bytes_len == 0 && guard.pending_dirty == 0;
        if !local_bytes.is_empty() {
            let chunk = std::mem::take(local_bytes);
            guard.pending_bytes_len = guard.pending_bytes_len.saturating_add(chunk.len());
            guard.pending_chunks.push(chunk);
        }
        guard.pending_dirty = guard.pending_dirty.saturating_add(dirty);
        if let Some(mut spare_chunk) = guard.spare_chunks.pop() {
            spare_chunk.clear();
            *local_bytes = spare_chunk;
        }
        let should_notify = was_idle || guard.pending_bytes_len >= AOF_NOTIFY_BYTES;
        drop(guard);
        if local_bytes.capacity() == 0 {
            *local_bytes = Vec::with_capacity(target_capacity);
        } else {
            local_bytes.clear();
        }
        if should_notify {
            shared.condvar.notify_one();
        }
    }
}

impl PersistenceThread {
    fn new(
        config: Config,
        store: Store,
        snapshot_path: PathBuf,
        aof_path: PathBuf,
        shared: Arc<SharedState>,
    ) -> Self {
        Self {
            config,
            store,
            snapshot_path,
            aof_path,
            aof_file: None,
            aof_size: 0,
            dirty_changes: 0,
            dirty_since: None,
            last_fsync: Instant::now(),
            shared,
            drained_chunks: Vec::new(),
        }
    }

    fn run(mut self) -> Result<(), String> {
        self.open_aof_if_enabled()?;

        loop {
            let (shutdown_requested, drained_dirty) = self.wait_for_work();

            if !self.drained_chunks.is_empty() {
                let mut drained_chunks = Vec::new();
                std::mem::swap(&mut drained_chunks, &mut self.drained_chunks);
                self.append_payload(drained_chunks.as_slice(), drained_dirty)?;
                self.recycle_drained_chunks(drained_chunks);
            }

            self.maybe_fsync()?;
            self.maybe_snapshot_or_rewrite()?;

            if shutdown_requested {
                let result = self.handle_shutdown();
                self.finish_shutdown(result.clone());
                return result;
            }
        }
    }

    fn wait_for_work(&mut self) -> (bool, u64) {
        let mut guard = self.shared.mutex.lock();
        if guard.pending_chunks.is_empty() && !guard.shutdown_requested {
            self.shared.condvar.wait_for(&mut guard, PERSISTENCE_TICK);
        }

        let shutdown_requested = guard.shutdown_requested;
        let drained_dirty = guard.pending_dirty;
        std::mem::swap(&mut self.drained_chunks, &mut guard.pending_chunks);
        guard.pending_bytes_len = 0;
        guard.pending_dirty = 0;
        (shutdown_requested, drained_dirty)
    }

    fn finish_shutdown(&self, result: Result<(), String>) {
        let mut guard = self.shared.mutex.lock();
        guard.shutdown_complete = true;
        guard.shutdown_error = result.err();
        self.shared.condvar.notify_all();
    }

    fn append_payload(&mut self, payload: &[Vec<u8>], dirty: u64) -> Result<(), String> {
        if self.config.appendonly {
            let aof_path = self.aof_path.display().to_string();
            let payload_len: usize = payload.iter().map(Vec::len).sum();
            let next_size = self.aof_size.saturating_add(payload_len as u64);
            let appendfsync = self.config.appendfsync;
            let file = self.ensure_aof_file()?;
            write_chunks_vectored(file, payload)
                .map_err(|err| format!("failed to append to appendonly file {aof_path}: {err}"))?;
            if appendfsync == AppendFsync::Always {
                file.sync_data()
                    .map_err(|err| format!("failed to fsync appendonly file {aof_path}: {err}"))?;
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

    fn recycle_drained_chunks(&mut self, drained_chunks: Vec<Vec<u8>>) {
        let mut guard = self.shared.mutex.lock();
        for mut chunk in drained_chunks {
            if guard.spare_chunks.len() >= SPARE_BUFFER_POOL_LIMIT {
                continue;
            }
            if chunk.capacity() > MAX_RECYCLED_BUFFER_CAPACITY {
                continue;
            }
            chunk.clear();
            guard.spare_chunks.push(chunk);
        }
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
                format!(
                    "failed to fsync appendonly file {}: {err}",
                    self.aof_path.display()
                )
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
        if self.config.appendonly
            && let Some(file) = &mut self.aof_file
        {
            file.sync_all().map_err(|err| {
                format!(
                    "failed to sync appendonly file {}: {err}",
                    self.aof_path.display()
                )
            })?;
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
                format!(
                    "failed to open appendonly file {}: {err}",
                    self.aof_path.display()
                )
            })?;
        let size = file
            .metadata()
            .map_err(|err| {
                format!(
                    "failed to stat appendonly file {}: {err}",
                    self.aof_path.display()
                )
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
        file.set_len(0)
            .map_err(|err| format!("failed to truncate appendonly file {aof_path}: {err}"))?;
        file.seek(SeekFrom::Start(0))
            .map_err(|err| format!("failed to seek appendonly file {aof_path}: {err}"))?;
        file.sync_all()
            .map_err(|err| format!("failed to sync appendonly file {aof_path}: {err}"))?;
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
        let bytes = tokio::fs::read(&aof_path).await.map_err(|err| {
            format!(
                "failed to read appendonly file {}: {err}",
                aof_path.display()
            )
        })?;
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
    let mut buffer = BytesMut::from(bytes);
    let mut args = Vec::with_capacity(16);
    let mut transaction_state = TransactionState::default();
    let mut replayed = 0u64;

    while parse_command_into(&mut buffer, &mut args)
        .map_err(|err| format!("failed to parse appendonly file: {err}"))?
        .is_some()
    {
        let command = identify(args[0].as_slice());
        let outcome = transaction_state.handle_args_with(
            store,
            &mut args,
            command,
            |inner_store, _, cmd_args| dispatch_args(inner_store, cmd_args),
        );
        if response_is_error(&outcome.response) {
            return Err("appendonly replay failed due to command error".to_string());
        }
        replayed = replayed.saturating_add(1);
    }

    Ok((replayed, !buffer.is_empty()))
}

fn persistence_enabled(config: &Config) -> bool {
    config.appendonly || config.snapshot_on_shutdown || !config.save_rules.is_empty()
}

pub fn should_log_command(command: CommandId, response: &RespFrame) -> bool {
    !response_is_error(response) && !response_is_queued(response) && is_aof_command(command)
}

fn should_track_dirty(command: CommandId, response: &RespFrame) -> bool {
    should_log_command(command, response)
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

fn encode_resp_command_into(out: &mut Vec<u8>, args: &[CompactArg]) {
    reserve_encoded_command(out, args);
    push_decimal_prefixed(out, b'*', args.len());
    for arg in args {
        let arg_slice = arg.as_slice();
        push_decimal_prefixed(out, b'$', arg_slice.len());
        out.extend_from_slice(arg_slice);
        out.extend_from_slice(b"\r\n");
    }
}

fn reserve_encoded_command(out: &mut Vec<u8>, args: &[CompactArg]) {
    let mut additional = 16;
    for arg in args {
        additional += 16 + arg.len();
    }
    out.reserve(additional);
}

fn push_decimal_prefixed(out: &mut Vec<u8>, prefix: u8, value: usize) {
    out.push(prefix);
    let mut buffer = ItoaBuffer::new();
    out.extend_from_slice(buffer.format(value).as_bytes());
    out.extend_from_slice(b"\r\n");
}

fn write_chunks_vectored(file: &mut std::fs::File, chunks: &[Vec<u8>]) -> std::io::Result<()> {
    let mut chunk_index = 0usize;
    let mut chunk_offset = 0usize;
    let mut io_slices = SmallVec::<[IoSlice<'_>; VECTORED_WRITE_BATCH]>::new();

    while chunk_index < chunks.len() {
        io_slices.clear();
        let mut scan_index = chunk_index;
        let mut scan_offset = chunk_offset;
        while scan_index < chunks.len() && io_slices.len() < VECTORED_WRITE_BATCH {
            let chunk = &chunks[scan_index];
            if scan_offset < chunk.len() {
                io_slices.push(IoSlice::new(&chunk[scan_offset..]));
            }
            scan_index += 1;
            scan_offset = 0;
        }

        let written = file.write_vectored(&io_slices)?;
        if written == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "write_vectored wrote zero bytes",
            ));
        }

        let mut remaining = written;
        while remaining > 0 && chunk_index < chunks.len() {
            let chunk = &chunks[chunk_index];
            let available = chunk.len().saturating_sub(chunk_offset);
            if remaining < available {
                chunk_offset += remaining;
                remaining = 0;
            } else {
                remaining -= available;
                chunk_index += 1;
                chunk_offset = 0;
            }
        }
    }

    Ok(())
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
        let mut bytes = Vec::new();
        encode_resp_command_into(&mut bytes, &[CompactArg::from_slice(b"MULTI")]);
        encode_resp_command_into(
            &mut bytes,
            &[
                CompactArg::from_slice(b"SET"),
                CompactArg::from_slice(b"tx:key"),
                CompactArg::from_slice(b"value"),
            ],
        );
        encode_resp_command_into(&mut bytes, &[CompactArg::from_slice(b"EXEC")]);

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
