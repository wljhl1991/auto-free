import { useState } from 'react';
import { convertFileSrc } from '../../adapters/tauri';

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
        backgroundColor: 'rgba(45, 55, 72, 0.3)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1100,
        backdropFilter: 'blur(4px)', WebkitBackdropFilter: 'blur(4px)',
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div style={{
        backgroundColor: 'rgba(255, 255, 255, 0.95)',
        border: '1px solid #e8e2d8',
        borderRadius: '16px',
        padding: '1.5rem',
        width: '90%',
        maxWidth: '640px',
        backdropFilter: 'blur(20px)', WebkitBackdropFilter: 'blur(20px)',
        boxShadow: '0 8px 32px rgba(0, 0, 0, 0.1), 0 2px 8px rgba(0, 0, 0, 0.05)',
      }}>
        <h3 style={{
          fontSize: '1.1rem',
          fontWeight: 600,
          color: '#2d3748',
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
                  ? '2px solid #e07a2f'
                  : '2px solid #e8e2d8',
                cursor: isRegenerating ? 'not-allowed' : 'pointer',
                transition: 'border-color 0.2s ease',
                aspectRatio: '16/10',
                backgroundColor: 'rgba(250, 248, 245, 0.9)',
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
                backgroundColor: 'rgba(250, 248, 245, 0.9)',
                fontSize: '0.8rem',
                color: '#4a5568',
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
                  backgroundColor: '#e07a2f',
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
