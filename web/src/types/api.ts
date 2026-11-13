export type Role = 'member' | 'admin' | 'owner';

export interface User {
  id: string;
  username: string;
  role: Role;
  disabled: boolean;
  created_at: string;
}

export interface AuthTokens {
  access_token: string;
  refresh_token: string;
  expires_in_secs: number;
}

export interface AuthResponse {
  user: User;
  tokens: AuthTokens;
}

export interface MeResponse {
  user: User;
}

export interface ApiErrorBody {
  error: string;
}
