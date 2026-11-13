<template>
  <section class="d-flex flex-column flex-grow-1 h-100 overflow-hidden">
    <header class="border-bottom px-3 py-2 d-flex align-items-center justify-content-between">
      <div v-if="chat.activeChannel">
        <span class="fw-semibold"># {{ chat.activeChannel.name }}</span>
        <span v-if="chat.activeChannel.topic" class="text-secondary small ms-2">
          {{ chat.activeChannel.topic }}
        </span>
      </div>
      <div v-else-if="chat.activeDm" class="d-flex align-items-center gap-2">
        <span
          class="rounded-circle d-inline-block"
          :class="realtime.isUserOnline(chat.activeDm.peer_id) ? 'bg-success' : 'bg-secondary'"
          style="width: 10px; height: 10px"
        ></span>
        <span class="fw-semibold">@ {{ chat.activeDm.peer_username }}</span>
        <span class="text-secondary small">
          {{ realtime.isUserOnline(chat.activeDm.peer_id) ? 'Online' : 'Offline' }}
        </span>
      </div>
      <div v-else class="text-secondary">Select a channel or direct message</div>
    </header>

    <div
      ref="scrollContainer"
      class="flex-grow-1 overflow-auto p-3 d-flex flex-column gap-2"
      @scroll="handleScroll"
    >
      <div v-if="chat.activeHasMore" class="text-center mb-2">
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
        v-if="!chat.loadingHistory && chat.activeMessages.length === 0"
        class="text-secondary small my-auto text-center"
      >
        No messages yet. Start the conversation below.
      </div>

      <div
        v-for="msg in chat.activeMessages"
        :key="msg.id"
        class="px-2 py-1 rounded hover-bg"
      >
        <div class="d-flex align-items-baseline gap-2">
          <span class="fw-semibold">{{ msg.author_username }}</span>
          <span class="text-secondary small">{{ formatTimestamp(msg.created_at) }}</span>
        </div>
        <div class="text-break" style="white-space: pre-wrap">{{ msg.content }}</div>
      </div>
    </div>

    <footer class="border-top p-3">
      <div v-if="sendError" class="alert alert-danger py-1 px-2 small mb-2">
        {{ sendError }}
      </div>
      <form class="d-flex gap-2" @submit.prevent="handleSend">
        <input
          v-model="draft"
          type="text"
          class="form-control"
          :placeholder="composerPlaceholder"
          :disabled="!chat.activeTarget || sending"
          maxlength="4000"
        />
        <button
          type="submit"
          class="btn btn-primary"
          :disabled="!chat.activeTarget || sending || !draft.trim()"
        >
          Send
        </button>
      </form>
    </footer>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { ApiError } from '@/api/client';
import { useChatStore } from '@/stores/chat';
import { useRealtimeStore } from '@/stores/realtime';

const chat = useChatStore();
const realtime = useRealtimeStore();

const scrollContainer = ref<HTMLElement | null>(null);
const draft = ref('');
const sending = ref(false);
const sendError = ref('');

const composerPlaceholder = computed(() => {
  if (chat.activeChannel) {
    return `Message #${chat.activeChannel.name}`;
  }
  if (chat.activeDm) {
    return `Message @${chat.activeDm.peer_username}`;
  }
  return 'Select a conversation';
});

function formatTimestamp(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? '' : date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function isNearBottom(el: HTMLElement): boolean {
  return el.scrollHeight - el.scrollTop - el.clientHeight < 80;
}

function scrollToBottom(): void {
  if (scrollContainer.value) {
    scrollContainer.value.scrollTop = scrollContainer.value.scrollHeight;
  }
}

async function loadOlderPreservingScroll(): Promise<void> {
  const el = scrollContainer.value;
  const previousHeight = el?.scrollHeight ?? 0;
  await chat.loadOlderMessages();
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
  if (el.scrollTop <= 24 && chat.activeHasMore && !chat.loadingHistory) {
    void loadOlderPreservingScroll();
  }
}

watch(
  () => chat.activeTarget,
  async () => {
    await nextTick();
    scrollToBottom();
  },
);

watch(
  () => chat.activeMessages.length,
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
    await chat.sendMessage(text);
    draft.value = '';
    await nextTick();
    scrollToBottom();
  } catch (err) {
    sendError.value = err instanceof ApiError ? err.message : 'Failed to send message.';
  } finally {
    sending.value = false;
  }
}
</script>

