<template>
  <section class="d-flex flex-column flex-grow-1 h-100 overflow-hidden min-w-0">
    <header class="border-bottom px-2 px-md-3 py-2 d-flex align-items-center justify-content-between min-w-0">
      <div v-if="mode === 'channel' && chat.activeChannel" class="text-truncate">
        <span class="fw-semibold"># {{ chat.activeChannel.name }}</span>
        <span v-if="chat.activeChannel.topic" class="text-secondary small ms-2">
          {{ chat.activeChannel.topic }}
        </span>
      </div>
      <div v-else-if="mode === 'dm' && chat.activeDm" class="d-flex align-items-center gap-2 text-truncate">
        <span
          class="rounded-circle d-inline-block flex-shrink-0"
          :class="realtime.isUserOnline(chat.activeDm.peer_id) ? 'bg-success' : 'bg-secondary'"
          style="width: 10px; height: 10px"
        ></span>
        <span class="fw-semibold">@ {{ chat.activeDm.peer_username }}</span>
        <span class="text-secondary small">
          {{ realtime.isUserOnline(chat.activeDm.peer_id) ? 'Online' : 'Offline' }}
        </span>
      </div>
      <div v-else class="text-secondary">
        {{ mode === 'dm' ? 'Select a direct message or member' : 'Select a channel' }}
      </div>
    </header>

    <div
      ref="scrollContainer"
      class="flex-grow-1 overflow-auto p-2 p-md-3 d-flex flex-column gap-2"
      style="min-height: 0"
      @scroll="handleScroll"
    >
      <div v-if="activeHasMore" class="text-center mb-2">
        <button
          type="button"
          class="btn btn-outline-secondary btn-sm"
          :disabled="chat.loadingHistory"
          @click="loadOlderPreservingScroll"
        >
          {{ chat.loadingHistory ? 'Loading...' : 'Load older messages' }}
        </button>
      </div>

      <div
        v-if="!chat.loadingHistory && activeMessages.length === 0"
        class="text-secondary small my-auto text-center"
      >
        No messages yet. Start the conversation below.
      </div>

      <div
        v-for="msg in activeMessages"
        :key="msg.id"
        class="d-flex flex-column"
        :class="isOwnMessage(msg.author_id) ? 'align-items-end' : 'align-items-start'"
      >
        <div
          class="d-flex align-items-baseline gap-2 px-1 mb-1 small"
          :class="isOwnMessage(msg.author_id) ? 'flex-row-reverse' : 'flex-row'"
        >
          <span class="fw-semibold">{{ msg.author_username }}</span>
          <span class="text-secondary">{{ formatTimestamp(msg.created_at) }}</span>
        </div>
        <div
          class="rounded-3 px-3 py-2 text-break chat-bubble"
          :class="
            isOwnMessage(msg.author_id)
              ? 'bg-primary text-white'
              : 'bg-body-secondary border'
          "
          style="white-space: pre-wrap"
        >
          {{ msg.content }}
        </div>
      </div>
    </div>

    <footer class="border-top p-2 p-md-3">
      <div v-if="sendError" class="alert alert-danger py-1 px-2 small mb-2">
        {{ sendError }}
      </div>
      <form class="d-flex gap-2" @submit.prevent="handleSend">
        <input
          ref="inputEl"
          v-model="draft"
          type="text"
          class="form-control"
          :placeholder="composerPlaceholder"
          :disabled="!activeTargetId || sending"
          maxlength="4000"
        />
        <button
          type="submit"
          class="btn btn-primary"
          :disabled="!activeTargetId || sending || !draft.trim()"
        >
          Send
        </button>
      </form>
    </footer>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue';
import { ApiError } from '@/api/client';
import { useAuthStore } from '@/stores/auth';
import { useChatStore } from '@/stores/chat';
import { useRealtimeStore } from '@/stores/realtime';

const props = withDefaults(
  defineProps<{
    mode?: 'channel' | 'dm';
  }>(),
  {
    mode: 'channel',
  },
);

const auth = useAuthStore();
const chat = useChatStore();
const realtime = useRealtimeStore();

const scrollContainer = ref<HTMLElement | null>(null);
const inputEl = ref<HTMLInputElement | null>(null);
const draft = ref('');
const sending = ref(false);
const sendError = ref('');

const activeTargetId = computed(() =>
  props.mode === 'channel' ? chat.activeChannelId : chat.activeDmId,
);

const activeMessages = computed(() => chat.getMessages(props.mode));
const activeHasMore = computed(() => chat.hasMore(props.mode));

const composerPlaceholder = computed(() => {
  if (props.mode === 'channel' && chat.activeChannel) {
    return `Message #${chat.activeChannel.name}`;
  }
  if (props.mode === 'dm' && chat.activeDm) {
    return `Message @${chat.activeDm.peer_username}`;
  }
  return 'Select a conversation';
});

function isOwnMessage(authorId: string): boolean {
  return auth.user?.id === authorId;
}

function formatTimestamp(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime())
    ? ''
    : date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function isNearBottom(el: HTMLElement): boolean {
  return el.scrollHeight - el.scrollTop - el.clientHeight < 80;
}

function scrollToBottom(): void {
  if (scrollContainer.value) {
    scrollContainer.value.scrollTop = scrollContainer.value.scrollHeight;
  }
}

function focusComposer(): void {
  inputEl.value?.focus();
}

async function loadOlderPreservingScroll(): Promise<void> {
  const el = scrollContainer.value;
  const previousHeight = el?.scrollHeight ?? 0;
  await chat.loadOlderMessages(props.mode);
  await nextTick();
  if (el) {
    el.scrollTop = el.scrollHeight - previousHeight;
  }
}

function handleScroll(): void {
  const el = scrollContainer.value;
  if (!el) {
    return;
  }
  if (el.scrollTop <= 24 && activeHasMore.value && !chat.loadingHistory) {
    void loadOlderPreservingScroll();
  }
}

onMounted(async () => {
  await nextTick();
  scrollToBottom();
  focusComposer();
});

watch(activeTargetId, async () => {
  await nextTick();
  scrollToBottom();
  focusComposer();
});

watch(
  () => activeMessages.value.length,
  async (nextLen, prevLen) => {
    const el = scrollContainer.value;
    if (!el || nextLen <= prevLen) {
      return;
    }
    const wasNearBottom = prevLen === 0 || isNearBottom(el);
    if (wasNearBottom) {
      await nextTick();
      scrollToBottom();
    }
  },
);

async function handleSend(): Promise<void> {
  const text = draft.value.trim();
  if (!text) {
    return;
  }
  sendError.value = '';
  sending.value = true;
  try {
    await chat.sendMessage(props.mode, text);
    draft.value = '';
    await nextTick();
    scrollToBottom();
    focusComposer();
  } catch (err) {
    sendError.value = err instanceof ApiError ? err.message : 'Failed to send message.';
  } finally {
    sending.value = false;
  }
}
</script>

<style scoped>
.chat-bubble {
  max-width: min(85%, 640px);
}
</style>
