import type { AIProviderConfig } from '@/types';
import ProviderCard from './ProviderCard';

interface ModalitySectionProps {
  modality: string;
  title: string;
  providers: AIProviderConfig[];
  onConfigure: (id: string) => void;
  onCheck: (id: string, testPrompt?: string) => void;
}

const modalityIcons: Record<string, string> = {
  text: '📝',
  image: '🖼️',
  video: '🎬',
  music: '🎵',
  voice: '🎙️',
};

export default function ModalitySection({
  modality,
  title,
  providers,
  onConfigure,
  onCheck,
}: ModalitySectionProps) {
  if (providers.length === 0) return null;

  return (
    <div style={{ marginBottom: '2rem' }}>
      <h3 style={{
        fontSize: '1.1rem',
        fontWeight: 600,
        color: '#c0c0d0',
        marginBottom: '1rem',
        display: 'flex',
        alignItems: 'center',
        gap: '0.5rem',
      }}>
        <span>{modalityIcons[modality] || '🔧'}</span>
        {title}
        <span style={{
          fontSize: '0.75rem',
          fontWeight: 400,
          color: '#666680',
          marginLeft: '0.25rem',
        }}>
          ({providers.length})
        </span>
      </h3>
      <div style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(auto-fill, minmax(300px, 1fr))',
        gap: '1rem',
      }}>
        {providers.map((provider) => (
          <ProviderCard
            key={provider.id}
            provider={provider}
            onConfigure={() => onConfigure(provider.id)}
            onCheck={() => onCheck(provider.id)}
          />
        ))}
      </div>
    </div>
  );
}
