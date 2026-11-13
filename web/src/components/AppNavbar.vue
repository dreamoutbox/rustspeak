<template>
  <nav class="navbar navbar-expand bg-body-tertiary border-bottom px-3">
    <RouterLink class="navbar-brand fw-semibold" :to="{ name: 'app' }">
      RustSpeak
    </RouterLink>
    <div class="navbar-nav me-auto">
      <RouterLink class="nav-link" active-class="active" :to="{ name: 'app' }">
        Chat
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
      <span class="small text-secondary">{{ auth.user?.username }}</span>
      <span class="badge text-bg-secondary text-uppercase">{{ auth.user?.role }}</span>
      <button class="btn btn-outline-secondary btn-sm" @click="handleLogout">
        Logout
      </button>
    </div>
  </nav>
</template>

<script setup lang="ts">
import { RouterLink, useRouter } from 'vue-router';
import { useAuthStore } from '@/stores/auth';

const auth = useAuthStore();
const router = useRouter();

async function handleLogout(): Promise<void> {
  await auth.logout();
  await router.push({ name: 'login' });
}
</script>

