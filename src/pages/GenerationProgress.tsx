import { useState, useEffect, useCallback, useRef } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useGeneration, GenerationStepEvent } from '../hooks/useGeneration';
import TaskManager from '../components/HUD/TaskManager';

interface ChapterProgress {
  chapterId: string;
  chapterTitle: string;
  totalAssets: number;
  completedAssets: number;
  assetStatus: Record<string, 'pending' | 'generating' | 'ready' | 'failed'>;
  assetSources: Record<string, 'ai_generated' | 'builtin'>;
  status: 'pending' | 'generating' | 'ready' | 'partial';
}

interface GenerationStatusData {
  firstChapterReady: boolean;
  backgroundGenerationActive: boolean;
  overallProgress: number;
}

const ASSET_LABELS: Record<string, string> = {
  scene_image: '场景图片',
  npc_portrait: 'NPC头像',
  bgm: 'BGM',
  cg_animation: 'CG动画',
  voice: '语音',
  item_image: '道具图片',
  effect_sound: '音效',
};

const SOURCE_LABELS: Record<string, string> = {
  ai_generated: 'AI 生成',
  builtin: '内置默认',
};

const STEP_ICONS: Record<string, string> = {
  starting: '🚀',
  generating_outline: '📝',
  generating_script: '✍️',
  parsing_script: '🔍',
  generating_assets: '🎨',
  asset_ready: '✅',
  first_chapter_ready: '📖',
  completed: '🎉',
};

function AssetStatusIcon({ status }: { status: 'pending' | 'generating' | 'ready' | 'failed' }) {
  switch (status) {
    case 'ready':
      return <span className="asset-icon asset-ready">✓</span>;
    case 'generating':
      return <span className="asset-icon asset-generating">⟳</span>;
    case 'failed':
      return <span className="asset-icon asset-failed">✗</span>;
    case 'pending':
    default:
      return <span className="asset-icon asset-pending">○</span>;
  }
}

function SourceBadge({ source }: { source?: string }) {
  if (!source) return null;
  const label = SOURCE_LABELS[source] || source;
  const isAi = source === 'ai_generated';
  return (
    <span className={`source-badge ${isAi ? 'source-ai' : 'source-builtin'}`}>
      {label}
    </span>
  );
}

function ChapterProgressCard({ chapter, isFirst }: { chapter: ChapterProgress; isFirst: boolean }) {
  const percent = chapter.totalAssets > 0
    ? Math.round((chapter.completedAssets / chapter.totalAssets) * 100)
    : 0;

  return (
    <div className={`chapter-card chapter-${chapter.status}`}>
      <div className="chapter-header">
        <h3 className="chapter-title">
          {chapter.chapterTitle}
          {isFirst && <span className="chapter-first-badge">第一章</span>}
        </h3>
        <span className="chapter-percent">{percent}%</span>
      </div>

      <div className="progress-bar-track">
        <div
          className="progress-bar-fill"
          style={{ width: `${percent}%` }}
        />
      </div>

      <div className="asset-status-list">
        {Object.entries(chapter.assetStatus).map(([key, status]) => (
          <div key={key} className="asset-status-item">
            <AssetStatusIcon status={status} />
            <span className="asset-label">{ASSET_LABELS[key] || key}</span>
            <SourceBadge source={chapter.assetSources[key]} />
          </div>
        ))}
      </div>

      {chapter.status === 'ready' && (
        <div className="chapter-badge">就绪</div>
      )}
      {chapter.status === 'pending' && (
        <div className="chapter-badge chapter-badge-pending">等待中</div>
      )}
    </div>
  );
}

function formatTimestamp(ts: number): string {
  const d = new Date(ts * 1000);
  return d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

export default function GenerationProgress() {
  const { gameId } = useParams<{ gameId: string }>();
  const navigate = useNavigate();
  const generation = useGeneration();

  const [gameTitle, setGameTitle] = useState('');
  const [chapters, setChapters] = useState<ChapterProgress[]>([]);
  const [overallProgress, setOverallProgress] = useState(0);
  const [error, setError] = useState('');
  const [taskManagerOpen, setTaskManagerOpen] = useState(false);
  const [genStatus, setGenStatus] = useState<GenerationStatusData>({
    firstChapterReady: false,
    backgroundGenerationActive: false,
    overallProgress: 0,
  });

  // 进度步骤事件
  const [progressSteps, setProgressSteps] = useState<GenerationStepEvent[]>([]);
  const [currentStep, setCurrentStep] = useState<GenerationStepEvent | null>(null);
  const timelineRef = useRef<HTMLDivElement>(null);

  const updateChapterProgress = useCallback(
    (chapterId: string, updater: (ch: ChapterProgress) => ChapterProgress) => {
      setChapters(prev =>
        prev.map(ch => (ch.chapterId === chapterId ? updater(ch) : ch))
      );
    },
    []
  );

  // 监听 generation-step 事件
  useEffect(() => {
    const unlisteners: (() => void)[] = [];

    generation.onGenerationStep((event: any) => {
      const payload = event.payload as GenerationStepEvent;
      if (!payload || payload.gameId !== gameId) return;

      setProgressSteps(prev => [...prev, payload]);
      setCurrentStep(payload);

      // 自动滚动到底部
      setTimeout(() => {
        if (timelineRef.current) {
          timelineRef.current.scrollTop = timelineRef.current.scrollHeight;
        }
      }, 50);
    }).then(unlisten => unlisteners.push(unlisten));

    return () => unlisteners.forEach(fn => fn());
  }, [gameId]);

  useEffect(() => {
    const unlisteners: (() => void)[] = [];

    generation.onGenerationProgress((event: any) => {
      const payload = event.payload;
      if (!payload) return;
      const { chapterId, chapterTitle, totalAssets, completedAssets } = payload;

      setChapters(prev => {
        const exists = prev.find(ch => ch.chapterId === chapterId);
        if (exists) {
          return prev.map(ch =>
            ch.chapterId === chapterId
              ? {
                  ...ch,
                  totalAssets: totalAssets ?? ch.totalAssets,
                  completedAssets: completedAssets ?? ch.completedAssets,
                  status: (completedAssets ?? ch.completedAssets) >= (totalAssets ?? ch.totalAssets) && (totalAssets ?? ch.totalAssets) > 0
                    ? 'ready' as const
                    : 'generating' as const,
                }
              : ch
          );
        }
        return [
          ...prev,
          {
            chapterId,
            chapterTitle: chapterTitle || `章节 ${prev.length + 1}`,
            totalAssets: totalAssets ?? 0,
            completedAssets: completedAssets ?? 0,
            assetStatus: {},
            assetSources: {},
            status: 'generating' as const,
          },
        ];
      });
    }).then(unlisten => unlisteners.push(unlisten));

    generation.onAssetReady((event: any) => {
      const payload = event.payload;
      if (!payload) return;
      const { chapterId, assetType, source } = payload;

      updateChapterProgress(chapterId, ch => {
        const sourceType: 'ai_generated' | 'builtin' = source === 'AiGenerated' ? 'ai_generated' : 'builtin';
        const newStatus = { ...ch.assetStatus, [assetType]: 'ready' as const };
        const newSources = { ...ch.assetSources, [assetType]: sourceType };
        const readyCount = Object.values(newStatus).filter(s => s === 'ready').length;
        return {
          ...ch,
          assetStatus: newStatus,
          assetSources: newSources,
          completedAssets: readyCount,
          totalAssets: ch.totalAssets || Object.keys(newStatus).length,
          status: readyCount >= (ch.totalAssets || Object.keys(newStatus).length) && (ch.totalAssets || Object.keys(newStatus).length) > 0
            ? 'ready' as const
            : 'partial' as const,
        };
      });
    }).then(unlisten => unlisteners.push(unlisten));

    generation.onAssetFailed((event: any) => {
      const payload = event.payload;
      if (!payload) return;
      const { chapterId, assetType, message } = payload;

      if (message) {
        setError(message);
      }

      updateChapterProgress(chapterId, ch => ({
        ...ch,
        assetStatus: { ...ch.assetStatus, [assetType]: 'failed' as const },
        assetSources: { ...ch.assetSources, [assetType]: 'builtin' as const },
      }));
    }).then(unlisten => unlisteners.push(unlisten));

    generation.onGenerationComplete((event: any) => {
      const payload = event.payload;
      if (!payload) return;
      const { chapterId, allChapters } = payload;

      if (allChapters) {
        setGenStatus(prev => ({ ...prev, backgroundGenerationActive: false }));
        return;
      }

      updateChapterProgress(chapterId, ch => ({
        ...ch,
        completedAssets: ch.totalAssets,
        status: 'ready' as const,
      }));

      // 第一章就绪
      if (chapters.length > 0 && chapters[0].chapterId === chapterId) {
        setGenStatus(prev => ({ ...prev, firstChapterReady: true }));
      }
    }).then(unlisten => unlisteners.push(unlisten));

    return () => unlisteners.forEach(fn => fn());
  }, [updateChapterProgress, chapters]);

  useEffect(() => {
    if (!gameId) return;

    generation.getGenerationStatus(gameId).then((status: any) => {
      if (!status) return;
      if (status.gameTitle) setGameTitle(status.gameTitle);
      if (status.firstChapterReady !== undefined) {
        setGenStatus(prev => ({
          ...prev,
          firstChapterReady: status.firstChapterReady,
          backgroundGenerationActive: status.backgroundGenerationActive ?? false,
          overallProgress: status.overallProgress ?? 0,
        }));
      }
      if (status.chapterStatus) {
        const chapterStatusMap = status.chapterStatus as Record<string, any>;
        setChapters(
          Object.values(chapterStatusMap).map((ch: any) => ({
            chapterId: ch.chapterId,
            chapterTitle: ch.chapterTitle,
            totalAssets: ch.totalAssets ?? 0,
            completedAssets: ch.completedAssets ?? 0,
            assetStatus: ch.assetStatus ?? {},
            assetSources: ch.assetSources ?? {},
            status: ch.status ?? 'pending',
          }))
        );
      }
    }).catch(() => {
      // status may not be available yet
    });
  }, [gameId]);

  useEffect(() => {
    if (chapters.length === 0) {
      setOverallProgress(0);
      return;
    }
    const total = chapters.reduce((sum, ch) => sum + ch.totalAssets, 0);
    const completed = chapters.reduce((sum, ch) => sum + ch.completedAssets, 0);
    setOverallProgress(total > 0 ? Math.round((completed / total) * 100) : 0);
  }, [chapters]);

  const firstChapterReady = chapters.length > 0 && (chapters[0].status === 'ready' || chapters[0].status === 'partial' || genStatus.firstChapterReady);
  const hasRemainingChapters = chapters.length > 1;
  const isCompleted = currentStep?.step === 'completed';

  return (
    <div className="page generation-progress">
      <div style={{ display: 'flex', gap: '0.5rem' }}>
        <button className="btn btn-back" onClick={() => navigate('/')}>
          ← 返回主菜单
        </button>
      </div>

      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.5rem' }}>
        <h2 className="page-title" style={{ marginBottom: 0 }}>
          {isCompleted ? `游戏生成完成：${gameTitle || '加载中...'}` : `正在生成游戏：${gameTitle || '加载中...'}`}
        </h2>
        <button className="btn btn-secondary" style={{ padding: '0.5rem 1rem', fontSize: '0.85rem' }}
          onClick={() => setTaskManagerOpen(true)}>
          任务管理
        </button>
      </div>

      {/* 当前步骤高亮显示 */}
      {currentStep && !isCompleted && (
        <div className="current-step-banner">
          <span className="current-step-icon">{STEP_ICONS[currentStep.step] || '⏳'}</span>
          <div className="current-step-info">
            <span className="current-step-detail">{currentStep.detail}</span>
            {currentStep.modelName && (
              <span className="current-step-model">等待 {currentStep.modelName} 返回</span>
            )}
          </div>
          <div className="current-step-pulse" />
        </div>
      )}

      {/* 进度时间线 */}
      {progressSteps.length > 0 && (
        <div className="progress-timeline-container">
          <div className="progress-timeline" ref={timelineRef}>
            {progressSteps.map((step, index) => {
              const isLast = index === progressSteps.length - 1;
              const isDone = !isLast || step.step === 'completed' || step.step === 'asset_ready' || step.step === 'first_chapter_ready';
              return (
                <div key={`${step.timestamp}-${index}`} className={`timeline-item ${isLast && !isDone ? 'timeline-active' : ''} ${isDone ? 'timeline-done' : ''}`}>
                  <div className="timeline-dot">
                    {isDone ? '✓' : '⏳'}
                  </div>
                  {!isLast && <div className="timeline-line" />}
                  <div className="timeline-content">
                    <div className="timeline-header">
                      <span className="timeline-icon">{STEP_ICONS[step.step] || '📌'}</span>
                      <span className="timeline-detail">{step.detail}</span>
                    </div>
                    <div className="timeline-meta">
                      <span className="timeline-time">{formatTimestamp(step.timestamp)}</span>
                      {step.modelName && (
                        <span className="timeline-model">{step.modelName}</span>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* 后台生成提示 */}
      {!firstChapterReady && !isCompleted && (
        <div className="background-generation-hint" style={{ marginBottom: '0.75rem' }}>
          <span className="hint-icon">💡</span>
          生成任务在后台运行，离开此页面不会中断生成
        </div>
      )}

      <div className="overall-progress">
        <div className="overall-progress-header">
          <span>整体进度</span>
          <span className="overall-percent">{overallProgress}%</span>
        </div>
        <div className="progress-bar-track progress-bar-track-lg">
          <div
            className="progress-bar-fill"
            style={{ width: `${overallProgress}%` }}
          />
        </div>
      </div>

      {error && (
        <div className="generation-error">
          <span className="error-icon">⚠</span> {error}
          <button className="btn-dismiss-error" onClick={() => setError('')}>✕</button>
        </div>
      )}

      <div className="chapter-list">
        {chapters.map((chapter, index) => (
          <ChapterProgressCard
            key={chapter.chapterId}
            chapter={chapter}
            isFirst={index === 0}
          />
        ))}
      </div>

      {chapters.length === 0 && progressSteps.length === 0 && (
        <div className="empty-state">
          <div className="spinner" />
          <p>正在初始化生成任务...</p>
        </div>
      )}

      {/* 后续章节后台生成提示 */}
      {firstChapterReady && hasRemainingChapters && genStatus.backgroundGenerationActive && (
        <div className="background-generation-hint">
          <span className="hint-icon">⟳</span>
          后续章节正在后台生成中，您可以先开始游玩第一章
        </div>
      )}

      {firstChapterReady && (
        <button
          className="btn btn-primary btn-play"
          onClick={() => navigate(`/play/${gameId}`)}
        >
          开始第一章 ▶
        </button>
      )}

      <TaskManager gameId={gameId || ''} isOpen={taskManagerOpen} onClose={() => setTaskManagerOpen(false)} />
    </div>
  );
}
