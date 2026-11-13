<template>
  <div class="vh-100 d-flex flex-column overflow-hidden">
    <AppNavbar />

    <div v-if="loadError" class="alert alert-danger m-3 mb-0 py-2 small">
      {{ loadError }}
    </div>

    <div class="d-flex flex-grow-1 overflow-hidden">
      <ChannelSidebar />
      <MessagePane />
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import { ApiError } from '@/api/client';
import AppNavbar from '@/components/AppNavbar.vue';
import ChannelSidebar from '@/components/ChannelSidebar.vue';
import MessagePane from '@/components/MessagePane.vue';
import { useChatStore } from '@/stores/chat';
import { useRealtimeStore } from '@/stores/realtime';

const chat = useChatStore();
const realtime = useRealtimeStore();
const loadError = ref('');

onMounted(async () => {
  realtime.connect();
  try {
    await chat.loadInitialData();
  } catch (err) {
    loadError.value = err instanceof ApiError ? err.message : 'Failed to load workspace data.';
  }
});

onUnmounted(() => {
  realtime.disconnect();
  chat.reset();
});
</script>
