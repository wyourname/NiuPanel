# NiuPanel 前端设计规范

本规范面向管理面板、任务工作区和插件原生页面。目标是保持安静、紧凑、专业的运维工具体验，不追求营销页面式的装饰效果。

## 基础原则

- 信息层级优先于装饰。页面背景、内容表面、选中态最多形成三层视觉层级。
- 页面区块默认不悬浮；卡片只用于独立重复项、对话框和确实需要边界的工具区域。
- 不使用渐变、装饰光斑、负字距和大面积毛玻璃。
- 交互反馈使用颜色、边框和轻阴影，不使用位移或缩放造成布局抖动。
- 所有可操作控件必须有 hover、disabled 和 `focus-visible` 状态。

## 设计令牌

令牌定义在 `niupanelweb/src/assets/styles/index.css`，组件优先使用语义类，不直接复制颜色值。

### 字体

- 正文：系统无衬线字体栈，中文优先 `PingFang SC`、`Microsoft YaHei`。
- 等宽内容：`SFMono-Regular`、`Consolas`、`Liberation Mono`。
- 字号只使用 `11px`、`12px`、`14px`、`16px`、`20px` 五级。
- 页面标题使用 `20px`，区块标题使用 `16px`，正文使用 `14px`，辅助信息使用 `11px` 或 `12px`。
- 字距固定为 `0`，不使用 uppercase 制造标签层级。

### 间距

- 采用 4px 基线：`4 / 8 / 12 / 16 / 20 / 24 / 32px`。
- 页面常规内边距：移动端 `16px`，桌面端 `16-20px`。
- 表单项和工具栏间距：`8px`；区块间距：`12-16px`。
- 移动端需要避让底部 Dock 的滚动区统一使用 `mobile-dock-safe`，禁止重复写 `pb-20`、`pb-24` 或安全区计算。

### 控件

- 小型控件：`32px`。
- 默认控件：`36px`。
- 强调操作和移动端主操作：`40px`。
- 图标按钮保持固定宽高，文字按钮使用图标加明确命令文本。

### 圆角与阴影

- 紧凑元素：`4px`。
- 输入框、按钮、标签：`6px`。
- 面板、列表、普通卡片：`8px`。
- 对话框和少量强调表面：`12px`。
- `16px` 只保留给特殊容器，不用于普通列表或卡片。
- 默认使用 `--shadow-sm`；窗口和对话框才允许使用 `--shadow-md`。

### 表面与颜色

- 页面底色：`bg-base`。
- 主内容表面：`bg-card`。
- 次级区域和 hover：`bg-subtle`。
- 选中态和品牌弱背景：`bg-soft`。
- 边界使用 `border-light`，需要更强区分时使用 `border-base`。

## 页面结构

- 桌面模块使用 `WorkspaceAppFrame` 管理侧栏、工具栏、内容和状态栏。
- 标准模块使用 `module-shell`、`module-panel`、`module-toolbar`。
- 同类设置项或菜单使用 `surface-list`，不要为每一行创建独立浮动卡片。
- 插件 Vue 页面应复用宿主 CSS 变量和插件 SDK，不自行创建另一套主题。
- 已在底部 Dock 暴露的一级功能，不应在“更多”页重复展示；“更多”只承载低频入口。

## 版本管理

- 前端版本唯一来源为 `niupanelweb/package.json`。
- Vite 在构建时注入 `__APP_VERSION__`，业务代码通过 `src/version.ts` 读取。
- 服务端版本和 Web 版本独立管理，界面必须明确标注“服务端”和“Web”。
- 发布 Web 资源时必须递增前端版本；只发布后端时不强制同步前端版本。

## 检查

在提交前运行：

```bash
cd niupanelweb
npm run verify:ui-design-system
npm exec vue-tsc -- --noEmit
npm run build
```
