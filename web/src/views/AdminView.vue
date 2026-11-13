<template>
  <div class="min-vh-100 d-flex flex-column">
    <AppNavbar />

    <main class="container py-4">
      <div class="card">
        <div class="card-body">
          <h2 class="h5 mb-2">Administration</h2>
          <p class="text-secondary mb-0">
            Channel and user management controls will appear here.
          </p>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import AppNavbar from '@/components/AppNavbar.vue';
</script>
