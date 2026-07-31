use tokio::process::Command;

#[cfg(not(unix))]
use niupanel_common::warn;
#[cfg(unix)]
use niupanel_common::{info, warn};

/// 应用系统级资源限制和进程隔离
///
/// # 参数
/// * `cmd` - Tokio Command 对象
/// * `memory_limit_mb` - 内存限制 (MB)
pub fn apply_limits(cmd: &mut Command, memory_limit_mb: Option<i32>) {
    #[cfg(unix)]
    {
        apply_unix_limits(cmd, memory_limit_mb);
    }

    #[cfg(not(unix))]
    {
        if let Some(mem) = memory_limit_mb {
            warn!("Memory limit ({}MB) ignored on non-Unix systems.", mem);
        }
    }
}

pub async fn apply_post_spawn_limits(
    task_id: i32,
    run_id: Option<i32>,
    pid: u32,
    cpu_limit_percent: Option<i32>,
) -> niupanel_common::error::Result<Option<std::path::PathBuf>> {
    #[cfg(unix)]
    {
        apply_unix_post_spawn_limits(task_id, run_id, pid, cpu_limit_percent)
    }

    #[cfg(not(unix))]
    {
        if let Some(limit) = cpu_limit_percent.filter(|limit| *limit > 0) {
            warn!(
                "CPU limit ({}%) ignored on non-Unix systems for task {}.",
                limit, task_id
            );
        }
        Ok(None)
    }
}

pub fn cleanup_post_spawn_limits(path: &std::path::Path) {
    #[cfg(unix)]
    {
        if let Err(e) = std::fs::remove_dir(path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                warn!("Failed to cleanup CPU cgroup {}: {}", path.display(), e);
            }
        }
    }

    #[cfg(not(unix))]
    let _ = path;
}

#[cfg(unix)]
fn apply_unix_limits(cmd: &mut Command, memory_limit_mb: Option<i32>) {
    unsafe {
        cmd.pre_exec(move || {
            if let Err(e) = nix::unistd::setsid() {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
            }

            // 2. Apply Memory Limits if configured
            if let Some(limit_mb) = memory_limit_mb {
                if limit_mb > 0 {
                    setup_rlimit_as(limit_mb)?;
                }
            }
            Ok(())
        });
    }
}

#[cfg(unix)]
fn apply_unix_post_spawn_limits(
    task_id: i32,
    run_id: Option<i32>,
    pid: u32,
    cpu_limit_percent: Option<i32>,
) -> niupanel_common::error::Result<Option<std::path::PathBuf>> {
    let Some(limit) = cpu_limit_percent.filter(|limit| *limit > 0) else {
        return Ok(None);
    };

    if !(1..=100).contains(&limit) {
        return Err(niupanel_common::error::AppError::ValidationError(format!(
            "CPU 限制必须在 1 到 100 之间，当前值为 {}",
            limit
        )));
    }

    let root = std::path::Path::new("/sys/fs/cgroup");
    let controllers = std::fs::read_to_string(root.join("cgroup.controllers")).map_err(|e| {
        niupanel_common::error::AppError::Environment(format!(
            "无法读取 cgroup v2 控制器列表，无法应用 CPU 限制: {}",
            e
        ))
    })?;

    if !controllers.split_whitespace().any(|ctrl| ctrl == "cpu") {
        return Err(niupanel_common::error::AppError::Environment(
            "当前系统未启用 cgroup v2 cpu 控制器，无法应用 CPU 限制".to_string(),
        ));
    }

    enable_cgroup_controller(root, "cpu")?;

    let niu_root = root.join("niupanel");
    std::fs::create_dir_all(&niu_root).map_err(|e| {
        niupanel_common::error::AppError::Environment(format!(
            "无法创建 NiuPanel cgroup 目录 {}: {}",
            niu_root.display(),
            e
        ))
    })?;
    enable_cgroup_controller(&niu_root, "cpu")?;

    let cgroup_name = match run_id {
        Some(run_id) => format!("task-{}-run-{}-pid-{}", task_id, run_id, pid),
        None => format!("task-{}-pid-{}", task_id, pid),
    };
    let task_group = niu_root.join(cgroup_name);

    std::fs::create_dir(&task_group).map_err(|e| {
        niupanel_common::error::AppError::Environment(format!(
            "无法创建 CPU cgroup {}: {}",
            task_group.display(),
            e
        ))
    })?;

    let period_us: u64 = 100_000;
    let quota_us = (period_us * limit as u64 / 100).max(1_000);
    std::fs::write(
        task_group.join("cpu.max"),
        format!("{} {}", quota_us, period_us),
    )
    .map_err(|e| {
        let _ = std::fs::remove_dir(&task_group);
        niupanel_common::error::AppError::Environment(format!(
            "无法写入 CPU 配额到 {}: {}",
            task_group.display(),
            e
        ))
    })?;

    std::fs::write(task_group.join("cgroup.procs"), pid.to_string()).map_err(|e| {
        let _ = std::fs::remove_dir(&task_group);
        niupanel_common::error::AppError::Environment(format!(
            "无法将进程 {} 加入 CPU cgroup {}: {}",
            pid,
            task_group.display(),
            e
        ))
    })?;

    info!(
        "Applied CPU limit {}% for task {} using cgroup {}",
        limit,
        task_id,
        task_group.display()
    );

    Ok(Some(task_group))
}

#[cfg(unix)]
fn enable_cgroup_controller(
    parent: &std::path::Path,
    controller: &str,
) -> niupanel_common::error::Result<()> {
    let subtree_control_path = parent.join("cgroup.subtree_control");
    let subtree_control = std::fs::read_to_string(&subtree_control_path).map_err(|e| {
        niupanel_common::error::AppError::Environment(format!(
            "无法读取 cgroup subtree_control {}: {}",
            subtree_control_path.display(),
            e
        ))
    })?;

    if subtree_control
        .split_whitespace()
        .any(|entry| entry == controller)
    {
        return Ok(());
    }

    std::fs::write(&subtree_control_path, format!("+{}", controller)).map_err(|e| {
        niupanel_common::error::AppError::Environment(format!(
            "无法在 {} 启用 cgroup 控制器 {}: {}",
            parent.display(),
            controller,
            e
        ))
    })?;

    Ok(())
}

#[cfg(unix)]
fn setup_rlimit_as(limit_mb: i32) -> std::io::Result<()> {
    let limit_bytes = (limit_mb as u64) * 1024 * 1024;
    let resource = nix::sys::resource::Resource::RLIMIT_AS; // Address Space limit

    // Try to get current hard limit
    let (_, hard) = nix::sys::resource::getrlimit(resource)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // Handle potential type differences (u32 vs u64) for RLIM types on different archs
    let hard_u64 = if hard == nix::sys::resource::RLIM_INFINITY {
        u64::MAX
    } else {
        hard as u64
    };

    let new_soft_u64 = limit_bytes;

    let new_hard_u64 = if hard_u64 == u64::MAX || hard_u64 > new_soft_u64 {
        hard_u64
    } else {
        new_soft_u64
    };

    // Convert back to native type, handling Infinity
    let new_soft_native = new_soft_u64
        .try_into()
        .unwrap_or(nix::sys::resource::RLIM_INFINITY);

    let new_hard_native = if new_hard_u64 == u64::MAX {
        nix::sys::resource::RLIM_INFINITY
    } else {
        new_hard_u64
            .try_into()
            .unwrap_or(nix::sys::resource::RLIM_INFINITY)
    };

    nix::sys::resource::setrlimit(resource, new_soft_native, new_hard_native)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    Ok(())
}
