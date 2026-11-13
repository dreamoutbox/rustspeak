<template>
  <aside class="border-end bg-body-tertiary d-flex flex-column h-100" style="width: 260px; min-width: 220px">
    <div class="p-3 overflow-auto flex-grow-1">
      <div class="d-flex align-items-center justify-content-between mb-2">
        <span class="text-uppercase text-secondary fw-semibold small">Text Channels</span>
        <span
          class="badge rounded-pill"
          :class="realtime.isConnected ? 'text-bg-success' : 'text-bg-warning'"
        >
          {{ realtime.status }}
        </span>
      </div>

      <div class="list-group list-group-flush mb-4">
        <button
          v-for="ch in chat.channels"
          :key="ch.id"
          type="button"
          class="list-group-item list-group-item-action border-0 rounded px-2 py-1 text-start"
          :class="{
            active: chat.activeTarget?.kind === 'channel' && chat.activeTarget.id === ch.id,
          }"
          @click="chat.selectChannel(ch.id)"
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
              v-for="uid in voice.participantsByChannel[vc.id]"
              :key="uid"
              class="small d-flex align-items-center gap-2 text-secondary"
            >
              <span
                class="rounded-circle d-inline-block flex-shrink-0"
                :class="
                  uid === auth.user?.id && voice.isSpeaking ? 'bg-success' : 'bg-secondary'
                "
                style="width: 7px; height: 7px"
              ></span>
              <span class="text-truncate">{{ resolveUsername(uid) }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="text-uppercase text-secondary fw-semibold small mb-2">Direct Messages</div>
      <div class="list-group list-group-flush mb-4">
        <button
          v-for="dm in chat.dms"
          :key="dm.id"
          type="button"
          class="list-group-item list-group-item-action border-0 rounded px-2 py-1 d-flex align-items-center gap-2 text-start"
          :class="{
            active: chat.activeTarget?.kind === 'dm' && chat.activeTarget.id === dm.id,
          }"
          @click="chat.selectDm(dm.id)"
        >
          <span
            class="rounded-circle d-inline-block flex-shrink-0"
            :class="realtime.isUserOnline(dm.peer_id) ? 'bg-success' : 'bg-secondary'"
            style="width: 8px; height: 8px"
          ></span>
          <span class="text-truncate">@ {{ dm.peer_username }}</span>
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
          @click="chat.openDmWithUser(member.id)"
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

    <div v-if="voice.errorMessage" class="alert alert-danger m-2 mb-0 py-1 px-2 small">
      {{ voice.errorMessage }}
    </div>

    <div
      v-if="voice.activeVoiceChannel"
      class="border-top p-2 bg-body d-flex flex-column gap-2"
    >
      <div class="d-flex align-items-center justify-content-between small">
        <span class="fw-semibold text-truncate">{{ voice.activeVoiceChannel.name }}</span>
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
          @click="voice.toggleMute()"
        >
          {{ voice.muted ? 'Unmute' : 'Mute' }}
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
import { computed } from 'vue';
import RemoteAudio from '@/components/RemoteAudio.vue';
import { useAuthStore } from '@/stores/auth';
import { useChatStore } from '@/stores/chat';
import { useRealtimeStore } from '@/stores/realtime';
import { useVoiceStore } from '@/stores/voice';

const auth = useAuthStore();
const chat = useChatStore();
const realtime = useRealtimeStore();
const voice = useVoiceStore();

const otherMembers = computed(() =>
  chat.directoryUsers.filter((u) => u.id !== auth.user?.id),
);

function resolveUsername(userId: string): string {
  if (auth.user?.id === userId) {
    return auth.user.username;
  }
  return chat.directoryUsers.find((u) => u.id === userId)?.username ?? userId.slice(0, 8);
}
</script>

