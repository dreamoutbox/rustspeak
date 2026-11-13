<template>
  <nav class="navbar navbar-expand bg-body-tertiary border-bottom px-2 px-md-3 gap-2">
    <button
      v-if="showSidebarToggle"
      type="button"
      class="btn btn-outline-secondary btn-sm d-md-none"
      aria-label="Toggle sidebar"
      @click="emit('toggle-sidebar')"
    >
      {{ sidebarLabel }}
    </button>
    <RouterLink class="navbar-brand fw-semibold me-2" :to="{ name: 'app' }">
      RustSpeak
    </RouterLink>
    <div class="navbar-nav me-auto">
      <RouterLink class="nav-link" active-class="active" :to="{ name: 'app' }">
        Chat
      </RouterLink>
      <RouterLink
        class="nav-link d-flex align-items-center gap-1"
        active-class="active"
        :to="{ name: 'dms' }"
      >
        <span>Inbox</span>
        <span v-if="chat.totalUnreadDms > 0" class="badge rounded-pill text-bg-danger">
          {{ chat.totalUnreadDms }}
        </span>
      </RouterLink>
      <RouterLink
        v-if="auth.isAdmin"
        class="nav-link"
        active-class="active"
        :to="{ name: 'admin' }"
      >
        Admin
      </RouterLink>
    </div>
    <div class="d-flex align-items-center gap-2">
      <span class="small text-secondary d-none d-sm-inline">{{ auth.user?.username }}</span>
      <span class="badge text-bg-secondary text-uppercase d-none d-sm-inline">
        {{ auth.user?.role }}
      </span>
      <button class="btn btn-outline-secondary btn-sm" @click="handleLogout">
        Logout
      </button>
    </div>
  </nav>
</template>

<script setup lang="ts">
import { RouterLink, useRouter } from 'vue-router';
import { useAuthStore } from '@/stores/auth';
import { useChatStore } from '@/stores/chat';

withDefaults(
  defineProps<{
    showSidebarToggle?: boolean;
    sidebarLabel?: string;
  }>(),
  {
    showSidebarToggle: false,
    sidebarLabel: 'Channels',
  },
);

const emit = defineEmits<{
  'toggle-sidebar': [];
}>();

const auth = useAuthStore();
const chat = useChatStore();
const router = useRouter();

async function handleLogout(): Promise<void> {
  await auth.logout();
  await router.push({ name: 'login' });
}
</script>
