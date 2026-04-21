<script setup lang="ts">
import { computed, type CSSProperties } from 'vue'

interface QuickLookupPosition {
  top: number
  left: number
  sourceTop?: number
  sourceBottom?: number
}

interface QuickLookupContentElement extends HTMLElement {}

function resolvePanelContainerRect(element: QuickLookupContentElement | null | undefined): DOMRect | null {
  if (!element) return null
  const container = element.closest('.page-container, .app-main') as HTMLElement | null
  return container?.getBoundingClientRect() || null
}

const props = defineProps<{
  visible: boolean
  type: 'word' | 'sentence'
  position?: QuickLookupPosition | null
  contentElement?: QuickLookupContentElement | null
  selectedText: string
  contextText?: string
  loading: boolean
  deepLoading?: boolean
  saving?: boolean
  error?: string
  deepError?: string
  wordPos?: string
  meaning?: string
  baseMeaning?: string
  otherMeanings?: string[]
  translation?: string
  parsedHtml?: string
  structureNote?: string
}>()

const emit = defineEmits<{
  close: []
  save: []
  edit: []
  retry: []
  deepen: []
  inline: []
}>()

const title = computed(() => (props.type === 'word' ? '快速查词' : '快速查句'))
const subtitle = computed(() => {
  if (props.loading) return '后台生成中，可继续阅读'
  if (props.error) return '查询失败，可重试或改为手动编辑'
  return '结果已就绪，不抢阅读焦点'
})
const saveLabel = computed(() => (props.type === 'word' ? '保存到生词本' : '保存到句库'))
const hasResult = computed(() => {
  if (props.type === 'word') {
    return Boolean(props.meaning?.trim())
  }
  return Boolean(props.translation?.trim() || props.parsedHtml?.trim())
})
const normalizedContext = computed(() => (props.contextText || '').trim())
const highlightedContext = computed(() => {
  const context = normalizedContext.value
  const selected = props.selectedText.trim()
  if (!context || !selected) {
    return { before: context, match: '', after: '' }
  }

  const lowerContext = context.toLocaleLowerCase()
  const lowerSelected = selected.toLocaleLowerCase()
  const index = lowerContext.indexOf(lowerSelected)
  if (index < 0) {
    return { before: context, match: '', after: '' }
  }

  return {
    before: context.slice(0, index),
    match: context.slice(index, index + selected.length),
    after: context.slice(index + selected.length),
  }
})
const hasExtendedWordMeanings = computed(() =>
  Boolean(props.baseMeaning?.trim()) || Boolean(props.otherMeanings?.length),
)
const canSave = computed(() => hasResult.value && !props.loading && !props.saving)
const panelWidth = computed(() => {
  if (typeof window === 'undefined') return 380
  return Math.min(380, Math.max(280, window.innerWidth - 32))
})
const estimatedPanelHeight = computed(() => {
  if (typeof window === 'undefined') return 480
  return Math.min(680, Math.round(window.innerHeight * 0.72))
})

const panelMode = computed<'floating' | 'aside-left' | 'aside-right'>(() => {
  if (!props.position || typeof window === 'undefined' || !props.contentElement) {
    return 'floating'
  }

  const rect = props.contentElement.getBoundingClientRect()
  const containerRect = resolvePanelContainerRect(props.contentElement)
  if (rect.width <= 0 || rect.height <= 0) {
    return 'floating'
  }

  const gap = 18
  const requiredWidth = panelWidth.value + gap + 16
  const leftBoundary = containerRect?.left ?? 0
  const rightBoundary = containerRect?.right ?? window.innerWidth
  const leftSpace = rect.left - leftBoundary
  const rightSpace = rightBoundary - rect.right

  if (rightSpace >= requiredWidth) {
    return 'aside-right'
  }

  if (leftSpace >= requiredWidth) {
    return 'aside-left'
  }

  return 'floating'
})

const panelPlacement = computed<'top' | 'bottom'>(() => {
  if (panelMode.value !== 'floating' || !props.position || typeof window === 'undefined') return 'bottom'
  const sourceTop = props.position.sourceTop ?? props.position.top
  const sourceBottom = props.position.sourceBottom ?? props.position.top
  const midpoint = sourceTop + (sourceBottom - sourceTop) / 2
  return midpoint > window.innerHeight * 0.5 ? 'top' : 'bottom'
})

const panelStyle = computed<CSSProperties>(() => {
  if (!props.position || typeof window === 'undefined') {
    return {
      right: '92px',
      bottom: '28px',
    }
  }

  const sideMargin = 16
  const viewportWidth = window.innerWidth
  const panelWidthValue = panelWidth.value
  const estimatedHeight = estimatedPanelHeight.value
  const contentRect = props.contentElement?.getBoundingClientRect()
  const containerRect = resolvePanelContainerRect(props.contentElement)

  if (panelMode.value !== 'floating' && contentRect) {
    const sourceTop = props.position.sourceTop ?? props.position.top
    const top = Math.max(
      sideMargin,
      Math.min(
        sourceTop,
        window.innerHeight - estimatedHeight - sideMargin,
      ),
    )
    const leftBoundary = containerRect?.left ?? 0
    const rightBoundary = containerRect?.right ?? viewportWidth
    const whitespaceGap = 18

    const left = panelMode.value === 'aside-right'
      ? Math.min(
        Math.max(contentRect.right + whitespaceGap, leftBoundary + sideMargin),
        rightBoundary - panelWidthValue - sideMargin,
      )
      : Math.max(
        leftBoundary + sideMargin,
        Math.min(contentRect.left - panelWidthValue - whitespaceGap, rightBoundary - panelWidthValue - sideMargin),
      )

    return {
      top: `${top}px`,
      left: `${left}px`,
      width: `${panelWidthValue}px`,
    }
  }

  const left = Math.min(
    Math.max(props.position.left - panelWidthValue / 2, sideMargin),
    viewportWidth - panelWidthValue - sideMargin,
  )
  const style: CSSProperties = {
    left: `${left}px`,
    width: `${panelWidthValue}px`,
  }

  if (panelPlacement.value === 'top') {
    const sourceTop = props.position.sourceTop ?? props.position.top
    style.bottom = `${Math.max(window.innerHeight - sourceTop + 14, sideMargin)}px`
  } else {
    const sourceBottom = props.position.sourceBottom ?? props.position.top
    style.top = `${Math.max(sourceBottom + 14, sideMargin)}px`
  }

  return style
})
</script>

<template>
  <teleport to="body">
    <transition name="qlp-fade">
      <aside
        v-if="visible"
        class="quick-lookup-panel"
        :class="[
          `quick-lookup-panel--${panelPlacement}`,
          panelMode !== 'floating' ? `quick-lookup-panel--${panelMode}` : '',
        ]"
        :style="panelStyle"
      >
        <div class="qlp-header">
          <div class="qlp-header-main">
            <div class="qlp-title">{{ title }}</div>
            <div class="qlp-subtitle">
              {{ subtitle }}
            </div>
          </div>
          <button class="qlp-close" @click="emit('close')" aria-label="关闭快速查询面板">✕</button>
        </div>

        <div class="qlp-body">
          <section class="qlp-card">
            <div class="qlp-label">{{ type === 'word' ? '当前词语' : '当前句子' }}</div>
            <div class="qlp-selected">{{ selectedText }}</div>
            <div v-if="type === 'word' && normalizedContext" class="qlp-context">
              <span class="qlp-context-label">语境</span>
              <span class="qlp-context-text">
                <template v-if="highlightedContext.match">
                  {{ highlightedContext.before }}<span class="qlp-context-match">{{ highlightedContext.match }}</span>{{ highlightedContext.after }}
                </template>
                <template v-else>
                  {{ normalizedContext }}
                </template>
              </span>
            </div>
          </section>

          <section v-if="loading" class="qlp-card qlp-loading">
            <div class="qlp-spinner"></div>
            <div class="qlp-loading-text">
              <div>{{ type === 'word' ? '正在提取当前语境义项...' : '正在生成快速中文释义...' }}</div>
              <div class="qlp-loading-hint">你可以继续滚动阅读，结果回来后会停在这里。</div>
            </div>
          </section>

          <section v-else-if="error" class="qlp-card qlp-error">
            <div class="qlp-error-title">查询失败</div>
            <div class="qlp-error-body">{{ error }}</div>
          </section>

          <template v-else>
            <section v-if="type === 'word' && meaning" class="qlp-card">
              <div class="qlp-label">当前语境义</div>
              <div class="qlp-meaning-row">
                <span v-if="wordPos" class="qlp-chip">{{ wordPos }}</span>
                <span class="qlp-meaning">{{ meaning }}</span>
              </div>
              <div v-if="hasExtendedWordMeanings" class="qlp-word-extensions">
                <div v-if="baseMeaning" class="qlp-word-extension">
                  <span class="qlp-word-extension__label">相关本意</span>
                  <span class="qlp-word-extension__value">{{ baseMeaning }}</span>
                </div>
                <div v-if="otherMeanings?.length" class="qlp-word-extension">
                  <span class="qlp-word-extension__label">其他常见义</span>
                  <div class="qlp-meaning-list">
                    <span
                      v-for="item in otherMeanings"
                      :key="item"
                      class="qlp-meaning-pill"
                    >
                      {{ item }}
                    </span>
                  </div>
                </div>
              </div>
            </section>

            <template v-if="type === 'sentence'">
              <section v-if="translation" class="qlp-card">
                <div class="qlp-label">快速释义</div>
                <div class="qlp-translation">{{ translation }}</div>
              </section>

              <section v-if="parsedHtml" class="qlp-card">
                <div class="qlp-label">深度解析</div>
                <div class="qlp-parsed" v-html="parsedHtml"></div>
                <div v-if="structureNote" class="qlp-note">{{ structureNote }}</div>
              </section>

              <section v-if="deepError" class="qlp-card qlp-error qlp-error--inline">
                <div class="qlp-error-title">深度解析失败</div>
                <div class="qlp-error-body">{{ deepError }}</div>
              </section>
            </template>
          </template>
        </div>

        <div class="qlp-actions">
          <template v-if="error">
            <button class="qlp-btn qlp-btn-secondary" @click="emit('retry')">重试</button>
            <button class="qlp-btn qlp-btn-secondary" @click="emit('edit')">手动编辑</button>
          </template>
          <template v-else>
            <button
              v-if="type === 'sentence' && !parsedHtml"
              class="qlp-btn qlp-btn-secondary"
              :disabled="loading || deepLoading || saving"
              @click="emit('deepen')"
            >
              {{ deepLoading ? '解析中...' : '深度解析' }}
            </button>
            <button
              v-if="type === 'sentence' && translation"
              class="qlp-btn qlp-btn-secondary"
              :disabled="loading || saving"
              @click="emit('inline')"
            >
              句下展开
            </button>
            <button
              class="qlp-btn qlp-btn-secondary"
              :disabled="loading || saving"
              @click="emit('edit')"
            >
              编辑
            </button>
            <button
              class="qlp-btn qlp-btn-primary"
              :disabled="!canSave"
              @click="emit('save')"
            >
              {{ saving ? '保存中...' : saveLabel }}
            </button>
          </template>
        </div>
      </aside>
    </transition>
  </teleport>
</template>

<style scoped>
.quick-lookup-panel {
  position: fixed;
  width: min(380px, calc(100vw - 32px));
  max-height: min(72vh, 680px);
  display: flex;
  flex-direction: column;
  background: var(--c-overlay-bg-strong);
  backdrop-filter: blur(22px) saturate(165%);
  -webkit-backdrop-filter: blur(22px) saturate(165%);
  border: 1px solid var(--c-overlay-border);
  border-radius: 18px;
  box-shadow: 0 20px 48px rgba(15, 23, 42, 0.24), 0 4px 14px rgba(15, 23, 42, 0.14);
  z-index: 1400;
  overflow: hidden;
}

.quick-lookup-panel--top::after,
.quick-lookup-panel--bottom::after {
  content: '';
  position: absolute;
  left: 50%;
  width: 14px;
  height: 14px;
  background: var(--c-overlay-bg-strong);
  border-right: 1px solid var(--c-overlay-border);
  border-bottom: 1px solid var(--c-overlay-border);
  transform: translateX(-50%) rotate(45deg);
}

.quick-lookup-panel--top::after {
  bottom: -8px;
}

.quick-lookup-panel--bottom::after {
  top: -8px;
  transform: translateX(-50%) rotate(225deg);
}

.quick-lookup-panel--aside-left::after,
.quick-lookup-panel--aside-right::after {
  display: none;
}

.qlp-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 18px 18px 14px;
  border-bottom: 1px solid var(--c-overlay-border);
}

.qlp-header-main {
  min-width: 0;
}

.qlp-title {
  font-size: 15px;
  font-weight: 700;
  color: var(--c-text);
}

.qlp-subtitle {
  margin-top: 4px;
  font-size: 12px;
  color: var(--c-text-lighter);
}

.qlp-close {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--c-text-lighter);
  cursor: pointer;
  transition: background 0.18s ease, color 0.18s ease;
}

.qlp-close:hover {
  background: var(--c-primary-light);
  color: var(--c-text);
}

.qlp-body {
  flex: 1;
  overflow-y: auto;
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.qlp-card {
  padding: 14px 14px 12px;
  border-radius: 14px;
  background: var(--c-bg-lighter);
  border: 1px solid var(--c-border);
}

.qlp-label {
  margin-bottom: 8px;
  font-size: 12px;
  font-weight: 700;
  color: var(--c-text-lighter);
  letter-spacing: 0.03em;
}

.qlp-selected {
  font-family: var(--font-serif);
  font-size: 15px;
  line-height: 1.8;
  color: var(--c-text);
}

.qlp-context {
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px dashed var(--c-border);
  font-size: 13px;
  line-height: 1.7;
  color: var(--c-text-lighter);
}

.qlp-context-label {
  display: inline-block;
  margin-right: 8px;
  color: #6366f1;
  font-weight: 600;
}

.qlp-context-text {
  word-break: break-word;
}

.qlp-context-match {
  text-decoration: underline;
  text-decoration-thickness: 1.5px;
  text-underline-offset: 2px;
  color: var(--c-primary-dark);
  font-weight: 600;
}

.qlp-loading {
  display: flex;
  gap: 12px;
  align-items: center;
}

.qlp-spinner {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  border: 3px solid rgba(148, 163, 184, 0.22);
  border-top-color: #3b82f6;
  animation: qlp-spin 0.8s linear infinite;
  flex-shrink: 0;
}

.qlp-loading-text {
  min-width: 0;
  font-size: 14px;
  color: var(--c-text);
  line-height: 1.6;
}

.qlp-loading-hint {
  font-size: 12px;
  color: var(--c-text-lighter);
}

.qlp-meaning-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
}

.qlp-word-extensions {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px dashed var(--c-border);
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.qlp-word-extension {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.qlp-word-extension__label {
  font-size: 12px;
  font-weight: 700;
  color: var(--c-text-lighter);
  letter-spacing: 0.02em;
}

.qlp-word-extension__value {
  font-size: 14px;
  line-height: 1.65;
  color: var(--c-text);
}

.qlp-meaning-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.qlp-meaning-pill {
  display: inline-flex;
  align-items: center;
  min-height: 28px;
  padding: 0 10px;
  border-radius: 999px;
  background: rgba(99, 102, 241, 0.1);
  color: #4338ca;
  font-size: 12px;
  font-weight: 600;
  line-height: 1.4;
}

.qlp-chip {
  display: inline-flex;
  align-items: center;
  height: 24px;
  padding: 0 10px;
  border-radius: 999px;
  background: rgba(59, 130, 246, 0.12);
  color: #2563eb;
  font-size: 12px;
  font-weight: 700;
}

.qlp-meaning,
.qlp-translation {
  font-size: 15px;
  line-height: 1.75;
  color: var(--c-text);
}

.qlp-parsed {
  font-family: var(--font-serif);
  font-size: 14px;
  line-height: 2;
  color: var(--c-text);
  word-break: break-word;
}

.qlp-note {
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px dashed var(--c-border);
  font-size: 13px;
  line-height: 1.7;
  color: var(--c-text-lighter);
}

.qlp-error {
  border-color: rgba(248, 113, 113, 0.28);
  background: rgba(254, 242, 242, 0.95);
}

.qlp-error-title {
  font-size: 13px;
  font-weight: 700;
  color: #dc2626;
}

.qlp-error-body {
  margin-top: 6px;
  font-size: 13px;
  line-height: 1.65;
  color: #7f1d1d;
  white-space: pre-wrap;
  word-break: break-word;
}

.qlp-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 10px;
  padding: 14px 18px 18px;
  border-top: 1px solid var(--c-overlay-border);
  background: var(--c-bg-lighter);
}

.qlp-btn {
  min-width: 96px;
  height: 36px;
  padding: 0 14px;
  border-radius: 10px;
  border: 1px solid transparent;
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
  transition: all 0.18s ease;
}

.qlp-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.qlp-btn-secondary {
  background: var(--c-bg-light);
  border-color: var(--c-border);
  color: var(--c-text);
}

.qlp-btn-secondary:hover:not(:disabled) {
  border-color: rgba(59, 130, 246, 0.4);
  color: #2563eb;
}

.qlp-btn-primary {
  background: linear-gradient(135deg, #007aff, #409cff);
  color: #fff;
  box-shadow: 0 10px 22px rgba(64, 156, 255, 0.24);
}

.qlp-btn-primary:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 12px 24px rgba(64, 156, 255, 0.3);
}

.qlp-fade-enter-active,
.qlp-fade-leave-active {
  transition: opacity 0.22s ease, transform 0.22s ease;
}

.qlp-fade-enter-from,
.qlp-fade-leave-to {
  opacity: 0;
  transform: translateY(8px);
}

@keyframes qlp-spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@media (max-width: 960px) {
  .quick-lookup-panel {
    width: min(420px, calc(100vw - 32px));
  }
}
</style>

<style>
.ps-predicate { color: #2563eb; font-weight: 700; }
.ps-nonfinite {
  color: #7c3aed;
  text-decoration: underline;
  text-decoration-style: wavy;
  text-underline-offset: 3px;
}
.ps-connector { color: #dc2626; font-weight: 600; }
.ps-italic { font-style: italic; }
.ps-main { font-weight: 700; }
.ps-structure { color: #059669; font-weight: 600; }
.ps-symbol { color: #e07b39; font-weight: 800; font-family: var(--font-mono, monospace); }
.parsed-html-content {
  line-height: 2.1;
  font-family: var(--font-serif, Georgia, 'Times New Roman', serif);
}

.inline-sentence-translation {
  margin: 0.85rem 0 1.15rem;
  padding: 12px 14px 14px;
  border: 1px solid rgba(96, 165, 250, 0.4);
  border-radius: 14px;
  background: linear-gradient(180deg, rgba(37, 99, 235, 0.08), rgba(59, 130, 246, 0.03));
  box-shadow: 0 8px 24px rgba(37, 99, 235, 0.12);
}

.inline-sentence-translation__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 8px;
}

.inline-sentence-translation__badge {
  display: inline-flex;
  align-items: center;
  padding: 3px 10px;
  border-radius: 999px;
  background: rgba(37, 99, 235, 0.14);
  color: var(--c-primary-dark);
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.01em;
}

.inline-sentence-translation__close {
  border: none;
  background: transparent;
  color: var(--c-text-lighter);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}

.inline-sentence-translation__close:hover {
  color: var(--c-primary-dark);
}

.inline-sentence-translation__translation {
  font-size: 0.98rem;
  line-height: 1.9;
  color: var(--c-text);
}

.inline-sentence-translation__parsed {
  margin-top: 10px;
  padding: 10px 12px;
  border-radius: 12px;
  background: var(--c-overlay-bg);
  border: 1px solid var(--c-overlay-border);
  font-size: 0.95rem;
  color: var(--c-text);
}

.inline-sentence-translation__note {
  margin-top: 8px;
  font-size: 0.84rem;
  line-height: 1.65;
  color: #7c3aed;
  font-style: italic;
}
</style>

<style>
@import '../styles/parsed-sentence.css';
</style>
