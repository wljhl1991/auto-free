import { useParams, useNavigate } from 'react-router-dom';
import { useEffect, useState, useRef, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/core';
import { invoke } from '@tauri-apps/api/core';
import { SceneExecutor, SceneEventType } from '../engine/SceneExecutor';
import { StateManager } from '../engine/StateManager';
import { AssetLoader } from '../engine/AssetLoader';
import { AudioEngine } from '../engine/AudioEngine';
import SceneRenderer from '../components/Scene/SceneRenderer';
import DialogueBox from '../components/Dialogue/DialogueBox';
import NarrationBox from '../components/Dialogue/NarrationBox';
import ChoicePanel from '../components/Choice/ChoicePanel';
import CGPlayer from '../components/CG/CGPlayer';
import CGGallery from '../components/CG/CGGallery';
import { GameMenu } from '../components/HUD/GameMenu';
import { InventoryPanel } from '../components/HUD/InventoryPanel';
import { StatsPanel } from '../components/HUD/StatsPanel';
import PromptEditor from '../components/Config/PromptEditor';
import CandidateSelector from '../components/Choice/CandidateSelector';
import { useGame } from '../hooks/useGame';
import { useGeneration } from '../hooks/useGeneration';

interface AssetInfo {
  assetRefId: string;
  assetType: string;
  localPath: string;
  source: string;
}

interface LocalAsset {
  id: string;
  type: string;
  localPath: string;
  source: string;
  cacheKey: string;
  createdAt: number;
}

interface ContextMenuState {
  visible: boolean;
  x: number;
  y: number;
  assetRefId: string;
  assetType: string;
  prompt: string;
  negativePrompt?: string;
}

function GamePlay() {
  const { gameId } = useParams<{ gameId: string }>();
  const navigate = useNavigate();
  const game = useGame();
  const generation = useGeneration();

  const executorRef = useRef<SceneExecutor | null>(null);
  const stateManagerRef = useRef<StateManager>(new StateManager());
  const assetLoaderRef = useRef<AssetLoader>(new AssetLoader());
  const audioEngineRef = useRef<AudioEngine>(new AudioEngine());

  const [currentEvent, setCurrentEvent] = useState<SceneEventType | null>(null);
  const [sceneBackground, setSceneBackground] = useState<string | undefined>();
  const [sceneVideo, setSceneVideo] = useState<string | undefined>();
  const [currentBgAssetRefId] = useState<string | undefined>();
  const [chapterTitle] = useState('');
  const [showMenu, setShowMenu] = useState(false);
  const [showInventory, setShowInventory] = useState(false);
  const [showStats, setShowStats] = useState(false);
  const [showGallery, setShowGallery] = useState(false);
  const [isLoading, setIsLoading] = useState(true);

  // 资源状态追踪：assetRefId -> { source, status }
  const [assetStates, setAssetStates] = useState<Record<string, { source: string; status: 'generating' | 'ready' | 'fallback' }>>({});
  // 热替换动画状态
  const [hotSwapping, setHotSwapping] = useState(false);

  // 右键菜单状态
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  // PromptEditor 状态
  const [promptEditorState, setPromptEditorState] = useState<{
    visible: boolean;
    assetRefId: string;
    prompt: string;
    negativePrompt?: string;
  } | null>(null);
  // CandidateSelector 状态
  const [candidateState, setCandidateState] = useState<{
    visible: boolean;
    candidates: LocalAsset[];
    assetRefId: string;
    isRegenerating: boolean;
  }>({ visible: false, candidates: [], assetRefId: '', isRegenerating: false });

  // 监听 asset-ready 事件，实现热替换
  useEffect(() => {
    if (!gameId) return;

    const unlisteners: (() => void)[] = [];

    listen('asset-ready', (event: any) => {
      const payload = event.payload;
      if (!payload || payload.gameId !== gameId) return;

      const { assetRefId, assetType, localPath, source } = payload as AssetInfo;

      // 更新资源状态
      setAssetStates(prev => ({
        ...prev,
        [assetRefId]: {
          source: source,
          status: source === 'Builtin' ? 'fallback' : 'ready',
        },
      }));

      // 如果当前正在展示该资源，执行热替换
      if (assetType === 'Image' && assetRefId === currentBgAssetRefId) {
        setHotSwapping(true);
        const resolvedUrl = resolveAssetUrl(localPath);
        setSceneBackground(resolvedUrl);
        setTimeout(() => setHotSwapping(false), 500);
      }
    }).then(unlisten => unlisteners.push(unlisten));

    listen('asset-failed', (event: any) => {
      const payload = event.payload;
      if (!payload || payload.gameId !== gameId) return;

      const { assetRefId } = payload;
      setAssetStates(prev => ({
        ...prev,
        [assetRefId]: { source: 'builtin', status: 'fallback' },
      }));
    }).then(unlisten => unlisteners.push(unlisten));

    return () => unlisteners.forEach(fn => fn());
  }, [gameId, currentBgAssetRefId]);

  // 点击其他区域关闭右键菜单
  useEffect(() => {
    const handleClick = () => setContextMenu(null);
    window.addEventListener('click', handleClick);
    return () => window.removeEventListener('click', handleClick);
  }, []);

  // 将本地路径转换为可访问的 URL
  const resolveAssetUrl = (localPath: string): string => {
    try {
      return convertFileSrc(localPath);
    } catch {
      return localPath;
    }
  };

  useEffect(() => {
    if (!gameId) return;

    game.getGameScript(gameId).then(script => {
      const executor = new SceneExecutor(
        script,
        stateManagerRef.current,
        assetLoaderRef.current,
        audioEngineRef.current,
      );

      executor.setOnEvent((event) => {
        setCurrentEvent(event);

        if (event.type === 'scene_change') {
          setSceneBackground(event.backgroundImage);
          setSceneVideo(event.backgroundVideo);
        }
        if (event.type === 'scene_transition') {
          // 场景转场由 SceneRenderer 处理
        }
      });

      executorRef.current = executor;
      executor.start();
      setIsLoading(false);
    }).catch(err => {
      console.error('Failed to load game:', err);
      setIsLoading(false);
    });
  }, [gameId]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setShowMenu(prev => !prev);
        return;
      }
      if (e.key === ' ' || e.key === 'Enter') {
        if (currentEvent?.type === 'narration' || currentEvent?.type === 'dialogue') {
          executorRef.current?.advance();
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [currentEvent]);

  const handleChoice = useCallback((index: number) => {
    executorRef.current?.onChoiceSelected(index);
  }, []);

  const handleTypingComplete = useCallback(() => {
    // 不自动推进，等待玩家点击
  }, []);

  const handleClick = useCallback(() => {
    if (showMenu || showInventory || showStats || showGallery) return;
    if (currentEvent?.type === 'narration' || currentEvent?.type === 'dialogue') {
      executorRef.current?.advance();
    }
  }, [currentEvent, showMenu, showInventory, showStats]);

  const handleSave = useCallback(async () => {
    if (!gameId) return;
    const state = stateManagerRef.current.serialize();
    await game.saveGame(gameId, state);
    setShowMenu(false);
  }, [gameId]);

  const handleLoad = useCallback(async () => {
    // 简化：暂不实现
  }, []);

  const handleBackToMenu = useCallback(() => {
    audioEngineRef.current.stopBgm();
    navigate('/');
  }, []);

  // 导出游戏
  const handleExportGame = useCallback(async () => {
    if (!gameId) return;
    try {
      const outputPath = await generation.exportGame(gameId, `autofree_${gameId}.zip`);
      alert(`游戏已导出至: ${outputPath}`);
    } catch (err) {
      console.error('Failed to export game:', err);
      alert(`导出失败: ${err}`);
    }
    setShowMenu(false);
  }, [gameId]);

  // 获取已解锁 CG 列表
  const getUnlockedCGList = useCallback(() => {
    const state = stateManagerRef.current.serialize();
    return (state.unlockedCGs || []).map((cgId: string) => ({
      id: cgId,
      url: cgId,
      description: cgId,
    }));
  }, []);

  // 重新生成资源（单个）
  const handleRegenerate = useCallback(async (assetRefId: string) => {
    if (!gameId) return;
    setAssetStates(prev => ({
      ...prev,
      [assetRefId]: { source: 'ai_generated', status: 'generating' },
    }));
    try {
      await generation.regenerateAsset(gameId, assetRefId);
    } catch (err) {
      console.error('Failed to regenerate asset:', err);
    }
  }, [gameId]);

  // 多候选重新生成
  const handleRegenerateCandidates = useCallback(async (assetRefId: string, count: number = 3) => {
    if (!gameId) return;
    setCandidateState(prev => ({ ...prev, isRegenerating: true }));
    setAssetStates(prev => ({
      ...prev,
      [assetRefId]: { source: 'ai_generated', status: 'generating' },
    }));
    try {
      const candidates = await invoke<LocalAsset[]>('regenerate_asset_candidates', {
        gameId,
        assetRefId,
        count,
      });
      setCandidateState({
        visible: true,
        candidates,
        assetRefId,
        isRegenerating: false,
      });
    } catch (err) {
      console.error('Failed to regenerate candidates:', err);
      setCandidateState(prev => ({ ...prev, isRegenerating: false }));
    }
  }, [gameId]);

  // 选择候选
  const handleSelectCandidate = useCallback((candidate: LocalAsset) => {
    // 更新资源状态为就绪
    setAssetStates(prev => ({
      ...prev,
      [candidateState.assetRefId]: { source: candidate.source, status: 'ready' },
    }));
    // 如果是当前背景图，执行热替换
    if (candidateState.assetRefId === currentBgAssetRefId && candidate.type === 'Image') {
      setHotSwapping(true);
      setSceneBackground(resolveAssetUrl(candidate.localPath));
      setTimeout(() => setHotSwapping(false), 500);
    }
    setCandidateState({ visible: false, candidates: [], assetRefId: '', isRegenerating: false });
  }, [candidateState.assetRefId, currentBgAssetRefId]);

  // 重新生成全部候选
  const handleRegenerateAllCandidates = useCallback(() => {
    handleRegenerateCandidates(candidateState.assetRefId);
  }, [candidateState.assetRefId, handleRegenerateCandidates]);

  // 右键菜单处理
  const handleContextMenu = useCallback((e: React.MouseEvent, assetRefId: string, assetType: string, prompt: string, negativePrompt?: string) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({
      visible: true,
      x: e.clientX,
      y: e.clientY,
      assetRefId,
      assetType,
      prompt,
      negativePrompt,
    });
  }, []);

  // 右键菜单 - 重新生成
  const handleMenuRegenerate = useCallback(() => {
    if (!contextMenu) return;
    handleRegenerate(contextMenu.assetRefId);
    setContextMenu(null);
  }, [contextMenu, handleRegenerate]);

  // 右键菜单 - 多候选生成（仅图片/视频）
  const handleMenuCandidates = useCallback(() => {
    if (!contextMenu) return;
    handleRegenerateCandidates(contextMenu.assetRefId);
    setContextMenu(null);
  }, [contextMenu, handleRegenerateCandidates]);

  // 右键菜单 - 编辑 Prompt
  const handleMenuEditPrompt = useCallback(() => {
    if (!contextMenu) return;
    setPromptEditorState({
      visible: true,
      assetRefId: contextMenu.assetRefId,
      prompt: contextMenu.prompt,
      negativePrompt: contextMenu.negativePrompt,
    });
    setContextMenu(null);
  }, [contextMenu]);

  // PromptEditor - 重新生成
  const handlePromptRegenerate = useCallback((_prompt: string, _negativePrompt: string) => {
    if (!promptEditorState) return;
    // 目前后端 regenerate_asset 使用 AssetRef 中已有的 prompt
    // 未来可扩展为传递修改后的 prompt
    handleRegenerate(promptEditorState.assetRefId);
    setPromptEditorState(null);
  }, [promptEditorState, handleRegenerate]);

  // 判断资源类型是否支持多候选
  const isVisualAsset = (assetType: string) => assetType === 'Image' || assetType === 'Video';

  // 获取当前背景图的资源状态
  const currentBgState = currentBgAssetRefId ? assetStates[currentBgAssetRefId] : undefined;

  if (isLoading) {
    return <div className="gameplay-loading"><div className="spinner" /></div>;
  }

  return (
    <div className="gameplay-page" onClick={handleClick}>
      <div className={`gameplay-scene-wrapper ${hotSwapping ? 'hot-swapping' : ''}`}>
        <SceneRenderer
          backgroundImage={sceneBackground}
          backgroundVideo={sceneVideo}
        >
          <div className="gameplay-hud">
            <span className="gameplay-chapter-title">{chapterTitle}</span>
            <div className="gameplay-hud-buttons">
              <button onClick={(e) => { e.stopPropagation(); setShowGallery(true); }}>CG回廊</button>
              <button onClick={(e) => { e.stopPropagation(); setShowInventory(true); }}>物品栏</button>
              <button onClick={(e) => { e.stopPropagation(); setShowStats(true); }}>状态</button>
              <button onClick={(e) => { e.stopPropagation(); setShowMenu(true); }}>菜单</button>
            </div>
          </div>

          <div className="gameplay-content">
            {currentEvent?.type === 'narration' && (
              <NarrationBox
                text={currentEvent.text}
                isTyping={true}
                onTypingComplete={handleTypingComplete}
              />
            )}

            {currentEvent?.type === 'dialogue' && (
              <DialogueBox
                speaker={currentEvent.speaker}
                text={currentEvent.text}
                speakerAvatar={currentEvent.avatarUrl}
                emotion={currentEvent.emotion}
                isTyping={true}
                onTypingComplete={handleTypingComplete}
              />
            )}

            {currentEvent?.type === 'choice' && (
              <ChoicePanel
                prompt={currentEvent.prompt}
                options={currentEvent.options}
                onSelect={handleChoice}
              />
            )}

            {currentEvent?.type === 'cg' && (
              <CGPlayer
                videoUrl={currentEvent.videoUrl}
                duration={currentEvent.duration}
                skipAllowed={currentEvent.skipAllowed}
                onComplete={() => executorRef.current?.advance()}
                onSkip={() => executorRef.current?.advance()}
              />
            )}
          </div>
        </SceneRenderer>

        {/* 资源状态指示器和重新生成按钮 */}
        {sceneBackground && currentBgAssetRefId && (
          <div
            className="asset-status-overlay"
            onContextMenu={(e) => handleContextMenu(e, currentBgAssetRefId, 'Image', '', undefined)}
          >
            <span className={`asset-status-badge ${currentBgState?.status ?? 'fallback'}`}>
              {currentBgState?.status === 'generating' && '生成中...'}
              {currentBgState?.status === 'ready' && 'AI 已就绪'}
              {currentBgState?.status === 'fallback' && '使用默认'}
              {!currentBgState && '加载中...'}
            </span>
            <button
              className="asset-regenerate-btn"
              onClick={(e) => {
                e.stopPropagation();
                handleRegenerate(currentBgAssetRefId);
              }}
              title="重新生成"
            >
              ↻
            </button>
          </div>
        )}
      </div>

      {/* 右键菜单 */}
      {contextMenu?.visible && (
        <div
          style={{
            position: 'fixed',
            left: contextMenu.x,
            top: contextMenu.y,
            zIndex: 1200,
            backgroundColor: '#1e1e2e',
            border: '1px solid #2a2a3a',
            borderRadius: '8px',
            padding: '0.4rem 0',
            minWidth: '160px',
            boxShadow: '0 4px 16px rgba(0,0,0,0.5)',
          }}
          onClick={(e) => e.stopPropagation()}
        >
          <button
            style={{
              display: 'block',
              width: '100%',
              padding: '0.5rem 1rem',
              fontSize: '0.9rem',
              color: '#e0e0f0',
              background: 'none',
              border: 'none',
              textAlign: 'left',
              cursor: 'pointer',
            }}
            onClick={handleMenuRegenerate}
            onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = 'rgba(74, 144, 217, 0.15)'; }}
            onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = 'none'; }}
          >
            ↻ 重新生成
          </button>
          {isVisualAsset(contextMenu.assetType) && (
            <button
              style={{
                display: 'block',
                width: '100%',
                padding: '0.5rem 1rem',
                fontSize: '0.9rem',
                color: '#e0e0f0',
                background: 'none',
                border: 'none',
                textAlign: 'left',
                cursor: 'pointer',
              }}
              onClick={handleMenuCandidates}
              onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = 'rgba(74, 144, 217, 0.15)'; }}
              onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = 'none'; }}
            >
              ◈ 多候选生成
            </button>
          )}
          <button
            style={{
              display: 'block',
              width: '100%',
              padding: '0.5rem 1rem',
              fontSize: '0.9rem',
              color: '#e0e0f0',
              background: 'none',
              border: 'none',
              textAlign: 'left',
              cursor: 'pointer',
            }}
            onClick={handleMenuEditPrompt}
            onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = 'rgba(74, 144, 217, 0.15)'; }}
            onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = 'none'; }}
          >
            ✎ 编辑 Prompt
          </button>
        </div>
      )}

      {/* PromptEditor 弹窗 */}
      {promptEditorState?.visible && (
        <PromptEditor
          prompt={promptEditorState.prompt}
          negativePrompt={promptEditorState.negativePrompt}
          onRegenerate={handlePromptRegenerate}
          onCancel={() => setPromptEditorState(null)}
        />
      )}

      {/* CandidateSelector 弹窗 */}
      {candidateState.visible && candidateState.candidates.length > 0 && (
        <CandidateSelector
          candidates={candidateState.candidates}
          onSelect={handleSelectCandidate}
          onRegenerateAll={handleRegenerateAllCandidates}
          onClose={() => setCandidateState({ visible: false, candidates: [], assetRefId: '', isRegenerating: false })}
          isRegenerating={candidateState.isRegenerating}
        />
      )}

      {showMenu && (
        <GameMenu
          onSave={handleSave}
          onLoad={handleLoad}
          onBackToMenu={handleBackToMenu}
          onExportGame={handleExportGame}
          onClose={() => setShowMenu(false)}
        />
      )}

      {showInventory && (
        <InventoryPanel
          items={stateManagerRef.current.serialize().inventory}
          onClose={() => setShowInventory(false)}
        />
      )}

      {showStats && (
        <StatsPanel
          stats={stateManagerRef.current.serialize().stats}
          onClose={() => setShowStats(false)}
        />
      )}

      {showGallery && (
        <CGGallery
          cgList={getUnlockedCGList()}
          onClose={() => setShowGallery(false)}
        />
      )}
    </div>
  );
}

export default GamePlay;
