import os
import tarfile
import json
import sqlite3
import shutil
from datetime import datetime

# 青龙面板默认路径
QL_DIR = os.environ.get("QL_DIR", "/ql")
# 数据库路径（青龙通常将数据存在 data/database/database.sqlite）
DB_PATH = os.path.join(QL_DIR, "data", "db", "database.sqlite")
# 脚本路径
SCRIPTS_DIR = os.path.join(QL_DIR, "data", "scripts")

def export_tasks():
    print(f"正在开始青龙任务导出...")
    if not os.path.exists(DB_PATH):
        print(f"错误: 未在 {DB_PATH} 找到数据库。请确保在青龙容器内运行此脚本。")
        return

    # 1. 连接数据库获取任务信息和环境变量
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()
    
    try:
        # 查询 crons 表获取任务
        cursor.execute("SELECT id, name, command, schedule, status FROM Crontabs")  
        crons = cursor.fetchall()
        
        # 查询 envs 表获取环境变量
        cursor.execute("SELECT name, value, remarks, status FROM envs")
        envs = cursor.fetchall()
    except Exception as e:
        print(f"读取数据失败: {e}")
        return
    finally:
        conn.close()

    tasks_data = []
    envs_data = []
    export_id = datetime.now().strftime('%Y%m%d%H%M%S')
    export_dir = f"/tmp/ql_export_{export_id}"
    os.makedirs(export_dir, exist_ok=True)
    
    scripts_export_dir = os.path.join(export_dir, "scripts")
    os.makedirs(scripts_export_dir, exist_ok=True)

    print(f"发现 {len(crons)} 个任务，{len(envs)} 个环境变量，正在准备打包...")

    for row in crons:
        task = {
            "name": row['name'],
            "command": row['command'],
            "schedule": row['schedule'],
            "status": row['status']
        }
        tasks_data.append(task)
        
    for row in envs:
        env_item = {
            "name": row['name'],
            "value": row['value'],
            "remarks": row['remarks'],
            "status": row['status']
        }
        envs_data.append(env_item)

    # 2. 写入清单文件
    manifest_path = os.path.join(export_dir, "manifest.json")
    with open(manifest_path, 'w', encoding='utf-8') as f:
        json.dump({
            "tasks": tasks_data,
            "envs": envs_data
        }, f, ensure_ascii=False, indent=2)
    
    # 3. 打包脚本文件
    # 青龙的脚本通常在 data/scripts 下，我们完整拷贝结构（排除无关大文件）
    print("正在拷贝脚本文件...")
    if os.path.exists(SCRIPTS_DIR):
        try:
            shutil.copytree(SCRIPTS_DIR, scripts_export_dir, dirs_exist_ok=True, 
                            ignore=shutil.ignore_patterns('.git', 'node_modules', '__pycache__', 'dist'))
        except Exception as e:
            print(f"拷贝脚本时出现警告 (已忽略): {e}")
    else:
        print(f"警告: 未找到脚本目录 {SCRIPTS_DIR}")

    # 4. 创建压缩包
    tar_filename = f"ql_migration_{export_id}.tar.gz"
    print(f"正在创建压缩包 {tar_filename}...")
    with tarfile.open(tar_filename, "w:gz") as tar:
        tar.add(export_dir, arcname="migration_data")
    
    # 清理临时目录
    shutil.rmtree(export_dir)
    print(f"\n导出完成！")
    print(f"打包文件路径: {os.path.abspath(tar_filename)}")
    print("请将此文件下载并上传到 NiuPanel 的文件管理中。")

if __name__ == "__main__":
    export_tasks()
