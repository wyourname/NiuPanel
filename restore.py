import os
import tarfile
import json
import glob
import shutil
from niu import niu
import time

def main():
    print("=== NiuPanel 导入青龙数据工具 ===")
    print("*** 使用前请前往系统设置 - API访问 - 创建密钥 创建一个名为 NiuPanel_key 的密钥 勾选开启 (*:*) 完整授权 ***")
    print("\n")
    print("*** 创建全局变量 key: NiuPanel_key value: 刚才创建的密钥 ***")
    print("\n")
    print("*** 导入前注意事项， 请确保在文件管理 创建 import/ql 文件夹 上传ql_migration_*.tar.gz 到文件夹下 ***")
    print("\n")
    time.sleep(5)
    # 获取当前目录
    base_dir = os.path.dirname(os.path.abspath(__file__))
    
    # 寻找青龙导出的 tar.gz 包
    tar_files = glob.glob(os.path.join(base_dir, "ql_migration_*.tar.gz"))
    if not tar_files:
        print("未在当前目录下找到 ql_migration_*.tar.gz 文件！")
        print("请先在青龙面板运行 qinglong_export.py，并将生成的压缩包上传到 NiuPanel 的 import/ql 目录中。")
        return
        
    # 如果有多个，取最新的
    tar_files.sort(key=os.path.getmtime, reverse=True)
    tar_file = tar_files[0]
    
    print(f"找到导出文件: {os.path.basename(tar_file)}")
    print("开始解压...")
    
    extract_dir = os.path.join(base_dir, "extracted_migration")
    if os.path.exists(extract_dir):
        shutil.rmtree(extract_dir)
        
    try:
        with tarfile.open(tar_file, "r:gz") as tar:
            tar.extractall(path=extract_dir)
    except Exception as e:
        print(f"解压失败: {e}")
        return
        
    print("解压完成。开始导入数据到 NiuPanel...")
    
    # 解析 manifest.json
    manifest_path = os.path.join(extract_dir, "migration_data", "manifest.json")
    if not os.path.exists(manifest_path):
        print(f"解析错误: 找不到 {manifest_path}")
        return
        
    with open(manifest_path, "r", encoding="utf-8") as f:
        data = json.load(f)
        
    tasks = data.get("tasks", data) if isinstance(data, dict) else data
    envs = data.get("envs", []) if isinstance(data, dict) else []
    
    // 1. 导入环境变量
    if envs:
        print(f"--- 发现 {len(envs)} 个环境变量，开始导入 ---")
        for env in envs:
            try:
                # 青龙 status: 0 (启用), 1 (禁用)
                is_enabled = env.get("status", 0) == 0
                payload = {
                    "key": env.get("name", ""),
                    "value": env.get("value", ""),
                    "scope": "Global",
                    "remarks": env.get("remarks", ""),
                    "enabled": is_enabled,
                }
                # {"key":"aaa","value":"111","remarks":"","enabled":true,"scope":"Global","scope_ids":[],"scope_id":null}
                # NiuPanel 目前 SDK 只提供了创建变量的方法
                # 如果遇到重名变量，后端可能返回错误，我们捕获即可
                niu.create_variable(payload)
                print(f"成功导入变量: {payload['key']}")
            except Exception as e:
                # 通常是重复或者格式不正确
                print(f"导入变量 {env.get('name')} 失败/或已存在: {e}")
                
    # 2. 迁移脚本文件
    scripts_src_dir = os.path.join(extract_dir, "migration_data", "scripts")
    print(f"--- 准备迁移脚本文件 ---")
    
    # 将脚本文件移动到 NiuPanel 的 import/ql/ 下（即当前执行目录）
    if os.path.exists(scripts_src_dir):
        files_count = 0
        for item in os.listdir(scripts_src_dir):
            s = os.path.join(scripts_src_dir, item)
            d = os.path.join(base_dir, item)
            
            # 不覆盖原有的 tar 文件和解压目录
            if item.startswith("ql_migration_") or item == "extracted_migration" or item == os.path.basename(__file__):
                continue
                
            try:
                if os.path.isdir(s):
                    if os.path.exists(d):
                        shutil.rmtree(d)
                    shutil.copytree(s, d)
                else:
                    shutil.copy2(s, d)
                files_count += 1
            except Exception as e:
                print(f"拷贝脚本文件 {item} 时出错: {e}")
                
        print(f"成功迁移了 {files_count} 个脚本或文件夹到当前目录。")
    
    # 3. 导入任务
    print(f"--- 发现 {len(tasks)} 个定时任务，开始导入 ---")
    for task in tasks:
        ql_cmd = task.get("command", "")
        script_file = ""
        
        # 尝试解析青龙的 task 命令，如 `task /ql/data/scripts/xxx.py` 或者 `task xxx.js`
        parts = ql_cmd.split()
        if len(parts) > 1 and parts[0] == "task":
            script_file = parts[1]
            if "/" in script_file:
                # 只保留文件名及在 scripts 下的相对路径，为简化我们直接取 basename 或者保留目录结构
                # 取决于青龙实际结构，由于拷贝到了 base_dir，我们使用提取出的路径
                
                # 如果是以 /ql/data/scripts/ 开头的绝对路径，去掉它
                if "/ql/data/scripts/" in script_file:
                    script_file = script_file.split("/ql/data/scripts/")[-1]
                else:
                    # 如果不能识别，取文件名
                    script_file = script_file
        
        if not script_file:
            script_file = ql_cmd
            
        # 根据后缀推断 env_type
        env_type = "shell"
        if script_file.endswith(".py"):
            env_type = "python"
        elif script_file.endswith(".js") or script_file.endswith(".ts"):
            env_type = "node"
            
        # 在 NiuPanel 中的相对路径 (NiuPanel 默认以 scripts_dir 为根)
        # 假设该脚本运行在 import/ql 下，所以 NiuPanel 看到的路径是 import/ql/script_file
        np_path = f"import/ql/{script_file}"
        
        # 构建命令
        if env_type == "python":
            new_cmd = f"python3 {np_path}"
        elif env_type == "node":
            new_cmd = f"node {np_path}"
        else:
            new_cmd = f"bash {np_path}"
        task_payload = {
            "name": task.get("name", "未命名任务"),
            "path": np_path,
            "command": new_cmd,
            "description": "从青龙面板导入",
            "env_type": env_type,
            "env_version": "venv_3.11" if env_type == "Python" else "",
            "cron_schedule": task.get("schedule", ""),
        }
        
        is_enabled = task.get("status", 0) == 0
        
        try:
            # OpenAPI 创建任务
            resp = niu.session.post(f"{niu.base_url}/tasks", headers=niu._get_headers(), json=task_payload)
            result = niu._handle_response(resp)
            task_id = result.get("id") if isinstance(result, dict) else None
            
            print(f"成功导入任务: {task_payload['name']} (ID: {task_id})")
            
            # NiuPanel 默认创建时是启用的，如果青龙中是禁用，则需要调用停用接口
            if task_id and not is_enabled:
                niu.disable_task(task_id)
                print(f"  └─ 任务在青龙中为停用状态，已在 NiuPanel 中停用。")
                
        except Exception as e:
            print(f"导入任务 {task_payload['name']} 失败: {e}")
            
    # 清理临时解压目录
    try:
        shutil.rmtree(extract_dir)
    except:
        pass
        
    print("=== 全部导入流程执行完毕 ===")

if __name__ == "__main__":
    main()
