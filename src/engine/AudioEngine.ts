import { Howl } from 'howler';

export class AudioEngine {
  private currentBgm: Howl | null = null;
  private currentBgmUrl: string | null = null;
  private currentVoice: Howl | null = null;
  private sfxCache: Map<string, Howl> = new Map();
  private masterVolume: number = 1;
  private bgmVolume: number = 1;
  private voiceVolume: number = 1;
  private sfxVolume: number = 1;
  private muted: boolean = false;

  // BGM 播放/切换/淡入淡出
  playBgm(url: string, fadeMs: number = 1000): void {
    if (this.currentBgmUrl === url && this.currentBgm?.playing()) {
      return; // 同一首 BGM，无需切换
    }

    const oldBgm = this.currentBgm;

    const effectiveBgmVolume = this.muted ? 0 : (this.masterVolume * this.bgmVolume);

    this.currentBgm = new Howl({
      src: [url],
      loop: true,
      volume: 0,
      onplay: () => {
        this.currentBgm?.fade(0, effectiveBgmVolume, fadeMs);
      },
    });

    this.currentBgmUrl = url;
    this.currentBgm.play();

    // 淡出旧 BGM
    if (oldBgm) {
      oldBgm.fade(oldBgm.volume(), 0, fadeMs);
      oldBgm.once('fade', () => {
        oldBgm.unload();
      });
    }
  }

  stopBgm(fadeMs: number = 1000): void {
    if (!this.currentBgm) return;

    const bgm = this.currentBgm;
    bgm.fade(bgm.volume(), 0, fadeMs);
    bgm.once('fade', () => {
      bgm.unload();
    });

    this.currentBgm = null;
    this.currentBgmUrl = null;
  }

  // 语音播放
  playVoice(url: string): Promise<void> {
    return new Promise((resolve) => {
      this.stopVoice();

      const effectiveVoiceVolume = this.muted ? 0 : (this.masterVolume * this.voiceVolume);

      this.currentVoice = new Howl({
        src: [url],
        volume: effectiveVoiceVolume,
        onend: () => {
          this.currentVoice = null;
          resolve();
        },
        onloaderror: () => {
          this.currentVoice = null;
          resolve();
        },
      });

      this.currentVoice.play();
    });
  }

  stopVoice(): void {
    if (this.currentVoice) {
      this.currentVoice.unload();
      this.currentVoice = null;
    }
  }

  // 音效播放
  playSfx(url: string): void {
    let sfx = this.sfxCache.get(url);
    const effectiveSfxVolume = this.muted ? 0 : (this.masterVolume * this.sfxVolume);
    if (!sfx) {
      sfx = new Howl({
        src: [url],
        volume: effectiveSfxVolume,
      });
      this.sfxCache.set(url, sfx);
    }
    sfx.volume(effectiveSfxVolume);
    sfx.play();
  }

  // 停止所有声音
  stopAll(): void {
    this.stopVoice();
    this.stopBgm(0);
  }

  // 主音量控制
  setMasterVolume(volume: number): void {
    this.masterVolume = volume;
    this.updateAllVolumes();
  }

  // BGM 音量控制
  setBgmVolume(volume: number): void {
    this.bgmVolume = volume;
    this.updateAllVolumes();
  }

  // 语音音量控制
  setVoiceVolume(volume: number): void {
    this.voiceVolume = volume;
    this.updateAllVolumes();
  }

  // 音效音量控制
  setSfxVolume(volume: number): void {
    this.sfxVolume = volume;
    this.updateAllVolumes();
  }

  // 静音
  setMuted(muted: boolean): void {
    this.muted = muted;
    this.updateAllVolumes();
  }

  // 获取当前音量设置
  getSettings() {
    return {
      masterVolume: this.masterVolume,
      bgmVolume: this.bgmVolume,
      voiceVolume: this.voiceVolume,
      sfxVolume: this.sfxVolume,
      muted: this.muted,
    };
  }

  // 播放测试声音（用于验证音频输出设备是否正常工作）
  playTestSound(type: 'voice' | 'bgm' | 'sfx' = 'voice'): void {
    const freq = type === 'bgm' ? 220 : type === 'sfx' ? 880 : 440;
    const duration = 0.8;
    try {
      const AudioCtx = (window.AudioContext || (window as any).webkitAudioContext);
      const ctx = new AudioCtx();
      const oscillator = ctx.createOscillator();
      const gainNode = ctx.createGain();
      oscillator.type = 'sine';
      oscillator.frequency.setValueAtTime(freq, ctx.currentTime);
      const effectiveVolume = this.muted
        ? 0
        : this.masterVolume * (type === 'bgm' ? this.bgmVolume : type === 'sfx' ? this.sfxVolume : this.voiceVolume);
      gainNode.gain.setValueAtTime(0.0001, ctx.currentTime);
      gainNode.gain.exponentialRampToValueAtTime(Math.max(effectiveVolume, 0.0001), ctx.currentTime + 0.05);
      gainNode.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + duration);
      oscillator.connect(gainNode).connect(ctx.destination);
      oscillator.start();
      oscillator.stop(ctx.currentTime + duration);
      oscillator.onended = () => ctx.close();
    } catch (err) {
      console.error('playTestSound failed', err);
    }
  }

  // 更新所有音量
  private updateAllVolumes(): void {
    const effectiveBgmVolume = this.muted ? 0 : (this.masterVolume * this.bgmVolume);
    const effectiveVoiceVolume = this.muted ? 0 : (this.masterVolume * this.voiceVolume);
    const effectiveSfxVolume = this.muted ? 0 : (this.masterVolume * this.sfxVolume);

    if (this.currentBgm) {
      this.currentBgm.volume(effectiveBgmVolume);
    }
    if (this.currentVoice) {
      this.currentVoice.volume(effectiveVoiceVolume);
    }
    this.sfxCache.forEach(sfx => sfx.volume(effectiveSfxVolume));
  }
}
