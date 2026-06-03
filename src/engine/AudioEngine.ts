import { Howl } from 'howler';

export class AudioEngine {
  private currentBgm: Howl | null = null;
  private currentBgmUrl: string | null = null;
  private currentVoice: Howl | null = null;
  private sfxCache: Map<string, Howl> = new Map();
  private volume: number = 1;
  private muted: boolean = false;

  // BGM 播放/切换/淡入淡出
  playBgm(url: string, fadeMs: number = 1000): void {
    if (this.currentBgmUrl === url && this.currentBgm?.playing()) {
      return; // 同一首 BGM，无需切换
    }

    const oldBgm = this.currentBgm;

    this.currentBgm = new Howl({
      src: [url],
      loop: true,
      volume: 0,
      onplay: () => {
        this.currentBgm?.fade(0, this.muted ? 0 : this.volume, fadeMs);
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

      this.currentVoice = new Howl({
        src: [url],
        volume: this.muted ? 0 : this.volume,
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
    if (!sfx) {
      sfx = new Howl({
        src: [url],
        volume: this.muted ? 0 : this.volume,
      });
      this.sfxCache.set(url, sfx);
    }
    sfx.play();
  }

  // 全局音量
  setVolume(volume: number): void {
    this.volume = volume;
    if (!this.muted) {
      if (this.currentBgm) {
        this.currentBgm.volume(volume);
      }
      if (this.currentVoice) {
        this.currentVoice.volume(volume);
      }
      this.sfxCache.forEach(sfx => sfx.volume(volume));
    }
  }

  // 静音
  setMuted(muted: boolean): void {
    this.muted = muted;
    const vol = muted ? 0 : this.volume;
    if (this.currentBgm) {
      this.currentBgm.volume(vol);
    }
    if (this.currentVoice) {
      this.currentVoice.volume(vol);
    }
    this.sfxCache.forEach(sfx => sfx.volume(vol));
  }
}
