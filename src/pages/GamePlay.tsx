import { useParams, useNavigate } from 'react-router-dom';
import { useEffect, useState, useRef, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/core';
import { SceneExecutor, SceneEventType } from '../engine/SceneExecutor';
import { StateManager } from '../engine/StateManager';
import { AssetLoader } from '../engine/AssetLoader';
import { AudioEngine } from '../engine/AudioEngine';
import SceneRenderer from '../components/Scene/SceneRenderer';
import DialogueBox from '../components/Dialogue/DialogueBox';
import NarrationBox from '../components/Dialogue/NarrationBox';
import ChoicePanel from '../components/Choice/ChoicePanel';
import CGPlayer from '../components/CG/CGPlayer';
import { GameMenu } from '../components/HUD/GameMenu';
import { InventoryPanel } from '../components/HUD/InventoryPanel';
import { StatsPanel } from '../components/HUD/StatsPanel';
import { useGame } from '../hooks/useGame';
import { useGeneration } from '../hooks/useGeneration';

interface AssetInfo {
  assetRefId: string;
  assetType: string;
  localPath: string;
  source: string;
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
  const [isLoading, setIsLoading] = useState(true);

  // 资源状态追踪：assetRefId -> { source, status }
  const [assetStates, setAssetStates] = useState<Record<string, { source: string; status: 'generating' | 'ready' | 'fallback' }>>({});
  // 热替换动画状态
  const [hotSwapping, setHotSwapping] = useState(false);

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
    if (showMenu || showInventory || showStats) return;
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

  // 重新生成资源
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
          <div className="asset-status-overlay">
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

      {showMenu && (
        <GameMenu
          onSave={handleSave}
          onLoad={handleLoad}
          onBackToMenu={handleBackToMenu}
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
    </div>
  );
}

export default GamePlay;
