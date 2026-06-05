import { useState } from 'react';

interface PromptEditorProps {
  prompt: string;
  negativePrompt?: string;
  onRegenerate: (prompt: string, negativePrompt: string) => void;
  onCancel: () => void;
}

function PromptEditor({ prompt, negativePrompt, onRegenerate, onCancel }: PromptEditorProps) {
  const [editedPrompt, setEditedPrompt] = useState(prompt);
  const [editedNegativePrompt, setEditedNegativePrompt] = useState(negativePrompt ?? '');

  const handleRegenerate = () => {
    onRegenerate(editedPrompt, editedNegativePrompt);
  };

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(45, 55, 72, 0.3)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1100,
        backdropFilter: 'blur(4px)', WebkitBackdropFilter: 'blur(4px)',
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onCancel();
      }}
    >
      <div style={{
        backgroundColor: 'rgba(255, 255, 255, 0.95)',
        border: '1px solid #e8e2d8',
        borderRadius: '16px',
        padding: '1.5rem',
        width: '90%',
        maxWidth: '480px',
        backdropFilter: 'blur(20px)', WebkitBackdropFilter: 'blur(20px)',
        boxShadow: '0 8px 32px rgba(0, 0, 0, 0.1), 0 2px 8px rgba(0, 0, 0, 0.05)',
      }}>
        <h3 style={{
          fontSize: '1.1rem',
          fontWeight: 600,
          color: '#2d3748',
          marginBottom: '1rem',
        }}>
          编辑 Prompt
        </h3>

        <div style={{ marginBottom: '1rem' }}>
          <label style={{
            display: 'block',
            fontSize: '0.85rem',
            color: '#4a5568',
            marginBottom: '0.4rem',
          }}>
            Prompt
          </label>
          <textarea
            value={editedPrompt}
            onChange={(e) => setEditedPrompt(e.target.value)}
            rows={4}
            style={{
              width: '100%',
              padding: '0.6rem 0.8rem',
              fontSize: '0.9rem',
              fontFamily: 'inherit',
              backgroundColor: '#faf8f5',
              color: '#2d3748',
              border: '1px solid #e8e2d8',
              borderRadius: '10px',
              outline: 'none',
              resize: 'vertical',
              boxSizing: 'border-box',
            }}
            onFocus={(e) => {
              e.currentTarget.style.borderColor = '#e07a2f';
            }}
            onBlur={(e) => {
              e.currentTarget.style.borderColor = '#e8e2d8';
            }}
          />
        </div>

        <div style={{ marginBottom: '1.25rem' }}>
          <label style={{
            display: 'block',
            fontSize: '0.85rem',
            color: '#4a5568',
            marginBottom: '0.4rem',
          }}>
            Negative Prompt
          </label>
          <textarea
            value={editedNegativePrompt}
            onChange={(e) => setEditedNegativePrompt(e.target.value)}
            rows={2}
            placeholder="可选，描述不想出现的内容"
            style={{
              width: '100%',
              padding: '0.6rem 0.8rem',
              fontSize: '0.9rem',
              fontFamily: 'inherit',
              backgroundColor: '#faf8f5',
              color: '#2d3748',
              border: '1px solid #e8e2d8',
              borderRadius: '10px',
              outline: 'none',
              resize: 'vertical',
              boxSizing: 'border-box',
            }}
            onFocus={(e) => {
              e.currentTarget.style.borderColor = '#e07a2f';
            }}
            onBlur={(e) => {
              e.currentTarget.style.borderColor = '#e8e2d8';
            }}
          />
        </div>

        <div style={{
          display: 'flex',
          gap: '0.75rem',
          justifyContent: 'flex-end',
        }}>
          <button
            className="btn btn-secondary"
            onClick={onCancel}
            style={{ padding: '0.6rem 1.2rem', fontSize: '0.9rem' }}
          >
            取消
          </button>
          <button
            className="btn btn-primary"
            onClick={handleRegenerate}
            style={{ padding: '0.6rem 1.2rem', fontSize: '0.9rem' }}
          >
            重新生成
          </button>
        </div>
      </div>
    </div>
  );
}

export default PromptEditor;
