use std::path::Path;

pub(super) async fn read_snapshot_bytes(path: &Path) -> Result<Vec<u8>, String> {
    tokio::fs::read(path)
        .await
        .map_err(|err| format!("failed to read snapshot {}: {err}", path.display()))
}

pub(super) fn sync_file_path(path: &Path) -> Result<(), String> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|err| format!("failed to open {} for fsync: {err}", path.display()))?;
    file.sync_all()
        .map_err(|err| format!("failed to fsync {}: {err}", path.display()))
}

pub(super) fn sync_directory(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    let directory = std::fs::File::open(parent)
        .map_err(|err| format!("failed to open directory {}: {err}", parent.display()))?;
    directory
        .sync_all()
        .map_err(|err| format!("failed to fsync directory {}: {err}", parent.display()))
}
