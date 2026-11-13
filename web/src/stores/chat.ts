import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import { apiFetch } from '@/api/client';
import { useRealtimeStore } from '@/stores/realtime';
import type {
  DmConversation,
  DmMessage,
  Message,
  ServerEvent,
  TextChannel,
  User,
} from '@/types/api';

const PAGE_SIZE = 50;

export type ActiveTarget =
  | { kind: 'channel'; id: string }
  | { kind: 'dm'; id: string }
  | null;

export interface ChatMessageItem {
  id: string;
  author_id: string;
  author_username: string;
  content: string;
  created_at: string;
}

export const useChatStore = defineStore('chat', () => {
  const channels = ref<TextChannel[]>([]);
  const dms = ref<DmConversation[]>([]);
  const directoryUsers = ref<User[]>([]);
  const activeTarget = ref<ActiveTarget>(null);

  const channelMessages = ref<Record<string, ChatMessageItem[]>>({});
  const channelHasMore = ref<Record<string, boolean>>({});
  const dmMessages = ref<Record<string, ChatMessageItem[]>>({});
  const dmHasMore = ref<Record<string, boolean>>({});
  const loadingHistory = ref(false);

  const realtime = useRealtimeStore();
  let wiredRealtime = false;

  const activeChannel = computed(() => {
    if (activeTarget.value?.kind !== 'channel') {
      return null;
    }
    return channels.value.find((c) => c.id === activeTarget.value?.id) ?? null;
  });

  const activeDm = computed(() => {
    if (activeTarget.value?.kind !== 'dm') {
      return null;
    }
    return dms.value.find((d) => d.id === activeTarget.value?.id) ?? null;
  });

  const activeMessages = computed<ChatMessageItem[]>(() => {
    if (!activeTarget.value) {
      return [];
    }
    if (activeTarget.value.kind === 'channel') {
      return channelMessages.value[activeTarget.value.id] ?? [];
    }
    return dmMessages.value[activeTarget.value.id] ?? [];
  });

  const activeHasMore = computed<boolean>(() => {
    if (!activeTarget.value) {
      return false;
    }
    if (activeTarget.value.kind === 'channel') {
      return channelHasMore.value[activeTarget.value.id] ?? false;
    }
    return dmHasMore.value[activeTarget.value.id] ?? false;
  });

  function upsertMessage(
    map: Record<string, ChatMessageItem[]>,
    key: string,
    incoming: ChatMessageItem[],
    prepend: boolean,
  ): void {
    const existing = map[key] ?? [];
    const seen = new Set(existing.map((m) => m.id));
    const uniqueIncoming = incoming.filter((m) => !seen.has(m.id));
    if (uniqueIncoming.length === 0) {
      return;
    }
    const merged = prepend ? [...uniqueIncoming, ...existing] : [...existing, ...uniqueIncoming];
    merged.sort((a, b) => a.id.localeCompare(b.id));
    map[key] = merged;
  }

  function handleServerEvent(event: ServerEvent): void {
    if (event.type === 'message_created') {
      upsertMessage(channelMessages.value, event.message.channel_id, [event.message], false);
    } else if (event.type === 'dm_created') {
      upsertMessage(dmMessages.value, event.message.conversation_id, [event.message], false);
      if (!dms.value.some((d) => d.id === event.message.conversation_id)) {
        void loadDms();
      }
    } else if (event.type === 'channel_created') {
      if (!channels.value.some((c) => c.id === event.channel.id)) {
        channels.value = [...channels.value, event.channel];
      }
    } else if (event.type === 'channel_deleted') {
      channels.value = channels.value.filter((c) => c.id !== event.channel_id);
      if (activeTarget.value?.kind === 'channel' && activeTarget.value.id === event.channel_id) {
        activeTarget.value = channels.value[0]
          ? { kind: 'channel', id: channels.value[0].id }
          : null;
        if (activeTarget.value) {
          void loadActiveHistory();
        }
      }
    }
  }

  function initRealtimeSync(): void {
    if (wiredRealtime) {
      return;
    }
    wiredRealtime = true;
    realtime.onEvent(handleServerEvent);
    realtime.onReconnect(() => {
      void loadInitialData();
      void gapFillActive();
    });
  }

  async function loadChannels(): Promise<void> {
    const res = await apiFetch<{ channels: TextChannel[] }>('/api/channels');
    channels.value = res.channels;
    if (!activeTarget.value && res.channels.length > 0) {
      activeTarget.value = { kind: 'channel', id: res.channels[0].id };
    }
  }

  async function loadDms(): Promise<void> {
    const res = await apiFetch<{ conversations: DmConversation[] }>('/api/dms');
    dms.value = res.conversations;
  }

  async function loadDirectory(): Promise<void> {
    const res = await apiFetch<{ users: User[] }>('/api/users');
    directoryUsers.value = res.users;
  }

  async function loadInitialData(): Promise<void> {
    initRealtimeSync();
    await Promise.all([loadChannels(), loadDms(), loadDirectory()]);
    if (activeTarget.value) {
      await loadActiveHistory();
    }
  }

  async function loadActiveHistory(beforeId?: string): Promise<void> {
    if (!activeTarget.value) {
      return;
    }
    const target = activeTarget.value;
    loadingHistory.value = true;
    try {
      const params = new URLSearchParams({ limit: String(PAGE_SIZE) });
      if (beforeId) {
        params.set('before', beforeId);
      }
      if (target.kind === 'channel') {
        const res = await apiFetch<{ messages: Message[] }>(
          `/api/channels/${target.id}/messages?${params.toString()}`,
        );
        // Backend returns newest-first when paginating; sort ascending for display.
        const chronological = [...res.messages].sort((a, b) => a.id.localeCompare(b.id));
        upsertMessage(channelMessages.value, target.id, chronological, Boolean(beforeId));
        channelHasMore.value[target.id] = res.messages.length === PAGE_SIZE;
      } else {
        const res = await apiFetch<{ messages: DmMessage[] }>(
          `/api/dms/${target.id}/messages?${params.toString()}`,
        );
        const chronological = [...res.messages].sort((a, b) => a.id.localeCompare(b.id));
        upsertMessage(dmMessages.value, target.id, chronological, Boolean(beforeId));
        dmHasMore.value[target.id] = res.messages.length === PAGE_SIZE;
      }
    } finally {
      loadingHistory.value = false;
    }
  }

  async function gapFillActive(): Promise<void> {
    if (!activeTarget.value) {
      return;
    }
    await loadActiveHistory();
  }

  async function loadOlderMessages(): Promise<void> {
    if (loadingHistory.value || !activeHasMore.value) {
      return;
    }
    const oldest = activeMessages.value[0];
    if (!oldest) {
      return;
    }
    await loadActiveHistory(oldest.id);
  }

  async function selectChannel(channelId: string): Promise<void> {
    activeTarget.value = { kind: 'channel', id: channelId };
    if (!channelMessages.value[channelId]) {
      await loadActiveHistory();
    }
  }

  async function selectDm(conversationId: string): Promise<void> {
    activeTarget.value = { kind: 'dm', id: conversationId };
    if (!dmMessages.value[conversationId]) {
      await loadActiveHistory();
    }
  }

  async function openDmWithUser(peerId: string): Promise<void> {
    const res = await apiFetch<{ conversation: DmConversation }>('/api/dms', {
      method: 'POST',
      body: JSON.stringify({ peer_id: peerId }),
    });
    if (!dms.value.some((d) => d.id === res.conversation.id)) {
      dms.value = [...dms.value, res.conversation];
    }
    await selectDm(res.conversation.id);
  }

  async function sendMessage(content: string): Promise<void> {
    const trimmed = content.trim();
    if (!trimmed || !activeTarget.value) {
      return;
    }
    if (activeTarget.value.kind === 'channel') {
      const res = await apiFetch<{ message: Message }>(
        `/api/channels/${activeTarget.value.id}/messages`,
        {
          method: 'POST',
          body: JSON.stringify({ content: trimmed }),
        },
      );
      upsertMessage(channelMessages.value, activeTarget.value.id, [res.message], false);
    } else {
      const res = await apiFetch<{ message: DmMessage }>(
        `/api/dms/${activeTarget.value.id}/messages`,
        {
          method: 'POST',
          body: JSON.stringify({ content: trimmed }),
        },
      );
      upsertMessage(dmMessages.value, activeTarget.value.id, [res.message], false);
    }
  }

  function reset(): void {
    channels.value = [];
    dms.value = [];
    directoryUsers.value = [];
    activeTarget.value = null;
    channelMessages.value = {};
    channelHasMore.value = {};
    dmMessages.value = {};
    dmHasMore.value = {};
  }

  return {
    channels,
    dms,
    directoryUsers,
    activeTarget,
    activeChannel,
    activeDm,
    activeMessages,
    activeHasMore,
    loadingHistory,
    loadChannels,
    loadDms,
    loadDirectory,
    loadInitialData,
    loadOlderMessages,
    selectChannel,
    selectDm,
    openDmWithUser,
    sendMessage,
    reset,
  };
});

