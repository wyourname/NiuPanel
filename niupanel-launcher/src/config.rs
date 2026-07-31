use super::*;

pub(crate) struct LauncherConfig {
    pub(crate) system_root: PathBuf,
    pub(crate) bundled_binary: PathBuf,
    pub(crate) bundled_manifest_root: PathBuf,
    pub(crate) working_dir: PathBuf,
    pub(crate) health_addr: String,
    pub(crate) database_path: PathBuf,
    pub(crate) shutdown: Arc<AtomicBool>,
}

pub(crate) enum CandidateOutcome {
    Healthy,
    Failed(String),
    Shutdown,
}

impl LauncherConfig {
    pub(crate) fn from_env() -> Result<Self> {
        let working_dir = env::current_dir().context("Failed to determine working directory")?;
        let executable = env::current_exe().context("Failed to determine launcher path")?;
        let executable_dir = executable
            .parent()
            .ok_or_else(|| anyhow!("Launcher path does not have a parent"))?;
        let system_root = absolute_from(
            &working_dir,
            Path::new(
                &env::var("NIUPANEL_SYSTEM_DIR")
                    .or_else(|_| env::var("SYSTEM_DIR"))
                    .unwrap_or_else(|_| "data/system".into()),
            ),
        );
        let bundled_binary = env::var("NIUPANEL_BUNDLED_CORE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| executable_dir.join("niupanel"));
        let bundled_manifest_root = bundled_binary
            .parent()
            .unwrap_or(executable_dir)
            .to_path_buf();
        let health_addr = env::var("NIUPANEL_HEALTH_ADDR").unwrap_or_else(|_| {
            health_addr_from_server().unwrap_or_else(|| "127.0.0.1:7788".into())
        });
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://data/database/niupanel.db?mode=rwc".into());
        let database_path = sqlite_path(&working_dir, &database_url)?;
        Ok(Self {
            system_root,
            bundled_binary,
            bundled_manifest_root,
            working_dir,
            health_addr,
            database_path,
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }
}

pub(crate) fn absolute_from(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

pub(crate) fn health_addr_from_server() -> Option<String> {
    let address = env::var("SERVER_ADDR").ok()?;
    let (host, port) = address.rsplit_once(':')?;
    let host = match host {
        "0.0.0.0" | "::" | "[::]" => "127.0.0.1",
        value => value.trim_matches(['[', ']']),
    };
    Some(format!("{host}:{port}"))
}

pub(crate) fn sqlite_path(working_dir: &Path, database_url: &str) -> Result<PathBuf> {
    let raw = database_url
        .strip_prefix("sqlite://")
        .ok_or_else(|| anyhow!("Core rollback currently requires a SQLite DATABASE_URL"))?;
    let path = raw.split('?').next().unwrap_or(raw);
    if path.is_empty() || path == ":memory:" {
        bail!("Core rollback requires a persistent SQLite database");
    }
    Ok(absolute_from(working_dir, Path::new(path)))
}

pub(crate) fn hash_file(path: &Path) -> Result<String> {
    let mut file =
        fs::File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}
