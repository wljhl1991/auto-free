// AI Provider 配置类型定义
// 从 shared/types/ai-provider.ts 和 asset.ts 重新导出

export type AIModality = 'text' | 'image' | 'video' | 'music' | 'voice';

export type AuthType = 'api_key' | 'oauth' | 'account';

export type ProviderStatus = 'unconfigured' | 'configured' | 'connected' | 'auth_failed' | 'quota_exceeded' | 'network_error' | 'error';

export type QualityLevel = 'fast' | 'standard' | 'high';

export type ConnectivityStatus = 'ok' | 'auth_failed' | 'network_error' | 'quota_exceeded' | 'unknown_error';

export interface ApiKeyField {
  value: string;
  label: string;
  placeholder: string;
  helpUrl: string;
}

export interface CredentialField {
  value: string;
  label: string;
  placeholder: string;
}

export interface AccountCredentials {
  username?: CredentialField;
  password?: CredentialField;
}

export interface OAuthConfig {
  clientId: string;
  redirectUri: string;
  accessToken?: string;
  refreshToken?: string;
  expiresAt?: number;
}

export interface ExtraParamField {
  value: string;
  label: string;
  placeholder: string;
  required: boolean;
  secret: boolean;
}

export interface AuthConfig {
  apiKey?: ApiKeyField;
  account?: AccountCredentials;
  oauth?: OAuthConfig;
  extraParams?: Record<string, ExtraParamField>;
}

export interface AIModelConfig {
  id: string;
  name: string;
  modality: AIModality;
  isDefault: boolean;
  endpoint: string;
  maxTokens?: number;
  supportedSizes?: string[];
  maxDuration?: number;
  costPerCall?: number;
  freeQuota?: string;
  quality: QualityLevel;
}

export interface AIProviderConfig {
  id: string;
  name: string;
  vendor: string;
  description: string;
  officialUrl: string;
  registerUrl: string;
  docsUrl: string;
  modality: AIModality[];
  authType: AuthType;
  authConfig: AuthConfig;
  models: AIModelConfig[];
  status: ProviderStatus;
  lastChecked?: number;
  errorMessage?: string;
}

export interface PresetProvider {
  providerId: string;
  modality: AIModality;
  modelId: string;
  note?: string;
}

export interface BuiltinFallback {
  image: boolean;
  video: boolean;
  music: boolean;
  voice: boolean;
}

export interface ConfigPreset {
  id: string;
  name: string;
  description: string;
  vendorCount: number;
  providers: PresetProvider[];
  builtinFallback: BuiltinFallback;
}

export interface GlobalSettings {
  autoRetryOnFail: boolean;
  fallbackToAlternative: boolean;
  maxConcurrentGenerations: number;
  defaultQuality: QualityLevel;
  language: string;
}

export interface AppConfig {
  activePresetId: string;
  providers: AIProviderConfig[];
  presets: ConfigPreset[];
  globalSettings: GlobalSettings;
}

export interface QuotaInfo {
  remaining?: number;
  total?: number;
  unit: string;
  resetAt?: number;
}

export interface ConnectivityCheck {
  providerId: string;
  timestamp: number;
  status: ConnectivityStatus;
  latency?: number;
  errorMessage?: string;
  quotaInfo?: QuotaInfo;
  responsePreview?: string;
  testPrompt?: string;
  mediaUrl?: string;
  mediaType?: string;
}
