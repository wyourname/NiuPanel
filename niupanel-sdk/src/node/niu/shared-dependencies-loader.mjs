import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";

const sharedNodeModules = process.env.NIUPANEL_NODE_SHARED_MODULES;
const sharedRoot = sharedNodeModules ? dirname(sharedNodeModules) : null;
const sharedRequire = sharedRoot
    ? createRequire(pathToFileURL(join(sharedRoot, ".niupanel-esm-resolver.cjs")))
    : null;

function isBarePackageSpecifier(specifier) {
    return !specifier.startsWith(".")
        && !specifier.startsWith("/")
        && !specifier.startsWith("file:")
        && !specifier.startsWith("node:");
}

/**
 * Node 的 ESM 解析不会使用 NODE_PATH。仅当脚本目录的正常解析失败时，
 * 回退到 NiuPanel 为当前 Node 版本维护的共享 node_modules；相对路径、
 * 内置模块和脚本自带依赖仍保持 Node 原有语义。
 */
export async function resolve(specifier, context, nextResolve) {
    try {
        return await nextResolve(specifier, context);
    } catch (error) {
        if (
            !sharedRequire
            || error?.code !== "ERR_MODULE_NOT_FOUND"
            || !isBarePackageSpecifier(specifier)
        ) {
            throw error;
        }

        try {
            return {
                url: pathToFileURL(sharedRequire.resolve(specifier)).href,
                shortCircuit: true,
            };
        } catch {
            throw error;
        }
    }
}
