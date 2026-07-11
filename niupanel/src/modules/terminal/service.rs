use niupanel_common::error::{AppError, Result};
use niupanel_core::audit::service::AuditService;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use sea_orm::DatabaseConnection;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub struct TerminalSession {
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    tx: mpsc::UnboundedSender<Vec<u8>>,
    input_buffer: Arc<Mutex<String>>,
    // Use Arc<Mutex> to satisfy Send/Sync bounds for async capture
    _child: Arc<Mutex<Box<dyn Child + Send>>>,
    db: DatabaseConnection,
    user_id: i32,
    ip: String,
}

impl TerminalSession {
    pub async fn new(
        rows: u16,
        cols: u16,
        tx: mpsc::UnboundedSender<Vec<u8>>,
        db: DatabaseConnection,
        user_id: i32,
        ip: String,
    ) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| AppError::Generic(format!("Failed to open PTY: {}", e)))?;

        let shell = if let Ok(s) = std::env::var("SHELL") {
            s
        } else {
            "/bin/bash".to_string()
        };

        let mut cmd = CommandBuilder::new(&shell);
        // Start as login shell to source .bashrc/.profile for aliases and completion
        cmd.args(&["-l"]);

        let scripts_dir = niupanel_common::config::Config::global()
            .scripts_dir
            .clone();
        cmd.cwd(scripts_dir);
        cmd.env("TERM", "xterm-256color");
        cmd.env("LANG", "C.UTF-8");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| AppError::Generic(format!("Failed to spawn shell: {}", e)))?;

        let master = pair.master;
        let writer = master
            .take_writer()
            .map_err(|e| AppError::Generic(e.to_string()))?;
        let mut reader = master
            .try_clone_reader()
            .map_err(|e| AppError::Generic(e.to_string()))?;

        let tx_clone = tx.clone();

        // Async reader task
        std::thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            while let Ok(n) = reader.read(&mut buffer) {
                if n == 0 {
                    break;
                }
                if tx_clone.send(buffer[..n].to_vec()).is_err() {
                    break;
                }
            }
        });

        // 审计日志记录终端启动
        let db_clone = db.clone();
        let ip_clone = ip.clone();
        tokio::spawn(async move {
            AuditService::log(
                &db_clone,
                Some(user_id),
                "User",
                "TERMINAL_START",
                "terminal",
                None,
                Some("启动了 Web 终端会话".to_string()),
                Some(ip_clone),
            )
            .await;
        });

        Ok(Self {
            master: Arc::new(Mutex::new(master)),
            writer: Arc::new(Mutex::new(writer)),
            tx,
            input_buffer: Arc::new(Mutex::new(String::new())),
            _child: Arc::new(Mutex::new(child)),
            db,
            user_id,
            ip,
        })
    }

    pub async fn write(&self, data: &[u8]) -> Result<()> {
        let mut should_clear_pty = false;
        let mut executed_cmds = Vec::new();

        {
            let mut buffer = self
                .input_buffer
                .lock()
                .map_err(|_| AppError::Generic("Terminal lock poisoned".into()))?;

            // 将输入的字节转为字符串处理，以支持中文等 UTF-8 字符进入安全审计缓存
            let incoming_text = String::from_utf8_lossy(data);

            for c in incoming_text.chars() {
                if c == '\r' || c == '\n' {
                    let cmd = buffer.trim();
                    if !cmd.is_empty() {
                        if self.is_dangerous(cmd) {
                            let msg = format!(
                                "\r\n\x1b[31m[安全拦截] 指令 \"{}\" 包含高危操作，已被系统拒绝执行！\x1b[0m\r\n",
                                cmd
                            );
                            let _ = self.tx.send(msg.into_bytes());
                            should_clear_pty = true;

                            // Audit log for interception
                            let db_clone = self.db.clone();
                            let uid = self.user_id;
                            let ip_clone = self.ip.clone();
                            let cmd_clone = cmd.to_string();
                            tokio::spawn(async move {
                                AuditService::log(
                                    &db_clone,
                                    Some(uid),
                                    "User",
                                    "TERMINAL_BLOCKED",
                                    "terminal",
                                    None,
                                    Some(format!("拦截高危指令: {}", cmd_clone)),
                                    Some(ip_clone),
                                )
                                .await;
                            });
                        } else {
                            executed_cmds.push(cmd.to_string());
                        }
                    }
                    buffer.clear();
                } else if c == '\x7f' || c == '\x08' {
                    buffer.pop();
                } else {
                    buffer.push(c);
                }
            }
        }

        let writer_arc = self.writer.clone();

        if should_clear_pty {
            // 发送 Ctrl+C (\x03) 和 Ctrl+U (\x15) 来强制清空 shell 行缓冲区，防止绕过
            tokio::task::spawn_blocking(move || {
                if let Ok(mut writer) = writer_arc.lock() {
                    let _ = writer.write_all(&[0x03, 0x15]);
                    let _ = writer.flush();
                }
            })
            .await
            .map_err(|e| AppError::Generic(e.to_string()))?;
            return Ok(());
        }

        // 记录成功执行的指令 (包含普通输入，虽然可能是 vim 内部操作，但可用于追溯)
        for cmd in executed_cmds {
            let db_clone = self.db.clone();
            let uid = self.user_id;
            let ip_clone = self.ip.clone();
            tokio::spawn(async move {
                AuditService::log(
                    &db_clone,
                    Some(uid),
                    "User",
                    "TERMINAL_CMD",
                    "terminal",
                    None,
                    Some(format!("执行指令: {}", cmd)),
                    Some(ip_clone),
                )
                .await;
            });
        }

        let data_vec = data.to_vec();

        tokio::task::spawn_blocking(move || {
            let mut writer = writer_arc
                .lock()
                .map_err(|_| AppError::Generic("Writer poisoned".into()))?;
            writer.write_all(&data_vec).map_err(AppError::Io)?;
            writer.flush().map_err(AppError::Io)?;
            Ok::<(), AppError>(())
        })
        .await
        .map_err(|e| AppError::Generic(e.to_string()))??;

        Ok(())
    }

    fn is_dangerous(&self, cmd: &str) -> bool {
        let cmd_lower = cmd.to_lowercase();

        // 1. 基础全模式黑名单 (针对整条指令或高危固定模式)
        let full_blacklist = [
            "rm -rf /",
            "rm -rf *",
            "mkfs",
            "dd if=",
            "dd of=",
            "shutdown",
            "reboot",
            "poweroff",
            "halt",
            ":(){ :|:& };:",
            "chmod 777 /",
            "chmod -r 777 /",
            "chmod -r 777 *",
            "find / -delete",
            "chown -R root:root /",
            "passwd root",
            "history -c",
            "crontab -r",
        ];

        // 2. 敏感目标保护 (无论路径如何，禁止 rm/mv/unlink 作用于这些关键词)
        let protected_targets = ["niupanel", "web", "data", "migration", ".git", ".env"];
        let destructive_verbs = ["rm ", "mv ", "unlink "];

        // 深度清洗：移除多余空格和常见混淆符，提取基础结构
        let clean_cmd = cmd_lower
            .chars()
            .filter(|c| c.is_alphanumeric() || "/*-._=><?".contains(*c))
            .collect::<String>();

        for item in full_blacklist {
            let clean_item = item.replace(" ", "");
            if clean_cmd.contains(&clean_item) {
                return true;
            }
        }

        for verb in destructive_verbs {
            let clean_verb = verb.replace(" ", "");
            if clean_cmd.contains(&clean_verb) {
                for target in protected_targets {
                    if clean_cmd.contains(target) {
                        return true;
                    }
                }
            }
        }

        // 拦截常见绕过手段：base64解码执行、Hex解码执行、eval执行
        if clean_cmd.contains("base64-d")
            || clean_cmd.contains("xxd-r")
            || clean_cmd.contains("eval")
        {
            return true;
        }

        false
    }

    pub async fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        let master_arc = self.master.clone();
        tokio::task::spawn_blocking(move || {
            let master = master_arc
                .lock()
                .map_err(|_| AppError::Generic("Lock poisoned".into()))?;
            master
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| AppError::Generic(e.to_string()))
        })
        .await
        .map_err(|e| AppError::Generic(e.to_string()))??;
        Ok(())
    }
}
