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
  skipAllowed,
  onComplete,
  onSkip,
}: CGPlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const [progress, setProgress] = useState(0);
  const [isPlaying, setIsPlaying] = useState(false);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;

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
      onComplete();
    };

    video.addEventListener('timeupdate', handleTimeUpdate);
    video.addEventListener('play', handlePlay);
    video.addEventListener('pause', handlePause);
    video.addEventListener('ended', handleEnded);

    return () => {
      video.removeEventListener('timeupdate', handleTimeUpdate);
      video.removeEventListener('play', handlePlay);
      video.removeEventListener('pause', handlePause);
      video.removeEventListener('ended', handleEnded);
    };
  }, [onComplete]);

  // Auto-complete after specified duration
  useEffect(() => {
    if (duration && duration > 0) {
      const timer = setTimeout(() => {
        onComplete();
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

  return (
    <div className={styles.container}>
      <video
        ref={videoRef}
        className={styles.video}
        src={videoUrl}
        autoPlay
        playsInline
      />

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

        {skipAllowed && isPlaying && (
          <button className={styles.skipBtn} onClick={onSkip}>
            跳过
          </button>
        )}
      </div>
    </div>
  );
}

export default CGPlayer;
