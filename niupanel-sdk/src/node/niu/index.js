const http = require('http');
const https = require('https');
const fs = require('fs');
const { URL } = require('url');

const DEFAULT_BASE_URL = "http://127.0.0.1:7788/open/api";

function loadInternalContext() {
    const contextPath = process.env.NIUPANEL_SDK_CONTEXT;
    if (!contextPath) {
        return {};
    }

    try {
        const data = JSON.parse(fs.readFileSync(contextPath, 'utf8'));
        return data && typeof data === 'object' ? data : {};
    } catch (_) {
        return {};
    }
}

class NiuPanelSDK {
    constructor(baseUrl = null, apiKey = null) {
        this.context = loadInternalContext();
        this.baseUrl = (
            baseUrl ||
            this.context.base_url ||
            process.env.NIUPANEL_SDK_BASE_URL ||
            DEFAULT_BASE_URL
        ).replace(/\/$/, "");
        this.apiKey = (
            apiKey ||
            this.context.token ||
            process.env.NIUPANEL_INTERNAL_TOKEN ||
            process.env.NiuPanel_Key
        );
        if (!this.apiKey) {
            console.warn("NiuPanel SDK context or API key is not set. Authenticated API calls will fail.");
        }
    }

    _request(method, path, body = null, params = null) {
        return new Promise((resolve, reject) => {
            let url = this.baseUrl + path;
            if (params && Object.keys(params).length > 0) {
                const query = new URLSearchParams();
                for (const key in params) {
                    if (params[key] !== undefined && params[key] !== null) {
                        query.append(key, params[key]);
                    }
                }
                const queryString = query.toString();
                if (queryString) {
                    url += "?" + queryString;
                }
            }

            const urlObj = new URL(url);
            const protocol = urlObj.protocol === 'https:' ? https : http;

            const headers = {
                "Content-Type": "application/json"
            };
            if (this.apiKey) {
                headers["X-API-Key"] = this.apiKey;
            }

            const options = {
                hostname: urlObj.hostname,
                port: urlObj.port || (urlObj.protocol === 'https:' ? 443 : 80),
                path: urlObj.pathname + urlObj.search,
                method: method,
                headers
            };

            const req = protocol.request(options, (res) => {
                let data = '';
                res.on('data', (chunk) => {
                    data += chunk;
                });
                res.on('end', () => {
                    try {
                        const json = JSON.parse(data);
                        if (json.code === 0) {
                            resolve(json.data !== undefined && json.data !== null ? json.data : true);
                        } else {
                            reject(new Error(`API Error (code ${json.code}): ${json.message || 'Unknown error'}`));
                        }
                    } catch (e) {
                        if (res.statusCode >= 200 && res.statusCode < 300) {
                            resolve(data);
                        } else {
                            reject(new Error(`HTTP Error: ${res.statusCode} - ${data}`));
                        }
                    }
                });
            });

            req.on('error', (e) => {
                reject(e);
            });

            if (body) {
                req.write(JSON.stringify(body));
            }
            req.end();
        });
    }

    hello() {
        console.log("Hello from NiuPanel SDK (Node.js)!");
    }

    getContext() {
        return process.env;
    }

    // --- Variables ---

    /**
     * 列出变量
     * @param {string|null} key - 变量键名过滤
     * @param {number} page - 页码
     * @param {number} pageSize - 每页大小
     * @returns {Promise<Object>} 分页的变量列表
     */
    async listVariables(key = null, page = 1, pageSize = 100) {
        const params = { page, page_size: pageSize };
        if (key) params.key = key;
        return await this._request('GET', '/variables/', null, params);
    }

    /**
     * 获取变量的全部数据
     * @param {string} key - 变量键名
     * @returns {Promise<Array>} 变量模型列表 [dict, dict, ...]
     */
    async getVariable(key) {
        const data = await this._request('GET', '/variables/by-key', null, { key });
        if (!Array.isArray(data)) {
            throw new TypeError("NiuPanel API returned an invalid variable response");
        }
        return data;
    }

    /**
     * 获取变量的值列表
     * @param {string} key - 变量键名
     * @returns {Promise<Array<string>>} 变量值列表 [string, string, ...]
     */
    async getVariableValues(key) {
        const vars = await this.getVariable(key);
        return vars.map(v => String(v.value || ""));
    }

    /**
     * 创建变量
     * @param {Object} data - 变量数据
     * @param {string} data.key - 键名
     * @param {string} data.value - 值
     * @param {string} data.scope - 作用域 (Global, Script)
     * @param {number} [data.scope_id] - 关联 ID (Legacy)
     * @param {Array<number>} [data.scope_ids] - 关联任务 ID 列表
     * @param {string} [data.remarks] - 备注
     * @returns {Promise<Object>} 创建的变量对象
     */
    async createVariable(data) {
        return await this._request('POST', '/variables/', data);
    }

    /**
     * 更新变量
     * @param {number} varId - 变量 ID
     * @param {Object} data - 变量数据 (参看 createVariable)
     * @returns {Promise<Object>} 更新后的变量对象
     */
    async updateVariable(varId, data) {
        return await this._request('PATCH', `/variables/${varId}`, data);
    }

    /**
     * 删除单个变量
     * @param {number} varId - 变量 ID
     * @returns {Promise<boolean>} 是否成功
     */
    async deleteVariable(varId) {
        return await this.batchDeleteVariables([varId]);
    }

    async batchDeleteVariables(ids) {
        /** 批量删除变量 */
        return await this._request('DELETE', `/variables/`, { ids });
    }

    /**
     * 通用 Key 更新变量
     * @param {string} key - 变量键名
     * @param {Object} data - 更新的数据
     * @returns {Promise<Array>} 更新后的变量列表
     */
    async updateVariableByKey(key, data) {
        return await this._request('PATCH', `/variables/by-key`, data, { key });
    }

    // --- Tasks ---

    /**
     * 列出任务
     * @param {string|null} name - 任务名称过滤
     * @param {number} page - 页码
     * @param {number} pageSize - 每页大小
     * @returns {Promise<Object>} 分页的任务列表
     */
    async listTasks(name = null, page = 1, pageSize = 100) {
        const params = { page, page_size: pageSize };
        if (name) params.name = name;
        return await this._request('GET', '/tasks/', null, params);
    }

    /**
     * 获取任务信息
     * @param {number|string} taskId - 任务 ID
     * @returns {Promise<Object|null>} 任务详情或 null
     */
    async getTaskInfo(taskId) {
        const data = await this.listTasks();
        const items = data.items || (Array.isArray(data) ? data : []);
        return items.find(item => String(item.id) === String(taskId)) || null;
    }

    /**
     * 运行特定任务
     * @param {number|string} taskId - 任务 ID
     */
    async runTask(taskId) { return await this.batchRunTasks([taskId]); }

    /**
     * 停止运行中的任务
     * @param {number|string} taskId - 任务 ID
     */
    async stopTask(taskId) { return await this.batchStopTasks([taskId]); }

    /**
     * 启用任务
     * @param {number|string} taskId - 任务 ID
     */
    async enableTask(taskId) { return await this.batchEnableTasks([taskId]); }

    /**
     * 禁用任务
     * @param {number|string} taskId - 任务 ID
     */
    async disableTask(taskId) { return await this.batchDisableTasks([taskId]); }

    /**
     * 暂停任务 (Cron)
     * @param {number|string} taskId - 任务 ID
     */
    async pauseTask(taskId) { return await this.batchPauseTasks([taskId]); }

    /**
     * 恢复任务 (Cron)
     * @param {number|string} taskId - 任务 ID
     */
    async resumeTask(taskId) { return await this.batchResumeTasks([taskId]); }

    /**
     * 批量运行任务
     * @param {Array<number>} ids - 任务 ID 列表
     * @returns {Promise<Array>} 启动结果
     */
    async batchRunTasks(ids) { return await this._request('POST', `/tasks/run`, { ids }); }

    /**
     * 批量停止任务
     * @param {Array<number>} ids - 任务 ID 列表
     */
    async batchStopTasks(ids) { return await this._request('POST', `/tasks/stop`, { ids }); }

    /**
     * 批量启用任务
     * @param {Array<number>} ids - 任务 ID 列表
     */
    async batchEnableTasks(ids) { return await this._request('POST', `/tasks/enable`, { ids }); }

    /**
     * 批量禁用任务
     * @param {Array<number>} ids - 任务 ID 列表
     */
    async batchDisableTasks(ids) { return await this._request('POST', `/tasks/disable`, { ids }); }

    /**
     * 批量暂停任务
     * @param {Array<number>} ids - 任务 ID 列表
     */
    async batchPauseTasks(ids) { return await this._request('POST', `/tasks/pause`, { ids }); }

    /**
     * 批量恢复任务
     * @param {Array<number>} ids - 任务 ID 列表
     */
    async batchResumeTasks(ids) { return await this._request('POST', `/tasks/resume`, { ids }); }

    async deleteTask(taskId, options = {}) {
        /** 删除单个任务
         * options: { deleteScript: bool, deleteVar: bool }
         */
        return await this.batchDeleteTasks([taskId], options);
    }

    /**
     * 批量删除任务
     * @param {Array<number>} ids - 任务 ID 列表
     * @param {Object} options - 删除选项
     * @param {boolean} [options.deleteScript] - 是否同时删除脚本文件
     * @param {boolean} [options.deleteVar] - 是否同时删除关联变量
     */
    async batchDeleteTasks(ids, options = {}) {
        const params = {};
        if (options.deleteScript) params.delete_script = true;
        if (options.deleteVar) params.delete_var = true;
        return await this._request('DELETE', `/tasks/`, { ids }, params);
    }

    /**
     * 获取最新任务日志
     * @param {number|string} taskId - 任务 ID
     * @returns {Promise<Object>} 日志内容对象
     */
    async getTaskLogs(taskId) {
        return await this._request('GET', `/tasks/${taskId}/logs/latest`);
    }

    /**
     * 获取特定运行历史的日志
     * @param {number|string} taskId - 任务 ID
     * @param {number|string} runId - 运行记录 ID
     * @returns {Promise<Object>} 日志内容
     */
    async getTaskRunLog(taskId, runId) {
        return await this._request('GET', `/tasks/${taskId}/history/${runId}/log`);
    }

    /**
     * 获取任务的历史记录
     * @param {number} taskId - 任务 ID
     * @param {number} page - 页码
     * @param {number} pageSize - 每页大小
     */
    async getTaskHistory(taskId, page = 1, pageSize = 20) {
        const params = { page, page_size: pageSize };
        return await this._request('GET', `/tasks/${taskId}/history`, null, params);
    }

    /**
     * 创建任务
     * @param {Object} data - 任务数据
     * @param {string} data.name - 任务名称
     * @param {string} [data.path] - 脚本路径
     * @param {string} [data.command] - 自定义命令
     * @param {string} [data.description] - 描述
     * @param {string} data.env_type - 环境类型 (Python, Nodejs, Shell)
     * @param {string} [data.env_version] - 环境版本
     * @param {string} [data.cron_schedule] - Cron 表达式
     * @param {Array<Object>} [data.variables] - 任务变量 [{key, value}]
     * @returns {Promise<Object>} 创建的任务对象
     */
    async createTask(data) {
        return await this._request('POST', '/tasks/', data);
    }

    /**
     * 通过 URL 快速创建任务
     * @param {string} url - 脚本下载 URL
     * @returns {Promise<Object>} 创建的任务对象
     */
    async quickCreateTask(url) {
        return await this._request('POST', '/tasks/quick_create', { url });
    }

    /**
     * 更新任务配置
     * @param {number|string} taskId - 任务 ID
     * @param {Object} data - 更新的数据 (参看 createTask)
     * @returns {Promise<Object>} 更新后的任务对象
     */
    async updateTask(taskId, data) {
        return await this._request('PATCH', `/tasks/${taskId}`, data);
    }

    /**
     * 删除任务运行记录
     * @param {number|string} taskId - 任务 ID
     * @param {number|string} runId - 运行记录 ID
     */
    async deleteTaskRun(taskId, runId) {
        return await this._request('DELETE', `/tasks/${taskId}/history/${runId}`);
    }

    /**
     * 置顶任务
     * @param {number|string} taskId - 任务 ID
     */
    async pinTask(taskId) {
        return await this.batchPinTasks([taskId]);
    }

    /**
     * 取消置顶
     * @param {number|string} taskId - 任务 ID
     */
    async unpinTask(taskId) {
        return await this.batchUnpinTasks([taskId]);
    }

    /**
     * 批量置顶
     * @param {Array<number>} ids - 任务 ID 列表
     */
    async batchPinTasks(ids) {
        return await this._request('POST', '/tasks/pin', { ids });
    }

    /**
     * 批量取消置顶
     * @param {Array<number>} ids - 任务 ID 列表
     */
    async batchUnpinTasks(ids) {
        return await this._request('POST', '/tasks/unpin', { ids });
    }

    // --- Jobs ---

    /**
     * 列出作业 (正在运行或队列中的任务)
     * @param {number} page - 页码
     * @param {number} pageSize - 每页大小
     * @returns {Promise<Object>} 分页的作业列表
     */
    async listJobs(page = 1, pageSize = 100) {
        const params = { page, page_size: pageSize };
        return await this._request('GET', '/jobs/', null, params);
    }

    /**
     * 获取作业详情
     * @param {string} jobId - 作业唯一 ID (UUID)
     */
    async getJob(jobId) {
        return await this._request('GET', `/jobs/${jobId}`);
    }

    async getJobLogs(jobId) {
        return await this._request('GET', `/jobs/${jobId}/logs`);
    }

    async getJobLatestLog(jobId) {
        return await this._request('GET', `/jobs/${jobId}/logs/latest`);
    }

    /**
     * 强制取消作业
     * @param {string} jobId - 作业 ID
     */
    async cancelJob(jobId) {
        return await this._request('POST', `/jobs/${jobId}/cancel`);
    }

    // --- Overview ---

    /**
     * 获取系统运行概览
     * @returns {Promise<Object>} 包含任务、变量、脚本等统计信息
     */
    async getOverview() {
        return await this._request('GET', '/overview/');
    }

    // --- Environments ---

    /**
     * 列出系统安装的所有环境
     */
    async listEnvironments() {
        return await this._request('GET', '/environments/');
    }

    /**
     * 查看可在线安装的环境版本
     */
    async listAvailableVersions() {
        return await this._request('GET', '/environments/versions');
    }

    /**
     * 查看环境已安装的包
     * @param {string} envType - 环境类型 (python, node, shell)
     * @param {string} [envName] - 环境名称 (版本号等)
     */
    async listPackages(envType, envName = null) {
        if (envType === "shell") {
            return await this._request('GET', '/environments/shell/packages');
        }
        return await this._request('GET', `/environments/${envType}/${envName}/packages`);
    }

    /**
     * 创建并安装新环境 (例如下载新的 Node.js 版本)
     * @param {string} envType - 环境类型 (python, node)
     * @param {string} name - 名称/版本号
     * @param {string} version - 完整版本标识
     */
    async createEnvironment(envType, name, version) {
        const data = { name, version };
        return await this._request('POST', `/environments/${envType}`, data);
    }

    /**
     * 安装依赖包
     * @param {string} envType - 环境类型
     * @param {string} envName - 环境名称
     * @param {Array<string>} packages - 包名列表 (例如 ['requests', 'flask'])
     */
    async installPackages(envType, envName, packages) {
        const data = { packages };
        if (envType === "shell") {
            return await this._request('POST', '/environments/shell/packages', data);
        }
        return await this._request('POST', `/environments/${envType}/${envName}/packages`, data);
    }

    /**
     * 卸载依赖包
     * @param {string} envType - 环境类型
     * @param {string} envName - 环境名称
     * @param {string} pkg - 包名
     */
    async uninstallPackage(envType, envName, pkg) {
        if (envType === "shell") {
            return await this._request('DELETE', `/environments/shell/packages/${pkg}`);
        }
        return await this._request('DELETE', `/environments/${envType}/${envName}/packages/${pkg}`);
    }

    /**
     * 设置 Node.js 默认环境
     * @param {string} envName - 环境版本号
     */
    async setNodeDefault(envName) {
        return await this._request('POST', `/environments/node/${envName}/set-default`);
    }

    /**
     * 删除环境 (物理删除)
     * @param {string} envType - 环境类型
     * @param {string} envName - 环境名称
     */
    async deleteEnvironment(envType, envName) {
        return await this._request('DELETE', `/environments/${envType}/${envName}`);
    }

    // --- Compiler ---

    /**
     * 获取加密编译器支持的版本
     */
    async getCompilerVersions() {
        return await this._request('GET', '/compiler/versions');
    }

    /**
     * 加密代码 (JSC/PYC)
     * @param {string} language - 语言 (nodejs, python)
     * @param {string} code - 源代码内容
     * @param {Object} [options] - 编译器选项
     */
    async encryptCode(language, code, options = {}) {
        return await this._request('POST', '/compiler/encrypt', { language, code, options });
    }

    // --- Files ---

    /**
     * 列出脚本文件夹内容
     * @param {string} [path] - 相对路径
     */
    async listFiles(path = "") {
        let urlPath = "/files/scripts";
        if (path) {
            urlPath += "/" + path.replace(/^\//, "");
        }
        return await this._request('GET', urlPath);
    }

    /**
     * 读取文件内容
     * @param {string} path - 文件路径
     * @returns {Promise<string>} 文件文本内容
     */
    async readFile(path) {
        const urlPath = "/files/file/" + path.replace(/^\//, "");
        return await this._request('GET', urlPath);
    }

    /**
     * 写文件内容
     * @param {string} path - 文件路径
     * @param {string} content - 文件内容
     */
    async writeFile(path, content) {
        const payload = { path, content };
        try {
            return await this._request('PUT', '/files/file', payload);
        } catch (e) {
            if (e.message.includes('404')) {
                return await this._request('POST', '/files/file', payload);
            }
            throw e;
        }
    }

    /**
     * 删除文件或目录
     * @param {string} path - 相对路径
     */
    async deleteFile(path) {
        const urlPath = "/files/scripts/" + path.replace(/^\//, "");
        return await this._request('DELETE', urlPath);
    }

    // --- Webhook ---

    /**
     * 推送系统通知 (Webhook)
     * @param {string} title - 通知标题
     * @param {string} content - 通知内容
     * @param {string|null} [level] - 级别 (info, success, warning, error)
     */
    async pushNotification(title, content, level = null) {
        const payload = { title, content };
        if (level) payload.level = level;
        return await this._request('POST', '/webhook/push', payload);
    }

    // --- Share (transit station) ---

    /**
     * 列出中转站文件
     */
    async listStationFiles() {
        return await this._request('GET', '/share/station/list');
    }

    /**
     * 获取中转站存储统计
     */
    async getStationStats() {
        return await this._request('GET', '/share/station/stats');
    }

    /**
     * 创建文件分享链接
     * @param {Object} data - 分享配置
     */
    async createShare(data) {
        return await this._request('POST', '/share/create', data);
    }

    // --- UI Helpers ---

    showQrCode(dataBase64) {
        /** 在日志查看器中显示二维码 */
        if (!dataBase64) return;
        let data = String(dataBase64).trim();
        if (!data.startsWith("data:")) {
            data = `data:image/png;base64,${data}`;
        }
        process.stdout.write(`[UI:QRCODE] ${data}\n`);
    }

    closeQrCode() {
        /** 关闭二维码显示 */
        process.stdout.write("[UI:CLOSE_QRCODE]\n");
    }

    updateProgress(percent) {
        /** 更新进度条百分比 (0-100) */
        let val = parseInt(percent, 10);
        if (isNaN(val)) return;
        val = Math.max(0, Math.min(100, val));
        process.stdout.write(`[UI:PROGRESS] ${val}\n`);
    }

    closeProgress() {
        /** 隐藏进度条 */
        process.stdout.write("[UI:CLOSE_PROGRESS]\n");
    }
}

const sdk = new NiuPanelSDK();
module.exports = sdk;

// Export individual functions for easier access
module.exports.showQrCode = sdk.showQrCode.bind(sdk);
module.exports.closeQrCode = sdk.closeQrCode.bind(sdk);
module.exports.updateProgress = sdk.updateProgress.bind(sdk);
module.exports.closeProgress = sdk.closeProgress.bind(sdk);
