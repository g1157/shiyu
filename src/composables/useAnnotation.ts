import { ref, watch, nextTick, type Ref } from 'vue'
import { storeToRefs } from 'pinia'
import {
    addVocabulary,
    deleteVocabulary as deleteVocabularyApi,
    addSentence,
    deleteSentence as deleteSentenceApi,
    type VocabularyItem,
    type SentenceItem,
} from '../services/api'
import { preCacheText } from '../services/ttsCache'
import { useAppStore } from '../stores/appStore'
import { getDocumentRef } from '../utils/documentSource'
import type { DocumentKind } from '../types/document'

interface AnnotationSourcePayload {
    article_path?: string
    ebook_id?: string
    ebook_cfi?: string
    ebook_href?: string
    document_kind?: DocumentKind
    document_id?: string
}

/**
 * 文章标注系统
 * 管理生词/长难句数据，提供 DOM 高亮渲染和悬浮提示能力
 */
export function useAnnotation(
    containerRef: Ref<HTMLElement | null>,
    articleId: Ref<string | null>,
    sourceResolver?: () => AnnotationSourcePayload,
    documentKind: Ref<DocumentKind | null> = ref('article')
) {
    const appStore = useAppStore()
    const { vocabulary, sentences } = storeToRefs(appStore)
    const annotationEnabled = ref(true)
    const loading = ref(false)

    // ── 数据加载（从后端获取）──────────────────────────────

    async function loadData() {
        loading.value = true
        try {
            await Promise.all([
                appStore.fetchVocabulary(true),
                appStore.fetchSentences(true),
            ])
        } catch (e) {
            console.error('加载标注数据失败:', e)
        } finally {
            loading.value = false
        }
    }

    // ── 添加操作（写入后端）──────────────────────────────

    async function saveWord(word: string, meaning: string, context: string) {
        const resolvedSource = sourceResolver?.() ?? {
            article_path: articleId.value ?? undefined,
            document_kind: documentKind.value ?? undefined,
            document_id: articleId.value ?? undefined,
        }
        const item = await addVocabulary({
            word,
            meaning,
            context: context || undefined,
            ...resolvedSource,
        })
        appStore.addVocabularyItem(item)
        await nextTick()
        await nextTick()
        // 使用 setTimeout 确保 DOM 完全更新
        setTimeout(() => {
            highlightAnnotatedContent()
        }, 100)
        // 后台预缓存 TTS 音频（单词用 -10% 语速）
        preCacheText(word, '-10%')
        return item
    }

    async function saveSentence(sentence: string, explanation: string) {
        const resolvedSource = sourceResolver?.() ?? {
            article_path: articleId.value ?? undefined,
            document_kind: documentKind.value ?? undefined,
            document_id: articleId.value ?? undefined,
        }
        const item = await addSentence({
            sentence,
            explanation,
            ...resolvedSource,
        })
        appStore.addSentenceItem(item)
        await nextTick()
        await nextTick()
        // 使用 setTimeout 确保 DOM 完全更新
        setTimeout(() => {
            highlightAnnotatedContent()
        }, 100)
        // 后台预缓存 TTS 音频（句子用默认语速）
        preCacheText(sentence, '+0%')
        return item
    }

    async function deleteWord(id: string) {
        await deleteVocabularyApi(id)
        appStore.removeVocabularyItem(id)
        await nextTick()
        await nextTick()
        setTimeout(() => {
            highlightAnnotatedContent()
        }, 50)
    }

    async function deleteSentence(id: string) {
        await deleteSentenceApi(id)
        appStore.removeSentenceItem(id)
        await nextTick()
        await nextTick()
        setTimeout(() => {
            highlightAnnotatedContent()
        }, 50)
    }

    // ── DOM 高亮渲染 ──────────────────────────────────────

    function escapeRegex(string: string): string {
        return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    }

    /**
     * 清除容器内的现有标注，恢复原始文本
     */
    function clearExistingAnnotations() {
        if (!containerRef.value) return

        const annotationClasses = [
            'annotated-word',
            'annotated-word-subtle',
            'annotated-sentence',
            'annotated-sentence-subtle',
        ]

        for (const className of annotationClasses) {
            const elements = containerRef.value.querySelectorAll(`.${className}`)
            elements.forEach((el) => {
                const parent = el.parentNode
                if (parent) {
                    const textNode = document.createTextNode(el.textContent || '')
                    parent.replaceChild(textNode, el)
                    parent.normalize()
                }
            })
        }
    }

    /**
     * 高亮已标注的内容（单词和句子）
     *
     * ⚠️ 关键功能 - 请勿随意修改！
     *
     * 核心要点：
     * 1. 必须先标注句子，再标注单词（顺序不能颠倒）
     * 2. 使用 TreeWalker 遍历文本节点进行替换
     * 3. 需要在 DOM 完全渲染后调用（配合 nextTick 和 setTimeout）
     */
    function highlightAnnotatedContent() {
        if (!containerRef.value || !annotationEnabled.value) return

        clearExistingAnnotations()

        const currentId = articleId.value
        const currentKind = documentKind.value
        if (!currentId || !currentKind) return

        const isSameDocument = (item: VocabularyItem | SentenceItem) => {
            const ref = getDocumentRef(item)
            return ref?.kind === currentKind && ref.id === currentId
        }

        const relevantWords = vocabulary.value.filter(isSameDocument)
        const relevantSentences = sentences.value.filter(isSameDocument)

        if (relevantWords.length === 0 && relevantSentences.length === 0) return

        const firstOccurrenceWords = new Set<string>()
        const firstOccurrenceSentences = new Set<string>()

        // ========== 第一遍：标注句子 ==========
        // ⚠️ 必须先标注句子，否则会破坏句子的完整性
        if (relevantSentences.length > 0) {
            const sentenceWalker = document.createTreeWalker(
                containerRef.value,
                NodeFilter.SHOW_TEXT,
                {
                    acceptNode: (node) => {
                        if (
                            node.parentNode?.nodeName === 'SCRIPT' ||
                            node.parentNode?.nodeName === 'STYLE'
                        ) {
                            return NodeFilter.FILTER_REJECT
                        }
                        const parent = node.parentElement
                        if (
                            parent?.classList.contains('annotated-sentence') ||
                            parent?.classList.contains('annotated-sentence-subtle')
                        ) {
                            return NodeFilter.FILTER_REJECT
                        }
                        if (node.textContent?.trim() === '') {
                            return NodeFilter.FILTER_REJECT
                        }
                        return NodeFilter.FILTER_ACCEPT
                    },
                }
            )

            const sentenceReplacements: {
                node: Text
                newHtml: string
                parent: Node
            }[] = []

            let node: Node | null
            while ((node = sentenceWalker.nextNode())) {
                const textNode = node as Text
                const textContent = textNode.textContent || ''
                let newHtml = textContent
                let modified = false

                for (const sentence of relevantSentences) {
                    const isFirstOccurrence = !firstOccurrenceSentences.has(sentence.id)
                    const className = isFirstOccurrence
                        ? 'annotated-sentence'
                        : 'annotated-sentence-subtle'

                    const escapedSentence = escapeRegex(sentence.sentence)
                    const regex = new RegExp(`(${escapedSentence})`, 'i')
                    if (regex.test(newHtml)) {
                        newHtml = newHtml.replace(
                            regex,
                            `<mark class="${className}" data-sentence-id="${sentence.id}">$1</mark>`
                        )
                        modified = true
                        if (isFirstOccurrence) {
                            firstOccurrenceSentences.add(sentence.id)
                        }
                    }
                }

                if (modified && textNode.parentNode) {
                    sentenceReplacements.push({
                        node: textNode,
                        newHtml,
                        parent: textNode.parentNode,
                    })
                }
            }

            for (const { node, newHtml, parent } of sentenceReplacements) {
                if (parent.contains(node)) {
                    const range = document.createRange()
                    range.setStartBefore(node)
                    range.setEndAfter(node)
                    range.deleteContents()
                    const fragment = range.createContextualFragment(newHtml)
                    range.insertNode(fragment)
                }
            }
        }

        // ========== 第二遍：标注单词（包括句子内的单词）==========
        if (relevantWords.length > 0) {
            const wordWalker = document.createTreeWalker(
                containerRef.value,
                NodeFilter.SHOW_TEXT,
                {
                    acceptNode: (node) => {
                        if (
                            node.parentNode?.nodeName === 'SCRIPT' ||
                            node.parentNode?.nodeName === 'STYLE'
                        ) {
                            return NodeFilter.FILTER_REJECT
                        }
                        const parent = node.parentElement
                        if (
                            parent?.classList.contains('annotated-word') ||
                            parent?.classList.contains('annotated-word-subtle')
                        ) {
                            return NodeFilter.FILTER_REJECT
                        }
                        if (node.textContent?.trim() === '') {
                            return NodeFilter.FILTER_REJECT
                        }
                        return NodeFilter.FILTER_ACCEPT
                    },
                }
            )

            const wordReplacements: {
                node: Text
                newHtml: string
                parent: Node
            }[] = []

            let node: Node | null
            while ((node = wordWalker.nextNode())) {
                const textNode = node as Text
                const textContent = textNode.textContent || ''
                let newHtml = textContent
                let modified = false

                for (const word of relevantWords) {
                    const regex = new RegExp(`\\b(${escapeRegex(word.word)})\\b`, 'gi')
                    if (regex.test(newHtml)) {
                        const isFirstOccurrence = !firstOccurrenceWords.has(word.id)
                        if (isFirstOccurrence) {
                            newHtml = newHtml.replace(
                                regex,
                                `<mark class="annotated-word" data-word-id="${word.id}">$1</mark>`
                            )
                            firstOccurrenceWords.add(word.id)
                        } else {
                            newHtml = newHtml.replace(
                                regex,
                                `<mark class="annotated-word-subtle" data-word-id="${word.id}">$1</mark>`
                            )
                        }
                        modified = true
                    }
                }

                if (modified && textNode.parentNode) {
                    wordReplacements.push({
                        node: textNode,
                        newHtml,
                        parent: textNode.parentNode,
                    })
                }
            }

            for (const { node, newHtml, parent } of wordReplacements) {
                if (parent.contains(node)) {
                    const range = document.createRange()
                    range.setStartBefore(node)
                    range.setEndAfter(node)
                    range.deleteContents()
                    const fragment = range.createContextualFragment(newHtml)
                    range.insertNode(fragment)
                }
            }
        }
    }

    // ── 悬浮事件处理 ──────────────────────────────────────

    /**
     * 根据标注元素的 data-*-id 查找对应数据
     */
    function findWordById(id: string): VocabularyItem | undefined {
        return vocabulary.value.find((w) => w.id === id)
    }

    function findSentenceById(id: string): SentenceItem | undefined {
        return sentences.value.find((s) => s.id === id)
    }

    // ── 监听数据变化自动刷新高亮 ──────────────────────────

    watch(
        [vocabulary, sentences, annotationEnabled, articleId, documentKind],
        () => {
            nextTick(() => {
                highlightAnnotatedContent()
            })
        },
        { deep: true }
    )

    return {
        vocabulary,
        sentences,
        annotationEnabled,
        loading,
        loadData,
        saveWord,
        saveSentence,
        deleteWord,
        deleteSentence,
        highlightAnnotatedContent,
        clearExistingAnnotations,
        findWordById,
        findSentenceById,
    }
}
