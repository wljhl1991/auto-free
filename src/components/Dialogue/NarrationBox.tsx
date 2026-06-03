import { useEffect, useState, useCallback } from 'react';
import styles from './NarrationBox.module.css';

interface NarrationBoxProps {
  text: string;
  isTyping?: boolean;
  onTypingComplete?: () => void;
}

const TYPING_INTERVAL = 30;

function NarrationBox({
  text,
  isTyping = true,
  onTypingComplete,
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

  const handleClick = useCallback(() => {
    if (displayedLength < text.length) {
      setDisplayedLength(text.length);
      setTypingDone(true);
      onTypingComplete?.();
    }
  }, [displayedLength, text.length, onTypingComplete]);

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
