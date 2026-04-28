<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { RouterLink, useRoute, useRouter } from 'vue-router'
import { APP_VERSION } from '../constants/app'
import { useAppStore } from '../stores/appStore'

interface NavItem {
  label: string
  to: string
  icon: string
  group: 'primary' | 'learning' | 'more'
}

const route = useRoute()
const router = useRouter()
const appStore = useAppStore()

const navItems: NavItem[] = [
  { label: '书架', to: '/books', icon: 'library', group: 'primary' },
  { label: '复习', to: '/review', icon: 'review', group: 'primary' },
  { label: '生词', to: '/vocabulary', icon: 'book', group: 'primary' },
  { label: '句库', to: '/sentences', icon: 'text', group: 'learning' },
  { label: '精读材料', to: '/articles', icon: 'article', group: 'learning' },
  { label: '导入 EPUB', to: '/epub-import', icon: 'epub', group: 'more' },
  { label: 'OCR 导入', to: '/ocr-import', icon: 'ocr', group: 'more' },
  { label: '翻译', to: '/translate', icon: 'ai', group: 'more' },
  { label: '数据管理', to: '/data', icon: 'data', group: 'more' },
  { label: '设置', to: '/settings', icon: 'settings', group: 'more' },
]

const navSections = computed(() => [
  {
    key: 'primary',
    title: '主路径',
    items: navItems.filter((item) => item.group === 'primary'),
  },
  {
    key: 'learning',
    title: '沉淀',
    items: navItems.filter((item) => item.group === 'learning'),
  },
  {
    key: 'more',
    title: '更多',
    items: navItems.filter((item) => item.group === 'more'),
  },
])

const currentBook = computed(() => appStore.recentEbooks[0] ?? null)

function buildContinueTarget() {
  if (currentBook.value) {
    const query: Record<string, string> = { bookId: currentBook.value.id }
    if (currentBook.value.cfi_position) query.cfi = currentBook.value.cfi_position
    return { path: '/books', query }
  }
  return '/epub-import'
}

function isActive(item: NavItem): boolean {
  if (item.to === '/') return route.path === '/'
  if (item.to === '/books') {
    return route.path === '/books' && typeof route.query.bookId !== 'string'
  }
  return route.path.startsWith(item.to)
}

function handleNavClick(item: NavItem) {
  void router.push(item.to)
}

async function handleResumeClick() {
  if (!appStore.ebooksLoaded) {
    await appStore.fetchEbooks()
  }
  void router.push(buildContinueTarget())
}

onMounted(() => {
  void appStore.fetchEbooks()
})
</script>

<template>
  <aside class="sidebar">
    <RouterLink class="sidebar-brand" to="/">
      <img class="sidebar-logo-img" src="/logo.png" alt="拾语" />
      <div class="sidebar-brand__copy">
        <span class="sidebar-brand__title">拾语</span>
        <small class="sidebar-brand__ver">v{{ APP_VERSION }}</small>
      </div>
    </RouterLink>

    <button class="resume-link" @click="handleResumeClick">
      <span class="resume-link__label">继续阅读</span>
      <span class="resume-link__title">{{ currentBook ? currentBook.title : '导入 EPUB' }}</span>
    </button>

    <div class="sidebar-sections">
      <section v-for="section in navSections" :key="section.key" class="sidebar-section">
        <div class="sidebar-section__title">{{ section.title }}</div>
        <nav class="sidebar-nav" :aria-label="section.title">
          <RouterLink
            v-for="item in section.items"
            :key="section.key + item.label"
            :to="item.to"
            class="sidebar-btn"
            :class="{ active: isActive(item) }"
            :title="item.label"
            @click.prevent="handleNavClick(item)"
          >
            <span class="sidebar-btn__indicator"></span>
            <svg v-if="item.icon === 'home'" viewBox="0 0 24 24" fill="none" stroke="currentColor"><path d="M3 10.5L12 3l9 7.5"/><path d="M5 9.5V21h5v-5.5a2 2 0 012-2h0a2 2 0 012 2V21h5V9.5"/></svg>
            <svg v-else-if="item.icon === 'book'" viewBox="0 0 24 24" fill="none" stroke="currentColor"><path d="M4 19.5A2.5 2.5 0 016.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 014 19.5v-15A2.5 2.5 0 016.5 2z"/><path d="M9 7h6"/><path d="M9 11h4"/></svg>
            <svg v-else-if="item.icon === 'library'" viewBox="0 0 24 24" fill="none" stroke="currentColor"><path d="M4 20V6"/><path d="M8 20V4"/><path d="M12 20V8"/><path d="M16 20V5"/><path d="M20 20V10"/><path d="M3 20h18"/></svg>
            <svg v-else-if="item.icon === 'text'" viewBox="0 0 24 24" fill="none" stroke="currentColor"><path d="M17 6H3"/><path d="M21 12H8"/><path d="M17 18H3"/><circle cx="5" cy="12" r="1" fill="currentColor" stroke="none"/></svg>
            <svg v-else-if="item.icon === 'article'" viewBox="0 0 24 24" fill="none" stroke="currentColor"><path d="M12 7c-1.5-2-4-3-7-3v13c3 0 5.5 1 7 3 1.5-2 4-3 7-3V4c-3 0-5.5 1-7 3z"/></svg>
            <svg v-else-if="item.icon === 'ai'" viewBox="0 0 24 24" fill="none" stroke="currentColor"><circle cx="12" cy="12" r="10"/><path d="M2 12h20"/><path d="M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z"/></svg>
            <svg v-else-if="item.icon === 'review'" viewBox="0 0 24 24" fill="none" stroke="currentColor"><path d="M21.5 2v6h-6"/><path d="M2.5 22v-6h6"/><path d="M2.7 8.5a10 10 0 0117.8-1l1 1"/><path d="M21.3 15.5a10 10 0 01-17.8 1l-1-1"/></svg>
            <svg v-else-if="item.icon === 'epub'" viewBox="0 0 24 24" fill="none" stroke="currentColor"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><path d="M14 2v6h6"/><path d="M10 13h4"/><path d="M10 17h4"/><path d="M10 9h1"/></svg>
            <svg v-else-if="item.icon === 'ocr'" viewBox="0 0 24 24" fill="none" stroke="currentColor"><path d="M2 7V2h5"/><path d="M22 7V2h-5"/><path d="M2 17v5h5"/><path d="M22 17v5h-5"/><circle cx="12" cy="12" r="3"/><path d="M12 5v2"/><path d="M12 17v2"/><path d="M5 12h2"/><path d="M17 12h2"/></svg>
            <svg v-else-if="item.icon === 'data'" viewBox="0 0 24 24" fill="none" stroke="currentColor"><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M21 12c0 1.66-4.03 3-9 3s-9-1.34-9-3"/><path d="M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5"/></svg>
            <svg v-else-if="item.icon === 'settings'" viewBox="0 0 24 24" fill="none" stroke="currentColor"><path d="M12.22 2h-.44a2 2 0 00-2 2v.18a2 2 0 01-1 1.73l-.43.25a2 2 0 01-2 0l-.15-.08a2 2 0 00-2.73.73l-.22.38a2 2 0 00.73 2.73l.15.1a2 2 0 011 1.72v.51a2 2 0 01-1 1.74l-.15.09a2 2 0 00-.73 2.73l.22.38a2 2 0 002.73.73l.15-.08a2 2 0 012 0l.43.25a2 2 0 011 1.73V20a2 2 0 002 2h.44a2 2 0 002-2v-.18a2 2 0 011-1.73l.43-.25a2 2 0 012 0l.15.08a2 2 0 002.73-.73l.22-.39a2 2 0 00-.73-2.73l-.15-.08a2 2 0 01-1-1.74v-.5a2 2 0 011-1.74l.15-.09a2 2 0 00.73-2.73l-.22-.38a2 2 0 00-2.73-.73l-.15.08a2 2 0 01-2 0l-.43-.25a2 2 0 01-1-1.73V4a2 2 0 00-2-2z"/><circle cx="12" cy="12" r="3"/></svg>
            <span class="sidebar-btn__label">{{ item.label }}</span>
          </RouterLink>
        </nav>
      </section>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: var(--sidebar-width);
  min-width: var(--sidebar-width);
  height: 100vh;
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 16px 10px;
  background: var(--c-sidebar-bg);
  border-right: 1px solid var(--c-sidebar-border);
  transition: width 0.2s ease, min-width 0.2s ease, padding 0.2s ease, transform 0.2s ease;
}

.sidebar-brand {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 4px 6px;
  text-decoration: none;
}

.sidebar-logo-img {
  width: 40px;
  height: 40px;
  object-fit: contain;
  flex-shrink: 0;
}

.sidebar-brand__copy {
  display: flex;
  flex-direction: column;
}

.sidebar-brand__title {
  color: var(--c-text);
  font-size: 1rem;
  font-weight: 700;
}

.sidebar-brand__ver {
  color: var(--c-text-lighter);
  font-size: 0.74rem;
}

.resume-link {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 4px;
  border: 1px solid var(--c-border);
  border-radius: 12px;
  padding: 10px 12px;
  background: var(--c-surface-1);
  text-align: left;
  cursor: pointer;
  transition: border-color 0.16s ease, background 0.16s ease;
}

.resume-link:hover {
  border-color: var(--c-border-strong);
  background: var(--c-hover-bg);
}

.resume-link__label {
  color: var(--c-primary);
  font-size: 0.74rem;
  font-weight: 700;
}

.resume-link__title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--c-text);
  font-size: 0.9rem;
  font-weight: 650;
}

.sidebar-sections {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 14px;
  overflow-y: auto;
  padding-right: 2px;
}

.sidebar-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.sidebar-section__title {
  padding: 0 10px;
  color: var(--c-text-lighter);
  font-size: 0.74rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.sidebar-nav {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.sidebar-btn {
  position: relative;
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 36px;
  padding: 0 10px;
  border-radius: 9px;
  color: var(--c-text-lighter);
  text-decoration: none;
  transition: background 0.16s ease, color 0.16s ease;
}

.sidebar-btn svg {
  width: 18px;
  height: 18px;
  stroke-width: 1.8;
  stroke-linecap: round;
  stroke-linejoin: round;
  flex-shrink: 0;
}

.sidebar-btn__label {
  font-size: 0.92rem;
  font-weight: 600;
}

.sidebar-btn__indicator {
  display: none;
}

.sidebar-btn:hover {
  background: var(--c-hover-bg);
  color: var(--c-text);
}

.sidebar-btn.active {
  background: var(--c-selected-bg);
  color: var(--c-text);
  font-weight: 700;
}

@media (max-width: 1080px) {
  .sidebar {
    width: 196px;
    min-width: 196px;
  }
}
</style>
