import type { AuthTokens } from '@/types/api';

const ACCESS_TOKEN_KEY = 'rustspeak_access_token';
const REFRESH_TOKEN_KEY = 'rustspeak_refresh_token';

export class ApiError extends Error {
  status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
  }
}

export function getStoredAccessToken(): string | null {
  return localStorage.getItem(ACCESS_TOKEN_KEY);
}

export function getStoredRefreshToken(): string | null {
  return localStorage.getItem(REFRESH_TOKEN_KEY);
}

export function storeTokens(tokens: AuthTokens): void {
  localStorage.setItem(ACCESS_TOKEN_KEY, tokens.access_token);
  localStorage.setItem(REFRESH_TOKEN_KEY, tokens.refresh_token);
}

export function clearStoredTokens(): void {
  localStorage.removeItem(ACCESS_TOKEN_KEY);
  localStorage.removeItem(REFRESH_TOKEN_KEY);
}

let refreshInFlight: Promise<string | null> | null = null;
let onUnauthorizedHandler: (() => void) | null = null;

export function setUnauthorizedHandler(handler: () => void): void {
  onUnauthorizedHandler = handler;
}

async function refreshAccessToken(): Promise<string | null> {
  if (refreshInFlight) {
    return refreshInFlight;
  }

  const refreshToken = getStoredRefreshToken();
  if (!refreshToken) {
    return null;
  }

  refreshInFlight = (async () => {
    try {
      const response = await fetch('/api/auth/refresh', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ refresh_token: refreshToken }),
      });
      if (!response.ok) {
        clearStoredTokens();
        return null;
      }
      const tokens = (await response.json()) as AuthTokens;
      storeTokens(tokens);
      return tokens.access_token;
    } catch {
      clearStoredTokens();
      return null;
    } finally {
      refreshInFlight = null;
    }
  })();

  return refreshInFlight;
}

async function parseErrorMessage(response: Response): Promise<string> {
  try {
    const data = (await response.json()) as { error?: string };
    if (data.error) {
      return data.error;
    }
  } catch {
    // Ignore non-JSON error payloads.
  }
  return response.statusText || `HTTP ${response.status}`;
}

export interface ApiRequestOptions extends RequestInit {
  skipAuth?: boolean;
}

export async function apiFetch<T>(
  path: string,
  options: ApiRequestOptions = {},
): Promise<T> {
  const { skipAuth = false, headers: rawHeaders, ...rest } = options;

  const buildHeaders = (token: string | null): Headers => {
    const headers = new Headers(rawHeaders);
    if (!headers.has('Content-Type') && rest.body !== undefined) {
      headers.set('Content-Type', 'application/json');
    }
    if (!skipAuth && token) {
      headers.set('Authorization', `Bearer ${token}`);
    }
    return headers;
  };

  let response = await fetch(path, {
    ...rest,
    headers: buildHeaders(getStoredAccessToken()),
  });

  if (response.status === 401 && !skipAuth) {
    const newAccessToken = await refreshAccessToken();
    if (newAccessToken) {
      response = await fetch(path, {
        ...rest,
        headers: buildHeaders(newAccessToken),
      });
    } else {
      onUnauthorizedHandler?.();
      throw new ApiError(401, 'Session expired. Please sign in again.');
    }
  }

  if (!response.ok) {
    const message = await parseErrorMessage(response);
    throw new ApiError(response.status, message);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}

