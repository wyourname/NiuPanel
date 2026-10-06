import type { Env, EnvType } from "@/types";

export const environmentLabel = (env: Env) => env.env_type === "sh" ? "Linux" : env.env_type === "node" ? `Node.js ${env.version || env.name}` : `Python ${env.version || env.name}`;

export function parsePackageInput(text: string, type: EnvType) {
  const lines = text.split(/\r?\n/).map(line => line.trim()).filter(line => line && !line.startsWith("#"));
  if (lines.some(line => /^(?:pip3?|uv|npm|pnpm|apt(?:-get)?)\s+(?:pip\s+)?(?:install|add|remove|uninstall)\b/.test(line))) {
    return { packages: [], error: "只填写包名和版本，不要粘贴安装命令" };
  }
  const packages = [...new Set(type === "python" ? lines : lines.flatMap(line => line.split(/\s+/)))];
  if (packages.some(pkg => pkg.startsWith("-"))) return { packages: [], error: "请填写依赖包，不要填写命令行选项" };
  if (type === "python" && packages.some(pkg => /\s/.test(pkg) && !/[<>=!~;@]/.test(pkg))) {
    return { packages: [], error: "Python 依赖请每行填写一个包，可包含版本约束" };
  }
  return { packages, error: "" };
}
