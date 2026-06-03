import type { GameScript, Scene, ChoiceNode, ConditionNode, ActionNode, Condition, Effect } from '../../shared/types/game-script';
import { StateManager } from './StateManager';
import { AssetLoader } from './AssetLoader';
import { AudioEngine } from './AudioEngine';

export type SceneEventType =
  | { type: 'narration'; text: string; voiceUrl?: string }
  | { type: 'dialogue'; speaker: string; text: string; avatarUrl?: string; emotion?: string; voiceUrl?: string }
  | { type: 'choice'; prompt: string; options: { text: string; enabled: boolean; visible: boolean }[] }
  | { type: 'cg'; videoUrl: string; duration?: number; skipAllowed: boolean }
  | { type: 'scene_transition'; targetSceneId: string; transitionType: string; duration?: number }
  | { type: 'scene_change'; backgroundImage?: string; backgroundVideo?: string; bgmUrl?: string }
  | { type: 'action'; actionType: string; params: Record<string, any> }
  | { type: 'chapter_end' };

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

  getCurrentChapterId(): string | null {
    return this.currentChapterId;
  }

  // 开始游戏
  start() {
    const firstChapter = this.script.chapters[0];
    if (!firstChapter) return;
    this.enterChapter(firstChapter.id);
  }

  // 进入章节
  enterChapter(chapterId: string) {
    this.currentChapterId = chapterId;
    const chapter = this.script.chapters.find(c => c.id === chapterId);
    if (!chapter) return;
    this.stateManager.setCurrentChapterId(chapterId);
    this.enterScene(chapter.scenes[0]?.id);
  }

  // 进入场景
  enterScene(sceneId: string) {
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
    this.assetLoader.preloadAssets(assetRefs);

    // 发送场景变更事件（背景图/BGM）
    this.emitEvent({
      type: 'scene_change',
      backgroundImage: scene.assets.backgroundImage?.url,
      backgroundVideo: scene.assets.backgroundVideo?.url,
      bgmUrl: scene.assets.bgm?.url,
    });

    // 播放 BGM
    if (scene.assets.bgm?.url) {
      this.audioEngine.playBgm(scene.assets.bgm.url);
    }

    // 开始执行场景序列
    if (scene.sequence.length > 0) {
      this.executeNode(scene.sequence[0].id);
    }
  }

  // 执行节点
  executeNode(nodeId: string) {
    this.currentNodeId = nodeId;
    const scene = this.findScene(this.currentSceneId!);
    if (!scene) return;

    const node = scene.sequence.find(n => n.id === nodeId);
    if (!node) return;

    this.stateManager.setCurrentNodeId(nodeId);

    switch (node.type) {
      case 'narration':
        this.emitEvent({ type: 'narration', text: node.text, voiceUrl: node.voiceAsset?.url });
        break;
      case 'dialogue':
        this.emitEvent({
          type: 'dialogue',
          speaker: node.speaker,
          text: node.text,
          avatarUrl: node.speakerAvatar?.url,
          emotion: node.emotion,
          voiceUrl: node.voiceAsset?.url,
        });
        break;
      case 'choice':
        this.handleChoiceNode(node);
        break;
      case 'condition':
        this.handleConditionNode(node);
        break;
      case 'action':
        this.handleActionNode(node);
        break;
      case 'cg':
        this.emitEvent({ type: 'cg', videoUrl: node.videoAsset.url ?? '', duration: node.duration, skipAllowed: node.skipAllowed });
        break;
      case 'scene_transition':
        this.emitEvent({ type: 'scene_transition', targetSceneId: node.targetSceneId, transitionType: node.transitionType, duration: node.duration });
        break;
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
  onChoiceSelected(optionIndex: number) {
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
      this.executeNode(option.nextNodeId);
    } else {
      this.advanceToNextNode();
    }
  }

  // 推进到下一个节点（旁白/对话/CG完成后调用）
  advance() {
    this.advanceToNextNode();
  }

  // 处理条件节点
  private handleConditionNode(node: ConditionNode) {
    const result = this.evaluateCondition(node.condition);
    const nextNodeId = result ? node.trueBranch : node.falseBranch;
    if (nextNodeId) {
      this.executeNode(nextNodeId);
    } else {
      this.advanceToNextNode();
    }
  }

  // 处理动作节点
  private handleActionNode(node: ActionNode) {
    this.applyEffect({ type: node.actionType as Effect['type'], target: node.params.target ?? '', value: node.params.value });

    this.emitEvent({ type: 'action', actionType: node.actionType, params: node.params });

    if (node.actionType === 'change_scene' && node.params.sceneId) {
      this.enterScene(node.params.sceneId);
      return;
    }

    if (node.nextNodeId) {
      this.executeNode(node.nextNodeId);
    } else {
      this.advanceToNextNode();
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
  private advanceToNextNode() {
    const scene = this.findScene(this.currentSceneId!);
    if (!scene) return;

    const currentIndex = scene.sequence.findIndex(n => n.id === this.currentNodeId);
    if (currentIndex === -1) return;

    const nextNode = scene.sequence[currentIndex + 1];
    if (nextNode) {
      this.executeNode(nextNode.id);
    } else {
      // 场景序列结束，检查是否是章节末尾
      this.emitEvent({ type: 'chapter_end' });
    }
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
