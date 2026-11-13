<template>
  <RouterView />
</template>

<script setup lang="ts">
import { watch } from 'vue';
import { RouterView } from 'vue-router';
import { useAuthStore } from '@/stores/auth';
import { useChatStore } from '@/stores/chat';
import { useRealtimeStore } from '@/stores/realtime';
import { useVoiceStore } from '@/stores/voice';

const auth = useAuthStore();
const chat = useChatStore();
const realtime = useRealtimeStore();
const voice = useVoiceStore();

watch(
  () => auth.isAuthenticated,
  (authenticated) => {
    if (authenticated) {
      realtime.connect();
      void Promise.all([chat.loadInitialData(), voice.loadVoiceChannels()]);
    } else {
      void voice.leaveVoice();
      voice.reset();
      realtime.disconnect();
      chat.reset();
    }
  },
  { immediate: true },
);
</script>
