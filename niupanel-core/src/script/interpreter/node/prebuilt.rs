use niupanel_common::download::{DownloadOptions, download_with_resume};
use niupanel_common::error::{AppError, Result};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;
use tokio::sync::Mutex;

pub(super) static INSTALL_LOCK: Mutex<()> = Mutex::const_new(());
const MAX_ARCHIVE_SIZE: u64 = 150 * 1024 * 1024;
const MAX_BINARY_SIZE: u64 = 300 * 1024 * 1024;

pub(super) async fn install(root: &Path, version: &str, mirror: &str) -> Result<()> {
    if std::env::consts::OS != "linux" || !super::pnpm::has_glibc() {
        return Err(AppError::Environment(
            "Node 官方预构建包需要 Linux glibc；请使用 Debian/Ubuntu 容器或系统提供的兼容 Node 环境，不会回退到源码编译".into(),
        ));
    }
    let platform = super::node_release_file_for_arch(std::env::consts::ARCH)
        .ok_or_else(|| AppError::Environment("当前架构没有受支持的 Node 预构建包".into()))?;
    let name = format!("node-v{version}-{platform}");
    let filename = format!("{name}.tar.gz");
    let staging = tempfile::tempdir_in(root).map_err(AppError::Io)?;
    let archive = staging.path().join(&filename);
    let sums = staging.path().join("SHASUMS256.txt");
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| AppError::Generic(error.to_string()))?;
    for (file, destination, max_size) in [
        ("SHASUMS256.txt", &sums, 1024 * 1024),
        (filename.as_str(), &archive, MAX_ARCHIVE_SIZE),
    ] {
        download_with_resume(
            &client,
            &format!("{mirror}/v{version}/{file}"),
            destination,
            None,
            DownloadOptions {
                timeout: Some(Duration::from_secs(300)),
                retries: 3,
                max_size: Some(max_size),
                ..DownloadOptions::default()
            },
            || false,
            |_| async {},
        )
        .await?;
    }
    let checksums = tokio::fs::read_to_string(&sums)
        .await
        .map_err(AppError::Io)?;
    let expected = checksum_for(&checksums, &filename)?;
    let bin_dir = staging.path().join("runtime/bin");
    tokio::fs::create_dir_all(&bin_dir)
        .await
        .map_err(AppError::Io)?;
    let node = bin_dir.join("node");
    let output = node.clone();
    tokio::task::spawn_blocking(move || extract_node(&archive, &output, &name, &expected))
        .await
        .map_err(|error| AppError::Generic(error.to_string()))??;
    if !super::node_matches_version(&node, version).await {
        return Err(AppError::Environment(format!(
            "预构建 Node {version} 无法运行或版本不符，请检查系统 glibc/libstdc++ 兼容性"
        )));
    }
    // Keep the previous environment untouched until both integrity and execution checks pass.
    let target_bin = root.join("runtime/bin");
    tokio::fs::create_dir_all(&target_bin)
        .await
        .map_err(AppError::Io)?;
    tokio::fs::rename(node, target_bin.join("node"))
        .await
        .map_err(AppError::Io)
}

fn checksum_for(checksums: &str, filename: &str) -> Result<String> {
    checksums
        .lines()
        .find_map(|line| {
            let mut fields = line.split_whitespace();
            let hash = fields.next()?;
            let name = fields.next()?.trim_start_matches('*');
            (name == filename && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
                .then(|| hash.to_ascii_lowercase())
        })
        .ok_or_else(|| AppError::Environment(format!("Node 校验清单中缺少 {filename} 的 SHA-256")))
}

fn extract_node(archive: &Path, output: &Path, name: &str, expected: &str) -> Result<()> {
    let mut file = std::fs::File::open(archive).map_err(AppError::Io)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(AppError::Io)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    if hex::encode(hash.finalize()) != expected {
        return Err(AppError::Environment(
            "Node 下载文件 SHA-256 校验失败".into(),
        ));
    }
    let file = std::fs::File::open(archive).map_err(AppError::Io)?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
    let expected_path = format!("{name}/bin/node");
    for entry in archive.entries().map_err(AppError::Io)? {
        let mut entry = entry.map_err(AppError::Io)?;
        if entry.path().map_err(AppError::Io)?.as_ref() != Path::new(&expected_path) {
            continue;
        }
        if !entry.header().entry_type().is_file() || entry.size() > MAX_BINARY_SIZE {
            return Err(AppError::Environment(
                "Node 压缩包中的二进制类型或大小无效".into(),
            ));
        }
        // Only copy the executable to a fixed destination; archive links and paths are never unpacked.
        let mut target = std::fs::File::create(output).map_err(AppError::Io)?;
        std::io::copy(&mut entry, &mut target).map_err(AppError::Io)?;
        target.flush().map_err(AppError::Io)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            target
                .set_permissions(std::fs::Permissions::from_mode(0o755))
                .map_err(AppError::Io)?;
        }
        return Ok(());
    }
    Err(AppError::Environment("Node 压缩包缺少 bin/node".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_requires_exact_filename_and_valid_digest() {
        let hash = "ab".repeat(32);
        assert_eq!(
            checksum_for(&format!("{hash}  node.tar.gz\n"), "node.tar.gz").unwrap(),
            hash
        );
        assert!(checksum_for(&format!("{hash}  other.tar.gz"), "node.tar.gz").is_err());
        assert!(checksum_for("bad  node.tar.gz", "node.tar.gz").is_err());
    }

    #[test]
    fn extracts_only_verified_regular_node_binary() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("node.tar.gz");
        let output = dir.path().join("node");
        let encoder = flate2::write::GzEncoder::new(
            std::fs::File::create(&path).unwrap(),
            flate2::Compression::fast(),
        );
        let mut tar = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(4);
        header.set_mode(0o755);
        header.set_cksum();
        tar.append_data(&mut header, "node-test/bin/node", &b"node"[..])
            .unwrap();
        tar.into_inner().unwrap().finish().unwrap();
        let hash = hex::encode(Sha256::digest(std::fs::read(&path).unwrap()));
        assert!(extract_node(&path, &output, "node-test", &"00".repeat(32)).is_err());
        assert!(!output.exists());
        extract_node(&path, &output, "node-test", &hash).unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), b"node");
        assert!(extract_node(&path, &output, "other", &hash).is_err());
    }
    #[test]
    fn rejects_archive_symlinks_in_place_of_node() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("node.tar.gz");
        let output = dir.path().join("node");
        let encoder = flate2::write::GzEncoder::new(
            std::fs::File::create(&archive).unwrap(),
            flate2::Compression::fast(),
        );
        let mut tar = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o755);
        header.set_cksum();
        tar.append_link(&mut header, "node-test/bin/node", "/bin/sh")
            .unwrap();
        tar.into_inner().unwrap().finish().unwrap();
        let hash = hex::encode(Sha256::digest(std::fs::read(&archive).unwrap()));
        assert!(extract_node(&archive, &output, "node-test", &hash).is_err());
        assert!(!output.exists());
    }

    #[tokio::test]
    #[ignore = "downloads a real Node distribution; run explicitly for installation smoke tests"]
    async fn installs_official_binary_without_pnpm() {
        let dir = tempfile::tempdir().unwrap();
        install(dir.path(), "22.14.0", "https://nodejs.org/dist")
            .await
            .unwrap();
        assert!(
            super::super::node_matches_version(&dir.path().join("runtime/bin/node"), "22.14.0")
                .await
        );
        assert!(!dir.path().join("node_modules").exists());
        let entries = std::fs::read_dir(dir.path()).unwrap().count();
        assert_eq!(entries, 1, "temporary downloads must be cleaned up");
    }
}
