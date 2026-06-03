import { useState } from 'react';
import styles from './ChoicePanel.module.css';

interface ChoiceOption {
  text: string;
  enabled?: boolean;
  visible?: boolean;
}

interface ChoicePanelProps {
  prompt: string;
  options: ChoiceOption[];
  onSelect: (index: number) => void;
}

function ChoicePanel({ prompt, options, onSelect }: ChoicePanelProps) {
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);

  const handleSelect = (index: number) => {
    if (selectedIndex !== null) return;
    if (options[index].enabled === false) return;
    setSelectedIndex(index);
    onSelect(index);
  };

  return (
    <div className={styles.container}>
      {prompt && <div className={styles.prompt}>{prompt}</div>}
      <div className={styles.optionList}>
        {options.map((option, index) => {
          if (option.visible === false) return null;
          const isDisabled = option.enabled === false;
          const isSelected = selectedIndex === index;
          return (
            <button
              key={index}
              className={`${styles.optionBtn} ${isSelected ? styles.selected : ''}`}
              disabled={isDisabled || selectedIndex !== null}
              onClick={() => handleSelect(index)}
            >
              {option.text}
            </button>
          );
        })}
      </div>
    </div>
  );
}

export default ChoicePanel;
