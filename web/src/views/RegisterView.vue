<template>
  <div class="min-vh-100 d-flex align-items-center justify-content-center bg-body-tertiary p-3">
    <div class="card shadow-sm" style="width: 100%; max-width: 400px">
      <div class="card-body p-4">
        <h1 class="h4 mb-1">RustSpeak</h1>
        <p class="text-secondary small mb-4">Create your account</p>

        <div v-if="errorMessage" class="alert alert-danger py-2 small" role="alert">
          {{ errorMessage }}
        </div>

        <form @submit.prevent="handleSubmit">
          <div class="mb-3">
            <label for="register-username" class="form-label">Username</label>
            <input
              id="register-username"
              v-model.trim="username"
              type="text"
              class="form-control"
              autocomplete="username"
              minlength="3"
              maxlength="32"
              required
              :disabled="submitting"
            />
          </div>

          <div class="mb-3">
            <label for="register-password" class="form-label">Password</label>
            <input
              id="register-password"
              v-model="password"
              type="password"
              class="form-control"
              autocomplete="new-password"
              minlength="8"
              required
              :disabled="submitting"
            />
            <div class="form-text">At least 8 characters.</div>
          </div>

          <button type="submit" class="btn btn-primary w-100" :disabled="submitting">
            {{ submitting ? 'Creating account...' : 'Register' }}
          </button>
        </form>

        <hr class="my-4" />
        <p class="text-center text-secondary small mb-0">
          Already have an account?
          <RouterLink :to="{ name: 'login' }">Sign in</RouterLink>
        </p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { RouterLink, useRouter } from 'vue-router';
import { ApiError } from '@/api/client';
import { useAuthStore } from '@/stores/auth';

const auth = useAuthStore();
const router = useRouter();

const username = ref('');
const password = ref('');
const submitting = ref(false);
const errorMessage = ref('');

async function handleSubmit(): Promise<void> {
  errorMessage.value = '';
  submitting.value = true;
  try {
    await auth.register(username.value, password.value);
    await router.push({ name: 'app' });
  } catch (err) {
    errorMessage.value = err instanceof ApiError ? err.message : 'Unable to register.';
  } finally {
    submitting.value = false;
  }
}
</script>
