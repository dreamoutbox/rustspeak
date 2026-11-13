import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import { getStoredAccessToken } from '@/api/client';
import type { ClientCommand, ServerEvent } from '@/types/api';

type EventListener = (event: ServerEvent) => void;
type ReconnectListener = () => void;

const HEARTBEAT_INTERVAL_MS = 25_000;
const BASE_RECONNECT_DELAY_MS = 1_000;
const MAX_RECONNECT_DELAY_MS = 15_000;

export const useRealtimeStore = defineStore('realtime', () => {
  const status = ref<'disconnected' | 'connecting' | 'connected'>('disconnected');
  const onlineUserIds = ref<Set<string>>(new Set());

  let socket: WebSocket | null = null;
  let shouldReconnect = false;
  let reconnectAttempts = 0;
  let reconnectTimer: number | null = null;
  let heartbeatTimer: number | null = null;
  let hasConnectedOnce = false;

  const eventListeners = new Set<EventListener>();
  const reconnectListeners = new Set<ReconnectListener>();

  const isConnected = computed(() => status.value === 'connected');

  function isUserOnline(userId: string): boolean {
    return onlineUserIds.value.has(userId);
  }

  function onEvent(listener: EventListener): () => void {
    eventListeners.add(listener);
    return () => eventListeners.delete(listener);
  }

  function onReconnect(listener: ReconnectListener): () => void {
    reconnectListeners.add(listener);
    return () => reconnectListeners.delete(listener);
  }

  function sendCommand(command: ClientCommand): boolean {
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      return false;
    }
    socket.send(JSON.stringify(command));
    return true;
  }

  function clearTimers(): void {
    if (reconnectTimer !== null) {
      window.clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
    if (heartbeatTimer !== null) {
      window.clearInterval(heartbeatTimer);
      heartbeatTimer = null;
    }
  }

  function startHeartbeat(): void {
    if (heartbeatTimer !== null) {
      window.clearInterval(heartbeatTimer);
    }
    heartbeatTimer = window.setInterval(() => {
      sendCommand({ type: 'ping' });
    }, HEARTBEAT_INTERVAL_MS);
  }

  function scheduleReconnect(): void {
    if (!shouldReconnect || reconnectTimer !== null) {
      return;
    }
    const delay = Math.min(
      BASE_RECONNECT_DELAY_MS * 2 ** reconnectAttempts,
      MAX_RECONNECT_DELAY_MS,
    );
    reconnectAttempts += 1;
    reconnectTimer = window.setTimeout(() => {
      reconnectTimer = null;
      openSocket();
    }, delay);
  }

  function openSocket(): void {
    const token = getStoredAccessToken();
    if (!token || !shouldReconnect) {
      status.value = 'disconnected';
      return;
    }
    if (socket && (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING)) {
      return;
    }

    status.value = 'connecting';
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const url = `${protocol}//${window.location.host}/ws?token=${encodeURIComponent(token)}`;
    const ws = new WebSocket(url);
    socket = ws;

    ws.onopen = () => {
      status.value = 'connected';
      reconnectAttempts = 0;
      startHeartbeat();
      if (hasConnectedOnce) {
        for (const listener of reconnectListeners) {
          listener();
        }
      }
      hasConnectedOnce = true;
    };

    ws.onmessage = (msgEvent) => {
      try {
        const event = JSON.parse(String(msgEvent.data)) as ServerEvent;
        if (event.type === 'ready') {
          onlineUserIds.value = new Set(event.online_user_ids);
        } else if (event.type === 'presence_updated') {
          const next = new Set(onlineUserIds.value);
          if (event.online) {
            next.add(event.user_id);
          } else {
            next.delete(event.user_id);
          }
          onlineUserIds.value = next;
        }
        for (const listener of eventListeners) {
          listener(event);
        }
      } catch {
        // Ignore malformed server frames.
      }
    };

    ws.onclose = () => {
      if (socket === ws) {
        socket = null;
      }
      clearTimers();
      status.value = 'disconnected';
      scheduleReconnect();
    };

    ws.onerror = () => {
      ws.close();
    };
  }

  function connect(): void {
    shouldReconnect = true;
    openSocket();
  }

  function disconnect(): void {
    shouldReconnect = false;
    hasConnectedOnce = false;
    clearTimers();
    if (socket) {
      socket.close();
      socket = null;
    }
    onlineUserIds.value = new Set();
    status.value = 'disconnected';
  }

  return {
    status,
    onlineUserIds,
    isConnected,
    isUserOnline,
    onEvent,
    onReconnect,
    sendCommand,
    connect,
    disconnect,
  };
});

