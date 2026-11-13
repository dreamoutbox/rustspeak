<template>
  <audio ref="audioEl" autoplay playsinline :muted="muted"></audio>
</template>

<script setup lang="ts">
import { onBeforeUnmount, ref, watchEffect } from 'vue';

const props = defineProps<{
  stream: MediaStream;
  muted: boolean;
}>();

const audioEl = ref<HTMLAudioElement | null>(null);

watchEffect(() => {
  if (audioEl.value && audioEl.value.srcObject !== props.stream) {
    audioEl.value.srcObject = props.stream;
  }
});

onBeforeUnmount(() => {
  if (audioEl.value) {
    audioEl.value.srcObject = null;
  }
});
</script>
