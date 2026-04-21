<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  visible: boolean
  type: 'word' | 'sentence'
  selectedText: string
  contextText?: string
  loading: boolean
  deepLoading?: boolean
  saving?: boolean
  error?: string
  deepError?: string
  wordPos?: string
  meaning?: string
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
const canSave = computed(() => hasResult.value && !props.loading && !props.saving)
</script>

<template>
  <teleport to="body">
    <transition name="qlp-fade">
      <aside v-if="visible" class="quick-lookup-panel">
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
            <div v-if="type === 'word' && contextText" class="qlp-context">
              <span class="qlp-context-label">语境</span>
              <span>{{ contextText }}</span>
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
  right: 92px;
  bottom: 28px;
  width: min(380px, calc(100vw - 32px));
  max-height: min(72vh, 680px);
  display: flex;
  flex-direction: column;
  background: rgba(255, 255, 255, 0.96);
  backdrop-filter: blur(18px) saturate(150%);
  -webkit-backdrop-filter: blur(18px) saturate(150%);
  border: 1px solid rgba(226, 232, 240, 0.9);
  border-radius: 18px;
  box-shadow: 0 18px 42px rgba(15, 23, 42, 0.14), 0 2px 10px rgba(15, 23, 42, 0.08);
  z-index: 1400;
  overflow: hidden;
}

.qlp-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 18px 18px 14px;
  border-bottom: 1px solid rgba(226, 232, 240, 0.8);
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
  background: rgba(148, 163, 184, 0.12);
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
  background: rgba(248, 250, 252, 0.94);
  border: 1px solid rgba(226, 232, 240, 0.85);
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
  border-top: 1px dashed rgba(203, 213, 225, 0.9);
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
  border-top: 1px dashed rgba(203, 213, 225, 0.9);
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
  border-top: 1px solid rgba(226, 232, 240, 0.8);
  background: rgba(248, 250, 252, 0.8);
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
  background: rgba(255, 255, 255, 0.92);
  border-color: rgba(203, 213, 225, 0.9);
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
    right: 16px;
    bottom: 16px;
    width: min(420px, calc(100vw - 32px));
  }
}
</style>

<style>
@import '../styles/parsed-sentence.css';
</style>
