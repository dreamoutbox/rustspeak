import chatNewUrl from '@/sounds/chat_new.wav';
import voiceJoinUrl from '@/sounds/voice_join.mp3';
import voiceLeaveUrl from '@/sounds/voice_leave.mp3';

export const SoundEffect = {
  VoiceJoin: voiceJoinUrl,
  VoiceLeave: voiceLeaveUrl,
  ChatNew: chatNewUrl,
} as const;

export type SoundEffectUrl = (typeof SoundEffect)[keyof typeof SoundEffect];

export function playSound(url: SoundEffectUrl): void {
  const audio = new Audio(url);
  audio.volume = 0.6;
  void audio.play().catch(() => {
    // Ignore browser autoplay blocks before user interaction.
  });
}

