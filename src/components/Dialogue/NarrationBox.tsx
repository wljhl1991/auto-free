import { useEffect, useState, useCallback } from 'react';
import styles from './NarrationBox.module.css';

interface NarrationBoxProps {
  text: string;
  isTyping?: boolean;
  onTypingComplete?: () => void;
  onAdvance?: () => void;
}

const TYPING_INTERVAL = 30;

function NarrationBox({
  text,
  isTyping = true,
  onTypingComplete,
  onAdvance,
}: NarrationBoxProps) {
  const [displayedLength, setDisplayedLength] = useState(0);
  const [typingDone, setTypingDone] = useState(false);

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
          clearInterval(timer);
        }
        return next;
      });
    }, TYPING_INTERVAL);

    return () => clearInterval(timer);
  }, [text, displayedLength, isTyping, typingDone, onTypingComplete]);

  const handleClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    if (displayedLength < text.length) {
      setDisplayedLength(text.length);
      setTypingDone(true);
      onTypingComplete?.();
    } else {
      onAdvance?.();
    }
  }, [displayedLength, text.length, onTypingComplete, onAdvance]);

  const displayedText = text.slice(0, displayedLength);
  const showCursor = displayedLength < text.length;

  return (
    <div className={styles.container} onClick={handleClick}>
      <div className={styles.text}>
        {displayedText}
        {showCursor && <span className={styles.cursor} />}
      </div>
    </div>
  );
}

export default NarrationBox;
