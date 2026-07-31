import os
import time
from niu import niu as sdk

# 设置日志级别以查看警告
import logging
logging.basicConfig(level=logging.INFO)

def run_tests():
    print("=== NiuPanel Python SDK Test Suite ===\n")

    # 1. 概览
    try:
        overview = sdk.get_overview()
        print("[PASS] Overview retrieved.")
    except Exception as e:
        print(f"[FAIL] Overview: {e}")

    # 2. 变量精准操作
    print("\n[Testing Variables]")
    test_key = f"test_py_{int(time.time())}"

    try:
        # 创建
        print(f"    Creating: {test_key}")
        sdk.create_variable({
            "key": test_key,
            "value": "initial",
            "scope": "Global",
            "remarks": "Python SDK Test"
        })

        # 获取全部数据模型
        vars_list = sdk.get_variable(test_key)
        print(f"    Models found: {len(vars_list)}")

        if vars_list:
            target = vars_list[0]
            var_id = target['id']
            print(f"    Target ID: {var_id}")

            # 获取所有值
            values = sdk.get_variable_values(test_key)
            print(f"    Values list: {values}")

            # 精准更新
            print(f"    Updating ID {var_id}...")
            sdk.update_variable(var_id, {
                "key": test_key,
                "value": "updated_by_py"
            })

            # 校验
            new_val = sdk.get_variable_values(test_key)
            print(f"    New value: {new_val[0]}")

            # 精准删除
            print(f"    Deleting ID {var_id}...")
            sdk.delete_variable(var_id)
            print("    [PASS] Variable CRUD")
        else:
            print("    [FAIL] Could not find created variable.")

    except Exception as e:
        print(f"    [FAIL] Variables: {e}")

    # 3. 任务与文件
    try:
        tasks = sdk.list_tasks()
        print(f"\n[PASS] Tasks list retrieved.")

        files = sdk.list_files("")
        print("[PASS] Files list retrieved.")
    except Exception as e:
        print(f"\n[FAIL] Tasks/Files: {e}")

    # 4. Jobs
    print("\n[Testing Jobs]")
    try:
        jobs = sdk.list_jobs()
        print(f"[PASS] Jobs list retrieved.")
    except Exception as e:
        print(f"[FAIL] Jobs: {e}")

    # 5. Environments
    print("\n[Testing Environments]")
    try:
        versions = sdk.list_available_versions()
        print(f"[PASS] Available versions: {versions}")
    except Exception as e:
        print(f"[FAIL] Environments: {e}")

    # 6. Compiler
    print("\n[Testing Compiler]")
    try:
        compiler_versions = sdk.get_compiler_versions()
        print(f"[PASS] Compiler versions: {compiler_versions}")
    except Exception as e:
        print(f"[FAIL] Compiler: {e}")

    # 7. Webhook 推送
    print("\n[Testing Webhook]")
    try:
        sdk.push_notification("Python SDK Test", "Hello from Python SDK!", "info")
        print("[PASS] Webhook push successful.")
    except Exception as e:
        print(f"[FAIL] Webhook push: {e}")

if __name__ == "__main__":
    if not os.environ.get("NiuPanel_Key"):
        print("Warning: NiuPanel_Key not set in environment.\n")
    run_tests()
