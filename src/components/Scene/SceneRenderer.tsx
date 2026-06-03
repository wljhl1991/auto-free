import { useEffect, useState, useRef } from 'react';
import styles from './SceneRenderer.module.css';

interface SceneRendererProps {
  backgroundImage?: string;
  backgroundVideo?: string;
  transition?: 'fade' | 'dissolve' | 'slide' | 'instant';
  transitionDuration?: number;
  children?: React.ReactNode;
}

function SceneRenderer({
  backgroundImage,
  backgroundVideo,
  transition = 'fade',
  transitionDuration = 500,
  children,
}: SceneRendererProps) {
  const [currentBg, setCurrentBg] = useState<string | undefined>(backgroundImage);
  const [prevBg, setPrevBg] = useState<string | undefined>(undefined);
  const [transitioning, setTransitioning] = useState(false);
  const firstRender = useRef(true);

  useEffect(() => {
    if (firstRender.current) {
      firstRender.current = false;
      return;
    }
    setPrevBg(currentBg);
    setCurrentBg(backgroundImage);
    setTransitioning(true);
    const timer = setTimeout(() => {
      setTransitioning(false);
      setPrevBg(undefined);
    }, transitionDuration);
    return () => clearTimeout(timer);
  }, [backgroundImage]);

  const transitionClass = {
    fade: styles.transitionFade,
    dissolve: styles.transitionDissolve,
    slide: styles.transitionSlide,
    instant: styles.transitionInstant,
  }[transition];

  const cssVars = { '--transition-duration': `${transitionDuration}ms` } as React.CSSProperties;

  return (
    <div className={styles.container} style={cssVars}>
      {/* Default gradient background */}
      {!backgroundImage && !backgroundVideo && <div className={styles.defaultBg} />}

      {/* Background video */}
      {backgroundVideo && (
        <video
          className={styles.bgVideo}
          autoPlay
          loop
          muted
          playsInline
          src={backgroundVideo}
        />
      )}

      {/* Background image with transition */}
      {!backgroundVideo && prevBg && transitioning && (
        <img
          className={styles.bgImage}
          src={prevBg}
          alt=""
          style={{ opacity: 1 - (transition === 'instant' ? 1 : 0.5) }}
        />
      )}
      {!backgroundVideo && currentBg && (
        <img
          className={`${styles.bgImage} ${transitionClass}`}
          src={currentBg}
          alt=""
        />
      )}

      {/* Content layer */}
      <div className={styles.contentLayer}>
        {children}
      </div>
    </div>
  );
}

export default SceneRenderer;
