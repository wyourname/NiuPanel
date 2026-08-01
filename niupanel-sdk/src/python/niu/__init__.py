import os
import json as json_module
import logging
from typing import Optional, List, Dict, Any, Union
import sys
from urllib import parse as url_parse
from urllib import request as url_request
from urllib import error as url_error

try:
    import requests
except ImportError:
    requests = None

DEFAULT_BASE_URL = "http://127.0.0.1:7788/open/api"


def _load_internal_context() -> Dict[str, Any]:
    context_path = os.environ.get("NIUPANEL_SDK_CONTEXT")
    if not context_path:
        return {}

    try:
        with open(context_path, "r", encoding="utf-8") as context_file:
            data = json_module.load(context_file)
            return data if isinstance(data, dict) else {}
    except Exception:
        return {}


class _UrllibResponse:
    def __init__(self, status_code: int, text: str):
        self.status_code = status_code
        self.text = text

    def json(self) -> Any:
        return json_module.loads(self.text)


class _UrllibSession:
    def __init__(self):
        self.opener = url_request.build_opener(url_request.ProxyHandler({}))
        self.trust_env = False

    def get(self, url: str, headers: Optional[Dict[str, str]] = None, params: Optional[Dict[str, Any]] = None):
        return self._request("GET", url, headers=headers, params=params)

    def post(self, url: str, headers: Optional[Dict[str, str]] = None, json: Any = None, params: Optional[Dict[str, Any]] = None):
        return self._request("POST", url, headers=headers, json_body=json, params=params)

    def put(self, url: str, headers: Optional[Dict[str, str]] = None, json: Any = None, params: Optional[Dict[str, Any]] = None):
        return self._request("PUT", url, headers=headers, json_body=json, params=params)

    def patch(self, url: str, headers: Optional[Dict[str, str]] = None, json: Any = None, params: Optional[Dict[str, Any]] = None):
        return self._request("PATCH", url, headers=headers, json_body=json, params=params)

    def delete(self, url: str, headers: Optional[Dict[str, str]] = None, json: Any = None, params: Optional[Dict[str, Any]] = None):
        return self._request("DELETE", url, headers=headers, json_body=json, params=params)

    def _request(
        self,
        method: str,
        url: str,
        headers: Optional[Dict[str, str]] = None,
        json_body: Any = None,
        params: Optional[Dict[str, Any]] = None,
    ):
        full_url = self._with_query(url, params)
        data = None
        if json_body is not None:
            data = json_module.dumps(json_body).encode("utf-8")

        req = url_request.Request(
            full_url,
            data=data,
            headers=headers or {},
            method=method,
        )
        try:
            with self.opener.open(req, timeout=30) as response:
                body = response.read().decode("utf-8", errors="replace")
                return _UrllibResponse(response.getcode(), body)
        except url_error.HTTPError as e:
            body = e.read().decode("utf-8", errors="replace")
            return _UrllibResponse(e.code, body)

    def _with_query(self, url: str, params: Optional[Dict[str, Any]]) -> str:
        if not params:
            return url

        filtered = {
            key: value
            for key, value in params.items()
            if value is not None
        }
        if not filtered:
            return url

        separator = "&" if "?" in url else "?"
        return f"{url}{separator}{url_parse.urlencode(filtered, doseq=True)}"


class NiuPanelSDK:
    def __init__(self, base_url: Optional[str] = None, api_key: Optional[str] = None):
        self.context = _load_internal_context()
        context_base_url = self.context.get("base_url")
        context_token = self.context.get("token")

        resolved_base_url = (
            base_url
            or context_base_url
            or os.environ.get("NIUPANEL_SDK_BASE_URL")
            or DEFAULT_BASE_URL
        )
        self.base_url = resolved_base_url.rstrip('/')
        self.api_key = (
            api_key
            or context_token
            or os.environ.get("NIUPANEL_INTERNAL_TOKEN")
            or os.environ.get("NiuPanel_Key")
        )
        self.logger = logging.getLogger("NiuPanelSDK")

        self.session = requests.Session() if requests else _UrllibSession()
        self.session.trust_env = False

        if not self.api_key:
             self.logger.warning("未检测到 NiuPanel SDK 上下文或 API Key，认证接口将无法调用")

    def _get_headers(self) -> Dict[str, str]:
        headers = {
            "Content-Type": "application/json"
        }
        if self.api_key:
            headers["X-API-Key"] = self.api_key
        return headers

    def _handle_response(self, response: Any) -> Any:
        try:
            # 尝试解析 JSON
            try:
                data = response.json()
                if isinstance(data, dict) and "code" in data:
                    if data.get("code") == 0:
                        result = data.get("data")
                        return result if result is not None else True
                    else:
                        msg = data.get('message', 'Unknown error')
                        code = data.get('code', -1)
                        raise Exception(f"API Error (code {code}): {msg}")
                # 如果是直接返回的列表或其他 JSON 结构
                return data
            except ValueError:
                # 非 JSON 响应，检查 HTTP 状态
                if 400 <= response.status_code < 600:
                    raise Exception(f"HTTP {response.status_code}: {response.text}")
                return response.text

        except Exception:
            raise

    # --- Variables ---

    def list_variables(self, key: Optional[str] = None, page: int = 1, page_size: int = 100) -> Dict[str, Any]:
        """
        列出变量
        :param key: 变量键名过滤 (optional)
        :param page: 页码 (int, default 1)
        :param page_size: 每页条数 (int, default 100)
        :return: 包含变量列表和总数的字典
        """
        params = {"page": page, "page_size": page_size}
        if key:
            params["key"] = key
        response = self.session.get(f"{self.base_url}/variables/", headers=self._get_headers(), params=params)
        return self._handle_response(response)

    def get_variable(self, key: str) -> List[Dict[str, Any]]:
        """
        获取变量的全部模型数据
        :param key: 变量键名
        :return: 变量模型列表 [dict, dict, ...]
        """
        params = {'key': key}
        response = self.session.get(f"{self.base_url}/variables/by-key", params=params, headers=self._get_headers())
        variables = self._handle_response(response)
        if not isinstance(variables, list):
            raise TypeError("NiuPanel API returned an invalid variable response")
        return variables

    def get_variable_values(self, key: str) -> List[str]:
        """
        获取变量的值列表
        :param key: 变量键名
        :return: 值列表 [str, str, ...]
        """
        vars_list = self.get_variable(key)
        return [str(v.get("value", "")) for v in vars_list]

    def create_variable(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """
        创建变量
        :param data: 变量数据字典
            - key (str): 键名
            - value (str): 值
            - scope (str): 作用域 ('Global' or 'Script')
            - scope_id (int, optional): 关联ID (Legacy)
            - scope_ids (list[int], optional): 关联任务ID列表
            - remarks (str, optional): 备注
            - enabled (bool, optional): 是否启用
        :return: 创建后的变量对象
        """
        response = self.session.post(f"{self.base_url}/variables/", headers=self._get_headers(), json=data)
        return self._handle_response(response)

    def update_variable(self, var_id: Union[int, str], data: Dict[str, Any]) -> Dict[str, Any]:
        """
        根据变量 ID 更新变量数据
        :param var_id: 变量 ID
        :param data: 更新数据字典 (参看 create_variable)
        """
        if data is None:
            raise ValueError("Update data cannot be None. Check if you used dict.update() which returns None.")
        response = self.session.patch(f"{self.base_url}/variables/{var_id}", headers=self._get_headers(), json=data)
        return self._handle_response(response)

    def delete_variable(self, var_id: Union[int, str]) -> bool:
        """根据变量 ID 删除单个变量"""
        return self.batch_delete_variables([int(var_id)])

    def batch_delete_variables(self, ids: List[int]) -> bool:
        """批量删除变量"""
        response = self.session.delete(f"{self.base_url}/variables/", headers=self._get_headers(), json={"ids": ids})
        return self._handle_response(response)

    def update_variable_by_key(self, key: str, data: Dict[str, Any]) -> List[Dict[str, Any]]:
        """
        通过 Key 更新所有匹配的变量
        :param key: 变量键名
        :param data: 更新数据字典
        :return: 更新后的变量列表
        """
        params = {'key': key}
        response = self.session.patch(f"{self.base_url}/variables/by-key", params=params, headers=self._get_headers(), json=data)
        return self._handle_response(response)

    # --- Tasks ---

    def list_tasks(self, name: Optional[str] = None, page: int = 1, page_size: int = 100) -> Dict[str, Any]:
        """
        列出任务
        :param name: 任务名称过滤 (Optional)
        :param page: 页码 (int, default 1)
        :param page_size: 每页条数 (int, default 100)
        :return: 包含任务列表和分页信息的字典
        """
        params = {"page": page, "page_size": page_size}
        if name:
            params["name"] = name
        response = self.session.get(f"{self.base_url}/tasks/", headers=self._get_headers(), params=params)
        return self._handle_response(response)

    def get_task_info(self, task_id: Union[int, str]) -> Optional[Dict[str, Any]]:
        """
        获取单个任务的详细信息
        :param task_id: 任务 ID
        :return: 任务数据字典或 None
        """
        data = self.list_tasks()
        items = data.get("items", []) if isinstance(data, dict) else (data if isinstance(data, list) else [])
        for item in items:
            if str(item.get("id")) == str(task_id):
                return item
        return None

    def _task_action(self, task_id: Union[int, str], action: str) -> Any:
        response = self.session.post(f"{self.base_url}/tasks/{action}", headers=self._get_headers(), json={"ids": [int(task_id)]})
        return self._handle_response(response)

    def run_task(self, task_id: Union[int, str]) -> Any:
        """运行任务"""
        return self.batch_run_tasks([int(task_id)])
    def stop_task(self, task_id: Union[int, str]) -> Any:
        """停止任务"""
        return self.batch_stop_tasks([int(task_id)])
    def enable_task(self, task_id: Union[int, str]) -> Any:
        """启用任务"""
        return self.batch_enable_tasks([int(task_id)])
    def disable_task(self, task_id: Union[int, str]) -> Any:
        """禁用任务"""
        return self.batch_disable_tasks([int(task_id)])
    def pause_task(self, task_id: Union[int, str]) -> Any:
        """暂停任务"""
        return self.batch_pause_tasks([int(task_id)])
    def resume_task(self, task_id: Union[int, str]) -> Any:
        """恢复任务"""
        return self.batch_resume_tasks([int(task_id)])

    def batch_run_tasks(self, ids: List[int]) -> Any:
        """批量运行任务"""
        return self._task_batch_action("run", ids)
    def batch_stop_tasks(self, ids: List[int]) -> Any:
        """批量停止任务"""
        return self._task_batch_action("stop", ids)
    def batch_enable_tasks(self, ids: List[int]) -> Any:
        """批量启用任务"""
        return self._task_batch_action("enable", ids)
    def batch_disable_tasks(self, ids: List[int]) -> Any:
        """批量禁用任务"""
        return self._task_batch_action("disable", ids)
    def batch_pause_tasks(self, ids: List[int]) -> Any:
        """批量暂停任务"""
        return self._task_batch_action("pause", ids)
    def batch_resume_tasks(self, ids: List[int]) -> Any:
        """批量恢复任务"""
        return self._task_batch_action("resume", ids)

    def _task_batch_action(self, action: str, ids: List[int]) -> Any:
        response = self.session.post(f"{self.base_url}/tasks/{action}", headers=self._get_headers(), json={"ids": ids})
        return self._handle_response(response)

    def delete_task(self, task_id: Union[int, str], delete_script: bool = False, delete_var: bool = False) -> Any:
        """删除单个任务"""
        return self.batch_delete_tasks([int(task_id)], delete_script=delete_script, delete_var=delete_var)

    def batch_delete_tasks(self, ids: List[int], delete_script: bool = False, delete_var: bool = False) -> Any:
        """
        批量删除任务
        :param ids: 任务 ID 列表
        :param delete_script: 是否同时删除脚本文件 (bool)
        :param delete_var: 是否同时删除关联变量 (bool)
        """
        params = {}
        if delete_script: params["delete_script"] = "true"
        if delete_var: params["delete_var"] = "true"
        response = self.session.delete(f"{self.base_url}/tasks/", headers=self._get_headers(), json={"ids": ids}, params=params)
        return self._handle_response(response)

    def get_task_logs(self, task_id: Union[int, str]) -> str:
        """
        获取任务最新日志内容
        :param task_id: 任务 ID
        """
        response = self.session.get(f"{self.base_url}/tasks/{task_id}/logs/latest", headers=self._get_headers())
        return self._handle_response(response)

    def get_task_run_log(self, task_id: Union[int, str], run_id: Union[int, str]) -> str:
        """
        获取特定历史运行记录的日志内容
        :param task_id: 任务 ID
        :param run_id: 运行历史记录 ID
        """
        response = self.session.get(f"{self.base_url}/tasks/{task_id}/history/{run_id}/log", headers=self._get_headers())
        return self._handle_response(response)

    def get_task_history(self, task_id: Union[int, str], page: int = 1, page_size: int = 20) -> Dict[str, Any]:
        """
        获取任务运行历史记录
        :param task_id: 任务 ID
        :param page: 页码 (default 1)
        :param page_size: 每页条数 (default 20)
        """
        params = {"page": page, "page_size": page_size}
        response = self.session.get(f"{self.base_url}/tasks/{task_id}/history", headers=self._get_headers(), params=params)
        return self._handle_response(response)

    def create_task(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """
        创建任务
        :param data: 任务数据字典
            - name (str): 任务名称
            - path (str, optional): 脚本路径
            - command (str, optional): 自定义命令
            - env_type (str): 环境类型 ('Python', 'Nodejs', 'Shell')
            - env_version (str, optional): 环境版本
            - cron_schedule (str, optional): Cron表达式
            - variables (list[dict], optional): 任务变量 [{key, value}, ...]
        :return: 创建的任务对象
        """
        response = self.session.post(f"{self.base_url}/tasks/", headers=self._get_headers(), json=data)
        return self._handle_response(response)

    def quick_create_task(self, url: str) -> Dict[str, Any]:
        """
        通过脚本 URL 快速创建任务
        :param url: 脚本下载 URL (str)
        """
        response = self.session.post(f"{self.base_url}/tasks/quick_create", headers=self._get_headers(), json={"url": url})
        return self._handle_response(response)

    def update_task(self, task_id: Union[int, str], data: Dict[str, Any]) -> Dict[str, Any]:
        """
        更新任务配置
        :param task_id: 任务 ID
        :param data: 更新后的任务数据 (dict, 参看 create_task)
        """
        response = self.session.patch(f"{self.base_url}/tasks/{task_id}", headers=self._get_headers(), json=data)
        return self._handle_response(response)

    def delete_task_run(self, task_id: Union[int, str], run_id: Union[int, str]) -> bool:
        """
        删除单条运行历史记录
        :param task_id: 任务 ID
        :param run_id: 运行记录 ID
        """
        response = self.session.delete(f"{self.base_url}/tasks/{task_id}/history/{run_id}", headers=self._get_headers())
        return self._handle_response(response)

    def pin_task(self, task_id: Union[int, str]) -> Any:
        """置顶任务"""
        return self.batch_pin_tasks([int(task_id)])

    def unpin_task(self, task_id: Union[int, str]) -> Any:
        """取消置顶"""
        return self.batch_unpin_tasks([int(task_id)])

    def batch_pin_tasks(self, ids: List[int]) -> Any:
        """批量置顶任务"""
        response = self.session.post(f"{self.base_url}/tasks/pin", headers=self._get_headers(), json={"ids": ids})
        return self._handle_response(response)

    def batch_unpin_tasks(self, ids: List[int]) -> Any:
        """批量取消置顶"""
        response = self.session.post(f"{self.base_url}/tasks/unpin", headers=self._get_headers(), json={"ids": ids})
        return self._handle_response(response)

    # --- Jobs ---

    def list_jobs(self, page: int = 1, page_size: int = 100) -> Dict[str, Any]:
        """
        列出所有正在执行或队列中的作业 (Jobs)
        :param page: 页码
        :param page_size: 每页数量
        """
        params = {"page": page, "page_size": page_size}
        response = self.session.get(f"{self.base_url}/jobs/", headers=self._get_headers(), params=params)
        return self._handle_response(response)

    def get_job(self, job_id: Union[int, str]) -> Dict[str, Any]:
        response = self.session.get(f"{self.base_url}/jobs/{job_id}", headers=self._get_headers())
        return self._handle_response(response)

    def get_job_logs(self, job_id: Union[int, str]) -> str:
        response = self.session.get(f"{self.base_url}/jobs/{job_id}/logs", headers=self._get_headers())
        return self._handle_response(response)

    def get_latest_job_log(self, job_id: Union[int, str]) -> str:
        response = self.session.get(f"{self.base_url}/jobs/{job_id}/logs/latest", headers=self._get_headers())
        return self._handle_response(response)

    def cancel_job(self, job_id: Union[int, str]) -> Any:
        """
        强制取消正在执行的作业
        :param job_id: 作业 ID (UUID str)
        """
        response = self.session.post(f"{self.base_url}/jobs/{job_id}/cancel", headers=self._get_headers())
        return self._handle_response(response)

    # --- Overview ---

    def get_overview(self) -> Dict[str, Any]:
        response = self.session.get(f"{self.base_url}/overview/", headers=self._get_headers())
        return self._handle_response(response)

    # --- Environments ---

    def list_environments(self) -> List[Dict[str, Any]]:
        response = self.session.get(f"{self.base_url}/environments/", headers=self._get_headers())
        return self._handle_response(response)

    def list_available_versions(self) -> List[str]:
        response = self.session.get(f"{self.base_url}/environments/versions", headers=self._get_headers())
        return self._handle_response(response)

    def list_packages(self, env_type: str, env_name: Optional[str] = None) -> List[str]:
        if env_type == "shell":
            url = f"{self.base_url}/environments/shell/packages"
        else:
            url = f"{self.base_url}/environments/{env_type}/{env_name}/packages"
        response = self.session.get(url, headers=self._get_headers())
        return self._handle_response(response)

    def create_environment(self, env_type: str, name: str, version: str) -> Dict[str, Any]:
        data = {"name": name, "version": version}
        response = self.session.post(f"{self.base_url}/environments/{env_type}", headers=self._get_headers(), json=data)
        return self._handle_response(response)

    def install_packages(self, env_type: str, env_name: str, packages: List[str]) -> Any:
        data = {"packages": packages}
        if env_type == "shell":
            url = f"{self.base_url}/environments/shell/packages"
        else:
            url = f"{self.base_url}/environments/{env_type}/{env_name}/packages"
        response = self.session.post(url, headers=self._get_headers(), json=data)
        return self._handle_response(response)

    def uninstall_package(self, env_type: str, env_name: str, pkg: str) -> Any:
        if env_type == "shell":
            url = f"{self.base_url}/environments/shell/packages/{pkg}"
        else:
            url = f"{self.base_url}/environments/{env_type}/{env_name}/packages/{pkg}"
        response = self.session.delete(url, headers=self._get_headers())
        return self._handle_response(response)

    def set_node_default(self, env_name: str) -> Any:
        response = self.session.post(f"{self.base_url}/environments/node/{env_name}/set-default", headers=self._get_headers())
        return self._handle_response(response)

    def delete_environment(self, env_type: str, env_name: str) -> Any:
        response = self.session.delete(f"{self.base_url}/environments/{env_type}/{env_name}", headers=self._get_headers())
        return self._handle_response(response)

    # --- Compiler ---

    def get_compiler_versions(self) -> List[str]:
        response = self.session.get(f"{self.base_url}/compiler/versions", headers=self._get_headers())
        return self._handle_response(response)

    def encrypt_code(self, language: str, code: str, options: Dict[str, Any] = None) -> Dict[str, Any]:
        data = {"language": language, "code": code, "options": options or {}}
        response = self.session.post(f"{self.base_url}/compiler/encrypt", headers=self._get_headers(), json=data)
        return self._handle_response(response)

    # --- Files ---

    def list_files(self, path: str = "") -> List[Dict[str, Any]]:
        url = f"{self.base_url}/files/scripts"
        if path:
            url = f"{url}/{path.lstrip('/')}"
        response = self.session.get(url, headers=self._get_headers())
        return self._handle_response(response)

    def read_file(self, path: str) -> str:
        """
        读取脚本文件内容
        :param path: 文件的相对路径 (str)
        :return: 文件文本内容
        """
        url = f"{self.base_url}/files/file/{path.lstrip('/')}"
        response = self.session.get(url, headers=self._get_headers())
        return self._handle_response(response)

    def write_file(self, path: str, content: str) -> Any:
        """
        保存/写入脚本文件
        :param path: 文件相对路径 (str)
        :param content: 文件文本内容 (str)
        """
        payload = {"path": path, "content": content}
        response = self.session.put(f"{self.base_url}/files/file", headers=self._get_headers(), json=payload)
        if response.status_code == 404:
             response = self.session.post(f"{self.base_url}/files/file", headers=self._get_headers(), json=payload)
        return self._handle_response(response)

    def delete_file(self, path: str) -> Any:
        """
        删除脚本文件或目录
        :param path: 相对路径 (str)
        """
        url = f"{self.base_url}/files/scripts/{path.lstrip('/')}"
        response = self.session.delete(url, headers=self._get_headers())
        return self._handle_response(response)

    # --- Webhook ---

    def push_notification(self, title: str, content: str, level: Optional[str] = None) -> Any:
        """推送系统通知"""
        payload = {"title": title, "content": content}
        if level:
            payload["level"] = level
        response = self.session.post(f"{self.base_url}/webhook/push", headers=self._get_headers(), json=payload)
        return self._handle_response(response)

    # --- Share (transit station) ---

    def list_station_files(self) -> List[Dict[str, Any]]:
        """列出中转站文件列表"""
        response = self.session.get(f"{self.base_url}/share/station/list", headers=self._get_headers())
        return self._handle_response(response)

    def get_station_stats(self) -> Dict[str, Any]:
        """获取中转站存储统计信息"""
        response = self.session.get(f"{self.base_url}/share/station/stats", headers=self._get_headers())
        return self._handle_response(response)

    def create_share(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """
        创建文件分享
        :param data: 分享配置字典 (dict)
        """
        response = self.session.post(f"{self.base_url}/share/create", headers=self._get_headers(), json=data)
        return self._handle_response(response)

# Default instance
niu = NiuPanelSDK()


def show_qrcode(data_base64: str) -> None:
    """
    Generate a UI command to display a QR Code in the task log viewer.

    Args:
        data_base64 (str): The base64 encoded image data (e.g., 'data:image/png;base64,...').
                           If the prefix is missing, it will be added assuming PNG format.
    """
    if not data_base64:
        return

    data = data_base64.strip()
    if not data.startswith("data:"):
        data = f"data:image/png;base64,{data}"

    print(f"[UI:QRCODE] {data}")
    sys.stdout.flush()

def close_qrcode() -> None:
    """
    Generate a UI command to close the QR Code display.
    """
    print("[UI:CLOSE_QRCODE]")
    sys.stdout.flush()

def update_progress(percent: int) -> None:
    """
    Generate a UI command to update the progress bar in the task log viewer.

    Args:
        percent (int): The progress percentage (0-100).
    """
    try:
        val = int(percent)
        val = max(0, min(100, val))
        print(f"[UI:PROGRESS] {val}")
        sys.stdout.flush()
    except (ValueError, TypeError):
        pass

def close_progress() -> None:
    """
    Generate a UI command to close/hide the progress bar.
    """
    print("[UI:CLOSE_PROGRESS]")
    sys.stdout.flush()
