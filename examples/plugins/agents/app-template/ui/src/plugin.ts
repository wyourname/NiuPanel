import { createApp, type App as VueApp } from "vue";
import { definePlugin, type NiuPanelPluginContext } from "@niupanel/plugin-sdk";
import App from "./App.vue";

type PluginInstance = VueApp<Element>;

export default definePlugin<PluginInstance>({
  mount(el: HTMLElement, context: NiuPanelPluginContext) {
    const app = createApp(App, { context });
    app.provide("niupanel", context);
    app.mount(el);
    return app;
  },
  unmount(app) {
    app.unmount();
  },
});
