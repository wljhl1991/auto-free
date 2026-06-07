import { useEffect, useState, useCallback, useRef } from 'react';
import styles from './DialogueBox.module.css';

interface DialogueBoxProps {
  speaker: string;
  speakerAvatar?: string;
  text: string;
  emotion?: string;
  isTyping?: boolean;
  onTypingComplete?: () => void;
  onAdvance?: () => void;
}

const TYPING_INTERVAL = 30;

function DialogueBox({
  speaker,
  speakerAvatar,
  text,
  emotion,
  isTyping = true,
  onTypingComplete,
  onAdvance,
}: DialogueBoxProps) {
  const [displayedLength, setDisplayedLength] = useState(0);
  const [typingDone, setTypingDone] = useState(false);
  const timerRef = useRef<number | null>(null);

  useEffect(() => {
    setDisplayedLength(0);
    setTypingDone(false);
  }, [text]);

  useEffect(() => {
    if (!isTyping) {
      setDisplayedLength(text.length);
      setTypingDone(true);
      return;
    }

    if (displayedLength >= text.length) {
      if (!typingDone) {
        setTypingDone(true);
        onTypingComplete?.();
      }
      return;
    }

    const timer = setInterval(() => {
      setDisplayedLength((prev) => {
        const next = prev + 1;
        if (next >= text.length) {
          if (timerRef.current) clearInterval(timerRef.current);
        }
        return next;
      });
    }, TYPING_INTERVAL);
    timerRef.current = timer;

    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [text, displayedLength, isTyping, typingDone, onTypingComplete]);

  const handleClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    if (displayedLength < text.length) {
      // 打字动画未完成，点击直接显示完整文本
      if (timerRef.current) clearInterval(timerRef.current);
      setDisplayedLength(text.length);
      setTypingDone(true);
      onTypingComplete?.();
    } else {
      // 打字动画已完成，点击推进剧情
      onAdvance?.();
    }
  }, [displayedLength, text.length, onTypingComplete, onAdvance]);

  const emotionClass = emotion
    ? styles[`emotion${emotion.charAt(0).toUpperCase()}${emotion.slice(1)}` as keyof typeof styles] || ''
    : '';

  const displayedText = text.slice(0, displayedLength);
  const showCursor = displayedLength < text.length;

  return (
    <div className={styles.container} onClick={handleClick}>
      <div className={styles.header}>
        {speakerAvatar ? (
          <img
            className={`${styles.avatar} ${emotionClass}`}
            src={speakerAvatar}
            alt={speaker}
          />
        ) : (
          <div className={`${styles.avatarPlaceholder} ${emotionClass}`}>
            {speaker.charAt(0)}
          </div>
        )}
        <span className={styles.speakerName}>{speaker}</span>
      </div>
      <div className={styles.text}>
        {displayedText}
        {showCursor && <span className={styles.cursor} />}
      </div>
    </div>
  );
}

export default DialogueBox;
