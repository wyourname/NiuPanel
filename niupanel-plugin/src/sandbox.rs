use super::*;

#[cfg(target_os = "linux")]
pub(super) fn secure_plugin_data_dir(data_dir: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(data_dir, fs::Permissions::from_mode(0o700))?;
    if unsafe { libc::geteuid() } == 0 {
        secure_plugin_data_tree(data_dir)?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub(super) fn secure_plugin_data_tree(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = fs::symlink_metadata(path)?;
    chown_plugin_path(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Ok(());
    }

    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    for entry in fs::read_dir(path)? {
        secure_plugin_data_tree(&entry?.path())?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub(super) fn chown_plugin_path(path: &Path) -> Result<()> {
    let path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).map_err(|_| {
        AppError::ValidationError("Plugin data path contains an invalid byte".to_string())
    })?;
    let result = unsafe { libc::lchown(path.as_ptr(), PLUGIN_SANDBOX_UID, PLUGIN_SANDBOX_GID) };
    if result == 0 {
        Ok(())
    } else {
        Err(AppError::Io(std::io::Error::last_os_error()))
    }
}

#[cfg(not(target_os = "linux"))]
pub(super) fn secure_plugin_data_dir(_data_dir: &Path) -> Result<()> {
    Ok(())
}

pub(super) fn sandboxed_plugin_command(
    spec: &ProcessPluginSpec,
    plugin_dir: &Path,
    plugin_data_dir: &Path,
) -> Result<Command> {
    validate_plugin_environment(&spec.env)?;
    let entry_path = absolute_existing_path(&safe_entry_path(plugin_dir, &spec.entry)?)?;
    let plugin_temp_dir = plugin_data_dir.join("tmp");
    fs::create_dir_all(&plugin_temp_dir)?;
    secure_plugin_data_dir(&plugin_temp_dir)?;
    let mut command = Command::new(entry_path);
    command
        .args(&spec.args)
        .current_dir(plugin_dir)
        .env_clear()
        .env(
            "PATH",
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        )
        .env("HOME", plugin_data_dir)
        .env("TMPDIR", &plugin_temp_dir)
        .env(
            "NIUPANEL_PLUGIN_PROTOCOL",
            PROCESS_PROTOCOL_VERSION.to_string(),
        )
        .env(
            "NIUPANEL_AGENT_PROTOCOL",
            PROCESS_PROTOCOL_VERSION.to_string(),
        )
        .env("NIUPANEL_PLUGIN_ID", &spec.plugin_id)
        .env("NIUPANEL_PLUGIN_EXTENSION", &spec.extension_point)
        .env("NIUPANEL_AGENT_ID", &spec.plugin_id)
        .env("NIUPANEL_PLUGIN_DATA_DIR", plugin_data_dir);
    for (key, value) in &spec.env {
        command.env(key, value);
    }

    configure_plugin_sandbox(
        &mut command,
        plugin_dir,
        plugin_data_dir,
        spec.runtime_permissions
            .contains(&PluginRuntimePermission::NetworkOutbound),
    )?;
    Ok(command)
}

#[cfg(target_os = "linux")]
pub(super) fn configure_plugin_sandbox(
    command: &mut Command,
    plugin_dir: &Path,
    plugin_data_dir: &Path,
    network_outbound: bool,
) -> Result<()> {
    use std::os::unix::process::CommandExt;

    let mut ruleset = if plugin_sandbox_capability().landlock_abi.is_some() {
        match create_landlock_ruleset(plugin_dir, plugin_data_dir, network_outbound) {
            Ok(ruleset) => Some(ruleset),
            Err(error) => {
                niupanel_common::warn!(
                    %error,
                    "Landlock is incompatible with this environment; using the mandatory UID/seccomp sandbox"
                );
                None
            }
        }
    } else {
        None
    };
    unsafe {
        command.as_std_mut().pre_exec(move || {
            drop_plugin_privileges()?;
            install_plugin_process_guards(network_outbound)?;
            if let Some(ruleset) = ruleset.take() {
                // Landlock is an optional filesystem hardening layer. Older kernels
                // continue with UID isolation, no_new_privs and seccomp.
                let _ = ruleset.restrict_self();
            }
            Ok(())
        });
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub(super) fn drop_plugin_privileges() -> std::io::Result<()> {
    if unsafe { libc::geteuid() } != 0 {
        return Ok(());
    }

    if unsafe { libc::setgroups(0, std::ptr::null()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    if unsafe { libc::setresgid(PLUGIN_SANDBOX_GID, PLUGIN_SANDBOX_GID, PLUGIN_SANDBOX_GID) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    if unsafe { libc::setresuid(PLUGIN_SANDBOX_UID, PLUGIN_SANDBOX_UID, PLUGIN_SANDBOX_UID) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub(super) fn configure_plugin_sandbox(
    _command: &mut Command,
    _plugin_dir: &Path,
    _plugin_data_dir: &Path,
    _network_outbound: bool,
) -> Result<()> {
    Err(AppError::ValidationError(
        "Process plugins require Linux Landlock sandbox support".to_string(),
    ))
}

#[cfg(target_os = "linux")]
pub(super) fn create_landlock_ruleset(
    plugin_dir: &Path,
    plugin_data_dir: &Path,
    network_outbound: bool,
) -> Result<landlock::RulesetCreated> {
    use landlock::{
        ABI, Access, AccessFs, AccessNet, CompatLevel, Compatible, NetPort, Ruleset, RulesetAttr,
        RulesetCreatedAttr, path_beneath_rules,
    };

    let abi = ABI::V4;
    let read_only_directories = [
        PathBuf::from("/usr"),
        PathBuf::from("/bin"),
        PathBuf::from("/lib"),
        PathBuf::from("/lib64"),
        PathBuf::from("/etc/ssl"),
        PathBuf::from("/etc/pki"),
        PathBuf::from("/etc/ca-certificates"),
        plugin_dir.to_path_buf(),
    ]
    .into_iter()
    .filter(|path| path.is_dir())
    .collect::<Vec<_>>();
    let read_only_files = [
        PathBuf::from("/etc/resolv.conf"),
        PathBuf::from("/etc/hosts"),
        PathBuf::from("/etc/host.conf"),
        PathBuf::from("/etc/nsswitch.conf"),
        PathBuf::from("/etc/gai.conf"),
        PathBuf::from("/etc/localtime"),
    ]
    .into_iter()
    .filter(|path| path.is_file())
    .collect::<Vec<_>>();
    let device_files = [
        PathBuf::from("/dev/null"),
        PathBuf::from("/dev/zero"),
        PathBuf::from("/dev/random"),
        PathBuf::from("/dev/urandom"),
    ]
    .into_iter()
    .filter(|path| path.is_file())
    .collect::<Vec<_>>();

    let mut ruleset = Ruleset::default()
        .set_compatibility(CompatLevel::BestEffort)
        .handle_access(AccessFs::from_all(abi));
    if network_outbound {
        ruleset = ruleset.and_then(|ruleset| {
            ruleset
                .set_compatibility(CompatLevel::BestEffort)
                .handle_access(AccessNet::from_all(abi))
        });
    }
    let mut ruleset = ruleset
        .and_then(|ruleset| ruleset.create())
        .and_then(|ruleset| {
            ruleset.add_rules(path_beneath_rules(
                read_only_directories,
                AccessFs::from_read(abi),
            ))
        })
        .and_then(|ruleset| {
            ruleset.add_rules(path_beneath_rules(read_only_files, AccessFs::ReadFile))
        })
        .and_then(|ruleset| {
            ruleset.add_rules(path_beneath_rules(
                [plugin_data_dir.to_path_buf()],
                AccessFs::from_all(abi),
            ))
        })
        .and_then(|ruleset| {
            ruleset.add_rules(path_beneath_rules(
                device_files,
                AccessFs::ReadFile | AccessFs::WriteFile,
            ))
        })
        .map_err(|error| {
            AppError::ValidationError(format!("Failed to prepare Landlock sandbox: {error}"))
        })?;

    if network_outbound {
        for port in PLUGIN_WEB_PORTS {
            ruleset = ruleset
                .add_rule(
                    NetPort::new(port, AccessNet::ConnectTcp)
                        .set_compatibility(CompatLevel::BestEffort),
                )
                .map_err(|error| {
                    AppError::ValidationError(format!(
                        "Failed to prepare plugin network sandbox: {error}"
                    ))
                })?;
        }
    }

    Ok(ruleset)
}

#[cfg(target_os = "linux")]
pub(super) fn install_plugin_process_guards(network_outbound: bool) -> std::io::Result<()> {
    const BPF_LD_W_ABS: u16 = 0x20;
    const BPF_JMP_JEQ_K: u16 = 0x15;
    const BPF_RET_K: u16 = 0x06;
    const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000;
    const SECCOMP_RET_ERRNO: u32 = 0x0005_0000;

    if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    if unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    if unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0) } != 0 {
        return Err(std::io::Error::last_os_error());
    }

    let mut denied_syscalls = vec![
        libc::SYS_ptrace,
        libc::SYS_process_vm_readv,
        libc::SYS_process_vm_writev,
        libc::SYS_mount,
        libc::SYS_umount2,
        libc::SYS_pivot_root,
        libc::SYS_setns,
        libc::SYS_unshare,
        libc::SYS_bpf,
        libc::SYS_perf_event_open,
        libc::SYS_userfaultfd,
        libc::SYS_open_by_handle_at,
        libc::SYS_name_to_handle_at,
        libc::SYS_keyctl,
        libc::SYS_add_key,
        libc::SYS_request_key,
        libc::SYS_reboot,
        libc::SYS_kexec_load,
        libc::SYS_init_module,
        libc::SYS_finit_module,
        libc::SYS_delete_module,
        libc::SYS_swapon,
        libc::SYS_swapoff,
        libc::SYS_io_uring_setup,
        libc::SYS_io_uring_enter,
        libc::SYS_io_uring_register,
    ];
    if !network_outbound {
        denied_syscalls.extend([
            libc::SYS_socket,
            libc::SYS_connect,
            libc::SYS_bind,
            libc::SYS_listen,
            libc::SYS_accept,
            libc::SYS_accept4,
            libc::SYS_sendto,
            libc::SYS_sendmsg,
            libc::SYS_recvfrom,
            libc::SYS_recvmsg,
        ]);
    }
    denied_syscalls.sort_unstable();
    denied_syscalls.dedup();

    let mut filters = Vec::with_capacity(denied_syscalls.len() * 2 + 2);
    filters.push(libc::sock_filter {
        code: BPF_LD_W_ABS,
        jt: 0,
        jf: 0,
        k: 0,
    });
    for syscall in denied_syscalls {
        filters.push(libc::sock_filter {
            code: BPF_JMP_JEQ_K,
            jt: 0,
            jf: 1,
            k: syscall as u32,
        });
        filters.push(libc::sock_filter {
            code: BPF_RET_K,
            jt: 0,
            jf: 0,
            k: SECCOMP_RET_ERRNO | libc::EPERM as u32,
        });
    }
    filters.push(libc::sock_filter {
        code: BPF_RET_K,
        jt: 0,
        jf: 0,
        k: SECCOMP_RET_ALLOW,
    });

    let program = libc::sock_fprog {
        len: filters.len() as u16,
        filter: filters.as_mut_ptr(),
    };
    let result = unsafe {
        libc::prctl(
            libc::PR_SET_SECCOMP,
            libc::SECCOMP_MODE_FILTER,
            &program as *const libc::sock_fprog,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

pub(super) fn validate_plugin_environment(env: &HashMap<String, String>) -> Result<()> {
    for key in env.keys() {
        let normalized = key.trim().to_ascii_uppercase();
        let valid_name = !key.is_empty()
            && key.len() <= 128
            && key.chars().enumerate().all(|(index, ch)| {
                ch == '_' || ch.is_ascii_alphanumeric() && (index > 0 || !ch.is_ascii_digit())
            });
        let reserved = normalized.starts_with("NIUPANEL_")
            || normalized.starts_with("LD_")
            || normalized.starts_with("DYLD_")
            || matches!(
                normalized.as_str(),
                "DATABASE_URL" | "HOME" | "PATH" | "TMPDIR" | "BASH_ENV" | "ENV" | "SHELLOPTS"
            );
        if !valid_name || reserved {
            return Err(AppError::ValidationError(format!(
                "Plugin environment variable '{key}' is reserved or invalid"
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_plugin_runtime_permissions(
    permissions: &[PluginRuntimePermission],
) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for permission in permissions {
        if !seen.insert(*permission) {
            return Err(AppError::ValidationError(format!(
                "Plugin runtime permission '{permission:?}' is duplicated"
            )));
        }
    }
    Ok(())
}

pub(super) fn absolute_existing_path(path: &Path) -> Result<PathBuf> {
    fs::canonicalize(path).map_err(AppError::from)
}

pub(super) async fn read_limited<R>(reader: R) -> std::io::Result<Vec<u8>>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut reader = reader.take(MAX_PROCESS_OUTPUT_BYTES as u64);
    let mut output = Vec::new();
    reader.read_to_end(&mut output).await?;
    Ok(output)
}
