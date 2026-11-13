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

export interface TextChannel {
  id: string;
  name: string;
  topic: string;
  created_at: string;
}

export interface Message {
  id: string;
  channel_id: string;
  author_id: string;
  author_username: string;
  content: string;
  created_at: string;
}

export interface DmConversation {
  id: string;
  peer_id: string;
  peer_username: string;
  created_at: string;
}

export interface DmMessage {
  id: string;
  conversation_id: string;
  author_id: string;
  author_username: string;
  content: string;
  created_at: string;
}

export interface VoiceParticipant {
  user_id: string;
  muted: boolean;
  deafened: boolean;
}

export interface VoiceChannel {
  id: string;
  name: string;
  created_at: string;
  participants?: VoiceParticipant[];
}

export type ServerEvent =
  | { type: 'ready'; user_id: string; online_user_ids: string[] }
  | { type: 'message_created'; message: Message }
  | { type: 'dm_created'; message: DmMessage }
  | { type: 'channel_created'; channel: TextChannel }
  | { type: 'channel_deleted'; channel_id: string }
  | { type: 'presence_updated'; user_id: string; online: boolean }
  | {
      type: 'voice.participants';
      channel_id: string;
      user_ids: string[];
      participants?: VoiceParticipant[];
    }
  | { type: 'voice.offer'; sdp: string }
  | { type: 'voice.answer'; sdp: string }
  | { type: 'voice.ice'; candidate: RTCIceCandidateInit }
  | { type: 'pong' };

export type ClientCommand =
  | { type: 'ping' }
  | { type: 'voice.join'; channel_id: string; muted?: boolean; deafened?: boolean }
  | { type: 'voice.state'; muted: boolean; deafened: boolean }
  | { type: 'voice.leave' }
  | { type: 'voice.offer'; sdp: string }
  | { type: 'voice.answer'; sdp: string }
  | { type: 'voice.ice'; candidate: RTCIceCandidateInit };
