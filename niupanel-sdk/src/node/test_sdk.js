const sdk = require('./niu/index');

if (!process.env.NiuPanel_Key) {
    console.warn("WARNING: NiuPanel_Key environment variable is not set.");
}

async function runTests() {
    console.log("=== NiuPanel Node.js SDK Test Suite ===\n");

    // 1. 系统概览
    try {
        const overview = await sdk.getOverview();
        console.log("[PASS] Overview retrieved.");
    } catch (e) {
        console.error("[FAIL] Overview:", e.message);
    }

    // 2. 变量操作 (新流程)
    console.log("\n[Testing Variables]");
    const testKey = `test_node_${Date.now()}`;
    const testVal = "init_value";

    try {
        // 创建
        console.log(`    Creating: ${testKey}`);
        await sdk.createVariable({
            key: testKey,
            value: testVal,
            scope: "Global",
            remarks: "SDK Test"
        });

        // 获取全部模型
        const vars = await sdk.getVariable(testKey);
        console.log(`    Models found: ${vars.length}`);
        if (vars.length > 0) {
            const varId = vars[0].id;
            console.log(`    Target ID: ${varId}`);

            // 获取全部值
            const values = await sdk.getVariableValues(testKey);
            console.log(`    Values list: ${JSON.stringify(values)}`);

            // 精准更新
            console.log(`    Updating ID ${varId}...`);
            await sdk.updateVariable(varId, {
                key: testKey,
                value: "updated_value"
            });

            // 验证更新
            const updatedVal = await sdk.getVariableValues(testKey);
            console.log(`    New value: ${updatedVal[0]}`);

            // 精准删除
            console.log(`    Deleting ID ${varId}...`);
            await sdk.deleteVariable(varId);
            console.log("    [PASS] Variable CRUD");
        }
    } catch (e) {
        console.error("    [FAIL] Variables:", e.message);
    }

    // 3. 任务与文件
    try {
        const tasks = await sdk.listTasks();
        console.log(`\n[PASS] Tasks list retrieved (${tasks.items ? tasks.items.length : 0} items)`)

        const files = await sdk.listFiles("");
        console.log("[PASS] Files list retrieved.");
    } catch (e) {
        console.error("\n[FAIL] Tasks/Files:", e.message);
    }

    // 4. Jobs
    console.log("\n[Testing Jobs]");
    try {
        const jobs = await sdk.listJobs();
        console.log(`[PASS] Jobs list retrieved (${jobs.items ? jobs.items.length : 0} items)`);
    } catch (e) {
        console.error("[FAIL] Jobs:", e.message);
    }

    // 5. Environments
    console.log("\n[Testing Environments]");
    try {
        const versions = await sdk.listAvailableVersions();
        console.log(`[PASS] Available versions: ${JSON.stringify(versions)}`);
    } catch (e) {
        console.error("[FAIL] Environments:", e.message);
    }

    // 6. Compiler
    console.log("\n[Testing Compiler]");
    try {
        const compilerVersions = await sdk.getCompilerVersions();
        console.log(`[PASS] Compiler versions: ${JSON.stringify(compilerVersions)}`);
    } catch (e) {
        console.error("[FAIL] Compiler:", e.message);
    }

    // 7. Webhook 推送
    console.log("\n[Testing Webhook]");
    try {
        await sdk.pushNotification("Node.js SDK Test", "Hello from Node.js SDK!", "info");
        console.log("[PASS] Webhook push successful.");
    } catch (e) {
        console.error("[FAIL] Webhook push:", e.message);
    }
}

runTests();