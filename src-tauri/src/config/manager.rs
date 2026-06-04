use crate::types::ai_provider::{
    AppConfig, AIProviderConfig, AuthConfig, AuthType, ConfigPreset, GlobalSettings,
    PresetProvider, ProviderStatus, QualityLevel,
};
use super::encryption::EncryptionManager;
use super::presets;
use super::providers;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct ConfigManager {
    config_dir: PathBuf,
    encryption: EncryptionManager,
    config: AppConfig,
}

impl ConfigManager {
    pub fn new(config_dir: PathBuf) -> Self {
        let encryption = EncryptionManager::from_machine_id();
        let config = Self::default_config();
        Self {
            config_dir,
            encryption,
            config,
        }
    }

    /// 从 config.json + secrets.enc 加载
    pub fn load(&mut self) -> Result<(), String> {
        let config_path = self.config_dir.join("config.json");
        let secrets_path = self.config_dir.join("secrets.enc");

        if !config_path.exists() {
            // 首次运行，创建默认配置
            self.save()?;
            return Ok(());
        }

        // 加载非敏感配置
        let config_str = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("读取配置文件失败: {}", e))?;
        let mut config: AppConfig = serde_json::from_str(&config_str)
            .map_err(|e| format!("解析配置文件失败: {}", e))?;

        // 加载敏感信息
        if secrets_path.exists() {
            let secrets_enc = std::fs::read_to_string(&secrets_path)
                .map_err(|e| format!("读取密钥文件失败: {}", e))?;
            let secrets_data = self.encryption.decrypt(&secrets_enc)
                .map_err(|e| format!("解密密钥文件失败: {}", e))?;
            let secrets: HashMap<String, String> = serde_json::from_slice(&secrets_data)
                .map_err(|e| format!("解析密钥数据失败: {}", e))?;

            // 将密钥还原到对应的 provider 配置中
            for provider in &mut config.providers {
                Self::restore_secrets(provider, &secrets);
            }
        }

        self.config = config;
        Ok(())
    }

    /// 保存到 config.json + secrets.enc
    pub fn save(&self) -> Result<(), String> {
        // 确保目录存在
        std::fs::create_dir_all(&self.config_dir)
            .map_err(|e| format!("创建配置目录失败: {}", e))?;

        // 分离敏感信息
        let mut config = self.config.clone();
        let mut secrets = HashMap::new();

        for provider in &mut config.providers {
            Self::extract_secrets(provider, &mut secrets);
        }

        // 保存非敏感配置
        let config_path = self.config_dir.join("config.json");
        let config_str = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("序列化配置失败: {}", e))?;
        std::fs::write(&config_path, config_str)
            .map_err(|e| format!("写入配置文件失败: {}", e))?;

        // 加密保存敏感信息
        if !secrets.is_empty() {
            let secrets_path = self.config_dir.join("secrets.enc");
            let secrets_json = serde_json::to_vec(&secrets)
                .map_err(|e| format!("序列化密钥数据失败: {}", e))?;
            let encrypted = self.encryption.encrypt(&secrets_json)
                .map_err(|e| format!("加密密钥数据失败: {}", e))?;
            std::fs::write(&secrets_path, encrypted)
                .map_err(|e| format!("写入密钥文件失败: {}", e))?;
        }

        Ok(())
    }

    pub fn get_config(&self) -> &AppConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: AppConfig) -> Result<(), String> {
        self.config = config;
        self.save()
    }

    /// 获取服务商列表（敏感字段脱敏）
    pub fn get_providers(&self) -> Vec<AIProviderConfig> {
        self.config.providers.iter().map(|p| Self::mask_provider(p)).collect()
    }

    /// 更新单个服务商配置
    pub fn update_provider(&mut self, provider: AIProviderConfig) -> Result<(), String> {
        if let Some(existing) = self.config.providers.iter_mut().find(|p| p.id == provider.id) {
            *existing = provider;
        } else {
            self.config.providers.push(provider);
        }
        self.save()
    }

    /// 更新服务商状态（连通性检测后调用）
    pub fn update_provider_status(
        &mut self,
        provider_id: &str,
        status: crate::types::ai_provider::ProviderStatus,
        error_message: Option<String>,
    ) {
        if let Some(provider) = self.config.providers.iter_mut().find(|p| p.id == provider_id) {
            provider.status = status;
            provider.last_checked = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
            provider.error_message = error_message;
        }
        // 不 save()，避免频繁写入磁盘。下次 update_provider 时会统一保存。
    }

    pub fn get_presets(&self) -> &[ConfigPreset] {
        &self.config.presets
    }

    /// 应用预设方案
    pub fn apply_preset(&mut self, preset_id: &str) -> Result<(), String> {
        let preset = self.config.presets.iter()
            .find(|p| p.id == preset_id)
            .ok_or_else(|| format!("预设方案 '{}' 不存在", preset_id))?;

        self.config.active_preset_id = preset.id.clone();
        self.save()
    }

    /// 脱敏导出配置
    pub fn export_config(&self) -> Result<String, String> {
        let mut export = self.config.clone();
        for provider in &mut export.providers {
            *provider = Self::mask_provider(provider);
        }
        serde_json::to_string_pretty(&export)
            .map_err(|e| format!("导出配置失败: {}", e))
    }

    /// 导入配置
    pub fn import_config(&mut self, json: &str) -> Result<(), String> {
        let imported: AppConfig = serde_json::from_str(json)
            .map_err(|e| format!("解析导入配置失败: {}", e))?;
        self.config = imported;
        self.save()
    }

    /// 创建默认配置
    fn default_config() -> AppConfig {
        AppConfig {
            active_preset_id: "default".to_string(),
            providers: providers::builtin_providers(),
            presets: presets::all_presets(),
            global_settings: GlobalSettings {
                auto_retry_on_fail: true,
                fallback_to_alternative: true,
                max_concurrent_generations: 3,
                default_quality: QualityLevel::Standard,
                language: "zh-CN".to_string(),
            },
        }
    }

    /// 脱敏处理：API Key 只显示前4位 + ***
    fn mask_provider(provider: &AIProviderConfig) -> AIProviderConfig {
        let mut masked = provider.clone();
        if let Some(ref mut api_key) = masked.auth_config.api_key {
            if api_key.value.len() > 4 {
                api_key.value = format!("{}***", &api_key.value[..4]);
            } else if !api_key.value.is_empty() {
                api_key.value = "***".to_string();
            }
        }
        if let Some(ref mut account) = masked.auth_config.account {
            if let Some(ref mut password) = account.password {
                password.value = "***".to_string();
            }
        }
        if let Some(ref mut extra) = masked.auth_config.extra_params {
            for (_key, field) in extra.iter_mut() {
                if field.secret && field.value.len() > 4 {
                    field.value = format!("{}***", &field.value[..4]);
                } else if field.secret && !field.value.is_empty() {
                    field.value = "***".to_string();
                }
            }
        }
        if let Some(ref mut oauth) = masked.auth_config.oauth {
            if let Some(ref mut access_token) = oauth.access_token {
                *access_token = "***".to_string();
            }
            if let Some(ref mut refresh_token) = oauth.refresh_token {
                *refresh_token = "***".to_string();
            }
        }
        masked
    }

    /// 提取敏感信息到 secrets map
    fn extract_secrets(provider: &mut AIProviderConfig, secrets: &mut HashMap<String, String>) {
        let prefix = &provider.id;
        if let Some(ref mut api_key) = provider.auth_config.api_key {
            if !api_key.value.is_empty() && api_key.value != "FREE" {
                secrets.insert(format!("{}.apiKey", prefix), api_key.value.clone());
                api_key.value.clear();
            }
        }
        if let Some(ref mut account) = provider.auth_config.account {
            if let Some(ref mut password) = account.password {
                if !password.value.is_empty() {
                    secrets.insert(format!("{}.accountPassword", prefix), password.value.clone());
                    password.value.clear();
                }
            }
        }
        if let Some(ref mut extra) = provider.auth_config.extra_params {
            for (key, field) in extra.iter_mut() {
                if field.secret && !field.value.is_empty() {
                    secrets.insert(format!("{}.extra.{}", prefix, key), field.value.clone());
                    field.value.clear();
                }
            }
        }
        if let Some(ref mut oauth) = provider.auth_config.oauth {
            if let Some(ref mut access_token) = oauth.access_token {
                if !access_token.is_empty() {
                    secrets.insert(format!("{}.oauthAccessToken", prefix), access_token.clone());
                    access_token.clear();
                }
            }
            if let Some(ref mut refresh_token) = oauth.refresh_token {
                if !refresh_token.is_empty() {
                    secrets.insert(format!("{}.oauthRefreshToken", prefix), refresh_token.clone());
                    refresh_token.clear();
                }
            }
        }
    }

    /// 从 secrets map 还原敏感信息
    fn restore_secrets(provider: &mut AIProviderConfig, secrets: &HashMap<String, String>) {
        let prefix = &provider.id;
        if let Some(ref mut api_key) = provider.auth_config.api_key {
            if let Some(value) = secrets.get(&format!("{}.apiKey", prefix)) {
                api_key.value = value.clone();
            }
        }
        if let Some(ref mut account) = provider.auth_config.account {
            if let Some(ref mut password) = account.password {
                if let Some(value) = secrets.get(&format!("{}.accountPassword", prefix)) {
                    password.value = value.clone();
                }
            }
        }
        if let Some(ref mut extra) = provider.auth_config.extra_params {
            for (key, field) in extra.iter_mut() {
                if field.secret {
                    if let Some(value) = secrets.get(&format!("{}.extra.{}", prefix, key)) {
                        field.value = value.clone();
                    }
                }
            }
        }
        if let Some(ref mut oauth) = provider.auth_config.oauth {
            if let Some(ref mut access_token) = oauth.access_token {
                if let Some(value) = secrets.get(&format!("{}.oauthAccessToken", prefix)) {
                    access_token.clone_from(value);
                }
            }
            if let Some(ref mut refresh_token) = oauth.refresh_token {
                if let Some(value) = secrets.get(&format!("{}.oauthRefreshToken", prefix)) {
                    refresh_token.clone_from(value);
                }
            }
        }
    }
}
