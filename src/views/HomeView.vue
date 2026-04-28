<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '../stores/appStore'
import type { ArticleItem, EbookItem, SentenceItem, VocabularyItem } from '../services/api'
import { formatDate } from '../utils/format'

const router = useRouter()
const appStore = useAppStore()

const loading = computed(() =>
  !appStore.ebooksLoaded ||
  !appStore.articlesLoaded ||
  !appStore.vocabularyLoaded ||
  !appStore.sentencesLoaded,
)

const currentBook = computed(() => appStore.recentEbooks[0] ?? null)
const recentBooks = computed(() => appStore.recentEbooks.slice(0, 3))
const recentArticles = computed(() =>
  [...appStore.articles]
    .sort((a, b) => (b.published_at ?? b.created_at) - (a.published_at ?? a.created_at))
    .slice(0, 2),
)

function countDue(items: Array<VocabularyItem | SentenceItem>) {
  const now = Date.now()
  return items.filter((item) => !item.srs_due || item.srs_due <= now).length
}

const dueVocabularyCount = computed(() => countDue(appStore.vocabulary))
const dueSentenceCount = computed(() => countDue(appStore.sentences))
const dueReviewCount = computed(() => dueVocabularyCount.value + dueSentenceCount.value)

function go(path: string) {
  void router.push(path)
}

function openBook(book: EbookItem) {
  const query: Record<string, string> = { bookId: book.id }
  if (book.cfi_position) query.cfi = book.cfi_position
  void router.push({ path: '/books', query })
}

function continueReading() {
  if (currentBook.value) {
    openBook(currentBook.value)
    return
  }
  go('/epub-import')
}

function openArticle(article: ArticleItem) {
  void router.push({
    path: '/articles',
    query: { articleId: article.id },
  })
}

function formatPercent(progress: number) {
  return `${Math.round(progress * 100)}%`
}

function formatRelativeTime(timestamp?: number) {
  if (!timestamp) return '尚未开始'

  const diff = Date.now() - timestamp
  const hour = 60 * 60 * 1000
  const day = 24 * hour

  if (diff < hour) return '刚刚读过'
  if (diff < day) return `${Math.max(1, Math.round(diff / hour))} 小时前`
  if (diff < day * 7) return `${Math.max(1, Math.round(diff / day))} 天前`
  return formatDate(timestamp)
}

onMounted(async () => {
  await Promise.all([
    appStore.fetchEbooks(),
    appStore.fetchArticles(),
    appStore.fetchVocabulary(),
    appStore.fetchSentences(),
  ])
})
</script>

<template>
  <div class="home-start">
    <section class="start-hero">
      <div class="start-hero__eyebrow">拾语</div>
      <h1>{{ currentBook ? '继续读。' : '从一本 EPUB 开始。' }}</h1>

      <div v-if="currentBook" class="current-reading">
        <div class="current-reading__main">
          <span class="current-reading__label">当前在读</span>
          <h2>{{ currentBook.title }}</h2>
          <p>{{ currentBook.author || '未填写作者' }} · {{ formatRelativeTime(currentBook.last_read_at) }}</p>
          <div class="progress-row">
            <div class="progress-track">
              <div class="progress-fill" :style="{ width: formatPercent(currentBook.progress) }"></div>
            </div>
            <span>{{ formatPercent(currentBook.progress) }}</span>
          </div>
        </div>
        <button class="btn btn-primary" @click="continueReading">继续阅读</button>
      </div>

      <div v-else class="current-reading current-reading--empty">
        <div>
          <span class="current-reading__label">书架为空</span>
          <h2>导入一本书，直接进入阅读。</h2>
          <p>词句沉淀会留在阅读过程中完成，不需要先整理材料。</p>
        </div>
        <button class="btn btn-primary" @click="continueReading">导入 EPUB</button>
      </div>
    </section>

    <section class="home-section" v-if="recentBooks.length">
      <div class="section-head">
        <h2>最近阅读</h2>
        <button class="text-link" @click="go('/books')">书架</button>
      </div>
      <div class="simple-list">
        <button
          v-for="book in recentBooks"
          :key="book.id"
          class="simple-row"
          @click="openBook(book)"
        >
          <span class="simple-row__title">{{ book.title }}</span>
          <span class="simple-row__meta">{{ formatPercent(book.progress) }} · {{ formatRelativeTime(book.last_read_at) }}</span>
        </button>
      </div>
    </section>

    <section class="home-section home-section--compact">
      <button class="review-line" @click="go('/review')">
        <span>今日复习</span>
        <strong>{{ loading ? '...' : dueReviewCount }} 项</strong>
      </button>
      <button class="review-line" @click="go('/epub-import')">
        <span>导入新书</span>
        <strong>EPUB</strong>
      </button>
    </section>

    <section class="home-section" v-if="recentArticles.length">
      <div class="section-head">
        <h2>精读材料</h2>
        <button class="text-link" @click="go('/articles')">全部</button>
      </div>
      <div class="simple-list">
        <button
          v-for="article in recentArticles"
          :key="article.id"
          class="simple-row"
          @click="openArticle(article)"
        >
          <span class="simple-row__title">{{ article.source_document_title || article.title }}</span>
          <span class="simple-row__meta">{{ article.content_kind }} · {{ formatDate(article.published_at || article.created_at) }}</span>
        </button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.home-start {
  width: min(100%, 860px);
  margin: 0 auto;
  padding: 48px 32px 56px;
  display: flex;
  flex-direction: column;
  gap: 22px;
}

.start-hero {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.start-hero__eyebrow,
.current-reading__label {
  color: var(--c-text-lighter);
  font-size: 0.78rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.start-hero h1 {
  margin: 0;
  color: var(--c-text);
  font-size: clamp(2rem, 5vw, 3.4rem);
  line-height: 1.05;
  letter-spacing: -0.055em;
}

.current-reading {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 24px;
  padding: 22px;
  border: 1px solid var(--c-border);
  border-radius: 18px;
  background: var(--c-surface-1);
}

.current-reading--empty {
  align-items: center;
}

.current-reading__main {
  flex: 1;
  min-width: 0;
}

.current-reading h2 {
  margin: 8px 0 6px;
  color: var(--c-text);
  font-size: 1.35rem;
  line-height: 1.25;
}

.current-reading p {
  margin: 0;
  color: var(--c-text-lighter);
  line-height: 1.6;
}

.progress-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 16px;
  color: var(--c-text-lighter);
  font-weight: 700;
}

.progress-track {
  flex: 1;
  height: 4px;
  border-radius: 999px;
  background: var(--c-track-bg);
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  border-radius: inherit;
  background: var(--c-primary);
}

.home-section {
  padding-top: 8px;
}

.home-section--compact {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
}

.section-head h2 {
  margin: 0;
  color: var(--c-text);
  font-size: 0.98rem;
}

.text-link {
  border: none;
  background: transparent;
  color: var(--c-primary);
  font-weight: 650;
  cursor: pointer;
}

.simple-list {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--c-border);
  border-radius: 14px;
  background: var(--c-surface-1);
  overflow: hidden;
}

.simple-row,
.review-line {
  width: 100%;
  border: none;
  background: transparent;
  color: var(--c-text);
  cursor: pointer;
  text-align: left;
}

.simple-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 16px;
  padding: 13px 15px;
  border-bottom: 1px solid var(--c-border-light);
}

.simple-row:last-child {
  border-bottom: none;
}

.simple-row:hover,
.review-line:hover {
  background: var(--c-hover-bg);
}

.simple-row__title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 650;
}

.simple-row__meta {
  color: var(--c-text-lighter);
  font-size: 0.88rem;
}

.review-line {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 14px 15px;
  border: 1px solid var(--c-border);
  border-radius: 14px;
  background: var(--c-surface-1);
}

.review-line span {
  color: var(--c-text-lighter);
}

.review-line strong {
  color: var(--c-text);
}

@media (max-width: 760px) {
  .home-start {
    padding: 32px 18px 40px;
  }

  .current-reading,
  .simple-row,
  .home-section--compact {
    grid-template-columns: 1fr;
  }

  .current-reading {
    align-items: flex-start;
    flex-direction: column;
  }
}
</style>
