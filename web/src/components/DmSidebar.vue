<template>
  <aside
    class="border-end bg-body-tertiary flex-column h-100 sidebar-panel"
    :class="mobileOpen ? 'd-flex sidebar-mobile-open' : 'd-none d-md-flex'"
  >
    <div class="p-3 overflow-auto flex-grow-1">
      <div class="d-flex align-items-center justify-content-between mb-2">
        <span class="text-uppercase text-secondary fw-semibold small">Direct Messages</span>
        <button
          type="button"
          class="btn-close d-md-none"
          aria-label="Close sidebar"
          @click="emit('close')"
        ></button>
      </div>

      <div class="list-group list-group-flush mb-4">
        <button
          v-for="dm in chat.dms"
          :key="dm.id"
          type="button"
          class="list-group-item list-group-item-action border-0 rounded px-2 py-1 d-flex align-items-center justify-content-between text-start"
          :class="{ active: chat.activeDmId === dm.id }"
          @click="handleSelectDm(dm.id)"
        >
          <span class="d-flex align-items-center gap-2 text-truncate">
            <span
              class="rounded-circle d-inline-block flex-shrink-0"
              :class="realtime.isUserOnline(dm.peer_id) ? 'bg-success' : 'bg-secondary'"
              style="width: 8px; height: 8px"
            ></span>
            <span class="text-truncate">@ {{ dm.peer_username }}</span>
          </span>
          <span
            v-if="(chat.unreadDmCounts[dm.id] ?? 0) > 0"
            class="badge rounded-pill text-bg-danger ms-2"
          >
            {{ chat.unreadDmCounts[dm.id] }}
          </span>
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
          @click="handleOpenDm(member.id)"
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

withDefaults(
  defineProps<{
    mobileOpen?: boolean;
  }>(),
  {
    mobileOpen: false,
  },
);

const emit = defineEmits<{
  close: [];
}>();

const auth = useAuthStore();
const chat = useChatStore();
const realtime = useRealtimeStore();

const otherMembers = computed(() =>
  chat.directoryUsers.filter((u) => u.id !== auth.user?.id),
);

async function handleSelectDm(id: string): Promise<void> {
  await chat.selectDm(id);
  emit('close');
}

async function handleOpenDm(peerId: string): Promise<void> {
  await chat.openDmWithUser(peerId);
  emit('close');
}
</script>

<style scoped>
.sidebar-panel {
  width: 260px;
  min-width: 240px;
  flex-shrink: 0;
}

@media (max-width: 767.98px) {
  .sidebar-mobile-open {
    position: fixed;
    top: 0;
    left: 0;
    bottom: 0;
    width: min(85vw, 300px);
    z-index: 1045;
  }
}
</style>

