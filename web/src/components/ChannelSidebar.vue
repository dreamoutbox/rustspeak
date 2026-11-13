<template>
  <aside
    class="border-end bg-body-tertiary flex-column h-100 sidebar-panel"
    :class="mobileOpen ? 'd-flex sidebar-mobile-open' : 'd-none d-md-flex'"
  >
    <div class="p-3 overflow-auto flex-grow-1">
      <div class="d-flex align-items-center justify-content-between mb-2">
        <span class="text-uppercase text-secondary fw-semibold small">Text Channels</span>
        <div class="d-flex align-items-center gap-2">
          <span
            class="badge rounded-pill"
            :class="realtime.isConnected ? 'text-bg-success' : 'text-bg-warning'"
          >
            {{ realtime.status }}
          </span>
          <button
            type="button"
            class="btn-close d-md-none"
            aria-label="Close sidebar"
            @click="emit('close')"
          ></button>
        </div>
      </div>

      <div class="list-group list-group-flush mb-4">
        <button
          v-for="ch in chat.channels"
          :key="ch.id"
          type="button"
          class="list-group-item list-group-item-action border-0 rounded px-2 py-1 text-start"
          :class="{ active: chat.activeChannelId === ch.id }"
          @click="handleSelectChannel(ch.id)"
        >
          # {{ ch.name }}
        </button>
      </div>

      <div class="text-uppercase text-secondary fw-semibold small mb-2">Voice Channels</div>
      <div class="list-group list-group-flush mb-4">
        <div v-for="vc in voice.voiceChannels" :key="vc.id" class="mb-1">
          <button
            type="button"
            class="list-group-item list-group-item-action border-0 rounded px-2 py-1 d-flex align-items-center justify-content-between text-start"
            :class="{ active: voice.activeVoiceChannelId === vc.id }"
            @click="voice.joinVoice(vc.id)"
          >
            <span class="text-truncate">{{ vc.name }}</span>
            <span class="small opacity-75">
              {{ voice.activeVoiceChannelId === vc.id ? 'Joined' : 'Join' }}
            </span>
          </button>
          <div
            v-if="(voice.participantsByChannel[vc.id]?.length ?? 0) > 0"
            class="ps-3 pt-1 d-flex flex-column gap-1"
          >
            <div
              v-for="p in voice.participantsByChannel[vc.id]"
              :key="p.user_id"
              class="small d-flex align-items-center justify-content-between gap-2 text-secondary"
            >
              <span class="d-flex align-items-center gap-2 text-truncate">
                <span
                  class="rounded-circle d-inline-block flex-shrink-0"
                  :class="
                    p.user_id === auth.user?.id && voice.isSpeaking ? 'bg-success' : 'bg-secondary'
                  "
                  style="width: 7px; height: 7px"
                ></span>
                <span class="text-truncate">{{ resolveUsername(p.user_id) }}</span>
              </span>
              <span class="d-flex align-items-center gap-1 flex-shrink-0">
                <span
                  :title="p.muted || p.deafened ? 'Microphone muted' : 'Microphone active'"
                >
                  {{ p.muted || p.deafened ? '' : '🎙️' }}
                </span>
                <span
                  v-if="p.deafened"
                  title="Deafened"
                >
                  🔇
                </span>
              </span>
            </div>
          </div>
        </div>
      </div>

    </div>

    <div v-if="voice.errorMessage" class="alert alert-danger m-2 mb-0 py-1 px-2 small">
      {{ voice.errorMessage }}
    </div>

    <div
      v-if="voice.activeVoiceChannel"
      class="border-top p-2 bg-body d-flex flex-column gap-2"
    >
      <div class="d-flex align-items-center justify-content-between small">
        <span class="fw-semibold text-truncate">
          {{ voice.activeVoiceChannel.name }}
          <span v-if="!voice.hasMicrophone" class="text-secondary fw-normal">(listen-only)</span>
        </span>
        <span
          class="badge"
          :class="
            voice.connectionState === 'connected'
              ? 'text-bg-success'
              : voice.connectionState === 'failed'
                ? 'text-bg-danger'
                : 'text-bg-warning'
          "
        >
          {{ voice.connectionState }}
        </span>
      </div>
      <div class="btn-group btn-group-sm" role="group">
        <button
          type="button"
          class="btn"
          :class="voice.muted ? 'btn-warning' : 'btn-outline-secondary'"
          :disabled="!voice.hasMicrophone"
          :title="voice.hasMicrophone ? 'Toggle microphone' : 'No microphone detected'"
          @click="voice.toggleMute()"
        >
          {{ voice.hasMicrophone ? (voice.muted ? 'Unmute' : 'Mute') : 'No Mic' }}
        </button>
        <button
          type="button"
          class="btn"
          :class="voice.deafened ? 'btn-warning' : 'btn-outline-secondary'"
          @click="voice.toggleDeafen()"
        >
          {{ voice.deafened ? 'Undeafen' : 'Deafen' }}
        </button>
        <button type="button" class="btn btn-outline-danger" @click="voice.leaveVoice()">
          Leave
        </button>
      </div>
    </div>

    <div class="d-none">
      <RemoteAudio
        v-for="stream in voice.remoteStreams"
        :key="stream.id"
        :stream="stream"
        :muted="voice.deafened"
      />
    </div>
  </aside>
</template>

<script setup lang="ts">
import RemoteAudio from '@/components/RemoteAudio.vue';
import { useAuthStore } from '@/stores/auth';
import { useChatStore } from '@/stores/chat';
import { useRealtimeStore } from '@/stores/realtime';
import { useVoiceStore } from '@/stores/voice';

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
const voice = useVoiceStore();

function resolveUsername(userId: string): string {
  if (auth.user?.id === userId) {
    return auth.user.username;
  }
  return chat.directoryUsers.find((u) => u.id === userId)?.username ?? userId.slice(0, 8);
}

async function handleSelectChannel(id: string): Promise<void> {
  await chat.selectChannel(id);
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

