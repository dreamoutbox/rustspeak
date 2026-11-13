import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import { apiFetch } from '@/api/client';
import { useRealtimeStore } from '@/stores/realtime';
import type { ServerEvent, VoiceChannel, VoiceParticipant } from '@/types/api';
import { SoundEffect, playSound } from '@/utils/sound';

const SPEAKING_RMS_THRESHOLD = 0.02;
const SPEAKING_POLL_MS = 80;

export const useVoiceStore = defineStore('voice', () => {
  const voiceChannels = ref<VoiceChannel[]>([]);
  const iceServers = ref<RTCIceServer[]>([]);
  const participantsByChannel = ref<Record<string, VoiceParticipant[]>>({});
  const activeVoiceChannelId = ref<string | null>(null);
  const connectionState = ref<'idle' | 'connecting' | 'connected' | 'failed'>('idle');
  const muted = ref(false);
  const deafened = ref(false);
  const hasMicrophone = ref(true);
  const isSpeaking = ref(false);
  const errorMessage = ref('');
  const remoteStreams = ref<MediaStream[]>([]);

  const realtime = useRealtimeStore();

  let pc: RTCPeerConnection | null = null;
  let localStream: MediaStream | null = null;
  let pendingIceCandidates: RTCIceCandidateInit[] = [];
  let audioCtx: AudioContext | null = null;
  let speakingTimer: number | null = null;
  let wiredRealtime = false;

  const activeVoiceChannel = computed(
    () => voiceChannels.value.find((vc) => vc.id === activeVoiceChannelId.value) ?? null,
  );

  function stopSpeakingDetector(): void {
    if (speakingTimer !== null) {
      window.clearInterval(speakingTimer);
      speakingTimer = null;
    }
    if (audioCtx) {
      void audioCtx.close();
      audioCtx = null;
    }
    isSpeaking.value = false;
  }

  function startSpeakingDetector(stream: MediaStream): void {
    stopSpeakingDetector();
    const AudioContextCtor = window.AudioContext;
    if (!AudioContextCtor) {
      return;
    }
    try {
      audioCtx = new AudioContextCtor();
      const source = audioCtx.createMediaStreamSource(stream);
      const analyser = audioCtx.createAnalyser();
      analyser.fftSize = 256;
      source.connect(analyser);
      const samples = new Float32Array(analyser.fftSize);

      speakingTimer = window.setInterval(() => {
        if (muted.value || deafened.value) {
          isSpeaking.value = false;
          return;
        }
        analyser.getFloatTimeDomainData(samples);
        let sumSq = 0;
        for (let i = 0; i < samples.length; i += 1) {
          const s = samples[i];
          sumSq += s * s;
        }
        const rms = Math.sqrt(sumSq / samples.length);
        isSpeaking.value = rms > SPEAKING_RMS_THRESHOLD;
      }, SPEAKING_POLL_MS);
    } catch {
      // Ignore Web Audio API errors on restricted browsers.
    }
  }

  async function flushPendingIce(): Promise<void> {
    if (!pc || !pc.remoteDescription) {
      return;
    }
    const queued = pendingIceCandidates;
    pendingIceCandidates = [];
    for (const cand of queued) {
      try {
        await pc.addIceCandidate(cand);
      } catch {
        // Ignore stale ICE candidates during renegotiation.
      }
    }
  }

  async function handleServerEvent(event: ServerEvent): Promise<void> {
    if (event.type === 'voice.participants') {
      const list: VoiceParticipant[] =
        event.participants ??
        event.user_ids.map((user_id) => ({ user_id, muted: false, deafened: false }));
      participantsByChannel.value = {
        ...participantsByChannel.value,
        [event.channel_id]: list,
      };
      return;
    }

    if (!pc) {
      return;
    }

    if (event.type === 'voice.answer') {
      try {
        await pc.setRemoteDescription({ type: 'answer', sdp: event.sdp });
        await flushPendingIce();
      } catch {
        errorMessage.value = 'Failed to apply voice answer from server.';
        connectionState.value = 'failed';
      }
    } else if (event.type === 'voice.offer') {
      try {
        await pc.setRemoteDescription({ type: 'offer', sdp: event.sdp });
        await flushPendingIce();
        const answer = await pc.createAnswer();
        await pc.setLocalDescription(answer);
        if (answer.sdp) {
          realtime.sendCommand({ type: 'voice.answer', sdp: answer.sdp });
        }
      } catch {
        errorMessage.value = 'Failed to negotiate voice renegotiation.';
        connectionState.value = 'failed';
      }
    } else if (event.type === 'voice.ice') {
      if (!pc.remoteDescription) {
        pendingIceCandidates.push(event.candidate);
      } else {
        try {
          await pc.addIceCandidate(event.candidate);
        } catch {
          // Ignore invalid/late ICE candidates.
        }
      }
    }
  }

  function initRealtimeSync(): void {
    if (wiredRealtime) {
      return;
    }
    wiredRealtime = true;
    realtime.onEvent((event) => {
      void handleServerEvent(event);
    });
  }

  async function loadVoiceChannels(): Promise<void> {
    initRealtimeSync();
    const [channelsRes, configRes] = await Promise.all([
      apiFetch<{ channels: VoiceChannel[] }>('/api/voice/channels'),
      apiFetch<{ ice_servers: RTCIceServer[] }>('/api/voice/config'),
    ]);
    voiceChannels.value = channelsRes.channels;
    const initialParticipants: Record<string, VoiceParticipant[]> = {};
    for (const ch of channelsRes.channels) {
      if (ch.participants) {
        initialParticipants[ch.id] = ch.participants;
      }
    }
    participantsByChannel.value = initialParticipants;
    iceServers.value = configRes.ice_servers;
  }

  function cleanupPeer(): void {
    stopSpeakingDetector();
    pendingIceCandidates = [];
    if (pc) {
      pc.onicecandidate = null;
      pc.ontrack = null;
      pc.oniceconnectionstatechange = null;
      pc.onconnectionstatechange = null;
      pc.close();
      pc = null;
    }
    if (localStream) {
      for (const track of localStream.getTracks()) {
        track.stop();
      }
      localStream = null;
    }
    remoteStreams.value = [];
    connectionState.value = 'idle';
  }

  function formatMicError(err: unknown): string {
    if (err instanceof DOMException) {
      if (err.name === 'NotAllowedError' || err.name === 'PermissionDeniedError') {
        return 'Microphone permission denied. Allow microphone access in your browser settings.';
      }
      if (err.name === 'NotFoundError' || err.name === 'DevicesNotFoundError') {
        return 'No microphone device found.';
      }
      if (err.name === 'NotReadableError') {
        return 'Microphone is already in use by another application.';
      }
    }
    return 'Unable to access microphone.';
  }

  async function joinVoice(channelId: string): Promise<void> {
    initRealtimeSync();
    errorMessage.value = '';

    if (!realtime.isConnected) {
      errorMessage.value = 'WebSocket is disconnected. Wait for reconnection and try again.';
      return;
    }

    if (activeVoiceChannelId.value) {
      await leaveVoice();
    }

    let stream: MediaStream | null = null;
    if (navigator.mediaDevices?.getUserMedia) {
      try {
        stream = await navigator.mediaDevices.getUserMedia({ audio: true, video: false });
        hasMicrophone.value = true;
      } catch (err) {
        if (
          err instanceof DOMException &&
          (err.name === 'NotFoundError' || err.name === 'DevicesNotFoundError')
        ) {
          // Allow listen-only join when no microphone hardware is attached.
          hasMicrophone.value = false;
          muted.value = true;
        } else {
          errorMessage.value = formatMicError(err);
          return;
        }
      }
    } else {
      hasMicrophone.value = false;
      muted.value = true;
    }

    const peer = new RTCPeerConnection({ iceServers: iceServers.value });
    pc = peer;
    connectionState.value = 'connecting';
    activeVoiceChannelId.value = channelId;

    if (stream) {
      localStream = stream;
      for (const track of stream.getAudioTracks()) {
        track.enabled = !muted.value && !deafened.value;
        peer.addTrack(track, stream);
      }
      startSpeakingDetector(stream);
    } else {
      peer.addTransceiver('audio', { direction: 'recvonly' });
    }

    peer.onicecandidate = (ev) => {
      if (ev.candidate) {
        realtime.sendCommand({ type: 'voice.ice', candidate: ev.candidate.toJSON() });
      }
    };

    peer.ontrack = (ev) => {
      const incomingStream = ev.streams[0] ?? new MediaStream([ev.track]);
      if (!remoteStreams.value.some((s) => s.id === incomingStream.id)) {
        remoteStreams.value = [...remoteStreams.value, incomingStream];
      }
      ev.track.onended = () => {
        remoteStreams.value = remoteStreams.value.filter((s) => s.id !== incomingStream.id);
      };
    };

    peer.oniceconnectionstatechange = () => {
      const state = peer.iceConnectionState;
      if (state === 'connected' || state === 'completed') {
        connectionState.value = 'connected';
      } else if (state === 'failed') {
        connectionState.value = 'failed';
        errorMessage.value = 'Voice ICE connection failed. Check UDP firewall/NAT settings.';
      } else if (state === 'disconnected') {
        connectionState.value = 'connecting';
      }
    };

    realtime.sendCommand({
      type: 'voice.join',
      channel_id: channelId,
      muted: muted.value,
      deafened: deafened.value,
    });
    playSound(SoundEffect.VoiceJoin);

    try {
      const offer = await peer.createOffer();
      await peer.setLocalDescription(offer);
      if (offer.sdp) {
        realtime.sendCommand({ type: 'voice.offer', sdp: offer.sdp });
      }
    } catch {
      errorMessage.value = 'Failed to create WebRTC voice offer.';
      connectionState.value = 'failed';
    }
  }

  async function leaveVoice(): Promise<void> {
    if (activeVoiceChannelId.value) {
      realtime.sendCommand({ type: 'voice.leave' });
      playSound(SoundEffect.VoiceLeave);
    }
    activeVoiceChannelId.value = null;
    cleanupPeer();
  }

  function applyTrackMuteState(): void {
    if (!localStream) {
      return;
    }
    const enabled = !muted.value && !deafened.value;
    for (const track of localStream.getAudioTracks()) {
      track.enabled = enabled;
    }
    if (!enabled) {
      isSpeaking.value = false;
    }
  }

  function syncVoiceState(): void {
    if (activeVoiceChannelId.value) {
      realtime.sendCommand({
        type: 'voice.state',
        muted: muted.value,
        deafened: deafened.value,
      });
    }
  }

  function toggleMute(): void {
    muted.value = !muted.value;
    applyTrackMuteState();
    syncVoiceState();
  }

  function toggleDeafen(): void {
    deafened.value = !deafened.value;
    applyTrackMuteState();
    syncVoiceState();
  }

  function reset(): void {
    cleanupPeer();
    activeVoiceChannelId.value = null;
    voiceChannels.value = [];
    participantsByChannel.value = {};
    errorMessage.value = '';
  }

  return {
    voiceChannels,
    participantsByChannel,
    activeVoiceChannelId,
    activeVoiceChannel,
    connectionState,
    muted,
    deafened,
    hasMicrophone,
    isSpeaking,
    errorMessage,
    remoteStreams,
    loadVoiceChannels,
    joinVoice,
    leaveVoice,
    toggleMute,
    toggleDeafen,
    reset,
  };
});

