import { useParams, useNavigate } from 'react-router-dom';
import { useEffect, useState, useRef, useCallback } from 'react';
import { invoke, listen, convertFileSrc } from '../adapters/tauri';
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

interface HistoryEntry {
  type: 'narration' | 'dialogue' | 'choice';
  speaker?: string;
  text: string;
  chosenOption?: string;
  timestamp: number;
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
  const [currentBgAssetRefId, setCurrentBgAssetRefId] = useState<string | undefined>();
  const [chapterTitle, setChapterTitle] = useState('');
  const [showMenu, setShowMenu] = useState(false);
  const [showInventory, setShowInventory] = useState(false);
  const [showStats, setShowStats] = useState(false);
  const [showGallery, setShowGallery] = useState(false);
  const [showProgress, setShowProgress] = useState(false);
  const [showHistory, setShowHistory] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [isRepairing, setIsRepairing] = useState(false);

  // 章节过渡状态
  const [chapterTransition, setChapterTransition] = useState<{
    show: boolean;
    chapterTitle: string;
    nextChapterTitle?: string;
  } | null>(null);
  // 游戏结束状态
  const [gameEnded, setGameEnded] = useState(false);

  // 资源状态追踪：assetRefId -> { source, status }
  const [assetStates, setAssetStates] = useState<Record<string, { source: string; status: 'generating' | 'ready' | 'fallback' }>>({});
  // 资源URL映射：assetRefId -> resolvedUrl（用于语音/BGM等资源的播放）
  const assetUrlMapRef = useRef<Record<string, string>>({});
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

  // 对话历史记录
  const [history, setHistory] = useState<HistoryEntry[]>([]);

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

      // 更新资源URL映射
      assetUrlMapRef.current[assetRefId] = resolveAssetUrl(localPath);

      // 同步更新 AssetLoader 缓存，以便后续场景切换时能正确解析 URL
      assetLoaderRef.current.setCachedUrl(assetRefId, resolveAssetUrl(localPath));

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

    // 监听 chapter-ready 事件，动态加载新章节
    listen('chapter-ready', (event: any) => {
      const payload = event.payload;
      if (!payload || payload.gameId !== gameId) return;

      // 重新加载 GameScript 以获取新章节
      game.getGameScript(gameId).then(script => {
        if (executorRef.current) {
          executorRef.current.updateScript(script);
        }
      }).catch(err => {
        console.error('Failed to reload game script after chapter-ready:', err);
      });
    }).then(unlisten => unlisteners.push(unlisten));

    // 监听 all-chapters-ready 事件
    listen('all-chapters-ready', (event: any) => {
      const payload = event.payload;
      if (!payload || payload.gameId !== gameId) return;

      // 重新加载 GameScript
      game.getGameScript(gameId).then(script => {
        if (executorRef.current) {
          executorRef.current.updateScript(script);
        }
      }).catch(err => {
        console.error('Failed to reload game script after all-chapters-ready:', err);
      });
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

    game.getGameScript(gameId).then(async (script) => {
      // 初始化所有已存在的资源 URL 到 AssetLoader 和 assetUrlMapRef
      for (const chapter of script.chapters) {
        for (const scene of chapter.scenes) {
          // 场景资产
          if (scene.assets.backgroundImage?.url) {
            const assetRef = scene.assets.backgroundImage;
            try {
              const dataUrl = await invoke<string>('read_file_as_data_url', {
                filePath: assetRef.url
              });
              assetLoaderRef.current.setCachedUrl(assetRef.id, dataUrl);
              assetUrlMapRef.current[assetRef.id] = dataUrl;
            } catch {
              const resolvedUrl = resolveAssetUrl(assetRef.url);
              assetLoaderRef.current.setCachedUrl(assetRef.id, resolvedUrl);
              assetUrlMapRef.current[assetRef.id] = resolvedUrl;
            }
          }
          if (scene.assets.backgroundVideo?.url) {
            const assetRef = scene.assets.backgroundVideo;
            try {
              const dataUrl = await invoke<string>('read_file_as_data_url', {
                filePath: assetRef.url
              });
              assetLoaderRef.current.setCachedUrl(assetRef.id, dataUrl);
              assetUrlMapRef.current[assetRef.id] = dataUrl;
            } catch {
              const resolvedUrl = resolveAssetUrl(assetRef.url);
              assetLoaderRef.current.setCachedUrl(assetRef.id, resolvedUrl);
              assetUrlMapRef.current[assetRef.id] = resolvedUrl;
            }
          }
          if (scene.assets.bgm?.url) {
            const assetRef = scene.assets.bgm;
            try {
              const dataUrl = await invoke<string>('read_file_as_data_url', {
                filePath: assetRef.url
              });
              assetLoaderRef.current.setCachedUrl(assetRef.id, dataUrl);
              assetUrlMapRef.current[assetRef.id] = dataUrl;
            } catch {
              const resolvedUrl = resolveAssetUrl(assetRef.url);
              assetLoaderRef.current.setCachedUrl(assetRef.id, resolvedUrl);
              assetUrlMapRef.current[assetRef.id] = resolvedUrl;
            }
          }
          if (scene.assets.ambientSound?.url) {
            const assetRef = scene.assets.ambientSound;
            try {
              const dataUrl = await invoke<string>('read_file_as_data_url', {
                filePath: assetRef.url
              });
              assetLoaderRef.current.setCachedUrl(assetRef.id, dataUrl);
              assetUrlMapRef.current[assetRef.id] = dataUrl;
            } catch {
              const resolvedUrl = resolveAssetUrl(assetRef.url);
              assetLoaderRef.current.setCachedUrl(assetRef.id, resolvedUrl);
              assetUrlMapRef.current[assetRef.id] = resolvedUrl;
            }
          }
          if (scene.assets.cgAnimation?.url) {
            const assetRef = scene.assets.cgAnimation;
            try {
              const dataUrl = await invoke<string>('read_file_as_data_url', {
                filePath: assetRef.url
              });
              assetLoaderRef.current.setCachedUrl(assetRef.id, dataUrl);
              assetUrlMapRef.current[assetRef.id] = dataUrl;
            } catch {
              const resolvedUrl = resolveAssetUrl(assetRef.url);
              assetLoaderRef.current.setCachedUrl(assetRef.id, resolvedUrl);
              assetUrlMapRef.current[assetRef.id] = resolvedUrl;
            }
          }
          
          // 节点资产
          for (const node of scene.sequence) {
            if ('speakerAvatar' in node && node.speakerAvatar?.url) {
              const assetRef = node.speakerAvatar;
              try {
                const dataUrl = await invoke<string>('read_file_as_data_url', {
                  filePath: assetRef.url
                });
                assetLoaderRef.current.setCachedUrl(assetRef.id, dataUrl);
                assetUrlMapRef.current[assetRef.id] = dataUrl;
              } catch {
                const resolvedUrl = resolveAssetUrl(assetRef.url);
                assetLoaderRef.current.setCachedUrl(assetRef.id, resolvedUrl);
                assetUrlMapRef.current[assetRef.id] = resolvedUrl;
              }
            }
            if ('voiceAsset' in node && node.voiceAsset?.url) {
              const assetRef = node.voiceAsset;
              try {
                const dataUrl = await invoke<string>('read_file_as_data_url', {
                  filePath: assetRef.url
                });
                assetLoaderRef.current.setCachedUrl(assetRef.id, dataUrl);
                assetUrlMapRef.current[assetRef.id] = dataUrl;
              } catch {
                const resolvedUrl = resolveAssetUrl(assetRef.url);
                assetLoaderRef.current.setCachedUrl(assetRef.id, resolvedUrl);
                assetUrlMapRef.current[assetRef.id] = resolvedUrl;
              }
            }
          }
        }
      }

      const executor = new SceneExecutor(
        script,
        stateManagerRef.current,
        assetLoaderRef.current,
        audioEngineRef.current,
      );

      executor.setOnEvent((event) => {
        setCurrentEvent(event);

        // 记录对话/旁白/选择到历史
        if (event.type === 'narration') {
          setHistory(prev => [...prev, { type: 'narration', text: event.text, timestamp: Date.now() }]);
        } else if (event.type === 'dialogue') {
          setHistory(prev => [...prev, { type: 'dialogue', speaker: event.speaker, text: event.text, timestamp: Date.now() }]);
        } else if (event.type === 'choice') {
          // 选择事件先记录提示，选择结果在 onChoiceSelected 中记录
        }

        if (event.type === 'scene_change') {
          setSceneBackground(event.backgroundImage);
          setSceneVideo(event.backgroundVideo);
          setCurrentBgAssetRefId(event.bgAssetRefId);
        }
        if (event.type === 'scene_transition') {
          // 场景转场由 SceneExecutor 自动处理（延迟后 enterScene）
        }
        if (event.type === 'chapter_end') {
          // 章节结束，显示过渡UI
          setChapterTransition({
            show: true,
            chapterTitle: event.chapterTitle,
            nextChapterTitle: event.nextChapterTitle,
          });
        }
        if (event.type === 'game_end') {
          // 游戏结束
          setGameEnded(true);
        }

        // 播放语音
        if ((event.type === 'narration' || event.type === 'dialogue') && event.voiceUrl) {
          // 直接用 event.voiceUrl，它已经通过 AssetLoader 加载过
          audioEngineRef.current.playVoice(event.voiceUrl).catch(() => {});
        } else if ((event.type === 'narration' || event.type === 'dialogue') && event.voiceAssetRefId) {
          // voiceUrl 为空但 voiceAssetRefId 存在，从 assetUrlMap 中查找
          const mappedUrl = assetUrlMapRef.current[event.voiceAssetRefId];
          if (mappedUrl) {
            audioEngineRef.current.playVoice(mappedUrl).catch(() => {});
          } else {
            audioEngineRef.current.stopVoice();
          }
        } else if (event.type === 'narration' || event.type === 'dialogue') {
          // 没有语音资源时停止当前语音
          audioEngineRef.current.stopVoice();
        }
      });

      executorRef.current = executor;
      // 设置初始章节标题
      const firstChapter = script.chapters[0];
      if (firstChapter) {
        setChapterTitle(firstChapter.title);
      }
      executor.start().catch(err => {
        console.error('Failed to start game:', err);
      });
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
        if (currentEvent?.type === 'narration' || currentEvent?.type === 'dialogue' || currentEvent?.type === 'cg') {
          executorRef.current?.advance();
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [currentEvent]);

  const handleChoice = useCallback((index: number) => {
    // 记录选择到历史
    if (currentEvent?.type === 'choice') {
      const chosenText = currentEvent.options[index]?.text;
      setHistory(prev => [...prev, { type: 'choice', text: currentEvent.prompt, chosenOption: chosenText, timestamp: Date.now() }]);
    }
    executorRef.current?.onChoiceSelected(index);
  }, [currentEvent]);

  const handleTypingComplete = useCallback(() => {
    // 不自动推进，等待玩家点击
  }, []);

  const handleAdvance = useCallback(() => {
    if (showMenu || showInventory || showStats || showGallery || showProgress || chapterTransition || gameEnded) return;
    executorRef.current?.advance();
  }, [showMenu, showInventory, showStats, showGallery, showProgress, chapterTransition, gameEnded]);

  const handleClick = useCallback(() => {
    // 点击空白区域时，只有在字幕显示完成后才执行 advance
    if (showMenu || showInventory || showStats || showGallery || showProgress || chapterTransition || gameEnded) return;
    // 对于 narration/dialogue 类型，我们不在这里处理，而是由专门的字幕组件处理点击
    // 对于 cg 类型，直接推进
    if (currentEvent?.type === 'cg') {
      executorRef.current?.advance();
    }
  }, [currentEvent, showMenu, showInventory, showStats, showProgress, chapterTransition, gameEnded]);

  // 进入下一章
  const handleEnterNextChapter = useCallback(() => {
    if (!chapterTransition?.nextChapterTitle) return;
    executorRef.current?.enterNextChapter();
    setChapterTitle(chapterTransition.nextChapterTitle);
    setChapterTransition(null);
    setCurrentEvent(null);
  }, [chapterTransition]);

  // 游戏结束 - 返回主菜单
  const handleGameEndBackToMenu = useCallback(() => {
    audioEngineRef.current.stopBgm();
    navigate('/');
  }, []);

  // 取消后续章节生成
  const handleCancelGeneration = useCallback(async () => {
    if (!gameId) return;
    try {
      await generation.cancelRemainingChapters(gameId);
    } catch (err) {
      console.error('取消后续生成失败:', err);
    }
  }, [gameId]);

  // 修复游戏并重新加载
  const handleRepairGame = useCallback(async () => {
    if (!gameId) return;
    setIsRepairing(true);
    try {
      const count = await game.repairGame(gameId);
      alert(`修复完成！成功移动了 ${count} 个资源到正确位置`);
      // 重新加载游戏脚本和资源
      await reloadGame();
    } catch (err) {
      console.error('修复游戏失败:', err);
      alert(`修复失败: ${err}`);
    } finally {
      setIsRepairing(false);
      setShowMenu(false);
    }
  }, [gameId]);

  // 重新加载游戏
  const reloadGame = useCallback(async () => {
    if (!gameId) return;
    setIsLoading(true);
    
    try {
      const script = await game.getGameScript(gameId);
      
      // 重新初始化所有资源 URL 到 AssetLoader 和 assetUrlMapRef
      for (const chapter of script.chapters) {
        for (const scene of chapter.scenes) {
          // 场景资产
          if (scene.assets.backgroundImage?.url) {
            const assetRef = scene.assets.backgroundImage;
            try {
              const dataUrl = await invoke<string>('read_file_as_data_url', {
                filePath: assetRef.url
              });
              assetLoaderRef.current.setCachedUrl(assetRef.id, dataUrl);
              assetUrlMapRef.current[assetRef.id] = dataUrl;
            } catch {
              const resolvedUrl = resolveAssetUrl(assetRef.url);
              assetLoaderRef.current.setCachedUrl(assetRef.id, resolvedUrl);
              assetUrlMapRef.current[assetRef.id] = resolvedUrl;
            }
          }
          if (scene.assets.backgroundVideo?.url) {
            const assetRef = scene.assets.backgroundVideo;
            try {
              const dataUrl = await invoke<string>('read_file_as_data_url', {
                filePath: assetRef.url
              });
              assetLoaderRef.current.setCachedUrl(assetRef.id, dataUrl);
              assetUrlMapRef.current[assetRef.id] = dataUrl;
            } catch {
              const resolvedUrl = resolveAssetUrl(assetRef.url);
              assetLoaderRef.current.setCachedUrl(assetRef.id, resolvedUrl);
              assetUrlMapRef.current[assetRef.id] = resolvedUrl;
            }
          }
          if (scene.assets.bgm?.url) {
            const assetRef = scene.assets.bgm;
            try {
              const dataUrl = await invoke<string>('read_file_as_data_url', {
                filePath: assetRef.url
              });
              assetLoaderRef.current.setCachedUrl(assetRef.id, dataUrl);
              assetUrlMapRef.current[assetRef.id] = dataUrl;
            } catch {
              const resolvedUrl = resolveAssetUrl(assetRef.url);
              assetLoaderRef.current.setCachedUrl(assetRef.id, resolvedUrl);
              assetUrlMapRef.current[assetRef.id] = resolvedUrl;
            }
          }
          if (scene.assets.ambientSound?.url) {
            const assetRef = scene.assets.ambientSound;
            try {
              const dataUrl = await invoke<string>('read_file_as_data_url', {
                filePath: assetRef.url
              });
              assetLoaderRef.current.setCachedUrl(assetRef.id, dataUrl);
              assetUrlMapRef.current[assetRef.id] = dataUrl;
            } catch {
              const resolvedUrl = resolveAssetUrl(assetRef.url);
              assetLoaderRef.current.setCachedUrl(assetRef.id, resolvedUrl);
              assetUrlMapRef.current[assetRef.id] = resolvedUrl;
            }
          }
          if (scene.assets.cgAnimation?.url) {
            const assetRef = scene.assets.cgAnimation;
            try {
              const dataUrl = await invoke<string>('read_file_as_data_url', {
                filePath: assetRef.url
              });
              assetLoaderRef.current.setCachedUrl(assetRef.id, dataUrl);
              assetUrlMapRef.current[assetRef.id] = dataUrl;
            } catch {
              const resolvedUrl = resolveAssetUrl(assetRef.url);
              assetLoaderRef.current.setCachedUrl(assetRef.id, resolvedUrl);
              assetUrlMapRef.current[assetRef.id] = resolvedUrl;
            }
          }
          
          // 节点资产
          for (const node of scene.sequence) {
            if ('speakerAvatar' in node && node.speakerAvatar?.url) {
              const assetRef = node.speakerAvatar;
              try {
                const dataUrl = await invoke<string>('read_file_as_data_url', {
                  filePath: assetRef.url
                });
                assetLoaderRef.current.setCachedUrl(assetRef.id, dataUrl);
                assetUrlMapRef.current[assetRef.id] = dataUrl;
              } catch {
                const resolvedUrl = resolveAssetUrl(assetRef.url);
                assetLoaderRef.current.setCachedUrl(assetRef.id, resolvedUrl);
                assetUrlMapRef.current[assetRef.id] = resolvedUrl;
              }
            }
            if ('voiceAsset' in node && node.voiceAsset?.url) {
              const assetRef = node.voiceAsset;
              try {
                const dataUrl = await invoke<string>('read_file_as_data_url', {
                  filePath: assetRef.url
                });
                assetLoaderRef.current.setCachedUrl(assetRef.id, dataUrl);
                assetUrlMapRef.current[assetRef.id] = dataUrl;
              } catch {
                const resolvedUrl = resolveAssetUrl(assetRef.url);
                assetLoaderRef.current.setCachedUrl(assetRef.id, resolvedUrl);
                assetUrlMapRef.current[assetRef.id] = resolvedUrl;
              }
            }
          }
        }
      }

      // 更新 executor 的脚本
      if (executorRef.current) {
        executorRef.current.updateScript(script);
      }
    } catch (err) {
      console.error('重新加载游戏失败:', err);
    } finally {
      setIsLoading(false);
    }
  }, [gameId]);

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
              <button onClick={(e) => { e.stopPropagation(); setShowProgress(true); }}>进度</button>
              <button onClick={(e) => { e.stopPropagation(); setShowHistory(true); }}>记录</button>
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
                onAdvance={handleAdvance}
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
                onAdvance={handleAdvance}
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

        {/* 资源状态指示器和重新生成按钮 - 调整位置到右下角，不遮挡菜单 */}
        {sceneBackground && currentBgAssetRefId && (
          <div
            className="asset-status-overlay"
            style={{ bottom: '20px', right: '20px', top: 'auto', left: 'auto' }}
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
            backgroundColor: '#faf8f5',
            border: '1px solid #e8e2d8',
            borderRadius: '8px',
            padding: '0.4rem 0',
            minWidth: '160px',
            boxShadow: '0 4px 16px rgba(0,0,0,0.12)',
          }}
          onClick={(e) => e.stopPropagation()}
        >
          <button
            style={{
              display: 'block',
              width: '100%',
              padding: '0.5rem 1rem',
              fontSize: '0.9rem',
              color: '#2d3748',
              background: 'none',
              border: 'none',
              textAlign: 'left',
              cursor: 'pointer',
            }}
            onClick={handleMenuRegenerate}
            onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = 'rgba(43, 108, 176, 0.1)'; }}
            onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = 'transparent'; }}
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
                color: '#2d3748',
                background: 'none',
                border: 'none',
                textAlign: 'left',
                cursor: 'pointer',
              }}
              onClick={handleMenuCandidates}
              onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = 'rgba(43, 108, 176, 0.1)'; }}
              onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = 'transparent'; }}
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
              color: '#2d3748',
              background: 'none',
              border: 'none',
              textAlign: 'left',
              cursor: 'pointer',
            }}
            onClick={handleMenuEditPrompt}
            onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = 'rgba(43, 108, 176, 0.1)'; }}
            onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = 'transparent'; }}
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
          onCancelGeneration={handleCancelGeneration}
          onRepairGame={handleRepairGame}
          isRepairing={isRepairing}
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

      {/* 章节过渡 */}
      {chapterTransition?.show && (
        <div className="overlay chapter-transition-overlay" onClick={(e) => e.stopPropagation()}>
          <div className="chapter-transition-card">
            <div className="chapter-transition-complete">章节完成</div>
            <h2 className="chapter-transition-title">{chapterTransition.chapterTitle}</h2>
            {chapterTransition.nextChapterTitle ? (
              <>
                <div className="chapter-transition-divider" />
                <p className="chapter-transition-next">下一章</p>
                <h3 className="chapter-transition-next-title">{chapterTransition.nextChapterTitle}</h3>
                <button className="btn btn-primary chapter-transition-btn" onClick={handleEnterNextChapter}>
                  继续旅程
                </button>
              </>
            ) : (
              <>
                <div className="chapter-transition-divider" />
                <p className="chapter-transition-next">已是最后一章</p>
                <button className="btn btn-primary chapter-transition-btn" onClick={handleGameEndBackToMenu}>
                  返回主菜单
                </button>
              </>
            )}
          </div>
        </div>
      )}

      {/* 游戏结束 */}
      {gameEnded && (
        <div className="overlay game-end-overlay" onClick={(e) => e.stopPropagation()}>
          <div className="game-end-card">
            <h2 className="game-end-title">故事结束</h2>
            <p className="game-end-chapter">最终章: {chapterTitle}</p>
            <div className="game-end-divider" />
            <p className="game-end-thanks">感谢游玩</p>
            <div className="game-end-buttons">
              <button className="btn btn-primary" onClick={handleGameEndBackToMenu}>
                返回主菜单
              </button>
              <button className="btn btn-secondary" onClick={() => { setGameEnded(false); }}>
                继续探索
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 游戏进度面板 */}
      {showProgress && executorRef.current && (
        <div className="overlay" onClick={(e) => e.stopPropagation()}>
          <div className="progress-panel">
            <div className="progress-panel-header">
              <h3>游戏进度</h3>
              <button className="close-btn" onClick={() => setShowProgress(false)}>✕</button>
            </div>
            {(() => {
              const progress = executorRef.current!.getProgress();
              const chapterPercent = Math.round((progress.currentChapterIndex / progress.totalChapters) * 100);
              const scenePercent = progress.totalScenesInChapter > 0
                ? Math.round((progress.visitedScenesInChapter / progress.totalScenesInChapter) * 100)
                : 0;
              return (
                <div className="progress-panel-body">
                  <div className="progress-section">
                    <div className="progress-label">当前章节</div>
                    <div className="progress-value">{progress.currentChapterTitle}</div>
                    <div className="progress-bar-container">
                      <div className="progress-bar" style={{ width: `${chapterPercent}%` }} />
                    </div>
                    <div className="progress-detail">第 {progress.currentChapterIndex} / {progress.totalChapters} 章 ({chapterPercent}%)</div>
                  </div>
                  <div className="progress-section">
                    <div className="progress-label">章节进度</div>
                    <div className="progress-bar-container">
                      <div className="progress-bar progress-bar-secondary" style={{ width: `${scenePercent}%` }} />
                    </div>
                    <div className="progress-detail">已探索 {progress.visitedScenesInChapter} / {progress.totalScenesInChapter} 个场景 ({scenePercent}%)</div>
                  </div>
                  <div className="progress-section">
                    <div className="progress-label">总场景数</div>
                    <div className="progress-detail">{progress.visitedScenes.length} / {progress.totalScenes} 个场景已探索</div>
                  </div>
                </div>
              );
            })()}
          </div>
        </div>
      )}

      {/* 对话历史记录面板 */}
      {showHistory && (
        <div className="overlay" onClick={(e) => e.stopPropagation()}>
          <div className="history-panel">
            <div className="history-panel-header">
              <h3>对话记录</h3>
              <button className="close-btn" onClick={() => setShowHistory(false)}>✕</button>
            </div>
            <div className="history-panel-body">
              {history.length === 0 ? (
                <div className="history-empty">暂无记录</div>
              ) : (
                [...history].reverse().map((entry, idx) => (
                  <div key={idx} className={`history-entry history-entry-${entry.type}`}>
                    {entry.type === 'dialogue' && (
                      <>
                        <span className="history-speaker">{entry.speaker}</span>
                        <span className="history-text">{entry.text}</span>
                      </>
                    )}
                    {entry.type === 'narration' && (
                      <span className="history-text history-narration-text">{entry.text}</span>
                    )}
                    {entry.type === 'choice' && (
                      <div className="history-choice">
                        <span className="history-choice-prompt">{entry.text}</span>
                        <span className="history-choice-selected">▸ {entry.chosenOption}</span>
                      </div>
                    )}
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default GamePlay;
