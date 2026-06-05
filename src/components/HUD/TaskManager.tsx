import { useState, useEffect, useCallback } from 'react';
import { useGeneration } from '../../hooks/useGeneration';

interface TaskItem {
  assetRefId: string;
  assetType: string;
  chapterId: string;
  chapterTitle: string;
  status: 'pending' | 'generating' | 'ready' | 'failed' | 'timeout';
  source?: 'ai_generated' | 'builtin';
  startTime?: number;
  errorMessage?: string;
  retrying?: boolean;
}

const ASSET_LABELS: Record<string, string> = {
  scene_image: '场景图片',
  npc_portrait: 'NPC头像',
  bgm: 'BGM',
  cg_animation: 'CG动画',
  voice: '语音',
  item_image: '道具图片',
  effect_sound: '音效',
  Image: '图片',
  Video: '视频',
  Audio: '音频',
  Voice: '语音',
};

const TIMEOUT_MS = 3 * 60 * 1000; // 3 分钟超时

interface TaskManagerProps {
  gameId: string;
  isOpen: boolean;
  onClose: () => void;
}

export default function TaskManager({ gameId, isOpen, onClose }: TaskManagerProps) {
  const generation = useGeneration();
  const [tasks, setTasks] = useState<Map<string, TaskItem>>(new Map());
  const [filter, setFilter] = useState<'all' | 'active' | 'failed'>('all');

  // 监听事件更新任务状态
  useEffect(() => {
    if (!isOpen) return;
    const unlisteners: (() => void)[] = [];

    generation.onAssetReady((event: any) => {
      const p = event.payload;
      if (!p) return;
      setTasks(prev => {
        const next = new Map(prev);
        const key = `${p.chapterId || ''}_${p.assetType}`;
        const existing = next.get(key);
        next.set(key, {
          assetRefId: p.assetRefId || key,
          assetType: p.assetType,
          chapterId: p.chapterId || '',
          chapterTitle: existing?.chapterTitle || p.chapterId || '',
          status: 'ready',
          source: p.source === 'AiGenerated' ? 'ai_generated' : 'builtin',
          startTime: existing?.startTime,
        });
        return next;
      });
    }).then(fn => unlisteners.push(fn));

    generation.onAssetFailed((event: any) => {
      const p = event.payload;
      if (!p) return;
      setTasks(prev => {
        const next = new Map(prev);
        const key = `${p.chapterId || ''}_${p.assetType}`;
        const existing = next.get(key);
        next.set(key, {
          assetRefId: p.assetRefId || key,
          assetType: p.assetType,
          chapterId: p.chapterId || '',
          chapterTitle: existing?.chapterTitle || p.chapterId || '',
          status: 'failed',
          source: 'builtin',
          startTime: existing?.startTime,
          errorMessage: p.error || p.message,
        });
        return next;
      });
    }).then(fn => unlisteners.push(fn));

    generation.onGenerationProgress((event: any) => {
      const p = event.payload;
      if (!p) return;
      // 章节进度事件，不直接更新任务状态
    }).then(fn => unlisteners.push(fn));

    return () => unlisteners.forEach(fn => fn());
  }, [isOpen, generation]);

  // 超时检测
  useEffect(() => {
    if (!isOpen) return;
    const interval = setInterval(() => {
      const now = Date.now();
      setTasks(prev => {
        let changed = false;
        const next = new Map(prev);
        next.forEach((task, key) => {
          if (task.status === 'generating' && task.startTime && (now - task.startTime) > TIMEOUT_MS) {
            next.set(key, { ...task, status: 'timeout' });
            changed = true;
          }
        });
        return changed ? next : prev;
      });
    }, 10000); // 每 10 秒检测一次
    return () => clearInterval(interval);
  }, [isOpen]);

  const handleRetry = useCallback(async (task: TaskItem) => {
    if (!task.assetRefId || task.assetRefId.includes('_progress')) return;
    setTasks(prev => {
      const next = new Map(prev);
      const key = `${task.chapterId}_${task.assetType}`;
      next.set(key, { ...task, status: 'generating', retrying: true, startTime: Date.now() });
      return next;
    });
    try {
      await generation.regenerateAsset(gameId, task.assetRefId);
    } catch (err) {
      // 重试失败，状态会通过事件更新
    }
  }, [gameId, generation]);

  if (!isOpen) return null;

  const taskList = Array.from(tasks.values());
  const filtered = filter === 'all' ? taskList
    : filter === 'active' ? taskList.filter(t => t.status === 'generating' || t.status === 'pending')
    : taskList.filter(t => t.status === 'failed' || t.status === 'timeout');

  const stats = {
    total: taskList.length,
    ready: taskList.filter(t => t.status === 'ready').length,
    generating: taskList.filter(t => t.status === 'generating').length,
    failed: taskList.filter(t => t.status === 'failed').length,
    timeout: taskList.filter(t => t.status === 'timeout').length,
    pending: taskList.filter(t => t.status === 'pending').length,
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'ready': return '#4ade80';
      case 'generating': return '#facc15';
      case 'failed': return '#f87171';
      case 'timeout': return '#fb923c';
      default: return '#555570';
    }
  };

  const getStatusLabel = (status: string) => {
    switch (status) {
      case 'ready': return '就绪';
      case 'generating': return '生成中';
      case 'failed': return '失败';
      case 'timeout': return '超时';
      case 'pending': return '等待中';
      default: return status;
    }
  };

  return (
    <div style={{
      position: 'fixed', inset: 0,
      backgroundColor: 'rgba(0,0,0,0.4)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      zIndex: 1000,
      backdropFilter: 'blur(4px)', WebkitBackdropFilter: 'blur(4px)',
    }} onClick={e => { if (e.target === e.currentTarget) onClose(); }}>
      <div style={{
        backgroundColor: 'rgba(26, 35, 50, 0.95)', border: '1px solid #2a3a4e',
        borderRadius: '16px', padding: '1.5rem',
        width: '90%', maxWidth: '700px', maxHeight: '80vh',
        display: 'flex', flexDirection: 'column',
        backdropFilter: 'blur(20px)', WebkitBackdropFilter: 'blur(20px)',
        boxShadow: '0 8px 32px rgba(0, 0, 0, 0.4), 0 2px 8px rgba(0, 0, 0, 0.2)',
      }} onClick={e => e.stopPropagation()}>
        {/* Header */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
          <h3 style={{ fontSize: '1.1rem', fontWeight: 600, color: '#e8eaed' }}>任务管理</h3>
          <button style={{ background: 'none', border: 'none', color: '#5a6577', fontSize: '1.2rem', cursor: 'pointer' }} onClick={onClose}>✕</button>
        </div>

        {/* Stats */}
        <div style={{ display: 'flex', gap: '1rem', marginBottom: '1rem', flexWrap: 'wrap' }}>
          <span style={{ fontSize: '0.85rem', color: '#b0b8c4' }}>总计: {stats.total}</span>
          <span style={{ fontSize: '0.85rem', color: '#4ade80' }}>就绪: {stats.ready}</span>
          <span style={{ fontSize: '0.85rem', color: '#facc15' }}>生成中: {stats.generating}</span>
          <span style={{ fontSize: '0.85rem', color: '#f87171' }}>失败: {stats.failed}</span>
          <span style={{ fontSize: '0.85rem', color: '#fb923c' }}>超时: {stats.timeout}</span>
          <span style={{ fontSize: '0.85rem', color: '#5a6577' }}>等待: {stats.pending}</span>
        </div>

        {/* Filter */}
        <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '1rem' }}>
          {(['all', 'active', 'failed'] as const).map(f => (
            <button key={f} className={`btn ${filter === f ? 'btn-primary' : 'btn-secondary'}`}
              style={{ padding: '0.4rem 0.8rem', fontSize: '0.8rem' }}
              onClick={() => setFilter(f)}>
              {{ all: '全部', active: '进行中', failed: '失败/超时' }[f]}
            </button>
          ))}
        </div>

        {/* Task List */}
        <div style={{ flex: 1, overflowY: 'auto' }}>
          {filtered.length === 0 ? (
            <p style={{ color: '#5a6577', textAlign: 'center', padding: '2rem' }}>暂无任务</p>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
              {filtered.map(task => (
                <div key={`${task.chapterId}_${task.assetType}`}
                  style={{
                    display: 'flex', alignItems: 'center', gap: '0.75rem',
                    padding: '0.75rem', backgroundColor: 'rgba(26, 35, 50, 0.8)',
                    border: '1px solid #2a3a4e', borderRadius: '10px',
                  }}>
                  <span style={{
                    width: '8px', height: '8px', borderRadius: '50%',
                    backgroundColor: getStatusColor(task.status),
                    flexShrink: 0,
                  }} />
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ fontSize: '0.9rem', color: '#e8eaed' }}>
                      {ASSET_LABELS[task.assetType] || task.assetType}
                      {task.chapterTitle && <span style={{ color: '#5a6577', marginLeft: '0.5rem' }}>- {task.chapterTitle}</span>}
                    </div>
                    {task.errorMessage && (
                      <div style={{ fontSize: '0.8rem', color: '#f87171', marginTop: '0.25rem' }}>{task.errorMessage}</div>
                    )}
                  </div>
                  <span style={{
                    fontSize: '0.8rem', color: getStatusColor(task.status),
                    flexShrink: 0,
                  }}>
                    {getStatusLabel(task.status)}
                  </span>
                  {(task.status === 'failed' || task.status === 'timeout') && task.assetRefId && !task.assetRefId.includes('_progress') && (
                    <button
                      className="btn btn-secondary"
                      style={{ padding: '0.3rem 0.6rem', fontSize: '0.75rem' }}
                      disabled={task.retrying}
                      onClick={() => handleRetry(task)}>
                      {task.retrying ? '重试中...' : '重试'}
                    </button>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
