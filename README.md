```bash
docker run -d   --name niupanel   --restart unless-stopped   -p 7788:7788   -v $(pwd)/data:/app/data   wyourname/niupanel:amd64-latest
```
当前仅支持x86_64架构机器,比不上青龙面板 现在功能缺失，尝鲜就行,安装完第一件事就是去设置更新
# NiuPanel

NiuPanel 是一个基于 Rust 和 Vue 3 构建的高性能、轻量级脚本管理与定时任务面板。

独特的**脚本/配置打包分享（Share）**功能。

图1：<img width="1920" height="888" alt="{68B1DE40-55C3-46CE-B606-7A479A93B7D2}" src="https://github.com/user-attachments/assets/40b70e49-0a5a-49e2-b035-025abceafe12" />
图2：<img width="1645" height="899" alt="{B2617237-5892-4EFB-9B4C-059A33992A57}" src="https://github.com/user-attachments/assets/88a6a77b-f4d7-4a05-8185-d9a70766fe0c" />
图3：<img width="1364" height="826" alt="image" src="https://github.com/user-attachments/assets/99dafe74-1d0d-465a-a3e8-a2423d2a7b3e" />
图4：<img width="1912" height="892" alt="{9FA2A1F0-9C18-47C7-A477-3232C67F80DA}" src="https://github.com/user-attachments/assets/f8096b6b-5660-453e-84f8-8f197492cbaa" />


## 核心特性

### 1. 服务占用低 (Rust Power)
后端完全采用 Rust 编写 (Axum + SeaORM)，相比 Node.js 或 Python 开发的面板：
- **内存占用极低**：原生二进制运行，无繁重运行时开销。
- **高并发**：利用 Tokio 异步运行时，轻松处理大量并发任务和请求。
- **单文件部署**：核心功能编译为单个可执行文件，配合 SQLite 数据库，部署迁移极其简单。

### 2. Npack 分享系统 (独家)
NiuPanel 引入了 `.npack` 格式，彻底改变了脚本分享方式：
- **一键打包**：将脚本代码、定时规则、环境变量依赖一次性打包。
- **安全分享**：支持**密码保护**、**阅后即焚**、**过期时间**和**最大下载次数**限制。
- **无缝导入**：接收者通过分享链接或 Token 即可一键导入脚本和配置，无需手动复制粘贴代码或配置 Git 仓库。

### 3. 现代 Web 终端
- 内置基于 `xterm.js` 和 `pty-process` 的全功能 Web 终端。
- 支持实时查看任务日志，交互式运行脚本。

### 4. 🛠️ python虚拟环境运行时支持不同的python版本
- 支持 **Node.js** 和 **Python** 脚本执行。
- 支持 **Python 3.9 10 11 12 13 14** 虚拟环境运行时。

---

## NiuPanel

| 特性 | NiuPanel 
| :--- | :--- | :--- |
| **核心架构** | **Rust** (高性能二进制) |
| **资源占用** | 🌟 **极低** (适合低配机器/软路由) |
| **脚本分享** | 🌟 **Npack 打包** (代码+配置+依赖整体分享，支持加密/阅后即焚) |
| **部署难度** | 简单 (单二进制/Docker) |
| **数据库** | SQLite (内置，单文件) |
| **前端技术** | Vue 3 + Element Plus | 
| **更新简单** | 🌟 **极快** (内置更新功能，更新简单) |

## UI全靠Gemini 2.5 pro+deepseek(主要负责翻译我的人话给ai听)+claude 4.5  
## 觉得偷脚本其实可以把二进制放在Debian上运行 抓包就知道了 npack也没加密gzip解压就行

