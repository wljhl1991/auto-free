import { useEffect, useRef, useState } from 'react';
import styles from './CGPlayer.module.css';

interface CGPlayerProps {
  videoUrl: string;
  duration?: number;
  skipAllowed: boolean;
  onComplete: () => void;
  onSkip: () => void;
}

function CGPlayer({
  videoUrl,
  duration,
  skipAllowed: _skipAllowed,
  onComplete,
  onSkip,
}: CGPlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const [progress, setProgress] = useState(0);
  const [_isPlaying, setIsPlaying] = useState(false);
  const completedRef = useRef(false);

  // 没有视频 URL 或视频无法播放时，3 秒后自动完成
  useEffect(() => {
    if (!videoUrl) {
      const timer = setTimeout(() => {
        if (!completedRef.current) {
          completedRef.current = true;
          onComplete();
        }
      }, 3000);
      return () => clearTimeout(timer);
    }
  }, [videoUrl, onComplete]);

  useEffect(() => {
    const video = videoRef.current;
    if (!video || !videoUrl) return;

    const handleTimeUpdate = () => {
      if (video.duration > 0) {
        setProgress((video.currentTime / video.duration) * 100);
      }
    };

    const handlePlay = () => setIsPlaying(true);
    const handlePause = () => setIsPlaying(false);
    const handleEnded = () => {
      setIsPlaying(false);
      setProgress(100);
      if (!completedRef.current) {
        completedRef.current = true;
        onComplete();
      }
    };

    const handleError = () => {
      // 视频加载失败，3 秒后自动完成
      console.warn('CG video failed to load, auto-completing in 3s');
      setTimeout(() => {
        if (!completedRef.current) {
          completedRef.current = true;
          onComplete();
        }
      }, 3000);
    };

    video.addEventListener('timeupdate', handleTimeUpdate);
    video.addEventListener('play', handlePlay);
    video.addEventListener('pause', handlePause);
    video.addEventListener('ended', handleEnded);
    video.addEventListener('error', handleError);

    return () => {
      video.removeEventListener('timeupdate', handleTimeUpdate);
      video.removeEventListener('play', handlePlay);
      video.removeEventListener('pause', handlePause);
      video.removeEventListener('ended', handleEnded);
      video.removeEventListener('error', handleError);
    };
  }, [videoUrl, onComplete]);

  // Auto-complete after specified duration
  useEffect(() => {
    if (duration && duration > 0) {
      const timer = setTimeout(() => {
        if (!completedRef.current) {
          completedRef.current = true;
          onComplete();
        }
      }, duration * 1000);
      return () => clearTimeout(timer);
    }
  }, [duration, onComplete]);

  const handleProgressClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const video = videoRef.current;
    if (!video || !video.duration) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const ratio = (e.clientX - rect.left) / rect.width;
    video.currentTime = ratio * video.duration;
  };

  const handleSkip = () => {
    if (!completedRef.current) {
      completedRef.current = true;
      onSkip();
    }
  };

  return (
    <div className={styles.container}>
      {videoUrl ? (
        <video
          ref={videoRef}
          className={styles.video}
          src={videoUrl}
          autoPlay
          playsInline
        />
      ) : (
        <div className={styles.placeholder}>
          <div className={styles.placeholderIcon}>🎬</div>
          <div className={styles.placeholderText}>CG 动画</div>
        </div>
      )}

      <div className={styles.controls}>
        <div
          className={styles.progressBar}
          onClick={handleProgressClick}
        >
          <div
            className={styles.progressFill}
            style={{ width: `${progress}%` }}
          />
        </div>

        <button className={styles.skipBtn} onClick={handleSkip}>
          跳过
        </button>
      </div>
    </div>
  );
}

export default CGPlayer;
