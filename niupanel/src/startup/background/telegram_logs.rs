pub(super) async fn read_latest_log(
    directory: &std::path::Path,
    max_bytes: usize,
) -> Option<String> {
    let mut entries = tokio::fs::read_dir(directory).await.ok()?;
    let mut logs = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "log")
            && let Ok(metadata) = entry.metadata().await
        {
            logs.push((entry.path(), metadata.modified().ok()));
        }
    }
    logs.sort_by(|left, right| right.1.cmp(&left.1));
    let content = tokio::fs::read_to_string(logs.first()?.0.as_path())
        .await
        .ok()?;
    if content.len() <= max_bytes {
        return Some(content);
    }
    let mut start = content.len().saturating_sub(max_bytes);
    while start < content.len() && !content.is_char_boundary(start) {
        start += 1;
    }
    Some(content[start..].to_string())
}
