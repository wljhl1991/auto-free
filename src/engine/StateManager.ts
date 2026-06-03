import type { ChoiceRecord, GameState } from '../../shared/types/game-state';

export class StateManager {
  private variables: Record<string, any> = {};
  private inventory: string[] = [];
  private stats: Record<string, number> = {};
  private choiceHistory: ChoiceRecord[] = [];
  private visitedScenes: string[] = [];
  private unlockedCGs: string[] = [];
  private currentChapterId: string = '';
  private currentSceneId: string = '';
  private currentNodeId: string = '';

  // 变量管理
  getVariable(name: string): any {
    return this.variables[name];
  }

  setVariable(name: string, value: any): void {
    this.variables[name] = value;
  }

  // 物品栏
  addItem(item: string): void {
    if (!this.inventory.includes(item)) {
      this.inventory.push(item);
    }
  }

  removeItem(item: string): void {
    this.inventory = this.inventory.filter(i => i !== item);
  }

  hasItem(item: string): boolean {
    return this.inventory.includes(item);
  }

  // 属性
  getStat(name: string): number {
    return this.stats[name] ?? 0;
  }

  setStat(name: string, value: number): void {
    this.stats[name] = value;
  }

  modifyStat(name: string, delta: number): void {
    this.stats[name] = (this.stats[name] ?? 0) + delta;
  }

  // 选择历史
  recordChoice(nodeId: string, optionIndex: number, optionText: string): void {
    this.choiceHistory.push({
      choiceNodeId: nodeId,
      selectedOptionIndex: optionIndex,
      selectedOptionText: optionText,
      timestamp: Date.now(),
      chapterId: this.currentChapterId,
      sceneId: this.currentSceneId,
    });
  }

  // 场景访问
  markSceneVisited(sceneId: string): void {
    if (!this.visitedScenes.includes(sceneId)) {
      this.visitedScenes.push(sceneId);
    }
  }

  isSceneVisited(sceneId: string): boolean {
    return this.visitedScenes.includes(sceneId);
  }

  // CG
  unlockCG(cgId: string): void {
    if (!this.unlockedCGs.includes(cgId)) {
      this.unlockedCGs.push(cgId);
    }
  }

  isCGUnlocked(cgId: string): boolean {
    return this.unlockedCGs.includes(cgId);
  }

  // 当前位置
  setCurrentChapterId(id: string): void { this.currentChapterId = id; }
  setCurrentSceneId(id: string): void { this.currentSceneId = id; }
  setCurrentNodeId(id: string): void { this.currentNodeId = id; }

  getCurrentChapterId(): string { return this.currentChapterId; }
  getCurrentSceneId(): string { return this.currentSceneId; }
  getCurrentNodeId(): string { return this.currentNodeId; }

  // 序列化（存档用）
  serialize(): GameState {
    return {
      saveId: crypto.randomUUID(),
      gameScript: null as any,
      currentChapterId: this.currentChapterId,
      currentSceneId: this.currentSceneId,
      currentNodeId: this.currentNodeId,
      variables: { ...this.variables },
      inventory: [...this.inventory],
      stats: { ...this.stats },
      choiceHistory: [...this.choiceHistory],
      visitedScenes: [...this.visitedScenes],
      unlockedCGs: [...this.unlockedCGs],
      generationProgress: {
        totalAssets: 0,
        completedAssets: 0,
        failedAssets: 0,
        chapterStatus: {},
      },
    };
  }

  deserialize(state: GameState): void {
    this.currentChapterId = state.currentChapterId;
    this.currentSceneId = state.currentSceneId;
    this.currentNodeId = state.currentNodeId;
    this.variables = { ...state.variables };
    this.inventory = [...state.inventory];
    this.stats = { ...state.stats };
    this.choiceHistory = [...state.choiceHistory];
    this.visitedScenes = [...state.visitedScenes];
    this.unlockedCGs = [...state.unlockedCGs];
  }
}
