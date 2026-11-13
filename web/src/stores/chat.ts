import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import { apiFetch } from '@/api/client';
import { useAuthStore } from '@/stores/auth';
import { useRealtimeStore } from '@/stores/realtime';
import type {
  DmConversation,
  DmMessage,
  Message,
  ServerEvent,
  TextChannel,
  User,
} from '@/types/api';
import { SoundEffect, playSound } from '@/utils/sound';

const PAGE_SIZE = 50;

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
  const activeChannelId = ref<string | null>(null);
  const activeDmId = ref<string | null>(null);
  const isViewingDms = ref(false);

  const channelMessages = ref<Record<string, ChatMessageItem[]>>({});
  const channelHasMore = ref<Record<string, boolean>>({});
  const dmMessages = ref<Record<string, ChatMessageItem[]>>({});
  const dmHasMore = ref<Record<string, boolean>>({});
  const unreadDmCounts = ref<Record<string, number>>({});
  const loadingHistory = ref(false);

  const auth = useAuthStore();
  const realtime = useRealtimeStore();
  let wiredRealtime = false;

  const activeChannel = computed(
    () => channels.value.find((c) => c.id === activeChannelId.value) ?? null,
  );

  const activeDm = computed(
    () => dms.value.find((d) => d.id === activeDmId.value) ?? null,
  );

  const totalUnreadDms = computed(() =>
    Object.values(unreadDmCounts.value).reduce((sum, count) => sum + count, 0),
  );

  function getMessages(mode: 'channel' | 'dm'): ChatMessageItem[] {
    if (mode === 'channel') {
      return activeChannelId.value ? (channelMessages.value[activeChannelId.value] ?? []) : [];
    }
    return activeDmId.value ? (dmMessages.value[activeDmId.value] ?? []) : [];
  }

  function hasMore(mode: 'channel' | 'dm'): boolean {
    if (mode === 'channel') {
      return activeChannelId.value ? (channelHasMore.value[activeChannelId.value] ?? false) : false;
    }
    return activeDmId.value ? (dmHasMore.value[activeDmId.value] ?? false) : false;
  }

  function markDmRead(conversationId: string): void {
    if (unreadDmCounts.value[conversationId]) {
      const next = { ...unreadDmCounts.value };
      delete next[conversationId];
      unreadDmCounts.value = next;
    }
  }

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
      if (event.message.author_id !== auth.user?.id) {
        playSound(SoundEffect.ChatNew);
      }
    } else if (event.type === 'dm_created') {
      const convId = event.message.conversation_id;
      upsertMessage(dmMessages.value, convId, [event.message], false);
      if (!dms.value.some((d) => d.id === convId)) {
        void loadDms();
      }
      const isFromOther = event.message.author_id !== auth.user?.id;
      if (isFromOther) {
        playSound(SoundEffect.ChatNew);
      }
      const isCurrentlyReading = isViewingDms.value && activeDmId.value === convId;
      if (isFromOther && !isCurrentlyReading) {
        unreadDmCounts.value = {
          ...unreadDmCounts.value,
          [convId]: (unreadDmCounts.value[convId] ?? 0) + 1,
        };
      }
    } else if (event.type === 'channel_created') {
      if (!channels.value.some((c) => c.id === event.channel.id)) {
        channels.value = [...channels.value, event.channel];
      }
    } else if (event.type === 'channel_deleted') {
      channels.value = channels.value.filter((c) => c.id !== event.channel_id);
      if (activeChannelId.value === event.channel_id) {
        activeChannelId.value = channels.value[0]?.id ?? null;
        if (activeChannelId.value) {
          void loadHistory('channel', activeChannelId.value);
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
      if (activeChannelId.value) {
        void loadHistory('channel', activeChannelId.value);
      }
      if (activeDmId.value) {
        void loadHistory('dm', activeDmId.value);
      }
    });
  }

  async function loadChannels(): Promise<void> {
    const res = await apiFetch<{ channels: TextChannel[] }>('/api/channels');
    channels.value = res.channels;
    if (!activeChannelId.value && res.channels.length > 0) {
      activeChannelId.value = res.channels[0].id;
    }
  }

  async function loadDms(): Promise<void> {
    const res = await apiFetch<{ conversations: DmConversation[] }>('/api/dms');
    dms.value = res.conversations;
    if (!activeDmId.value && res.conversations.length > 0) {
      activeDmId.value = res.conversations[0].id;
    }
  }

  async function loadDirectory(): Promise<void> {
    const res = await apiFetch<{ users: User[] }>('/api/users');
    directoryUsers.value = res.users;
  }

  async function loadInitialData(): Promise<void> {
    initRealtimeSync();
    await Promise.all([loadChannels(), loadDms(), loadDirectory()]);
    if (activeChannelId.value) {
      await loadHistory('channel', activeChannelId.value);
    }
    if (activeDmId.value) {
      await loadHistory('dm', activeDmId.value);
    }
  }

  async function loadHistory(
    mode: 'channel' | 'dm',
    targetId: string,
    beforeId?: string,
  ): Promise<void> {
    loadingHistory.value = true;
    try {
      const params = new URLSearchParams({ limit: String(PAGE_SIZE) });
      if (beforeId) {
        params.set('before', beforeId);
      }
      if (mode === 'channel') {
        const res = await apiFetch<{ messages: Message[] }>(
          `/api/channels/${targetId}/messages?${params.toString()}`,
        );
        const chronological = [...res.messages].sort((a, b) => a.id.localeCompare(b.id));
        upsertMessage(channelMessages.value, targetId, chronological, Boolean(beforeId));
        channelHasMore.value[targetId] = res.messages.length === PAGE_SIZE;
      } else {
        const res = await apiFetch<{ messages: DmMessage[] }>(
          `/api/dms/${targetId}/messages?${params.toString()}`,
        );
        const chronological = [...res.messages].sort((a, b) => a.id.localeCompare(b.id));
        upsertMessage(dmMessages.value, targetId, chronological, Boolean(beforeId));
        dmHasMore.value[targetId] = res.messages.length === PAGE_SIZE;
      }
    } finally {
      loadingHistory.value = false;
    }
  }

  async function loadOlderMessages(mode: 'channel' | 'dm'): Promise<void> {
    if (loadingHistory.value || !hasMore(mode)) {
      return;
    }
    const targetId = mode === 'channel' ? activeChannelId.value : activeDmId.value;
    const oldest = getMessages(mode)[0];
    if (!targetId || !oldest) {
      return;
    }
    await loadHistory(mode, targetId, oldest.id);
  }

  async function selectChannel(channelId: string): Promise<void> {
    activeChannelId.value = channelId;
    if (!channelMessages.value[channelId]) {
      await loadHistory('channel', channelId);
    }
  }

  async function selectDm(conversationId: string): Promise<void> {
    activeDmId.value = conversationId;
    markDmRead(conversationId);
    if (!dmMessages.value[conversationId]) {
      await loadHistory('dm', conversationId);
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

  async function sendMessage(mode: 'channel' | 'dm', content: string): Promise<void> {
    const trimmed = content.trim();
    if (!trimmed) {
      return;
    }
    if (mode === 'channel') {
      if (!activeChannelId.value) {
        return;
      }
      const res = await apiFetch<{ message: Message }>(
        `/api/channels/${activeChannelId.value}/messages`,
        {
          method: 'POST',
          body: JSON.stringify({ content: trimmed }),
        },
      );
      upsertMessage(channelMessages.value, activeChannelId.value, [res.message], false);
    } else {
      if (!activeDmId.value) {
        return;
      }
      const res = await apiFetch<{ message: DmMessage }>(
        `/api/dms/${activeDmId.value}/messages`,
        {
          method: 'POST',
          body: JSON.stringify({ content: trimmed }),
        },
      );
      upsertMessage(dmMessages.value, activeDmId.value, [res.message], false);
    }
  }

  function setViewingDms(viewing: boolean): void {
    isViewingDms.value = viewing;
    if (viewing && activeDmId.value) {
      markDmRead(activeDmId.value);
    }
  }

  function reset(): void {
    channels.value = [];
    dms.value = [];
    directoryUsers.value = [];
    activeChannelId.value = null;
    activeDmId.value = null;
    isViewingDms.value = false;
    channelMessages.value = {};
    channelHasMore.value = {};
    dmMessages.value = {};
    dmHasMore.value = {};
    unreadDmCounts.value = {};
  }

  return {
    channels,
    dms,
    directoryUsers,
    activeChannelId,
    activeDmId,
    activeChannel,
    activeDm,
    unreadDmCounts,
    totalUnreadDms,
    loadingHistory,
    getMessages,
    hasMore,
    loadChannels,
    loadDms,
    loadDirectory,
    loadInitialData,
    loadOlderMessages,
    selectChannel,
    selectDm,
    openDmWithUser,
    sendMessage,
    setViewingDms,
    reset,
  };
});
