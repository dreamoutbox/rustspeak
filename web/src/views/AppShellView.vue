<template>
  <div class="d-flex flex-column overflow-hidden app-shell">
    <AppNavbar
      :show-sidebar-toggle="true"
      sidebar-label="Channels"
      @toggle-sidebar="sidebarOpen = !sidebarOpen"
    />

    <div class="d-flex flex-grow-1 overflow-hidden position-relative min-h-0">
      <div
        v-if="sidebarOpen"
        class="d-md-none position-fixed top-0 start-0 w-100 h-100 bg-dark bg-opacity-50"
        style="z-index: 1040"
        @click="sidebarOpen = false"
      ></div>
      <ChannelSidebar :mobile-open="sidebarOpen" @close="sidebarOpen = false" />
      <MessagePane mode="channel" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import AppNavbar from '@/components/AppNavbar.vue';
import ChannelSidebar from '@/components/ChannelSidebar.vue';
import MessagePane from '@/components/MessagePane.vue';

const sidebarOpen = ref(false);
</script>

<style scoped>
.app-shell {
  height: 100vh;
  height: 100dvh;
}

.min-h-0 {
  min-height: 0;
}
</style>
