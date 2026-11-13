<template>
  <aside class="border-end bg-body-tertiary d-flex flex-column h-100" style="width: 260px; min-width: 220px">
    <div class="p-3 overflow-auto flex-grow-1">
      <div class="d-flex align-items-center justify-content-between mb-2">
        <span class="text-uppercase text-secondary fw-semibold small">Text Channels</span>
        <span
          class="badge rounded-pill"
          :class="realtime.isConnected ? 'text-bg-success' : 'text-bg-warning'"
        >
          {{ realtime.status }}
        </span>
      </div>

      <div class="list-group list-group-flush mb-4">
        <button
          v-for="ch in chat.channels"
          :key="ch.id"
          type="button"
          class="list-group-item list-group-item-action border-0 rounded px-2 py-1 text-start"
          :class="{
            active: chat.activeTarget?.kind === 'channel' && chat.activeTarget.id === ch.id,
          }"
          @click="chat.selectChannel(ch.id)"
        >
          # {{ ch.name }}
        </button>
      </div>

      <div class="text-uppercase text-secondary fw-semibold small mb-2">Direct Messages</div>
      <div class="list-group list-group-flush mb-4">
        <button
          v-for="dm in chat.dms"
          :key="dm.id"
          type="button"
          class="list-group-item list-group-item-action border-0 rounded px-2 py-1 d-flex align-items-center gap-2 text-start"
          :class="{
            active: chat.activeTarget?.kind === 'dm' && chat.activeTarget.id === dm.id,
          }"
          @click="chat.selectDm(dm.id)"
        >
          <span
            class="rounded-circle d-inline-block flex-shrink-0"
            :class="realtime.isUserOnline(dm.peer_id) ? 'bg-success' : 'bg-secondary'"
            style="width: 8px; height: 8px"
          ></span>
          <span class="text-truncate">@ {{ dm.peer_username }}</span>
        </button>
        <div v-if="chat.dms.length === 0" class="text-secondary small px-2">
          No conversations yet
        </div>
      </div>

      <div class="text-uppercase text-secondary fw-semibold small mb-2">Members</div>
      <div class="list-group list-group-flush">
        <button
          v-for="member in otherMembers"
          :key="member.id"
          type="button"
          class="list-group-item list-group-item-action border-0 rounded px-2 py-1 d-flex align-items-center justify-content-between text-start"
          title="Open direct message"
          @click="chat.openDmWithUser(member.id)"
        >
          <span class="d-flex align-items-center gap-2 text-truncate">
            <span
              class="rounded-circle d-inline-block flex-shrink-0"
              :class="realtime.isUserOnline(member.id) ? 'bg-success' : 'bg-secondary'"
              style="width: 8px; height: 8px"
            ></span>
            <span class="text-truncate">{{ member.username }}</span>
          </span>
          <span class="small text-secondary">DM</span>
        </button>
      </div>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useAuthStore } from '@/stores/auth';
import { useChatStore } from '@/stores/chat';
import { useRealtimeStore } from '@/stores/realtime';

const auth = useAuthStore();
const chat = useChatStore();
const realtime = useRealtimeStore();

const otherMembers = computed(() =>
  chat.directoryUsers.filter((u) => u.id !== auth.user?.id),
);
</script>

