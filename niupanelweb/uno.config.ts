import { defineConfig, presetUno, presetAttributify, presetIcons } from 'unocss'

export default defineConfig({
  safelist: [
    'i-ep-data-analysis',
    'i-carbon-task',
    'i-ep-key',
    'i-ep-folder',
    'i-ep-box',
    'i-ep-guide',
    'i-ep-refresh',
    'i-ep-promotion',
    'i-ep-cpu',
    'i-ep-monitor',
    'i-carbon-terminal',
    'i-carbon-container-services',
    'i-carbon-share',
    'i-carbon-locked',
    'i-ep-setting',
    'i-ep-user',
    'i-ep-plus',
    'i-ep-edit',
    'i-ep-document',
    'i-ep-clock',
    'i-ep-copy-document',
    'i-ep-minus',
    'i-ep-close',
  ],

  presets: [
    presetUno(),
    presetAttributify(),
    presetIcons({
      scale: 1.2,
      // Dynamic Vue templates in this codebase can produce false-positive icon warnings.
      warn: false,
      extraProperties: {
        'display': 'inline-block',
        'vertical-align': 'middle',
      },
    }),
  ],

  shortcuts: {
    // --- Layout ---
    'flex-center': 'flex items-center justify-center',
    'flex-between': 'flex items-center justify-between',
    'flex-col-center': 'flex flex-col items-center justify-center',
    'full': 'w-full h-full',

    // --- Safe Areas ---
    'pt-safe': 'pt-[env(safe-area-inset-top,0px)]',
    'pb-safe': 'pb-[env(safe-area-inset-bottom,0px)]',
    'pl-safe': 'pl-[env(safe-area-inset-left,0px)]',
    'pr-safe': 'pr-[env(safe-area-inset-right,0px)]',

    // --- Semantic Backgrounds ---
    'bg-base': 'bg-[var(--bg-base)]',
    'bg-card': 'bg-[var(--bg-card)]',
    'bg-subtle': 'bg-[var(--bg-subtle)]',
    'bg-soft': 'bg-[var(--bg-soft)]',
    'bg-glass': 'bg-[var(--bg-glass)]',
    'bg-bubble': 'bg-white dark:bg-[#18222d] shadow-sm',
    'bg-hover': 'bg-gray-100 dark:bg-white/5',
    'bg-active': 'bg-gray-200 dark:bg-white/10',

    // --- Semantic Text ---
    'text-default': 'text-[var(--text-default)]',
    'text-secondary': 'text-[var(--text-secondary)]',
    'text-muted': 'text-[var(--text-muted)]',
    'text-inverted': 'text-white dark:text-slate-900',

    // --- Paired interactive states ---
    'accent-subtle': 'bg-[var(--accent-subtle-bg)] text-[var(--accent-subtle-text)] border border-[var(--accent-subtle-border)]',
    'success-subtle': 'bg-[var(--success-subtle-bg)] text-[var(--success-subtle-text)]',
    'warning-subtle': 'bg-[var(--warning-subtle-bg)] text-[var(--warning-subtle-text)]',
    'danger-subtle': 'bg-[var(--danger-subtle-bg)] text-[var(--danger-subtle-text)]',

    // --- Semantic Borders ---
    'border-base': 'border-[var(--border-base)]',
    'border-light': 'border-[var(--border-light)]',
    'border-glass': 'border-white/20 dark:border-white/5',

    // --- Typography (consistent, clean) ---
    'label-sm': 'text-xs font-semibold text-muted',
    'label-xs': 'text-[11px] font-medium text-muted',
    'label-micro': 'text-[10px] font-medium text-muted',
    'stat-value': 'text-xl font-bold text-default',
    'title-lg': 'text-base font-bold text-default',
    'title-md': 'text-sm font-semibold text-default',

    // --- Components ---
    'common-card': 'bg-card text-default rounded-lg border border-light shadow-sm transition-colors duration-200',
    'card-header': 'px-4 py-3 border-b border-light flex items-center justify-between',
    'card-body': 'p-4',
    'btn-icon': 'bg-transparent border-none outline-none p-2 rounded-md cursor-pointer transition-colors duration-150 hover:bg-hover text-secondary hover:text-primary',
    'link-primary': 'text-primary cursor-pointer hover:underline transition-colors',
    'input-base': 'h-9 bg-card border border-light rounded-md px-3 text-sm focus:ring-2 focus:ring-primary/20 transition-colors outline-none',
    'btn-gradient-blue': 'bg-primary text-white',

    // --- Flat Components ---
    'glass-panel': 'bg-card dark:bg-[#17212b] border-r border-[var(--border-light)]',
    'glass-header': 'bg-card dark:bg-[#17212b] border-b border-[var(--border-light)] transition-colors',
    'glass-bottom-nav': 'bg-card dark:bg-[#17212b] border-t border-[var(--border-light)] transition-colors pb-safe',
    'glass-card': 'bg-card dark:bg-[#17212b] rounded-lg border border-light shadow-sm transition-colors duration-200',

    // --- Floating Dock ---
    'dock-shell': 'pointer-events-auto flex items-center gap-1 rounded-xl border border-light bg-card px-1.5 py-1.5 shadow-[0_8px_24px_rgba(15,23,42,0.14)] dark:bg-[#151d27] dark:shadow-[0_10px_28px_rgba(0,0,0,0.34)] md:px-2 md:py-2',
    'dock-divider': 'h-7 w-px shrink-0 bg-slate-900/10 dark:bg-white/10',
    'dock-item-base': 'relative grid shrink-0 cursor-pointer place-items-center outline-none transition-colors duration-150 active:opacity-80 focus-visible:ring-2 focus-visible:ring-primary/35',
    'dock-item-idle': 'text-muted hover:bg-soft hover:text-default dark:hover:bg-white/8',
    'dock-item-active': 'bg-slate-100 text-slate-900 shadow-sm ring-1 ring-slate-900/10 dark:bg-white/12 dark:text-white dark:ring-white/12',
    'dock-tooltip': 'pointer-events-none absolute bottom-[calc(100%+10px)] left-1/2 z-50 hidden -translate-x-1/2 whitespace-nowrap rounded-lg bg-gray-900 px-2.5 py-1.5 text-xs text-white shadow-lg group-hover:block',

    // --- Utilities ---
    'clickable': 'cursor-pointer select-none transition-colors active:opacity-80',
    'scroll-y': 'overflow-y-auto custom-scrollbar',
  },

  rules: [],

  theme: {
    breakpoints: {
      sm: '640px',
      md: '769px',
      lg: '1024px',
      xl: '1280px',
      '2xl': '1536px',
    },
    colors: {
      primary: 'rgb(var(--brand-primary-rgb) / %alpha)',
      secondary: 'var(--text-secondary)',
      muted: 'var(--text-muted)',
    },
    preflight: {
      darkMode: 'class'
    }
  }
})
