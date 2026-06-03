import { useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';

interface LocalAsset {
  id: string;
  type: string;
  localPath: string;
  source: string;
  cacheKey: string;
  createdAt: number;
}

interface CandidateSelectorProps {
  candidates: LocalAsset[];
  onSelect: (candidate: LocalAsset) => void;
  onRegenerateAll: () => void;
  onClose: () => void;
  isRegenerating: boolean;
}

function CandidateSelector({ candidates, onSelect, onRegenerateAll, onClose, isRegenerating }: CandidateSelectorProps) {
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);

  const handleSelect = (index: number) => {
    if (isRegenerating) return;
    setSelectedIndex(index);
    onSelect(candidates[index]);
  };

  const resolveAssetUrl = (localPath: string): string => {
    try {
      return convertFileSrc(localPath);
    } catch {
      return localPath;
    }
  };

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.7)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1100,
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div style={{
        backgroundColor: '#16162a',
        border: '1px solid #2a2a3a',
        borderRadius: '12px',
        padding: '1.5rem',
        width: '90%',
        maxWidth: '640px',
      }}>
        <h3 style={{
          fontSize: '1.1rem',
          fontWeight: 600,
          color: '#e0e0f0',
          marginBottom: '1rem',
        }}>
          选择候选资源
        </h3>

        <div style={{
          display: 'grid',
          gridTemplateColumns: `repeat(${Math.min(candidates.length, 3)}, 1fr)`,
          gap: '0.75rem',
          marginBottom: '1.25rem',
        }}>
          {candidates.map((candidate, index) => (
            <div
              key={candidate.id}
              onClick={() => handleSelect(index)}
              style={{
                position: 'relative',
                borderRadius: '8px',
                overflow: 'hidden',
                border: selectedIndex === index
                  ? '2px solid #4a90d9'
                  : '2px solid #2a2a3a',
                cursor: isRegenerating ? 'not-allowed' : 'pointer',
                transition: 'border-color 0.2s ease',
                aspectRatio: '16/10',
                backgroundColor: '#0a0a1a',
              }}
            >
              {candidate.type === 'Video' ? (
                <video
                  src={resolveAssetUrl(candidate.localPath)}
                  style={{
                    width: '100%',
                    height: '100%',
                    objectFit: 'cover',
                  }}
                  muted
                />
              ) : (
                <img
                  src={resolveAssetUrl(candidate.localPath)}
                  alt={`候选 ${index + 1}`}
                  style={{
                    width: '100%',
                    height: '100%',
                    objectFit: 'cover',
                  }}
                />
              )}
              <div style={{
                position: 'absolute',
                bottom: 0,
                left: 0,
                right: 0,
                padding: '0.4rem 0.6rem',
                backgroundColor: 'rgba(10, 10, 26, 0.8)',
                fontSize: '0.8rem',
                color: '#9999bb',
              }}>
                候选 {index + 1}
              </div>
              {selectedIndex === index && (
                <div style={{
                  position: 'absolute',
                  top: '0.4rem',
                  right: '0.4rem',
                  width: '24px',
                  height: '24px',
                  borderRadius: '50%',
                  backgroundColor: '#4a90d9',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  color: '#fff',
                  fontSize: '0.8rem',
                  fontWeight: 700,
                }}>
                  ✓
                </div>
              )}
            </div>
          ))}
        </div>

        <div style={{
          display: 'flex',
          gap: '0.75rem',
          justifyContent: 'flex-end',
        }}>
          <button
            className="btn btn-secondary"
            onClick={onClose}
            style={{ padding: '0.6rem 1.2rem', fontSize: '0.9rem' }}
          >
            取消
          </button>
          <button
            className="btn btn-secondary"
            onClick={onRegenerateAll}
            disabled={isRegenerating}
            style={{ padding: '0.6rem 1.2rem', fontSize: '0.9rem' }}
          >
            {isRegenerating ? '生成中...' : '重新生成全部'}
          </button>
        </div>
      </div>
    </div>
  );
}

export default CandidateSelector;
