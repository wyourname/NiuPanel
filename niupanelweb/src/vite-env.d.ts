/// <reference types="vite/client" />

declare const __APP_VERSION__: string;

declare module 'virtual:niupanel-plugin-dev' {
  const entries: Record<string, () => Promise<unknown>>;
  export default entries;
}
