import type { GameScript, Scene, ChoiceNode, ConditionNode, ActionNode, Condition, Effect } from '../../shared/types/game-script';
import { StateManager } from './StateManager';
import { AssetLoader } from './AssetLoader';
import { AudioEngine } from './AudioEngine';

export type SceneEventType =
  | { type: 'narration'; text: string; voiceUrl?: string; voiceAssetRefId?: string }
  | { type: 'dialogue'; speaker: string; text: string; avatarUrl?: string; emotion?: string; voiceUrl?: string; voiceAssetRefId?: string }
  | { type: 'choice'; prompt: string; options: { text: string; enabled: boolean; visible: boolean }[] }
  | { type: 'cg'; videoUrl: string; duration?: number; skipAllowed: boolean }
  | { type: 'scene_transition'; targetSceneId: string; transitionType: string; duration?: number }
  | { type: 'scene_change'; backgroundImage?: string; backgroundVideo?: string; bgmUrl?: string; bgAssetRefId?: string }
  | { type: 'action'; actionType: string; params: Record<string, any> }
  | { type: 'chapter_end'; chapterId: string; chapterTitle: string; nextChapterId?: string; nextChapterTitle?: string }
  | { type: 'game_end'; finalChapterTitle: string };

export class SceneExecutor {
  private script: GameScript;
  private stateManager: StateManager;
  private assetLoader: AssetLoader;
  private audioEngine: AudioEngine;
  private currentNodeId: string | null = null;
  private currentSceneId: string | null = null;
  private currentChapterId: string | null = null;

  private onEvent: ((event: SceneEventType) => void) | null = null;

  constructor(script: GameScript, stateManager: StateManager, assetLoader: AssetLoader, audioEngine: AudioEngine) {
    this.script = script;
    this.stateManager = stateManager;
    this.assetLoader = assetLoader;
    this.audioEngine = audioEngine;
  }

  setOnEvent(callback: (event: SceneEventType) => void) {
    this.onEvent = callback;
  }

  // 更新 GameScript（用于后续章节动态加载）
  updateScript(script: GameScript) {
    this.script = script;
  }

  getCurrentChapterId(): string | null {
    return this.currentChapterId;
  }

  // 开始游戏
  async start() {
    const firstChapter = this.script.chapters[0];
    if (!firstChapter) return;
    await this.enterChapter(firstChapter.id);
  }

  // 进入章节
  async enterChapter(chapterId: string) {
    this.currentChapterId = chapterId;
    const chapter = this.script.chapters.find(c => c.id === chapterId);
    if (!chapter) return;
    this.stateManager.setCurrentChapterId(chapterId);
    await this.enterScene(chapter.scenes[0]?.id);
  }

  // 进入场景
  async enterScene(sceneId: string) {
    if (!sceneId) return;
    this.currentSceneId = sceneId;
    const scene = this.findScene(sceneId);
    if (!scene) return;
    this.stateManager.setCurrentSceneId(sceneId);
    this.stateManager.markSceneVisited(sceneId);

    // 预加载场景资源
    const assetRefs = [
      scene.assets.backgroundImage,
      scene.assets.backgroundVideo,
      scene.assets.bgm,
      scene.assets.ambientSound,
      scene.assets.cgAnimation,
    ].filter((ref): ref is NonNullable<typeof ref> => ref != null);
    await this.assetLoader.preloadAssets(assetRefs);

    // 通过 AssetLoader 解析资源 URL（确保经过 convertFileSrc 转换）
    const bgImageRef = scene.assets.backgroundImage;
    const bgVideoRef = scene.assets.backgroundVideo;
    const bgmRef = scene.assets.bgm;

    const resolvedBgImage = bgImageRef 
      ? (await this.assetLoader.loadAsset(bgImageRef))
      : undefined;
    const resolvedBgVideo = bgVideoRef 
      ? (await this.assetLoader.loadAsset(bgVideoRef))
      : undefined;
    const resolvedBgmUrl = bgmRef 
      ? (await this.assetLoader.loadAsset(bgmRef))
      : undefined;

    // 发送场景变更事件（背景图/BGM）
    this.emitEvent({
      type: 'scene_change',
      backgroundImage: resolvedBgImage,
      backgroundVideo: resolvedBgVideo,
      bgmUrl: resolvedBgmUrl,
      bgAssetRefId: bgImageRef?.id,
    });

    // 播放 BGM
    if (resolvedBgmUrl) {
      this.audioEngine.playBgm(resolvedBgmUrl);
    }

    // 开始执行场景序列
    if (scene.sequence.length > 0) {
      await this.executeNode(scene.sequence[0].id);
    } else {
      // 空场景，直接尝试进入下一个场景
      await this.advanceToNextSceneOrChapter();
    }
  }

  // 执行节点
  async executeNode(nodeId: string) {
    this.currentNodeId = nodeId;
    const scene = this.findScene(this.currentSceneId!);
    if (!scene) return;

    const node = scene.sequence.find(n => n.id === nodeId);
    if (!node) return;

    this.stateManager.setCurrentNodeId(nodeId);

    switch (node.type) {
      case 'narration': {
        let voiceUrl: string | undefined;
        if (node.voiceAsset) {
          voiceUrl = await this.assetLoader.loadAsset(node.voiceAsset);
        }
        this.emitEvent({ type: 'narration', text: node.text, voiceUrl, voiceAssetRefId: node.voiceAsset?.id });
        break;
      }
      case 'dialogue': {
        let voiceUrl: string | undefined;
        let avatarUrl: string | undefined;
        if (node.voiceAsset) {
          voiceUrl = await this.assetLoader.loadAsset(node.voiceAsset);
        }
        if (node.speakerAvatar) {
          avatarUrl = await this.assetLoader.loadAsset(node.speakerAvatar);
        }
        this.emitEvent({
          type: 'dialogue',
          speaker: node.speaker,
          text: node.text,
          avatarUrl,
          emotion: node.emotion,
          voiceUrl,
          voiceAssetRefId: node.voiceAsset?.id,
        });
        break;
      }
      case 'choice':
        this.handleChoiceNode(node);
        break;
      case 'condition':
        this.handleConditionNode(node);
        break;
      case 'action':
        this.handleActionNode(node);
        break;
      case 'cg': {
        const videoUrl = node.videoAsset ? await this.assetLoader.loadAsset(node.videoAsset) : undefined;
        this.emitEvent({ type: 'cg', videoUrl: videoUrl ?? '', duration: node.duration, skipAllowed: node.skipAllowed });
        break;
      }
      case 'scene_transition': {
        // 场景转场：先发出事件（用于视觉转场效果），然后自动进入目标场景
        // 如果 targetSceneId 为空，则进入同章节的下一个场景
        let resolvedTargetId = node.targetSceneId;
        if (!resolvedTargetId) {
          const scene = this.findScene(this.currentSceneId!);
          const chapter = this.findChapterBySceneId(this.currentSceneId!);
          if (scene && chapter) {
            const sceneIndex = chapter.scenes.findIndex(s => s.id === scene.id);
            const nextScene = chapter.scenes[sceneIndex + 1];
            if (nextScene) {
              resolvedTargetId = nextScene.id;
            }
          }
        }
        this.emitEvent({ type: 'scene_transition', targetSceneId: resolvedTargetId, transitionType: node.transitionType, duration: node.duration });
        // 延迟进入目标场景，让转场动画有时间播放
        const delay = node.duration ? node.duration * 1000 : 500;
        setTimeout(() => {
          if (resolvedTargetId) {
            void this.enterScene(resolvedTargetId);
          } else {
            // 没有下一个场景，触发章节结束逻辑
            this.handleChapterEnd();
          }
        }, delay);
        break;
      }
    }
  }

  // 处理选择节点
  handleChoiceNode(node: ChoiceNode) {
    const options = node.options.map(opt => ({
      text: opt.text,
      enabled: opt.condition ? this.evaluateCondition(opt.condition) : true,
      visible: opt.condition ? this.evaluateCondition(opt.condition) : true,
    }));
    this.emitEvent({ type: 'choice', prompt: node.prompt, options });
  }

  // 玩家选择后调用
  async onChoiceSelected(optionIndex: number) {
    const scene = this.findScene(this.currentSceneId!);
    const node = scene?.sequence.find(n => n.id === this.currentNodeId && n.type === 'choice') as ChoiceNode | undefined;
    if (!node || !node.options[optionIndex]) return;

    const option = node.options[optionIndex];

    // 记录选择
    this.stateManager.recordChoice(node.id, optionIndex, option.text);

    // 执行效果
    if (option.effects) {
      option.effects.forEach(effect => this.applyEffect(effect));
    }

    // 跳转到下一个节点
    if (option.nextNodeId) {
      await this.executeNode(option.nextNodeId);
    } else {
      await this.advanceToNextNode();
    }
  }

  // 推进到下一个节点（旁白/对话/CG完成后调用）
  async advance() {
    await this.advanceToNextNode();
  }

  // 处理条件节点
  private async handleConditionNode(node: ConditionNode) {
    const result = this.evaluateCondition(node.condition);
    const nextNodeId = result ? node.trueBranch : node.falseBranch;
    if (nextNodeId) {
      await this.executeNode(nextNodeId);
    } else {
      await this.advanceToNextNode();
    }
  }

  // 处理动作节点
  private async handleActionNode(node: ActionNode) {
    this.applyEffect({ type: node.actionType as Effect['type'], target: node.params.target ?? '', value: node.params.value });

    this.emitEvent({ type: 'action', actionType: node.actionType, params: node.params });

    if (node.actionType === 'change_scene' && node.params.sceneId) {
      await this.enterScene(node.params.sceneId);
      return;
    }

    if (node.nextNodeId) {
      await this.executeNode(node.nextNodeId);
    } else {
      await this.advanceToNextNode();
    }
  }

  // 应用效果
  private applyEffect(effect: Effect) {
    switch (effect.type) {
      case 'set_variable':
        this.stateManager.setVariable(effect.target, effect.value);
        break;
      case 'add_item':
        this.stateManager.addItem(effect.target);
        break;
      case 'remove_item':
        this.stateManager.removeItem(effect.target);
        break;
      case 'modify_stat':
        this.stateManager.modifyStat(effect.target, effect.value);
        break;
    }
  }

  // 评估条件
  private evaluateCondition(condition: Condition): boolean {
    switch (condition.type) {
      case 'variable_check': {
        const val = this.stateManager.getVariable(condition.target);
        return this.compareValues(val, condition.operator, condition.value);
      }
      case 'item_check': {
        const has = this.stateManager.hasItem(condition.target);
        return condition.operator === 'has' ? has : !has;
      }
      case 'stat_check': {
        const stat = this.stateManager.getStat(condition.target);
        return this.compareValues(stat, condition.operator, condition.value);
      }
      case 'composite': {
        const andResults = condition.and?.map(c => this.evaluateCondition(c)) ?? [];
        const orResults = condition.or?.map(c => this.evaluateCondition(c)) ?? [];
        if (condition.and && condition.and.length > 0) {
          return andResults.every(Boolean);
        }
        if (condition.or && condition.or.length > 0) {
          return orResults.some(Boolean);
        }
        return false;
      }
      default:
        return false;
    }
  }

  private compareValues(actual: any, operator: string, expected: any): boolean {
    switch (operator) {
      case '==': return actual == expected;
      case '!=': return actual != expected;
      case '>': return actual > expected;
      case '<': return actual < expected;
      case '>=': return actual >= expected;
      case '<=': return actual <= expected;
      case 'has': return Array.isArray(actual) ? actual.includes(expected) : false;
      case 'not_has': return Array.isArray(actual) ? !actual.includes(expected) : true;
      default: return false;
    }
  }

  // 推进到下一个节点
  private async advanceToNextNode() {
    const scene = this.findScene(this.currentSceneId!);
    if (!scene) return;

    const currentIndex = scene.sequence.findIndex(n => n.id === this.currentNodeId);
    if (currentIndex === -1) return;

    const nextNode = scene.sequence[currentIndex + 1];
    if (nextNode) {
      await this.executeNode(nextNode.id);
    } else {
      // 场景序列结束，尝试进入下一个场景或章节
      await this.advanceToNextSceneOrChapter();
    }
  }

  // 场景序列结束后，尝试进入同章节的下一个场景，或触发章节结束
  private async advanceToNextSceneOrChapter() {
    const chapter = this.findChapterBySceneId(this.currentSceneId!);
    if (chapter) {
      const sceneIndex = chapter.scenes.findIndex(s => s.id === this.currentSceneId);
      const nextScene = chapter.scenes[sceneIndex + 1];
      if (nextScene) {
        // 同章节还有下一个场景，自动进入
        await this.enterScene(nextScene.id);
        return;
      }
    }
    // 章节结束
    this.handleChapterEnd();
  }

  // 处理章节结束
  handleChapterEnd() {
    const currentChapterIndex = this.script.chapters.findIndex(c => c.id === this.currentChapterId);
    const currentChapter = this.script.chapters[currentChapterIndex];
    if (!currentChapter) return;

    const nextChapter = this.script.chapters[currentChapterIndex + 1];

    if (nextChapter) {
      // 有下一章，发送章节结束事件（包含下一章信息）
      this.emitEvent({
        type: 'chapter_end',
        chapterId: currentChapter.id,
        chapterTitle: currentChapter.title,
        nextChapterId: nextChapter.id,
        nextChapterTitle: nextChapter.title,
      });
    } else {
      // 没有下一章，游戏结束
      this.emitEvent({
        type: 'game_end',
        finalChapterTitle: currentChapter.title,
      });
    }
  }

  // 根据场景ID查找所属章节
  private findChapterBySceneId(sceneId: string) {
    return this.script.chapters.find(c => c.scenes.some(s => s.id === sceneId));
  }

  // 进入下一章（由前端确认后调用）
  async enterNextChapter() {
    const currentChapterIndex = this.script.chapters.findIndex(c => c.id === this.currentChapterId);
    const nextChapter = this.script.chapters[currentChapterIndex + 1];
    if (nextChapter) {
      await this.enterChapter(nextChapter.id);
    }
  }

  // 获取游戏进度信息
  getProgress() {
    const totalChapters = this.script.chapters.length;
    const currentChapterIndex = this.script.chapters.findIndex(c => c.id === this.currentChapterId);
    const currentChapter = this.script.chapters[currentChapterIndex];

    let totalScenes = 0;
    let visitedScenesInChapter = 0;
    let totalScenesInChapter = 0;

    for (const chapter of this.script.chapters) {
      totalScenes += chapter.scenes.length;
    }

    if (currentChapter) {
      totalScenesInChapter = currentChapter.scenes.length;
      const visited = this.stateManager.serialize().visitedScenes;
      visitedScenesInChapter = currentChapter.scenes.filter(s => visited.includes(s.id)).length;
    }

    return {
      currentChapterIndex: currentChapterIndex + 1,
      totalChapters,
      currentChapterTitle: currentChapter?.title ?? '',
      currentSceneId: this.currentSceneId,
      totalScenes,
      totalScenesInChapter,
      visitedScenesInChapter,
      visitedScenes: this.stateManager.serialize().visitedScenes,
    };
  }

  // 查找场景
  private findScene(sceneId: string): Scene | undefined {
    for (const chapter of this.script.chapters) {
      const scene = chapter.scenes.find(s => s.id === sceneId);
      if (scene) return scene;
    }
    return undefined;
  }

  private emitEvent(event: SceneEventType) {
    this.onEvent?.(event);
  }
}
