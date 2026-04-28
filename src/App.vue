<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, watch } from 'vue'
import Sidebar from './components/Sidebar.vue'
import GlobalToast from './components/GlobalToast.vue'
import VersionAnnouncement from './components/VersionAnnouncement.vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useRoute } from 'vue-router'
import { useSettingsStore } from './stores/settingsStore'

const appWindow = getCurrentWindow()
function minimize() { appWindow.minimize() }
function toggleMaximize() { appWindow.toggleMaximize() }
function close() { appWindow.close() }

const route = useRoute()
const settingsStore = useSettingsStore()
const keepAliveRoutes = new Set(['home', 'vocabulary', 'sentences', 'review', 'translate', 'settings', 'guide', 'data'])
const currentTheme = computed(() => (settingsStore.theme === 'dark' ? 'dark' : 'light'))
const themeToggleTitle = computed(() => currentTheme.value === 'dark' ? '切换到浅色模式' : '切换到深色模式')
const shouldKeepAlive = computed(() => keepAliveRoutes.has(String(route.name || '')))

void settingsStore.loadSettings()

function preventContextMenu(event: Event) {
  event.preventDefault()
}

function applyTheme(theme: string) {
  const isDark = theme === 'dark'
  document.documentElement.classList.toggle('dark', isDark)
  document.documentElement.dataset.theme = isDark ? 'dark' : 'light'
  document.documentElement.style.colorScheme = isDark ? 'dark' : 'light'
}

async function toggleTheme() {
  const previous = currentTheme.value
  const next = previous === 'dark' ? 'light' : 'dark'

  applyTheme(next)
  try {
    await settingsStore.setSetting('theme', next)
  } catch (error) {
    console.error('Failed to toggle theme:', error)
    applyTheme(previous)
  }
}

watch(currentTheme, (theme) => {
  applyTheme(theme)
}, { immediate: true })

onMounted(() => {
  document.addEventListener('contextmenu', preventContextMenu)
})

onBeforeUnmount(() => {
  document.removeEventListener('contextmenu', preventContextMenu)
})
</script>

<template>
  <div class="app-shell">
    <Sidebar />
    <div class="app-right">
      <div class="custom-titlebar" data-tauri-drag-region>
        <div class="titlebar-spacer" data-tauri-drag-region></div>
        <div class="titlebar-controls">
          <button class="tb-btn tb-btn--theme" @click="toggleTheme" :title="themeToggleTitle">
            <svg v-if="currentTheme === 'dark'" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="4.5" />
              <path d="M12 2.5v2.5" />
              <path d="M12 19v2.5" />
              <path d="M4.93 4.93l1.77 1.77" />
              <path d="M17.3 17.3l1.77 1.77" />
              <path d="M2.5 12H5" />
              <path d="M19 12h2.5" />
              <path d="M4.93 19.07l1.77-1.77" />
              <path d="M17.3 6.7l1.77-1.77" />
            </svg>
            <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 12.8A8.5 8.5 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8Z" />
            </svg>
          </button>
          <button class="tb-btn" @click="minimize" title="最小化">
            <svg width="10" height="1" viewBox="0 0 10 1"><rect width="10" height="1" fill="currentColor"/></svg>
          </button>
          <button class="tb-btn" @click="toggleMaximize" title="最大化">
            <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1"><rect x="0.5" y="0.5" width="9" height="9"/></svg>
          </button>
          <button class="tb-btn tb-btn--close" @click="close" title="关闭">
            <svg width="10" height="10" viewBox="0 0 10 10" stroke="currentColor" stroke-width="1.2"><line x1="1" y1="1" x2="9" y2="9"/><line x1="9" y1="1" x2="1" y2="9"/></svg>
          </button>
        </div>
      </div>
      <main class="app-main">
        <RouterView v-slot="{ Component, route }">
          <KeepAlive v-if="shouldKeepAlive">
            <component :is="Component" :key="route.path" />
          </KeepAlive>
          <component v-else :is="Component" :key="route.path" />
        </RouterView>
      </main>
    </div>
    <GlobalToast />
    <VersionAnnouncement />
  </div>
</template>

<style>
:root {
  --c-text: #20201d;
  --c-text-lighter: #6f6f67;
  --c-bg: #f7f7f4;
  --c-bg-light: #ffffff;
  --c-bg-lighter: #f1f1ed;
  --c-border: #e2e2dc;
  --c-border-light: #edede7;
  --c-border-strong: #cfcfc7;
  --c-primary: #2f66d0;
  --c-primary-dark: #2455b5;
  --c-primary-light: rgba(47, 102, 208, 0.08);
  --c-accent: #2f66d0;
  --c-danger: #c73535;
  --c-sidebar-bg: #f2f2ee;
  --c-sidebar-border: #dfdfd8;
  --c-surface-1: #ffffff;
  --c-surface-2: #f7f7f4;
  --c-selected-bg: rgba(47, 102, 208, 0.08);
  --c-hover-bg: rgba(32, 32, 29, 0.05);
  --c-bg-glow: transparent;
  --c-accent-soft: rgba(47, 102, 208, 0.06);
  --c-accent-border: rgba(47, 102, 208, 0.18);
  --c-accent-pill: rgba(47, 102, 208, 0.09);
  --c-track-bg: rgba(112, 112, 102, 0.18);
  --sidebar-width: 212px;
  --radius: 10px;
  --fs-xs: 11px;
  --fs-sm: 12px;
  --fs-base: 13px;
  --fs-md: 15px;
  --fs-lg: 18px;
  --font-sans: 'Inter', 'PingFang SC', 'HarmonyOS Sans SC', 'Microsoft YaHei', sans-serif;
  --font-serif: 'Georgia', 'Times New Roman', serif;
  --font-mono: 'Cascadia Code', 'Fira Code', 'JetBrains Mono', 'Consolas', monospace;
  --c-scrollbar-thumb: rgba(15, 23, 42, 0.12);
  --c-scrollbar-thumb-hover: rgba(15, 23, 42, 0.22);
  --c-overlay-bg: rgba(255, 255, 255, 0.92);
  --c-overlay-bg-strong: rgba(255, 255, 255, 0.985);
  --c-overlay-border: rgba(226, 232, 240, 0.88);
  --c-glass-bg: rgba(255, 255, 255, 0.86);
  --c-glass-border: rgba(255, 255, 255, 0.72);
  --c-shadow-lg: 0 12px 28px rgba(32, 32, 29, 0.08);
  --c-shadow-md: 0 6px 16px rgba(32, 32, 29, 0.05);
}

:root.dark {
  --c-text: #edf2ff;
  --c-text-lighter: #96a4bf;
  --c-bg: #0b1220;
  --c-bg-light: #111a2e;
  --c-bg-lighter: #162238;
  --c-border: #25324a;
  --c-border-light: #1c273a;
  --c-border-strong: #355072;
  --c-primary: #7cb4ff;
  --c-primary-dark: #5b9cff;
  --c-primary-light: rgba(124, 180, 255, 0.18);
  --c-accent: #7cb4ff;
  --c-danger: #ff7b7b;
  --c-sidebar-bg: #0c1628;
  --c-sidebar-border: #1a2841;
  --c-surface-1: #111b2f;
  --c-surface-2: #162236;
  --c-selected-bg: rgba(124, 180, 255, 0.16);
  --c-hover-bg: rgba(124, 180, 255, 0.08);
  --c-bg-glow: rgba(91, 156, 255, 0.16);
  --c-accent-soft: linear-gradient(180deg, rgba(124, 180, 255, 0.14), rgba(124, 180, 255, 0.05));
  --c-accent-border: rgba(124, 180, 255, 0.28);
  --c-accent-pill: rgba(124, 180, 255, 0.16);
  --c-track-bg: rgba(148, 163, 184, 0.18);
  --c-scrollbar-thumb: rgba(148, 163, 184, 0.26);
  --c-scrollbar-thumb-hover: rgba(148, 163, 184, 0.42);
  --c-overlay-bg: rgba(17, 26, 46, 0.94);
  --c-overlay-bg-strong: rgba(17, 26, 46, 0.985);
  --c-overlay-border: rgba(71, 85, 105, 0.62);
  --c-glass-bg: rgba(17, 26, 46, 0.82);
  --c-glass-border: rgba(71, 85, 105, 0.34);
  --c-shadow-lg: 0 14px 32px rgba(0, 0, 0, 0.3);
  --c-shadow-md: 0 8px 18px rgba(0, 0, 0, 0.2);
}

* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
  user-select: none;
  -webkit-user-select: none;
}

input, textarea, [contenteditable],
.reader-typography, .reader-typography *,
.reader-body, .reader-body *,
.reader-view, .reader-view *,
.article-body, .article-body *,
.sentence-original,
.result-content, .result-content *,
.rendered-preview, .rendered-preview *,
.markdown-preview, .markdown-preview *,
.text-display,
pre, code {
  user-select: text;
  -webkit-user-select: text;
}

html, body, #app {
  margin: 0;
  height: 100vh;
  overflow: hidden;
}

body {
  font-family: var(--font-sans);
  font-size: var(--fs-base);
  font-weight: 400;
  color: var(--c-text);
  background: var(--c-bg);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  transition: background-color 0.22s ease, color 0.22s ease;
}

a { color: inherit; text-decoration: none; }

.app-shell {
  display: flex;
  height: 100vh;
  overflow: hidden;
  background: transparent;
}

:root.book-reader-active .sidebar {
  width: 0;
  min-width: 0;
  padding-left: 0;
  padding-right: 0;
  border-right: 0;
  overflow: hidden;
  transform: translateX(-100%);
}

.app-right {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: transparent;
}

.custom-titlebar {
  height: 38px;
  display: flex;
  align-items: center;
  background: var(--c-surface-1);
  border-bottom: 1px solid var(--c-border-light);
  -webkit-app-region: drag;
  flex-shrink: 0;
}

.titlebar-spacer { flex: 1; }

.titlebar-controls {
  display: flex;
  align-items: center;
  gap: 0;
  -webkit-app-region: no-drag;
}

.tb-btn {
  width: 46px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--c-text-lighter);
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}

.tb-btn:hover {
  background: var(--c-hover-bg);
  color: var(--c-text);
}

.tb-btn--close:hover {
  background: #ef4444;
  color: #fff;
}

.tb-btn--theme {
  margin-right: 4px;
}

/* ── 主内容区 ── */
.app-main {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  background: transparent;
}
.app-main::-webkit-scrollbar { width: 5px; }
.app-main::-webkit-scrollbar-track { background: transparent; }
.app-main::-webkit-scrollbar-thumb {
  background: var(--c-scrollbar-thumb);
  border-radius: 3px;
}
.app-main::-webkit-scrollbar-thumb:hover {
  background: var(--c-scrollbar-thumb-hover);
}

.page-container {
  width: 100%;
  max-width: 980px;
  margin: 0 auto;
  padding: 28px 32px;
}
.page-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: var(--fs-lg);
  font-weight: 700;
  color: var(--c-text);
  margin-bottom: 24px;
  letter-spacing: -0.01em;
}
.title-icon,
.page-title > svg {
  width: 28px;
  height: 28px;
  color: var(--c-primary);
  flex-shrink: 0;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.card {
  background: var(--c-bg-light);
  border: 1px solid var(--c-border);
  border-radius: var(--radius);
  padding: 20px 22px;
  margin-bottom: 14px;
  transition: border-color 0.18s ease;
}
.card:hover { border-color: var(--c-border-strong); }

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  height: 34px;
  padding: 0 14px;
  border-radius: 8px;
  font-size: var(--fs-base);
  font-weight: 600;
  border: 1px solid transparent;
  cursor: pointer;
  transition: background 0.16s ease, border-color 0.16s ease, color 0.16s ease, transform 0.16s ease;
  white-space: nowrap;
  letter-spacing: -0.01em;
}
.btn-primary {
  color: #ffffff;
  background: var(--c-primary);
}
.btn-primary:hover {
  background: var(--c-primary-dark);
}
.btn-primary:active { transform: scale(0.98); }
.btn-outline {
  color: var(--c-text);
  background: var(--c-bg-light);
  border-color: var(--c-border);
}
.btn-outline:hover {
  border-color: var(--c-border-strong);
  background: var(--c-hover-bg);
}
.btn-outline:active { transform: scale(0.98); }
.btn-secondary {
  color: #5856D6;
  background: rgba(88, 86, 214, 0.1);
}
.btn-secondary:hover { background: rgba(88, 86, 214, 0.16); }
.btn-secondary:active { transform: scale(0.97); }
.btn-danger {
  color: var(--c-danger);
  background: rgba(255, 59, 48, 0.1);
}
.btn-danger:hover {
  color: #fff;
  background: var(--c-danger);
  box-shadow: 0 2px 8px rgba(255, 59, 48, 0.25);
}
.btn-danger:active { transform: scale(0.97); }
.btn-danger-outline {
  color: var(--c-danger);
  background: var(--c-bg-light);
  border: 1px solid rgba(255, 59, 48, 0.25);
  border-radius: 8px;
  font-size: var(--fs-base);
  font-weight: 500;
  height: 36px;
  padding: 0 16px;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94);
  letter-spacing: -0.01em;
}
.btn-danger-outline:hover {
  background: rgba(255, 59, 48, 0.06);
  border-color: var(--c-danger);
}
.btn-danger-outline:active { transform: scale(0.97); }
.btn:disabled, .btn-danger-outline:disabled {
  opacity: 0.4;
  cursor: not-allowed;
  transform: none !important;
  box-shadow: none !important;
}

.input {
  width: 100%;
  height: 36px;
  padding: 0 12px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  font-size: var(--fs-base);
  font-family: inherit;
  color: var(--c-text);
  background: var(--c-bg-light);
  transition: border-color 0.16s, box-shadow 0.16s, background 0.16s;
  outline: none;
}
.input:focus {
  background: var(--c-bg-light);
  border-color: var(--c-primary);
  box-shadow: 0 0 0 3px var(--c-primary-light);
}
.textarea {
  width: 100%;
  min-height: 120px;
  padding: 12px 14px;
  border: 1px solid var(--c-border);
  border-radius: 10px;
  font-size: var(--fs-base);
  font-family: inherit;
  color: var(--c-text);
  background: var(--c-bg-light);
  resize: vertical;
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s, background 0.2s;
}
.textarea:focus {
  background: var(--c-bg-light);
  border-color: #bbb;
  box-shadow: 0 0 0 2px rgba(0, 0, 0, 0.04);
}

.empty-state {
  text-align: center;
  padding: 60px 20px;
  color: var(--c-text-lighter);
}
.empty-state svg {
  width: 56px; height: 56px;
  stroke-width: 1;
  margin-bottom: 14px;
  opacity: 0.35;
}
.empty-state p { font-size: var(--fs-base); margin-top: 8px; }

@keyframes fade-in {
  from { opacity: 0; transform: translateY(6px); }
  to { opacity: 1; transform: translateY(0); }
}
.fade-in { animation: fade-in 0.35s ease; }
</style>
