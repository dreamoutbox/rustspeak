<template>
  <div class="min-vh-100 d-flex flex-column">
    <AppNavbar />

    <main class="container py-4">
      <div class="card">
        <div class="card-body">
          <h2 class="h5 mb-2">Welcome, {{ auth.user?.username }}</h2>
          <p class="text-secondary mb-0">
            Session authenticated. Text channels, DMs, and voice UI will appear here.
          </p>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import AppNavbar from '@/components/AppNavbar.vue';
import { useAuthStore } from '@/stores/auth';

const auth = useAuthStore();
</script>
