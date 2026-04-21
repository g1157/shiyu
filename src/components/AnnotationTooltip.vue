<script setup lang="ts">
import { computed } from 'vue'
import { getSentenceMeaning } from '../utils/sentenceExplanation'

const props = defineProps<{
  content: string
  type: 'word' | 'sentence'
  position: { top: number; left: number; anchor?: 'center' | 'start' }
}>()

const emit = defineEmits<{
  close: []
  remove: []
  hoverEnter: []
  hoverLeave: []
}>()

const displayContent = computed(() => {
  if (props.type === 'sentence') {
    return getSentenceMeaning(props.content) || props.content
  }
  return props.content
})

const style = computed(() => {
  const anchor = props.position.anchor ?? 'center'
  return {
    position: 'fixed' as const,
    top: `${props.position.top}px`,
    left: `${props.position.left}px`,
    transform: anchor === 'center' ? 'translateX(-50%)' : 'none',
    zIndex: 1500,
  }
})

</script>

<template>
  <div
    class="annotation-tooltip"
    :class="type === 'sentence' ? 'sentence-tooltip' : 'word-tooltip'"
    :style="style"
    @mouseenter="emit('hoverEnter')"
    @mouseleave="emit('hoverLeave')"
  >
    <button
      v-if="type === 'sentence'"
      class="tooltip-remove"
      type="button"
      title="移出句库"
      @click.stop="emit('remove')"
    >
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="3 6 5 6 21 6"/>
        <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
      </svg>
    </button>
    <div class="tooltip-content" v-text="displayContent" />
  </div>
</template>

<style scoped>
.annotation-tooltip {
  position: relative;
  max-width: 320px;
  min-width: 100px;
  padding: 10px 14px;
  background: var(--c-overlay-bg);
  backdrop-filter: blur(16px) saturate(150%);
  -webkit-backdrop-filter: blur(16px) saturate(150%);
  border-radius: 10px;
  box-shadow:
    0 8px 24px rgba(0, 0, 0, 0.18),
    0 2px 8px rgba(0, 0, 0, 0.1);
  border: 1px solid var(--c-overlay-border);
  animation: tooltipIn 0.15s ease-out;
  pointer-events: auto;
}

.sentence-tooltip {
  padding-right: 42px;
  padding-bottom: 16px;
}

@keyframes tooltipIn {
  from { opacity: 0; transform: translateX(-50%) translateY(4px); }
  to { opacity: 1; transform: translateX(-50%) translateY(0); }
}

/* ⚠️ 颜色需与标注一致 */
.word-tooltip { border-left: 3px solid rgba(244, 63, 94, 0.7); }
.sentence-tooltip { border-left: 3px solid rgba(59, 130, 246, 0.7); }

.tooltip-content {
  font-size: 14px;
  line-height: 1.6;
  color: var(--c-text);
  white-space: pre-wrap;
  word-break: break-word;
}

.tooltip-remove {
  position: absolute;
  right: 8px;
  bottom: 8px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: 0;
  border: 1px solid rgba(248, 113, 113, 0.28);
  border-radius: 999px;
  background: rgba(254, 242, 242, 0.96);
  color: #dc2626;
  cursor: pointer;
  box-shadow: 0 2px 8px rgba(248, 113, 113, 0.12);
}

.tooltip-remove:hover {
  background: rgba(254, 226, 226, 0.98);
}

:global(html.dark) .tooltip-remove {
  border-color: rgba(248, 113, 113, 0.34);
  background: rgba(69, 10, 10, 0.72);
  color: #fca5a5;
}

:global(html.dark) .tooltip-remove:hover {
  background: rgba(127, 29, 29, 0.84);
}
</style>
