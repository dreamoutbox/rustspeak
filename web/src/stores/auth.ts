import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import {
  apiFetch,
  clearStoredTokens,
  getStoredAccessToken,
  getStoredRefreshToken,
  storeTokens,
} from '@/api/client';
import type { AuthResponse, MeResponse, User } from '@/types/api';

export const useAuthStore = defineStore('auth', () => {
  const user = ref<User | null>(null);
  const initialized = ref(false);
  let initPromise: Promise<void> | null = null;

  const isAuthenticated = computed(() => user.value !== null);
  const isAdmin = computed(
    () => user.value?.role === 'admin' || user.value?.role === 'owner',
  );
  const isOwner = computed(() => user.value?.role === 'owner');

  function clearSession(): void {
    user.value = null;
    clearStoredTokens();
  }

  async function fetchMe(): Promise<User> {
    const data = await apiFetch<MeResponse>('/api/users/me');
    user.value = data.user;
    return data.user;
  }

  async function ensureInitialized(): Promise<void> {
    if (initialized.value) {
      return;
    }
    if (initPromise) {
      return initPromise;
    }

    initPromise = (async () => {
      const hasToken = Boolean(getStoredAccessToken() || getStoredRefreshToken());
      if (hasToken) {
        try {
          await fetchMe();
        } catch {
          clearSession();
        }
      }
      initialized.value = true;
      initPromise = null;
    })();

    return initPromise;
  }

  async function login(username: string, password: string): Promise<void> {
    const data = await apiFetch<AuthResponse>('/api/auth/login', {
      method: 'POST',
      skipAuth: true,
      body: JSON.stringify({ username, password }),
    });
    storeTokens(data.tokens);
    user.value = data.user;
    initialized.value = true;
  }

  async function register(username: string, password: string): Promise<void> {
    const data = await apiFetch<AuthResponse>('/api/auth/register', {
      method: 'POST',
      skipAuth: true,
      body: JSON.stringify({ username, password }),
    });
    storeTokens(data.tokens);
    user.value = data.user;
    initialized.value = true;
  }

  async function logout(): Promise<void> {
    const refreshToken = getStoredRefreshToken();
    if (refreshToken) {
      try {
        await apiFetch<void>('/api/auth/logout', {
          method: 'POST',
          skipAuth: true,
          body: JSON.stringify({ refresh_token: refreshToken }),
        });
      } catch {
        // Always clear local session even if server logout fails.
      }
    }
    clearSession();
  }

  return {
    user,
    initialized,
    isAuthenticated,
    isAdmin,
    isOwner,
    clearSession,
    fetchMe,
    ensureInitialized,
    login,
    register,
    logout,
  };
});

