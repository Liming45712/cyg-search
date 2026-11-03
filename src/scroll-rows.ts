import { computed, nextTick, ref, type Ref } from 'vue';

export function useScrollRows<T>(rows: Ref<T[]>) {
  const limit = ref(50);
  const visibleRows = computed(() => rows.value.slice(0, limit.value));
  const hasMore = computed(() => limit.value < rows.value.length);
  const loading = ref(false);
  let viewport: HTMLElement | undefined;

  async function loadMore() {
    if (loading.value || !hasMore.value) return;
    loading.value = true;
    limit.value = Math.min(limit.value + 50, rows.value.length);
    await nextTick();
    loading.value = false;
  }

  function onScroll(event: Event) {
    const target = event.target;
    if (!(target instanceof HTMLElement) || target.clientHeight === 0
      || target.scrollHeight <= target.clientHeight) return;
    viewport = target;
    if (target.scrollHeight - target.scrollTop - target.clientHeight <= 120) void loadMore();
  }

  function resetRows() {
    limit.value = 50;
    if (viewport?.isConnected) viewport.scrollTop = 0;
  }

  return { visibleRows, hasMore, loading, loadMore, onScroll, resetRows };
}
