import type { ClientRequest, IncomingMessage } from 'node:http'
import { createHash } from 'node:crypto'
import { readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs'
import { relative, resolve } from 'node:path'
import { fileURLToPath, URL } from 'node:url'
import { defineConfig, type Plugin, type ResolvedConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import UnoCSS from 'unocss/vite'
import monacoEditorPlugin from 'vite-plugin-monaco-editor'

const isCjs = typeof module !== 'undefined' && module.exports
const monacoEditor = (monacoEditorPlugin as any).default || monacoEditorPlugin
const apiProxyTarget = process.env.VITE_API_PROXY_TARGET || 'http://127.0.0.1:7788'
const frontendPackage = JSON.parse(
  readFileSync(fileURLToPath(new URL('./package.json', import.meta.url)), 'utf8')
) as { version: string }
const corePackageSource = readFileSync(
  fileURLToPath(new URL('../niupanel/Cargo.toml', import.meta.url)),
  'utf8'
)
const corePackageVersion =
  corePackageSource.match(/^version\s*=\s*"([^"]+)"/m)?.[1] || ''
if (!corePackageVersion) {
  throw new Error('Unable to read Core version from niupanel/Cargo.toml')
}
const versionContractSource = readFileSync(
  fileURLToPath(new URL('../niupanel-common/src/version.rs', import.meta.url)),
  'utf8'
)
const apiContractVersion = Number.parseInt(
  versionContractSource.match(/pub const API_CONTRACT_VERSION: u32 = (\d+);/)?.[1] || '',
  10
)
if (!Number.isFinite(apiContractVersion)) {
  throw new Error('Unable to read API_CONTRACT_VERSION from niupanel-common/src/version.rs')
}
const webCoreMinVersion = process.env.NIUPANEL_WEB_CORE_MIN || corePackageVersion
const webCoreMaxVersion = process.env.NIUPANEL_WEB_CORE_MAX || null
const setForwardedHost = (proxyReq: ClientRequest, req: IncomingMessage) => {
  const host = req.headers.host
  if (host) {
    proxyReq.setHeader('x-forwarded-host', host)
  }
}

const webReleaseManifestPlugin = (): Plugin => {
  let resolvedConfig: ResolvedConfig
  return {
    name: 'niupanel-web-release-manifest',
    apply: 'build',
    configResolved(config) {
      resolvedConfig = config
    },
    closeBundle() {
      const outputRoot = resolve(process.cwd(), resolvedConfig.build.outDir)
      const manifestPath = resolve(outputRoot, 'release-manifest.json')
      const files: Record<string, string> = {}

      const collectFiles = (directory: string) => {
        for (const name of readdirSync(directory)) {
          const path = resolve(directory, name)
          const stats = statSync(path)
          if (stats.isDirectory()) {
            collectFiles(path)
            continue
          }
          if (path === manifestPath) continue
          const key = relative(outputRoot, path).replaceAll('\\', '/')
          files[key] = createHash('sha256').update(readFileSync(path)).digest('hex')
        }
      }

      collectFiles(outputRoot)
      const sourceDateEpoch = Number.parseInt(process.env.SOURCE_DATE_EPOCH || '', 10)
      const builtAt = Number.isFinite(sourceDateEpoch)
        ? new Date(sourceDateEpoch * 1000).toISOString()
        : undefined
      const manifest = {
        component: 'web',
        version: frontendPackage.version,
        api_contract: apiContractVersion,
        core: {
          min: webCoreMinVersion,
          max: webCoreMaxVersion
        },
        ...(builtAt ? { built_at: builtAt } : {}),
        files
      }
      writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`)
    }
  }
}

// https://vitejs.dev/config/
export default defineConfig({
  define: {
    __APP_VERSION__: JSON.stringify(frontendPackage.version)
  },
  plugins: [
    vue(),
    UnoCSS(),
    monacoEditor({
      languages: ['javascript', 'typescript', 'python', 'shell', 'json', 'html', 'css', 'yaml', 'toml', 'rust', 'go', 'sql', 'xml', 'ini', 'markdown', 'scss', 'less', 'dockerfile']
    }),
    webReleaseManifestPlugin()
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
      '@niupanel/plugin-sdk': fileURLToPath(new URL('../packages/plugin-sdk/src', import.meta.url))
    }
  },
  optimizeDeps: {
    include: [
      'vue',
      'vue-router',
      'pinia',
      'element-plus',
      'element-plus/es/locale/lang/zh-cn',
      'axios',
      'echarts',
      '@vueuse/core',
      'ansi_up'
    ],
    exclude: ['@guolao/vue-monaco-editor']
  },
  server: {
    port: 7787,
    host: true,
    strictPort: true,
    // Increase server responsiveness in dev
    hmr: {
      overlay: true
    },
    proxy: {
      '/mcp': {
        target: apiProxyTarget,
        changeOrigin: false,
        xfwd: true,
        configure(proxy) {
          proxy.on('proxyReq', setForwardedHost)
        }
      },
      '/api': {
        target: apiProxyTarget,
        changeOrigin: true,
        xfwd: true,
        ws: true,
        configure(proxy) {
          proxy.on('proxyReq', setForwardedHost)
          proxy.on('proxyReqWs', setForwardedHost)
        }
      }
    }
  },
  build: {
    chunkSizeWarningLimit: 4000,
    reportCompressedSize: false,
    rollupOptions: {
      output: {
        // Aggressive merging of smaller chunks to reduce request count
        manualChunks(id) {
          if (id.includes('node_modules')) {
            if (id.includes('echarts') || id.includes('zrender')) {
              return 'vendor-charts'
            }
            // ⚠️ 不要手动分割 monaco-editor，让 vite-plugin-monaco-editor 自己处理
            // 否则会破坏 Web Worker 的加载机制导致生产环境报错
            if (id.includes('element-plus')) {
              return 'vendor-ui'
            }
            if (id.includes('vue') || id.includes('pinia') || id.includes('router')) {
              return 'vendor-core'
            }
            // Group all other dependencies to prevent 'fragmentation'
            return 'vendor-others'
          }
        }
      }
    }
  }
})
