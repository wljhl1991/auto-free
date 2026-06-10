import { useState, useEffect, useCallback } from 'react';
import styles from './AudioControl.module.css';

interface AudioControlProps {
  audioEngine: any;
  onClose: () => void;
}

function AudioControl({ audioEngine, onClose }: AudioControlProps) {
  const [settings, setSettings] = useState({
    masterVolume: 1,
    bgmVolume: 1,
    voiceVolume: 1,
    sfxVolume: 1,
    muted: false,
  });

  // 初始化时获取当前设置
  useEffect(() => {
    const currentSettings = audioEngine.getSettings();
    setSettings(currentSettings);
  }, [audioEngine]);

  const handleMasterVolumeChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const volume = parseFloat(e.target.value);
      audioEngine.setMasterVolume(volume);
      setSettings((prev) => ({ ...prev, masterVolume: volume }));
    },
    [audioEngine]
  );

  const handleBgmVolumeChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const volume = parseFloat(e.target.value);
      audioEngine.setBgmVolume(volume);
      setSettings((prev) => ({ ...prev, bgmVolume: volume }));
    },
    [audioEngine]
  );

  const handleVoiceVolumeChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const volume = parseFloat(e.target.value);
      audioEngine.setVoiceVolume(volume);
      setSettings((prev) => ({ ...prev, voiceVolume: volume }));
    },
    [audioEngine]
  );

  const handleSfxVolumeChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const volume = parseFloat(e.target.value);
      audioEngine.setSfxVolume(volume);
      setSettings((prev) => ({ ...prev, sfxVolume: volume }));
    },
    [audioEngine]
  );

  const handleToggleMute = useCallback(() => {
    const newMuted = !settings.muted;
    audioEngine.setMuted(newMuted);
    setSettings((prev) => ({ ...prev, muted: newMuted }));
  }, [audioEngine, settings.muted]);

  return (
    <div className={styles.overlay} onClick={onClose}>
      <div className={styles.panel} onClick={(e) => e.stopPropagation()}>
        <div className={styles.header}>
          <h3>声音设置</h3>
          <button className={styles.closeBtn} onClick={onClose}>
            ✕
          </button>
        </div>
        <div className={styles.body}>
          {/* 静音按钮 */}
          <div className={styles.muteSection}>
            <button
              className={`${styles.muteBtn} ${settings.muted ? styles.muted : ''}`}
              onClick={handleToggleMute}
            >
              {settings.muted ? '🔇 已静音' : '🔊 播放中'}
            </button>
          </div>

          {/* 音量控制 */}
          <div className={styles.volumeSection}>
            <div className={styles.volumeControl}>
              <div className={styles.volumeLabel}>
                <span>主音量</span>
                <span>{Math.round(settings.masterVolume * 100)}%</span>
              </div>
              <input
                type="range"
                min="0"
                max="1"
                step="0.01"
                value={settings.masterVolume}
                onChange={handleMasterVolumeChange}
                className={styles.volumeSlider}
              />
            </div>

            <div className={styles.volumeControl}>
              <div className={styles.volumeLabel}>
                <span>背景音乐</span>
                <span>{Math.round(settings.bgmVolume * 100)}%</span>
              </div>
              <input
                type="range"
                min="0"
                max="1"
                step="0.01"
                value={settings.bgmVolume}
                onChange={handleBgmVolumeChange}
                className={styles.volumeSlider}
              />
            </div>

            <div className={styles.volumeControl}>
              <div className={styles.volumeLabel}>
                <span>语音</span>
                <span>{Math.round(settings.voiceVolume * 100)}%</span>
              </div>
              <input
                type="range"
                min="0"
                max="1"
                step="0.01"
                value={settings.voiceVolume}
                onChange={handleVoiceVolumeChange}
                className={styles.volumeSlider}
              />
            </div>

            <div className={styles.volumeControl}>
              <div className={styles.volumeLabel}>
                <span>音效</span>
                <span>{Math.round(settings.sfxVolume * 100)}%</span>
              </div>
              <input
                type="range"
                min="0"
                max="1"
                step="0.01"
                value={settings.sfxVolume}
                onChange={handleSfxVolumeChange}
                className={styles.volumeSlider}
              />
            </div>
          </div>

          {/* 测试声音 */}
          <div className={styles.testSection}>
            <div className={styles.testLabel}>测试声音</div>
            <div className={styles.testButtons}>
              <button
                className={styles.testBtn}
                onClick={() => audioEngine.playTestSound('voice')}
              >
                测试语音
              </button>
              <button
                className={styles.testBtn}
                onClick={() => audioEngine.playTestSound('bgm')}
              >
                测试 BGM
              </button>
              <button
                className={styles.testBtn}
                onClick={() => audioEngine.playTestSound('sfx')}
              >
                测试音效
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default AudioControl;
