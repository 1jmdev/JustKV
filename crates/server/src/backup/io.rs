use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
pub(super) async fn read_snapshot_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let _trace = profiler::scope("server::backup::read_snapshot_bytes");
    match read_snapshot_bytes_io_uring(path).await {
        Ok(bytes) => Ok(bytes),
        Err(err) => {
            tracing::warn!(
                error = %err,
                path = %path.display(),
                "io_uring snapshot read failed, falling back to std::fs::read"
            );
            std::fs::read(path).map_err(|read_err| {
                format!("failed to read snapshot {}: {read_err}", path.display())
            })
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub(super) async fn read_snapshot_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let _trace = profiler::scope("server::backup::read_snapshot_bytes");
    std::fs::read(path).map_err(|err| format!("failed to read snapshot {}: {err}", path.display()))
}

#[cfg(target_os = "linux")]
async fn read_snapshot_bytes_io_uring(path: &Path) -> Result<Vec<u8>, String> {
    let _trace = profiler::scope("server::backup::read_snapshot_bytes_io_uring");
    let path_buf = path.to_path_buf();
    let path_for_runtime = path_buf.clone();
    let file_size = file_size_bytes(&path_buf)?;

    tokio::task::spawn_blocking(move || {
        tokio_uring::start(async move { read_with_io_uring(path_for_runtime, file_size).await })
    })
    .await
    .map_err(|err| {
        format!(
            "failed to join io_uring reader task for {}: {err}",
            path.display()
        )
    })?
}

#[cfg(target_os = "linux")]
async fn read_with_io_uring(path: PathBuf, file_size: usize) -> Result<Vec<u8>, String> {
    let _trace = profiler::scope("server::backup::read_with_io_uring");
    let file = tokio_uring::fs::File::open(path.clone())
        .await
        .map_err(|err| format!("failed to open snapshot {}: {err}", path.display()))?;

    let mut bytes = Vec::with_capacity(file_size);
    let mut offset = 0u64;
    const CHUNK_SIZE: usize = 4 * 1024 * 1024;

    while bytes.len() < file_size {
        let remaining = file_size - bytes.len();
        let read_len = remaining.min(CHUNK_SIZE);
        let read_buf = vec![0u8; read_len];
        let (result, read_buf) = file.read_at(read_buf, offset).await;
        let read = result
            .map_err(|err| format!("failed to read snapshot chunk {}: {err}", path.display()))?;
        if read == 0 {
            return Err(format!(
                "snapshot {} ended early: expected {file_size} bytes, got {}",
                path.display(),
                bytes.len()
            ));
        }
        bytes.extend_from_slice(&read_buf[..read]);
        offset = offset.saturating_add(read as u64);
    }

    Ok(bytes)
}

#[cfg(target_os = "linux")]
fn file_size_bytes(path: &Path) -> Result<usize, String> {
    let _trace = profiler::scope("server::backup::file_size_bytes");
    let len = std::fs::metadata(path)
        .map_err(|err| format!("failed to stat snapshot {}: {err}", path.display()))?
        .len();
    usize::try_from(len).map_err(|_| {
        format!(
            "snapshot {} is too large to fit in memory on this platform",
            path.display()
        )
    })
}

pub(super) fn sync_file_path(path: &Path) -> Result<(), String> {
    let _trace = profiler::scope("server::backup::sync_file_path");
    let file = std::fs::OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|err| format!("failed to open {} for fsync: {err}", path.display()))?;
    file.sync_all()
        .map_err(|err| format!("failed to fsync {}: {err}", path.display()))
}

pub(super) fn sync_directory(path: &Path) -> Result<(), String> {
    let _trace = profiler::scope("server::backup::sync_directory");
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    let directory = std::fs::File::open(parent)
        .map_err(|err| format!("failed to open directory {}: {err}", parent.display()))?;
    directory
        .sync_all()
        .map_err(|err| format!("failed to fsync directory {}: {err}", parent.display()))
}
