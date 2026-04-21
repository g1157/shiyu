import { ref, type Ref } from 'vue'
import type { Router } from 'vue-router'
import type { HighlightType } from './useRouteQuery'
import { useGlobalToast } from './useGlobalToast'
import { translateText } from '../services/api'
import { buildSentenceExplanation } from '../utils/sentenceExplanation'

interface TooltipState {
  visible: boolean
  content: string
  type: HighlightType
  position: { top: number; left: number }
}

interface ToastInstance {
  show: (msg: string) => void
}

interface TextSelectionAPI {
  selection: Ref<{ text: string; range: Range | null; type: string | null }>
  clearSelection: () => void
  getContext: () => string
  popoverPosition: Ref<{ visible: boolean; top: number; left: number }>
}

interface AnnotationAPI {
  annotationEnabled: Ref<boolean>
  findWordById: (id: string) => { meaning: string } | undefined
  findSentenceById: (id: string) => { explanation: string } | undefined
  saveWord: (word: string, meaning: string, context: string) => Promise<any>
  saveSentence: (text: string, explanation: string) => Promise<any>
}

interface AnnotationDraftState {
  meaning: string
  sentenceTranslation: string
  structureParsed: string
  structureNote: string
}

interface QuickLookupAnchorPosition {
  top: number
  left: number
}

function parseModelResult(text: string) {
  const raw = text.trim()
  try {
    return JSON.parse(raw)
  } catch {
    const match = raw.match(/\{[\s\S]*\}/)
    if (match) {
      try {
        return JSON.parse(match[0])
      } catch {
        return null
      }
    }
    return null
  }
}

/**
 * Annotation interaction composable — 管理阅读器中的标注表单、tooltip、点击和 hover 事件
 */
export function useAnnotationInteraction(
  router: Router,
  toastRef: Ref<ToastInstance | null>,
  textSelectionAPI: TextSelectionAPI,
  annotationAPI: AnnotationAPI,
) {
  const { selection, clearSelection, getContext, popoverPosition } = textSelectionAPI
  const { annotationEnabled, findWordById, findSentenceById, saveWord, saveSentence } = annotationAPI

  // State
  const showAnnotationForm = ref(false)
  const annotationType = ref<HighlightType>('word')
  const cachedSelectedText = ref('')
  const cachedContextText = ref('')
  const annotationDraft = ref<AnnotationDraftState>({
    meaning: '',
    sentenceTranslation: '',
    structureParsed: '',
    structureNote: '',
  })
  const tooltipState = ref<TooltipState>({
    visible: false,
    content: '',
    type: 'word' as HighlightType,
    position: { top: 0, left: 0 },
  })
  const quickLookupVisible = ref(false)
  const quickLookupType = ref<HighlightType>('word')
  const quickLookupSelectedText = ref('')
  const quickLookupContextText = ref('')
  const quickLookupWordPos = ref('')
  const quickLookupMeaning = ref('')
  const quickLookupTranslation = ref('')
  const quickLookupParsedHtml = ref('')
  const quickLookupStructureNote = ref('')
  const quickLookupLoading = ref(false)
  const quickLookupDeepLoading = ref(false)
  const quickLookupSaving = ref(false)
  const quickLookupError = ref('')
  const quickLookupDeepError = ref('')
  const quickLookupAnchor = ref<QuickLookupAnchorPosition | null>(null)
  let tooltipHideTimeout: ReturnType<typeof setTimeout> | null = null
  let quickLookupRequestId = 0
  let quickLookupDeepRequestId = 0

  function resetAnnotationDraft() {
    annotationDraft.value = {
      meaning: '',
      sentenceTranslation: '',
      structureParsed: '',
      structureNote: '',
    }
  }

  function resetQuickLookupData() {
    quickLookupWordPos.value = ''
    quickLookupMeaning.value = ''
    quickLookupTranslation.value = ''
    quickLookupParsedHtml.value = ''
    quickLookupStructureNote.value = ''
    quickLookupError.value = ''
    quickLookupDeepError.value = ''
  }

  function buildWordMeaningText() {
    const segments: string[] = []
    if (quickLookupWordPos.value.trim()) {
      segments.push(`词性：${quickLookupWordPos.value.trim()}`)
    }
    if (quickLookupMeaning.value.trim()) {
      segments.push(`中文释义：${quickLookupMeaning.value.trim()}`)
    }
    return segments.join(' - ') || quickLookupMeaning.value.trim()
  }

  async function runQuickLookup(type: HighlightType, text: string, contextText: string, requestId: number) {
    try {
      const req = type === 'word'
        ? {
            text: contextText ? `单词：${text}\n语境：${contextText}` : text,
            prompt_type: 'word_quick' as const,
          }
        : {
            text,
            prompt_type: 'sentence_quick' as const,
          }
      const { result } = await translateText(req)
      if (requestId !== quickLookupRequestId) return

      const content = result.trim()
      if (!content) {
        throw new Error('模型未返回结果。')
      }

      const parsed = parseModelResult(content)
      if (type === 'word') {
        quickLookupWordPos.value = String(parsed?.pos || '').trim()
        quickLookupMeaning.value = String(parsed?.meaning || parsed?.zh || content).trim()
        if (!quickLookupMeaning.value) {
          throw new Error('模型未返回可保存的释义。')
        }
      } else {
        quickLookupTranslation.value = String(parsed?.translation || content).trim()
        if (!quickLookupTranslation.value) {
          throw new Error('模型未返回可保存的句子释义。')
        }
      }
    } catch (error: any) {
      if (requestId !== quickLookupRequestId) return
      quickLookupError.value = error?.message || error?.toString() || '查询失败'
    } finally {
      if (requestId === quickLookupRequestId) {
        quickLookupLoading.value = false
      }
    }
  }

  function startQuickLookup(type: HighlightType) {
    if (!selection.value.text) return

    const selectedText = selection.value.text
    const contextText = getContext()
    const anchor = {
      top: popoverPosition.value.top,
      left: popoverPosition.value.left,
    }

    quickLookupType.value = type
    quickLookupSelectedText.value = selectedText
    quickLookupContextText.value = contextText
    quickLookupAnchor.value = anchor
    quickLookupVisible.value = true
    quickLookupLoading.value = true
    quickLookupDeepLoading.value = false
    quickLookupSaving.value = false
    resetQuickLookupData()
    clearSelection()

    const requestId = ++quickLookupRequestId
    void runQuickLookup(type, selectedText, contextText, requestId)
  }

  // Methods
  function handleAddWord() {
    startQuickLookup('word')
  }

  function handleAddSentence() {
    startQuickLookup('sentence')
  }

  function handleCloseForm() {
    showAnnotationForm.value = false
    resetAnnotationDraft()
    clearSelection()
  }

  async function handleSaveAnnotation(content: string) {
    try {
      if (annotationType.value === 'word') {
        await saveWord(cachedSelectedText.value, content, cachedContextText.value)
        toastRef.value?.show('单词已添加到生词本')
      } else {
        await saveSentence(cachedSelectedText.value, content)
        toastRef.value?.show('长难句已保存')
      }
    } catch (error: any) {
      const toast = useGlobalToast()
      toast.error('保存失败: ' + error)
    }
    handleCloseForm()
  }

  function closeQuickLookup() {
    quickLookupRequestId += 1
    quickLookupDeepRequestId += 1
    quickLookupVisible.value = false
    quickLookupLoading.value = false
    quickLookupDeepLoading.value = false
    quickLookupSaving.value = false
    quickLookupSelectedText.value = ''
    quickLookupContextText.value = ''
    quickLookupAnchor.value = null
    resetQuickLookupData()
  }

  function retryQuickLookup() {
    if (!quickLookupSelectedText.value) return
    quickLookupLoading.value = true
    quickLookupDeepLoading.value = false
    quickLookupSaving.value = false
    resetQuickLookupData()
    const requestId = ++quickLookupRequestId
    void runQuickLookup(
      quickLookupType.value,
      quickLookupSelectedText.value,
      quickLookupContextText.value,
      requestId,
    )
  }

  async function requestSentenceDeepAnalysis() {
    if (
      quickLookupType.value !== 'sentence'
      || !quickLookupSelectedText.value
      || quickLookupLoading.value
      || quickLookupDeepLoading.value
    ) {
      return
    }

    const requestId = ++quickLookupDeepRequestId
    quickLookupDeepLoading.value = true
    quickLookupDeepError.value = ''

    try {
      const { result } = await translateText({
        text: `句子：${quickLookupSelectedText.value}`,
        prompt_type: 'sentence_structure',
      })
      if (requestId !== quickLookupDeepRequestId) return

      const content = result.trim()
      const parsed = parseModelResult(content)
      if (!parsed || !parsed.parsed_html) {
        throw new Error('模型未返回结构化成分解析。')
      }

      quickLookupParsedHtml.value = String(parsed.parsed_html).trim()
      quickLookupStructureNote.value = String(parsed.structure_note || '').trim()
      const translation = String(parsed.translation || '').trim()
      if (translation) {
        quickLookupTranslation.value = translation
      }
    } catch (error: any) {
      if (requestId !== quickLookupDeepRequestId) return
      quickLookupDeepError.value = error?.message || error?.toString() || '深度解析失败'
    } finally {
      if (requestId === quickLookupDeepRequestId) {
        quickLookupDeepLoading.value = false
      }
    }
  }

  function openQuickLookupEditor() {
    cachedSelectedText.value = quickLookupSelectedText.value
    cachedContextText.value = quickLookupContextText.value
    annotationType.value = quickLookupType.value
    annotationDraft.value = {
      meaning: buildWordMeaningText(),
      sentenceTranslation: quickLookupTranslation.value,
      structureParsed: quickLookupParsedHtml.value,
      structureNote: quickLookupStructureNote.value,
    }
    showAnnotationForm.value = true
    quickLookupVisible.value = false
  }

  async function saveQuickLookup() {
    if (quickLookupSaving.value || quickLookupLoading.value) return

    quickLookupSaving.value = true
    try {
      if (quickLookupType.value === 'word') {
        await saveWord(
          quickLookupSelectedText.value,
          buildWordMeaningText(),
          quickLookupContextText.value,
        )
        toastRef.value?.show('单词已添加到生词本')
      } else {
        await saveSentence(
          quickLookupSelectedText.value,
          buildSentenceExplanation(
            quickLookupParsedHtml.value,
            quickLookupStructureNote.value,
            quickLookupTranslation.value,
          ),
        )
        toastRef.value?.show('长难句已保存')
      }
      closeQuickLookup()
    } catch (error: any) {
      const toast = useGlobalToast()
      toast.error('保存失败: ' + error)
    } finally {
      quickLookupSaving.value = false
    }
  }

  function toggleAnnotations() {
    annotationEnabled.value = !annotationEnabled.value
    if (!annotationEnabled.value) {
      clearSelection()
      tooltipState.value.visible = false
    }
  }

  function handleAnnotationClick(e: MouseEvent) {
    if (!annotationEnabled.value) return
    if (e.button !== 0) return

    const target = e.target as HTMLElement

    const wordEl = target.closest('.annotated-word, .annotated-word-subtle') as HTMLElement | null
    if (wordEl) {
      const id = wordEl.getAttribute('data-word-id')
      if (id) {
        e.preventDefault()
        e.stopPropagation()
        void router.push({ path: '/vocabulary', query: { highlight: id, type: 'word' } })
      }
      return
    }

    const sentenceEl = target.closest('.annotated-sentence, .annotated-sentence-subtle') as HTMLElement | null
    if (sentenceEl) {
      const id = sentenceEl.getAttribute('data-sentence-id')
      if (id) {
        e.preventDefault()
        e.stopPropagation()
        void router.push({ path: '/sentences', query: { highlight: id, type: 'sentence' } })
      }
    }
  }

  function showTooltipContent(target: HTMLElement, type: HighlightType) {
    let content = ''
    if (type === 'word') {
      const id = target.getAttribute('data-word-id')
      if (id) {
        const word = findWordById(id)
        if (word) content = word.meaning
      }
    } else {
      const id = target.getAttribute('data-sentence-id')
      if (id) {
        const sentence = findSentenceById(id)
        if (sentence) content = sentence.explanation
      }
    }

    if (content) {
      const rect = target.getBoundingClientRect()
      tooltipState.value = {
        visible: true,
        content,
        type,
        position: {
          top: rect.bottom + 5,
          left: rect.left + rect.width / 2,
        },
      }
    }
  }

  function handleHighlightHover(e: MouseEvent) {
    if (!annotationEnabled.value) return
    const target = e.target as HTMLElement

    const wordEl = target.closest('.annotated-word, .annotated-word-subtle') as HTMLElement | null
    const sentenceEl = target.closest('.annotated-sentence, .annotated-sentence-subtle') as HTMLElement | null

    if (!wordEl && !sentenceEl) return

    if (tooltipHideTimeout) {
      clearTimeout(tooltipHideTimeout)
      tooltipHideTimeout = null
    }

    if (wordEl) {
      showTooltipContent(wordEl, 'word')
      return
    }

    if (sentenceEl) {
      showTooltipContent(sentenceEl, 'sentence')
    }
  }

  function handleHighlightLeave() {
    if (!tooltipState.value.visible) return
    tooltipHideTimeout = setTimeout(() => {
      tooltipState.value.visible = false
    }, 300)
  }

  function handleTooltipClose() {
    tooltipState.value.visible = false
  }

  function handlePageClick(e: MouseEvent) {
    const target = e.target as HTMLElement

    if (!target.closest('.annotation-tooltip') && !target.closest('.annotated-word') && !target.closest('.annotated-sentence') && !target.closest('.annotated-word-subtle') && !target.closest('.annotated-sentence-subtle')) {
      tooltipState.value.visible = false
    }

    if (showAnnotationForm.value) {
      return
    }

    if (!target.closest('.selection-popover')) {
      if (popoverPosition.value.visible) {
        clearSelection()
      }
    }
  }

  return {
    // State
    showAnnotationForm,
    annotationType,
    cachedSelectedText,
    cachedContextText,
    annotationDraft,
    tooltipState,
    quickLookupVisible,
    quickLookupType,
    quickLookupAnchor,
    quickLookupSelectedText,
    quickLookupContextText,
    quickLookupWordPos,
    quickLookupMeaning,
    quickLookupTranslation,
    quickLookupParsedHtml,
    quickLookupStructureNote,
    quickLookupLoading,
    quickLookupDeepLoading,
    quickLookupSaving,
    quickLookupError,
    quickLookupDeepError,
    // Methods
    handleAddWord,
    handleAddSentence,
    handleCloseForm,
    handleSaveAnnotation,
    closeQuickLookup,
    retryQuickLookup,
    requestSentenceDeepAnalysis,
    openQuickLookupEditor,
    saveQuickLookup,
    toggleAnnotations,
    handleAnnotationClick,
    handleHighlightHover,
    handleHighlightLeave,
    handleTooltipClose,
    handlePageClick,
  }
}
