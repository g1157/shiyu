<script setup lang="ts">
import { computed, ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { marked } from 'marked'
import { useRouter } from 'vue-router'
import { parseEpubToc, extractEpubChapters, addArticle, importEpubAsBook, type TocEntry, type ChapterResult } from '../services/api'
import { useGlobalToast } from '../composables/useGlobalToast'
import { useAppStore } from '../stores/appStore'
import { resolveLocalImages, resolveLocalImagesInMarkdown } from '../utils/imageResolver'
import { sanitizeRichHtml } from '../utils/sanitizeHtml'
import '../styles/reader-typography.css'

marked.setOptions({ gfm: true, breaks: true })

const toast = useGlobalToast()
const appStore = useAppStore()
const router = useRouter()

type TocEntryWithMeta = TocEntry & {
  breadcrumbs: string[]
}

type ExtractedChapter = ChapterResult & {
  toc: TocEntryWithMeta
}

function renderMarkdown(md: string): string {
  const resolved = resolveLocalImagesInMarkdown(md)
  return sanitizeRichHtml(resolveLocalImages(marked.parse(resolved) as string))
}

function resolveImagePath(filePath: string): string {
  return resolveLocalImages(`<img src="${filePath}" />`).match(/src="([^"]+)"/)?.[1] || filePath
}

const showRaw = ref<Record<number, boolean>>({})
const extractProgress = ref(0)
const step = ref(1)
const filePath = ref('')
const fileName = ref('')
const toc = ref<TocEntry[]>([])
const selectedPaths = ref<Set<string>>(new Set())
const loading = ref(false)
const extracting = ref(false)
const results = ref<ExtractedChapter[]>([])
const savedCount = ref(0)
const previewIndex = ref(-1)
const importingBook = ref(false)
const collapsedPaths = ref<Set<string>>(new Set())

const hasSelection = computed(() => selectedPaths.value.size > 0)
const selectionCount = computed(() => selectedPaths.value.size)
const bookTitle = computed(() => fileName.value.replace(/\.epub$/i, ''))
const hasParsedBook = computed(() => Boolean(filePath.value && toc.value.length > 0))
const canImportBook = computed(() =>
  hasParsedBook.value && !loading.value && !importingBook.value && !extracting.value,
)

function flattenToc(entries: TocEntry[]): TocEntry[] {
  const flat: TocEntry[] = []
  for (const entry of entries) {
    flat.push(entry)
    if (entry.children?.length) flat.push(...flattenToc(entry.children))
  }
  return flat
}

const flatToc = computed(() => flattenToc(toc.value))

function flattenTocWithMeta(entries: TocEntry[], ancestors: string[] = []): TocEntryWithMeta[] {
  const flat: TocEntryWithMeta[] = []
  for (const entry of entries) {
    const breadcrumbs = [...ancestors, entry.label]
    flat.push({ ...entry, breadcrumbs })
    if (entry.children?.length) {
      flat.push(...flattenTocWithMeta(entry.children, breadcrumbs))
    }
  }
  return flat
}

const flatTocWithMeta = computed(() => flattenTocWithMeta(toc.value))
const selectedEntries = computed(() =>
  flatTocWithMeta.value
    .filter((entry) => selectedPaths.value.has(entry.path))
    .sort((a, b) => a.index - b.index)
)

const isPartialSelection = computed(() => {
  const total = flatToc.value.length
  return total > 0 && selectionCount.value < total
})

const combinedArticleTitle = computed(() =>
  isPartialSelection.value ? `${bookTitle.value}（节选）` : bookTitle.value
)

const combinedDescription = computed(() => {
  const scope = isPartialSelection.value ? '节选合集' : '完整合集'
  return `EPUB ${scope} · ${results.value.length} 个章节 · 保留原目录层级`
})

async function selectFile() {
  try {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'EPUB', extensions: ['epub'] }],
    })
    if (!selected) return

    filePath.value = selected as string
    fileName.value = (selected as string).split(/[/\\]/).pop() || ''
    loading.value = true
    step.value = 1
    toc.value = []
    selectedPaths.value = new Set()
    results.value = []
    previewIndex.value = -1
    showRaw.value = {}
    collapsedPaths.value = new Set()

    try {
      toc.value = await parseEpubToc(filePath.value)
      step.value = 2
    } catch (e: any) {
      toast.error('解析 EPUB 失败: ' + e)
    } finally {
      loading.value = false
    }
  } catch (e: any) {
    toast.error('选择文件失败: ' + e)
  }
}

function getDescendantPaths(entries: TocEntry[]): string[] {
  const paths: string[] = []
  for (const entry of entries) {
    paths.push(entry.path)
    if (entry.children?.length) paths.push(...getDescendantPaths(entry.children))
  }
  return paths
}

function toggleSelectWithChildren(entry: TocEntry) {
  const set = new Set(selectedPaths.value)
  const allPaths = [entry.path, ...getDescendantPaths(entry.children || [])]
  const isSelected = set.has(entry.path)
  for (const path of allPaths) {
    if (isSelected) {
      set.delete(path)
    } else {
      set.add(path)
    }
  }
  selectedPaths.value = set
}

function toggleCollapse(path: string) {
  const set = new Set(collapsedPaths.value)
  if (set.has(path)) {
    set.delete(path)
  } else {
    set.add(path)
  }
  collapsedPaths.value = set
}

function selectAll() {
  const set = new Set<string>()
  for (const entry of flatToc.value) {
    set.add(entry.path)
  }
  selectedPaths.value = set
}

function deselectAll() {
  selectedPaths.value = new Set()
}

async function extractSelected() {
  const entries = selectedEntries.value
  const paths = entries.map((entry) => entry.path)
  if (paths.length === 0) return

  extracting.value = true
  results.value = []
  extractProgress.value = 0

  try {
    const extracted = await extractEpubChapters(filePath.value, paths)
    results.value = extracted.map((result, idx) => ({
      ...result,
      markdown: stripLeadingDuplicateHeading(
        result.markdown,
        entries[idx] ? [entries[idx].label, result.title] : [result.title],
      ),
      toc: entries[idx],
    }))
    extractProgress.value = extracted.length
    step.value = 3
  } catch (e: any) {
    toast.error('提取章节失败: ' + e)
  } finally {
    extracting.value = false
  }
}

async function importCurrentFileAsBook() {
  if (!filePath.value || importingBook.value) return

  importingBook.value = true
  try {
    const saved = await importEpubAsBook(filePath.value)
    appStore.addEbook(saved)
    toast.success(`《${saved.title}》已导入书架`)
    await router.push({ path: '/books', query: { bookId: saved.id } })
  } catch (e: any) {
    toast.error('导入图书失败: ' + e)
  } finally {
    importingBook.value = false
  }
}

function toggleRaw(idx: number) {
  showRaw.value = { ...showRaw.value, [idx]: !showRaw.value[idx] }
}

function normalizeText(value: string): string {
  return value
    .replace(/^#+\s*/, '')
    .replace(/\s+/g, ' ')
    .trim()
    .toLowerCase()
}

function stripLeadingDuplicateHeading(markdown: string, candidates: string[]): string {
  const lines = markdown.split('\n')
  const normalizedCandidates = candidates
    .map((candidate) => normalizeText(candidate || ''))
    .filter(Boolean)

  let firstContentIdx = 0
  while (firstContentIdx < lines.length && !lines[firstContentIdx].trim()) {
    firstContentIdx++
  }

  const firstLine = lines[firstContentIdx]?.trim()
  const headingMatch = firstLine?.match(/^#{1,6}\s+(.+)$/)
  if (!headingMatch) {
    return markdown.trim()
  }

  if (!normalizedCandidates.includes(normalizeText(headingMatch[1]))) {
    return markdown.trim()
  }

  lines.splice(firstContentIdx, 1)
  if (firstContentIdx < lines.length && !lines[firstContentIdx].trim()) {
    lines.splice(firstContentIdx, 1)
  }

  return lines.join('\n').trim()
}

function buildChapterHeading(level: number, title: string): string {
  const depth = Math.min(Math.max(level + 2, 2), 6)
  return `${'#'.repeat(depth)} ${title}`
}

function buildCombinedMarkdown(): string {
  const lines: string[] = []
  lines.push(`# ${combinedArticleTitle.value}`)
  lines.push('')
  lines.push(`> 来源：${bookTitle.value}.epub`)
  lines.push(`> 章节数：${results.value.length}`)
  lines.push(`> 导入方式：保留目录层级的 EPUB 合集`)
  lines.push('')

  if (results.value.length > 1) {
    lines.push('## 导入目录')
    lines.push('')
    for (const chapter of results.value) {
      const indent = '  '.repeat(Math.max(chapter.toc.level, 0))
      lines.push(`${indent}- ${chapter.toc.label}`)
    }
    lines.push('')
  }

  for (const chapter of results.value) {
    lines.push(buildChapterHeading(chapter.toc.level, chapter.toc.label || chapter.title))
    lines.push('')
    if (chapter.markdown.trim()) {
      lines.push(chapter.markdown.trim())
      lines.push('')
    }
  }

  return lines.join('\n').trim()
}

function buildChapterDescription(result: ExtractedChapter): string {
  return `EPUB 章节导入 · ${bookTitle.value} / ${result.toc.breadcrumbs.join(' / ')}`
}

async function saveToArticles(result: ExtractedChapter, index: number) {
  try {
    const saved = await addArticle({
      title: result.toc.label || result.title || `第 ${index + 1} 章`,
      content: result.markdown,
      category: bookTitle.value,
      description: buildChapterDescription(result),
    })
    appStore.addArticle(saved)
    savedCount.value++
    toast.success(`"${saved.title}" 已保存到文章库`)
  } catch (e: any) {
    toast.error('保存失败: ' + e)
  }
}

async function saveAllToArticles() {
  extracting.value = true
  let count = 0
  try {
    for (const result of results.value) {
      if (result.markdown) {
        const saved = await addArticle({
          title: result.toc.label || result.title || `章节 ${count + 1}`,
          content: result.markdown,
          category: bookTitle.value,
          description: buildChapterDescription(result),
        })
        appStore.addArticle(saved)
        count++
      }
    }
    savedCount.value += count
    toast.success(`已保存 ${count} 篇文章到材料库`)
  } catch (e: any) {
    toast.error('批量保存失败: ' + e)
  } finally {
    extracting.value = false
  }
}

async function saveAsCombinedArticle() {
  if (results.value.length === 0) return

  extracting.value = true
  try {
    const saved = await addArticle({
      title: combinedArticleTitle.value,
      content: buildCombinedMarkdown(),
      category: bookTitle.value,
      description: combinedDescription.value,
    })
    appStore.addArticle(saved)
    savedCount.value++
    toast.success(`"${saved.title}" 已保存为保留目录层级的合集材料`)
  } catch (e: any) {
    toast.error('保存合集失败: ' + e)
  } finally {
    extracting.value = false
  }
}

function copyMarkdown(md: string) {
  navigator.clipboard.writeText(md)
  toast.success('Markdown 已复制到剪贴板')
}
</script>

<template>
  <section class="page-container epub-import-page">
    <header class="intro-card panel-card">
      <div class="intro-copy">
        <span class="intro-kicker">阅读主入口</span>
        <h1 class="intro-title">先把 EPUB 导入为图书，再开始阅读。</h1>
        <p class="intro-subtitle">
          默认主动作是把整本书放进书架并立即进入阅读。章节提取仍保留，但退到精读材料的次路径。
        </p>
        <div class="intro-actions">
          <button class="btn btn-primary" @click="selectFile">
            {{ filePath ? '重新选择 EPUB' : '选择 EPUB 文件' }}
          </button>
          <button class="btn btn-outline" :disabled="!canImportBook" @click="importCurrentFileAsBook">
            {{ importingBook ? '导入图书中...' : '导入为图书并开始阅读' }}
          </button>
        </div>
        <div class="intro-notes">
          <span>主路径：导入 EPUB → 开始阅读 → 保存词句 → 进入复习</span>
          <span v-if="fileName">当前文件：{{ fileName }}</span>
        </div>
      </div>

      <div class="intro-status">
        <div class="status-card">
          <span class="status-value">{{ filePath ? '1' : '0' }}</span>
          <span class="status-label">已选择文件</span>
        </div>
        <div class="status-card">
          <span class="status-value">{{ flatToc.length }}</span>
          <span class="status-label">可读章节</span>
        </div>
        <div class="status-card">
          <span class="status-value">{{ selectionCount }}</span>
          <span class="status-label">已选节选</span>
        </div>
      </div>
    </header>

    <section class="primary-flow panel-card">
      <div class="section-head">
        <div>
          <h2>导入为图书</h2>
          <p>这是默认主路径。导入完成后会直接跳到书架并打开这本书。</p>
        </div>
        <button class="btn btn-primary" :disabled="!canImportBook" @click="importCurrentFileAsBook">
          {{ importingBook ? '导入图书中...' : '导入为图书并开始阅读' }}
        </button>
      </div>

      <div v-if="loading" class="loading-card">
        <div class="spinner"></div>
        <span>正在解析 EPUB 目录...</span>
      </div>

      <div v-else-if="!filePath" class="empty-card">
        <p>先选择一本 EPUB，系统会解析目录，然后你可以直接导入整本书开始阅读。</p>
      </div>

      <div v-else class="file-summary">
        <div class="file-summary__main">
          <div class="file-name">{{ fileName }}</div>
          <div class="file-meta">共 {{ flatToc.length }} 个章节，适合直接进入连续阅读。</div>
        </div>
        <button class="btn-sm" @click="selectFile">更换文件</button>
      </div>
    </section>

    <section class="secondary-flow panel-card">
      <div class="section-head section-head--secondary">
        <div>
          <span class="section-tag">次路径</span>
          <h2>提取节选到精读材料</h2>
          <p>只有当你想把部分章节沉淀为材料时，再走这条路径。</p>
        </div>
        <button class="btn-sm" :disabled="!hasParsedBook" @click="selectAll">全选章节</button>
      </div>

      <div v-if="!hasParsedBook" class="empty-card empty-card--subtle">
        <p>选好 EPUB 后，这里才会显示章节目录和节选保存能力。</p>
      </div>

      <template v-else>
        <div class="toc-toolbar">
          <div class="toc-info">
            <strong>{{ fileName }}</strong>
            <span>已解析 {{ flatToc.length }} 个章节</span>
          </div>
          <div class="toc-actions">
            <button class="btn-sm" @click="selectAll">全选</button>
            <button class="btn-sm" @click="deselectAll">取消全选</button>
            <button class="btn btn-primary" :disabled="!hasSelection || extracting" @click="extractSelected">
              <span v-if="extracting" class="spinner-sm"></span>
              {{ extracting ? `提取中 (${extractProgress}/${selectionCount})...` : `提取 ${selectionCount} 个章节` }}
            </button>
          </div>
        </div>

        <div class="toc-list">
          <template v-for="entry in toc" :key="entry.path + '-' + entry.index">
            <div
              class="toc-item"
              :class="{ selected: selectedPaths.has(entry.path) }"
              :style="{ paddingLeft: 16 + entry.level * 24 + 'px' }"
              role="checkbox"
              :aria-checked="selectedPaths.has(entry.path)"
              :aria-label="`选择章节 ${entry.label}`"
              tabindex="0"
              @click="toggleSelectWithChildren(entry)"
              @keydown.enter.prevent="toggleSelectWithChildren(entry)"
              @keydown.space.prevent="toggleSelectWithChildren(entry)"
            >
              <button
                v-if="entry.children && entry.children.length"
                type="button"
                class="toc-toggle"
                :aria-expanded="!collapsedPaths.has(entry.path)"
                :aria-label="collapsedPaths.has(entry.path) ? '展开子章节' : '折叠子章节'"
                @click.stop="toggleCollapse(entry.path)"
              >
                <svg :class="{ collapsed: collapsedPaths.has(entry.path) }" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="6 9 12 15 18 9"/>
                </svg>
              </button>
              <span v-else class="toc-toggle-placeholder"></span>
              <div class="toc-checkbox">
                <svg v-if="selectedPaths.has(entry.path)" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2.5">
                  <polyline points="20 6 9 17 4 12"/>
                </svg>
              </div>
              <span class="toc-label">{{ entry.label }}</span>
              <span v-if="entry.children && entry.children.length" class="toc-child-count">{{ entry.children.length }}</span>
            </div>

            <template v-if="entry.children && entry.children.length && !collapsedPaths.has(entry.path)">
              <template v-for="child in entry.children" :key="child.path + '-' + child.index">
                <div
                  class="toc-item"
                  :class="{ selected: selectedPaths.has(child.path) }"
                  :style="{ paddingLeft: 16 + child.level * 24 + 'px' }"
                  role="checkbox"
                  :aria-checked="selectedPaths.has(child.path)"
                  :aria-label="`选择章节 ${child.label}`"
                  tabindex="0"
                  @click="toggleSelectWithChildren(child)"
                  @keydown.enter.prevent="toggleSelectWithChildren(child)"
                  @keydown.space.prevent="toggleSelectWithChildren(child)"
                >
                  <button
                    v-if="child.children && child.children.length"
                    type="button"
                    class="toc-toggle"
                    :aria-expanded="!collapsedPaths.has(child.path)"
                    :aria-label="collapsedPaths.has(child.path) ? '展开子章节' : '折叠子章节'"
                    @click.stop="toggleCollapse(child.path)"
                  >
                    <svg :class="{ collapsed: collapsedPaths.has(child.path) }" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
                      <polyline points="6 9 12 15 18 9"/>
                    </svg>
                  </button>
                  <span v-else class="toc-toggle-placeholder"></span>
                  <div class="toc-checkbox">
                    <svg v-if="selectedPaths.has(child.path)" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2.5">
                      <polyline points="20 6 9 17 4 12"/>
                    </svg>
                  </div>
                  <span class="toc-label">{{ child.label }}</span>
                  <span v-if="child.children && child.children.length" class="toc-child-count">{{ child.children.length }}</span>
                </div>

                <template v-if="child.children && child.children.length && !collapsedPaths.has(child.path)">
                  <div
                    v-for="grandchild in child.children"
                    :key="grandchild.path + '-' + grandchild.index"
                    class="toc-item"
                    :class="{ selected: selectedPaths.has(grandchild.path) }"
                    :style="{ paddingLeft: 16 + grandchild.level * 24 + 'px' }"
                    role="checkbox"
                    :aria-checked="selectedPaths.has(grandchild.path)"
                    :aria-label="`选择章节 ${grandchild.label}`"
                    tabindex="0"
                    @click="toggleSelectWithChildren(grandchild)"
                    @keydown.enter.prevent="toggleSelectWithChildren(grandchild)"
                    @keydown.space.prevent="toggleSelectWithChildren(grandchild)"
                  >
                    <span class="toc-toggle-placeholder"></span>
                    <div class="toc-checkbox">
                      <svg v-if="selectedPaths.has(grandchild.path)" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2.5">
                        <polyline points="20 6 9 17 4 12"/>
                      </svg>
                    </div>
                    <span class="toc-label">{{ grandchild.label }}</span>
                  </div>
                </template>
              </template>
            </template>
          </template>
        </div>
      </template>
    </section>

    <section v-if="step === 3" class="results-panel panel-card">
      <div class="section-head">
        <div>
          <h2>节选结果</h2>
          <p>节选已提取完成，可以沉淀到精读材料库。</p>
        </div>
        <div class="result-actions">
          <button class="btn btn-primary" @click="saveAsCombinedArticle" :disabled="extracting || !results.length">
            {{ extracting ? '保存中...' : '保存为一本合集材料' }}
          </button>
          <button class="btn-sm" @click="saveAllToArticles" :disabled="extracting || !results.length">
            {{ `按章节分别保存 (${results.length})` }}
          </button>
        </div>
      </div>

      <div class="result-summary">
        <div class="summary-main">
          <div class="summary-title">{{ combinedArticleTitle }}</div>
          <div class="summary-desc">{{ combinedDescription }}</div>
        </div>
        <div class="summary-side">
          <span class="summary-badge">次路径</span>
          <span>适合做后续精读</span>
        </div>
      </div>

      <div class="result-list">
        <div v-for="(result, idx) in results" :key="idx" class="result-card">
          <div class="result-card-header" @click="previewIndex = previewIndex === idx ? -1 : idx">
            <div class="result-info">
              <h3>{{ result.toc.label || result.title || `章节 ${idx + 1}` }}</h3>
              <div class="result-path">{{ result.toc.breadcrumbs.join(' / ') }}</div>
              <div class="result-stats">
                <span>层级 {{ result.toc.level + 1 }}</span>
                <span>{{ result.markdown.length.toLocaleString() }} 字符</span>
                <span v-if="result.images.length">{{ result.images.length }} 张图片</span>
              </div>
            </div>
            <div class="result-btns">
              <button class="btn-sm" @click.stop="copyMarkdown(result.markdown)">复制</button>
              <button class="btn-sm save" @click.stop="saveToArticles(result, idx)">保存</button>
              <svg :class="{ rotated: previewIndex === idx }" viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="6 9 12 15 18 9"/>
              </svg>
            </div>
          </div>

          <transition name="expand">
            <div v-if="previewIndex === idx" class="result-preview">
              <div class="preview-toolbar">
                <button class="btn-sm" :class="{ active: !showRaw[idx] }" @click="showRaw[idx] = false">渲染预览</button>
                <button class="btn-sm" :class="{ active: showRaw[idx] }" @click="toggleRaw(idx)">原始 Markdown</button>
                <span v-if="result.images.length" class="img-badge">{{ result.images.length }} 张图片</span>
              </div>
              <div
                v-if="!showRaw[idx]"
                class="rendered-preview reader-typography"
                v-html="renderMarkdown(result.markdown) || '<p>无内容</p>'"
              ></div>
              <pre v-else class="markdown-preview">{{ result.markdown }}</pre>
              <div v-if="result.images && result.images.length" class="image-gallery">
                <div class="gallery-title">章节图片 ({{ result.images.length }})</div>
                <div class="gallery-grid">
                  <div v-for="(img, imgIdx) in result.images" :key="imgIdx" class="gallery-item">
                    <img :src="resolveImagePath(img.data_base64)" :alt="img.filename" />
                    <span>{{ img.filename }}</span>
                  </div>
                </div>
              </div>
            </div>
          </transition>
        </div>
      </div>
    </section>
  </section>
</template>

<style scoped>
.epub-import-page {
  max-width: 1180px;
  margin: 0 auto;
  padding: 24px 28px 36px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.panel-card {
  border-radius: 24px;
  border: 1px solid var(--c-border);
  background: var(--c-surface-1);
  box-shadow: var(--c-shadow-md);
}

.intro-card {
  display: grid;
  grid-template-columns: minmax(0, 1.6fr) minmax(300px, 0.9fr);
  gap: 18px;
  padding: 24px;
  border-color: var(--c-border-strong);
  background: linear-gradient(180deg, color-mix(in srgb, var(--c-surface-1) 94%, transparent), var(--c-surface-2));
}

.intro-copy {
  display: flex;
  flex-direction: column;
}

.intro-kicker {
  display: inline-flex;
  align-self: flex-start;
  padding: 5px 10px;
  border-radius: 999px;
  background: var(--c-accent-pill);
  color: var(--c-primary);
  font-size: 0.78rem;
  font-weight: 700;
  letter-spacing: 0.04em;
}

.intro-title {
  margin: 14px 0 10px;
  font-size: 2rem;
  line-height: 1.15;
  color: var(--c-text);
}

.intro-subtitle {
  margin: 0;
  max-width: 700px;
  color: var(--c-text-lighter);
  line-height: 1.75;
}

.intro-actions {
  display: flex;
  gap: 12px;
  margin-top: 22px;
  flex-wrap: wrap;
}

.intro-notes {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 16px;
  color: var(--c-text-lighter);
  font-size: 0.9rem;
  line-height: 1.6;
}

.intro-status {
  display: grid;
  gap: 12px;
}

.status-card {
  display: flex;
  flex-direction: column;
  justify-content: center;
  min-height: 96px;
  padding: 18px 20px;
  border-radius: 18px;
  border: 1px solid var(--c-border);
  background: var(--c-surface-2);
}

.status-value {
  font-size: 1.8rem;
  font-weight: 800;
  color: var(--c-text);
}

.status-label {
  margin-top: 4px;
  color: var(--c-text-lighter);
  font-size: 0.86rem;
}

.primary-flow,
.secondary-flow,
.results-panel {
  padding: 22px;
}

.secondary-flow {
  background: var(--c-surface-2);
}

.section-head {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: flex-start;
  flex-wrap: wrap;
}

.section-head h2 {
  margin: 0;
  font-size: 1.22rem;
  color: var(--c-text);
}

.section-head p {
  margin: 6px 0 0;
  color: var(--c-text-lighter);
  line-height: 1.7;
}

.section-tag {
  display: inline-flex;
  margin-bottom: 8px;
  padding: 4px 10px;
  border-radius: 999px;
  background: var(--c-accent-pill);
  color: var(--c-primary);
  font-size: 0.76rem;
  font-weight: 700;
}

.loading-card,
.empty-card,
.file-summary,
.result-summary {
  margin-top: 18px;
  padding: 18px;
  border-radius: 18px;
  border: 1px solid var(--c-border);
  background: var(--c-surface-2);
}

.loading-card,
.empty-card {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  min-height: 120px;
  text-align: center;
  color: var(--c-text-lighter);
}

.empty-card--subtle {
  min-height: 100px;
}

.file-summary,
.result-summary {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: center;
  flex-wrap: wrap;
}

.file-name,
.summary-title {
  font-size: 1rem;
  font-weight: 700;
  color: var(--c-text);
}

.file-meta,
.summary-desc {
  margin-top: 4px;
  color: var(--c-text-lighter);
}

.toc-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  margin-top: 18px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}

.toc-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.toc-info span {
  color: var(--c-text-lighter);
  font-size: 0.88rem;
}

.toc-actions,
.result-actions,
.result-btns,
.preview-toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}

.toc-list {
  border: 1px solid var(--c-border);
  border-radius: 18px;
  overflow: hidden;
  background: var(--c-surface-1);
}

.toc-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  cursor: pointer;
  border-bottom: 1px solid var(--c-border-light);
  transition: background 0.18s ease, border-color 0.18s ease;
}

.toc-item:last-child {
  border-bottom: none;
}

.toc-item:hover,
.toc-item.selected {
  background: var(--c-hover-bg);
}

.toc-item.selected {
  border-color: var(--c-accent-border);
}

.toc-toggle {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  flex-shrink: 0;
  color: var(--c-text-lighter);
  border: none;
  padding: 0;
  background: transparent;
  border-radius: 6px;
  transition: background 0.15s ease, color 0.15s ease;
}

.toc-item:focus-visible,
.toc-toggle:focus-visible {
  outline: 2px solid var(--c-primary);
  outline-offset: 2px;
}

.toc-toggle:hover {
  background: var(--c-selected-bg);
  color: var(--c-text);
}

.toc-toggle svg {
  transition: transform 0.2s;
}

.toc-toggle svg.collapsed {
  transform: rotate(-90deg);
}

.toc-toggle-placeholder {
  width: 20px;
  flex-shrink: 0;
}

.toc-checkbox {
  width: 22px;
  height: 22px;
  border: 1.5px solid var(--c-border-strong);
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background 0.15s ease, border-color 0.15s ease, color 0.15s ease;
}

.toc-item.selected .toc-checkbox {
  background: var(--c-primary);
  border-color: var(--c-primary);
}

.toc-item.selected .toc-checkbox svg {
  color: #fff;
}

.toc-label {
  font-size: 0.95rem;
  color: var(--c-text);
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.toc-child-count {
  font-size: 0.72rem;
  color: var(--c-text-lighter);
  background: var(--c-selected-bg);
  padding: 2px 7px;
  border-radius: 999px;
  flex-shrink: 0;
}

.btn-sm {
  padding: 6px 12px;
  border: 1px solid var(--c-border);
  border-radius: 10px;
  background: var(--c-surface-1);
  color: var(--c-text);
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
  transition: border-color 0.15s ease, background 0.15s ease, color 0.15s ease;
}

.btn-sm:hover {
  border-color: var(--c-primary);
  color: var(--c-primary);
  background: var(--c-hover-bg);
}

.btn-sm.save {
  border-color: rgba(22, 163, 74, 0.22);
  color: #15803d;
  background: rgba(22, 163, 74, 0.08);
}

.btn-sm.save:hover {
  background: rgba(22, 163, 74, 0.14);
}

.summary-main {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.summary-side {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 0.82rem;
  color: var(--c-primary);
  white-space: nowrap;
}

.summary-badge {
  padding: 3px 8px;
  border-radius: 999px;
  background: var(--c-accent-pill);
  color: var(--c-primary);
  font-weight: 700;
}

.result-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.result-card {
  border: 1px solid var(--c-border);
  border-radius: 18px;
  background: var(--c-surface-1);
  overflow: hidden;
  transition: border-color 0.18s ease, transform 0.18s ease;
}

.result-card:hover {
  border-color: var(--c-border-strong);
  transform: translateY(-1px);
}

.result-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  padding: 16px 18px;
  cursor: pointer;
}

.result-info h3 {
  margin: 0 0 2px;
  font-size: 1rem;
  color: var(--c-text);
}

.result-path {
  font-size: 0.8rem;
  color: var(--c-text-lighter);
  margin-bottom: 4px;
  word-break: break-word;
}

.result-stats {
  display: flex;
  gap: 12px;
  font-size: 0.8rem;
  color: var(--c-text-lighter);
  flex-wrap: wrap;
}

.result-btns svg {
  color: var(--c-text-lighter);
  transition: transform 0.2s;
  flex-shrink: 0;
}

.result-btns svg.rotated {
  transform: rotate(180deg);
}

.result-preview {
  border-top: 1px solid var(--c-border-light);
  padding: 16px;
  background: var(--c-surface-2);
}

.preview-toolbar {
  margin-bottom: 12px;
}

.preview-toolbar .btn-sm.active {
  background: var(--c-primary);
  color: #fff;
  border-color: var(--c-primary);
}

.img-badge {
  font-size: 0.8rem;
  color: var(--c-text-lighter);
  margin-left: auto;
}

.rendered-preview {
  padding: 1rem;
  background: var(--c-surface-1);
  border: 1px solid var(--c-border);
  border-radius: 12px;
  font-size: 0.95rem;
  line-height: 1.8;
  color: var(--c-text);
  max-height: 500px;
  overflow-y: auto;
}

.rendered-preview img {
  max-width: 100%;
  border-radius: 8px;
  margin: 8px 0;
}

.markdown-preview {
  margin: 0;
  padding: 1rem;
  background: var(--c-bg-lighter);
  border: 1px solid var(--c-border);
  border-radius: 12px;
  font-size: 0.85rem;
  line-height: 1.6;
  color: var(--c-text);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 400px;
  overflow-y: auto;
  font-family: var(--font-mono);
}

.image-gallery {
  margin-top: 16px;
  border-top: 1px solid var(--c-border);
  padding-top: 12px;
}

.gallery-title {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--c-text-lighter);
  margin-bottom: 10px;
}

.gallery-grid {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.gallery-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.gallery-item img {
  max-width: 240px;
  max-height: 180px;
  border-radius: 8px;
  border: 1px solid var(--c-border);
  object-fit: contain;
  background: var(--c-surface-1);
}

.gallery-item span {
  font-size: 0.7rem;
  color: var(--c-text-lighter);
}

.spinner {
  width: 40px;
  height: 40px;
  border: 3px solid var(--c-border);
  border-top-color: var(--c-primary);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.spinner-sm {
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #fff;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.expand-enter-active,
.expand-leave-active {
  transition: all 0.3s ease;
}

.expand-enter-from,
.expand-leave-to {
  opacity: 0;
  max-height: 0;
  padding: 0 16px;
}

@media (max-width: 960px) {
  .epub-import-page {
    padding: 18px 16px 30px;
  }

  .intro-card {
    grid-template-columns: 1fr;
  }

  .file-summary,
  .toc-toolbar,
  .section-head,
  .result-summary,
  .result-card-header {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
