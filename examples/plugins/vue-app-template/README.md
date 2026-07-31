# Vue App Plugin Template

This template builds a native Vue plugin UI for NiuPanel.

Build the UI:

```bash
cd examples/plugins/vue-app-template/ui
pnpm install
pnpm run build
```

Install the plugin package from the panel using the server-side path:

```text
examples/plugins/vue-app-template
```

The plugin UI entry is `ui/dist/niupanel-plugin.js`, and the panel mounts it through the `@niupanel/plugin-sdk` contract:

```ts
export default definePlugin({
  mount(el, context) {},
  unmount(instance, context) {},
});
```
