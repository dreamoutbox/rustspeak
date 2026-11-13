<template>
  <div class="min-vh-100 d-flex flex-column">
    <AppNavbar />

    <main class="container py-4">
      <div v-if="errorMessage" class="alert alert-danger py-2 small" role="alert">
        {{ errorMessage }}
      </div>

      <div class="row g-4">
        <div class="col-lg-5">
          <div class="card mb-4">
            <div class="card-header fw-semibold">Text Channels</div>
            <div class="card-body">
              <form class="mb-3" @submit.prevent="createTextChannel">
                <div class="input-group input-group-sm mb-2">
                  <input
                    v-model.trim="newChannelName"
                    type="text"
                    class="form-control"
                    placeholder="channel-name"
                    required
                  />
                  <button type="submit" class="btn btn-primary">Create</button>
                </div>
                <input
                  v-model.trim="newChannelTopic"
                  type="text"
                  class="form-control form-control-sm"
                  placeholder="Optional topic"
                />
              </form>

              <ul class="list-group list-group-flush">
                <li
                  v-for="ch in textChannels"
                  :key="ch.id"
                  class="list-group-item d-flex align-items-center justify-content-between px-0"
                >
                  <div>
                    <div class="fw-semibold"># {{ ch.name }}</div>
                    <div v-if="ch.topic" class="small text-secondary">{{ ch.topic }}</div>
                  </div>
                  <button
                    type="button"
                    class="btn btn-outline-danger btn-sm"
                    @click="deleteTextChannel(ch.id)"
                  >
                    Delete
                  </button>
                </li>
              </ul>
            </div>
          </div>

          <div class="card">
            <div class="card-header fw-semibold">Voice Channels</div>
            <div class="card-body">
              <form class="input-group input-group-sm mb-3" @submit.prevent="createVoiceChannel">
                <input
                  v-model.trim="newVoiceName"
                  type="text"
                  class="form-control"
                  placeholder="voice-room-name"
                  required
                />
                <button type="submit" class="btn btn-primary">Create</button>
              </form>

              <ul class="list-group list-group-flush">
                <li
                  v-for="vc in voiceChannels"
                  :key="vc.id"
                  class="list-group-item d-flex align-items-center justify-content-between px-0"
                >
                  <span>{{ vc.name }}</span>
                  <button
                    type="button"
                    class="btn btn-outline-danger btn-sm"
                    @click="deleteVoiceChannel(vc.id)"
                  >
                    Delete
                  </button>
                </li>
              </ul>
            </div>
          </div>
        </div>

        <div class="col-lg-7">
          <div class="card">
            <div class="card-header fw-semibold">Users</div>
            <div class="table-responsive">
              <table class="table table-sm align-middle mb-0">
                <thead>
                  <tr>
                    <th>Username</th>
                    <th>Role</th>
                    <th>Status</th>
                    <th class="text-end">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="u in users" :key="u.id">
                    <td>{{ u.username }}</td>
                    <td>
                      <select
                        v-if="auth.isOwner && u.role !== 'owner' && u.id !== auth.user?.id"
                        class="form-select form-select-sm w-auto"
                        :value="u.role"
                        @change="onRoleChange(u.id, ($event.target as HTMLSelectElement).value as Role)"
                      >
                        <option value="member">member</option>
                        <option value="admin">admin</option>
                      </select>
                      <span v-else class="badge text-bg-secondary text-uppercase">{{ u.role }}</span>
                    </td>
                    <td>
                      <span
                        class="badge"
                        :class="u.disabled ? 'text-bg-danger' : 'text-bg-success'"
                      >
                        {{ u.disabled ? 'Disabled' : 'Active' }}
                      </span>
                    </td>
                    <td class="text-end">
                      <button
                        v-if="u.id !== auth.user?.id && u.role !== 'owner'"
                        type="button"
                        class="btn btn-sm"
                        :class="u.disabled ? 'btn-outline-success' : 'btn-outline-warning'"
                        @click="toggleDisabled(u)"
                      >
                        {{ u.disabled ? 'Enable' : 'Disable' }}
                      </button>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { ApiError, apiFetch } from '@/api/client';
import AppNavbar from '@/components/AppNavbar.vue';
import { useAuthStore } from '@/stores/auth';
import type { Role, TextChannel, User, VoiceChannel } from '@/types/api';

const auth = useAuthStore();

const textChannels = ref<TextChannel[]>([]);
const voiceChannels = ref<VoiceChannel[]>([]);
const users = ref<User[]>([]);
const newChannelName = ref('');
const newChannelTopic = ref('');
const newVoiceName = ref('');
const errorMessage = ref('');

async function refreshAll(): Promise<void> {
  errorMessage.value = '';
  try {
    const [textRes, voiceRes, usersRes] = await Promise.all([
      apiFetch<{ channels: TextChannel[] }>('/api/channels'),
      apiFetch<{ channels: VoiceChannel[] }>('/api/voice/channels'),
      apiFetch<{ users: User[] }>('/api/admin/users'),
    ]);
    textChannels.value = textRes.channels;
    voiceChannels.value = voiceRes.channels;
    users.value = usersRes.users;
  } catch (err) {
    errorMessage.value = err instanceof ApiError ? err.message : 'Failed to load admin data.';
  }
}

async function createTextChannel(): Promise<void> {
  errorMessage.value = '';
  try {
    await apiFetch<{ channel: TextChannel }>('/api/channels', {
      method: 'POST',
      body: JSON.stringify({ name: newChannelName.value, topic: newChannelTopic.value }),
    });
    newChannelName.value = '';
    newChannelTopic.value = '';
    await refreshAll();
  } catch (err) {
    errorMessage.value = err instanceof ApiError ? err.message : 'Failed to create channel.';
  }
}

async function deleteTextChannel(id: string): Promise<void> {
  errorMessage.value = '';
  try {
    await apiFetch<void>(`/api/channels/${id}`, { method: 'DELETE' });
    await refreshAll();
  } catch (err) {
    errorMessage.value = err instanceof ApiError ? err.message : 'Failed to delete channel.';
  }
}

async function createVoiceChannel(): Promise<void> {
  errorMessage.value = '';
  try {
    await apiFetch<{ channel: VoiceChannel }>('/api/voice/channels', {
      method: 'POST',
      body: JSON.stringify({ name: newVoiceName.value }),
    });
    newVoiceName.value = '';
    await refreshAll();
  } catch (err) {
    errorMessage.value = err instanceof ApiError ? err.message : 'Failed to create voice channel.';
  }
}

async function deleteVoiceChannel(id: string): Promise<void> {
  errorMessage.value = '';
  try {
    await apiFetch<void>(`/api/voice/channels/${id}`, { method: 'DELETE' });
    await refreshAll();
  } catch (err) {
    errorMessage.value = err instanceof ApiError ? err.message : 'Failed to delete voice channel.';
  }
}

async function onRoleChange(userId: string, role: Role): Promise<void> {
  errorMessage.value = '';
  try {
    await apiFetch<{ user: User }>(`/api/admin/users/${userId}/role`, {
      method: 'PATCH',
      body: JSON.stringify({ role }),
    });
    await refreshAll();
  } catch (err) {
    errorMessage.value = err instanceof ApiError ? err.message : 'Failed to update role.';
  }
}

async function toggleDisabled(target: User): Promise<void> {
  errorMessage.value = '';
  try {
    await apiFetch<{ user: User }>(`/api/admin/users/${target.id}/disabled`, {
      method: 'PATCH',
      body: JSON.stringify({ disabled: !target.disabled }),
    });
    await refreshAll();
  } catch (err) {
    errorMessage.value = err instanceof ApiError ? err.message : 'Failed to update user status.';
  }
}

onMounted(() => {
  void refreshAll();
});
</script>
