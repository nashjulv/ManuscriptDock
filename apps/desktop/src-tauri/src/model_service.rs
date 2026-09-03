use keyring::v1::Entry;
use reqwest::{redirect::Policy, Client};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use url::Url;

const SETTINGS_SCHEMA_VERSION: u32 = 1;
const KEYRING_SERVICE: &str = "com.manuscriptdock.model-api";
const SETTINGS_FILE: &str = "model-settings.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum ModelSlotRole {
    #[serde(rename = "primary")]
    Primary,
    #[serde(rename = "fallback_1")]
    Fallback1,
    #[serde(rename = "fallback_2")]
    Fallback2,
}

impl ModelSlotRole {
    pub const ALL: [Self; 3] = [Self::Primary, Self::Fallback1, Self::Fallback2];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Fallback1 => "fallback_1",
            Self::Fallback2 => "fallback_2",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSlotInput {
    pub role: ModelSlotRole,
    pub enabled: bool,
    pub provider_label: String,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub clear_api_key: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredModelSlot {
    role: ModelSlotRole,
    enabled: bool,
    provider_label: String,
    base_url: String,
    model: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredModelSettings {
    schema_version: u32,
    slots: Vec<StoredModelSlot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSlotSummary {
    pub role: ModelSlotRole,
    pub enabled: bool,
    pub provider_label: String,
    pub base_url: String,
    pub model: String,
    pub has_api_key: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSettingsSummary {
    pub schema_version: u32,
    pub slots: Vec<ModelSlotSummary>,
    pub secure_store: String,
}

#[derive(Debug, Clone)]
struct ConfiguredModel {
    role: ModelSlotRole,
    provider_label: String,
    endpoint: Url,
    model: String,
    api_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelAnswer {
    pub slot: ModelSlotRole,
    pub provider_label: String,
    pub model: String,
    pub content: String,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<ThinkingConfig<'a>>,
}

#[derive(Serialize)]
struct ThinkingConfig<'a> {
    r#type: &'a str,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    #[serde(default)]
    content: Option<ChatResponseContent>,
    #[serde(default)]
    reasoning_content: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ChatResponseContent {
    Text(String),
    Parts(Vec<ChatResponsePart>),
}

#[derive(Deserialize)]
struct ChatResponsePart {
    #[serde(default)]
    text: Option<String>,
}

pub fn load_summary(root: &Path) -> Result<ModelSettingsSummary, String> {
    let settings = read_settings(root)?;
    let mut summaries = Vec::with_capacity(3);
    for role in ModelSlotRole::ALL {
        let stored = settings
            .slots
            .iter()
            .find(|slot| slot.role == role)
            .cloned()
            .unwrap_or_else(|| empty_slot(role));
        summaries.push(ModelSlotSummary {
            role,
            enabled: stored.enabled,
            provider_label: stored.provider_label,
            base_url: stored.base_url,
            model: stored.model,
            has_api_key: key_exists(role)?,
        });
    }
    Ok(ModelSettingsSummary {
        schema_version: SETTINGS_SCHEMA_VERSION,
        slots: summaries,
        secure_store: secure_store_label().to_owned(),
    })
}

pub fn save_settings(
    root: &Path,
    inputs: Vec<ModelSlotInput>,
) -> Result<ModelSettingsSummary, String> {
    validate_inputs(&inputs)?;
    validate_credential_requirements(&inputs, key_exists)?;
    fs::create_dir_all(root).map_err(|error| format!("无法创建模型设置目录：{error}"))?;
    for input in &inputs {
        let entry = credential_entry(input.role)?;
        if input.clear_api_key {
            if let Err(error) = entry.delete_credential() {
                if !matches!(error, keyring::Error::NoEntry) {
                    return Err(format!("无法从系统凭据库删除 API Key：{error}"));
                }
            }
        } else if let Some(api_key) = input
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty())
        {
            entry
                .set_password(api_key)
                .map_err(|error| format!("无法将 API Key 保存到系统凭据库：{error}"))?;
        }
    }
    let stored = StoredModelSettings {
        schema_version: SETTINGS_SCHEMA_VERSION,
        slots: inputs
            .into_iter()
            .map(|input| StoredModelSlot {
                role: input.role,
                enabled: input.enabled,
                provider_label: input.provider_label.trim().to_owned(),
                base_url: input.base_url.trim().trim_end_matches('/').to_owned(),
                model: input.model.trim().to_owned(),
            })
            .collect(),
    };
    write_settings(root, &stored)?;
    load_summary(root)
}

pub async fn ask_with_failover(
    root: &Path,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<ModelAnswer, String> {
    let models = configured_models(root)?;
    if models.is_empty() {
        return Err("尚未配置可用模型，请点击应用顶部的“模型设置”保存主模型或备选模型".to_owned());
    }
    let client = Client::builder()
        .timeout(Duration::from_secs(45))
        .redirect(Policy::none())
        .build()
        .map_err(|error| format!("无法初始化模型连接：{error}"))?;
    let mut failures = Vec::new();
    for configured in models {
        let request = ChatRequest {
            model: &configured.model,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: system_prompt,
                },
                ChatMessage {
                    role: "user",
                    content: user_prompt,
                },
            ],
            temperature: 0.2,
            max_tokens: 2_400,
            thinking: is_deepseek_endpoint(&configured.endpoint)
                .then_some(ThinkingConfig { r#type: "disabled" }),
        };
        let response = client
            .post(configured.endpoint.clone())
            .bearer_auth(&configured.api_key)
            .json(&request)
            .send()
            .await;
        match response {
            Ok(response) if response.status().is_success() => {
                match response.json::<ChatResponse>().await {
                    Ok(body) => {
                        if let Some(content) = visible_answer(&body) {
                            return Ok(ModelAnswer {
                                slot: configured.role,
                                provider_label: configured.provider_label,
                                model: configured.model,
                                content,
                            });
                        }
                        failures.push(empty_answer_message(configured.role, &body));
                    }
                    Err(_) => {
                        failures.push(format!("{} 返回了无法识别的响应", configured.role.as_str()))
                    }
                }
            }
            Ok(response) => failures.push(http_failure_message(
                configured.role,
                response.status().as_u16(),
            )),
            Err(error) if error.is_timeout() => {
                failures.push(format!("{} 连接超时", configured.role.as_str()))
            }
            Err(_) => failures.push(format!("{} 连接失败", configured.role.as_str())),
        }
    }
    Err(format!(
        "主模型和备选模型均未完成回答：{}",
        failures.join("；")
    ))
}

fn is_deepseek_endpoint(endpoint: &Url) -> bool {
    endpoint
        .host_str()
        .is_some_and(|host| host.eq_ignore_ascii_case("api.deepseek.com"))
}

fn visible_answer(body: &ChatResponse) -> Option<String> {
    body.choices.iter().find_map(|choice| {
        let content = match choice.message.content.as_ref()? {
            ChatResponseContent::Text(content) => content.trim().to_owned(),
            ChatResponseContent::Parts(parts) => parts
                .iter()
                .filter_map(|part| part.text.as_deref().map(str::trim))
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n"),
        };
        (!content.is_empty()).then_some(content)
    })
}

fn empty_answer_message(role: ModelSlotRole, body: &ChatResponse) -> String {
    let reached_limit = body.choices.iter().any(|choice| {
        choice
            .finish_reason
            .as_deref()
            .is_some_and(|reason| matches!(reason, "length" | "max_tokens"))
    });
    let contains_reasoning = body.choices.iter().any(|choice| {
        choice
            .message
            .reasoning_content
            .as_deref()
            .is_some_and(|content| !content.trim().is_empty())
    });
    let reason = if reached_limit {
        "达到输出上限，但没有形成最终回答"
    } else if contains_reasoning {
        "已完成内部推理，但没有形成最终回答"
    } else {
        "返回了空回答"
    };
    format!("{} {reason}", role.as_str())
}

fn http_failure_message(role: ModelSlotRole, status: u16) -> String {
    let reason = match status {
        400 => "请求格式未被模型服务接受，请检查模型兼容性",
        401 | 403 => "API Key 无效或无权调用该模型，请检查提供方权限",
        402 => "账户余额不足或计费未开通，请检查模型提供方账户余额",
        404 => "未找到对话接口，请检查 API 地址",
        422 => "请求参数未被模型服务接受，请检查模型名称和接口兼容性",
        429 => "请求频率或账户限额已达到上限，请稍后重试或使用备选模型",
        500..=599 => "模型服务暂时故障或繁忙，请稍后重试或使用备选模型",
        _ => return format!("{} 返回 HTTP {status}", role.as_str()),
    };
    format!("{} {reason}（HTTP {status}）", role.as_str())
}

fn configured_models(root: &Path) -> Result<Vec<ConfiguredModel>, String> {
    let settings = read_settings(root)?;
    let mut result = Vec::new();
    for role in ModelSlotRole::ALL {
        let Some(slot) = settings
            .slots
            .iter()
            .find(|slot| slot.role == role && slot.enabled)
        else {
            continue;
        };
        let api_key = match credential_entry(role)?.get_password() {
            Ok(api_key) => api_key,
            Err(keyring::Error::NoEntry) => continue,
            Err(error) => {
                return Err(format!(
                    "无法从系统凭据库读取 {} API Key：{error}",
                    role.as_str()
                ));
            }
        };
        result.push(ConfiguredModel {
            role,
            provider_label: slot.provider_label.clone(),
            endpoint: chat_endpoint(&slot.base_url)?,
            model: slot.model.clone(),
            api_key,
        });
    }
    Ok(result)
}

fn validate_inputs(inputs: &[ModelSlotInput]) -> Result<(), String> {
    if inputs.len() != 3 {
        return Err("模型设置必须包含 1 个主模型和 2 个备选模型".to_owned());
    }
    let roles = inputs
        .iter()
        .map(|input| input.role)
        .collect::<BTreeSet<_>>();
    if roles != ModelSlotRole::ALL.into_iter().collect() {
        return Err("模型设置槽位重复或缺失".to_owned());
    }
    for input in inputs {
        if input.provider_label.chars().count() > 100
            || input.base_url.chars().count() > 500
            || input.model.chars().count() > 200
            || input
                .api_key
                .as_ref()
                .is_some_and(|key| key.chars().count() > 2_000)
        {
            return Err("模型设置字段超过长度限制".to_owned());
        }
        if input.enabled {
            if input.provider_label.trim().is_empty()
                || input.base_url.trim().is_empty()
                || input.model.trim().is_empty()
            {
                return Err(format!(
                    "{} 已启用，但提供方、地址或模型名称不完整",
                    input.role.as_str()
                ));
            }
            chat_endpoint(input.base_url.trim())?;
        }
    }
    Ok(())
}

fn validate_credential_requirements(
    inputs: &[ModelSlotInput],
    mut stored_key_exists: impl FnMut(ModelSlotRole) -> Result<bool, String>,
) -> Result<(), String> {
    for input in inputs.iter().filter(|input| input.enabled) {
        let has_new_key = input
            .api_key
            .as_deref()
            .map(str::trim)
            .is_some_and(|key| !key.is_empty());
        if input.clear_api_key || (!has_new_key && !stored_key_exists(input.role)?) {
            return Err(format!(
                "{} 已启用，但尚未提供 API Key；请输入 Key 后再保存",
                input.role.as_str()
            ));
        }
    }
    Ok(())
}

fn chat_endpoint(base_url: &str) -> Result<Url, String> {
    let parsed = Url::parse(base_url).map_err(|_| "模型 API 地址无效".to_owned())?;
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err("模型 API 地址不能包含账号、密码、查询参数或片段".to_owned());
    }
    let host = parsed.host_str().unwrap_or_default();
    if host.eq_ignore_ascii_case("api-docs.deepseek.com") {
        return Err(
            "当前填写的是 DeepSeek 文档地址；API 地址应为 https://api.deepseek.com".to_owned(),
        );
    }
    let is_local = matches!(host, "localhost" | "127.0.0.1" | "::1");
    if parsed.scheme() != "https" && !(parsed.scheme() == "http" && is_local) {
        return Err("远程模型 API 必须使用 HTTPS；只有本机 localhost 可以使用 HTTP".to_owned());
    }
    let endpoint = if parsed
        .path()
        .trim_end_matches('/')
        .ends_with("/chat/completions")
    {
        base_url.trim_end_matches('/').to_owned()
    } else {
        format!("{}/chat/completions", base_url.trim_end_matches('/'))
    };
    Url::parse(&endpoint).map_err(|_| "无法生成模型问答地址".to_owned())
}

fn read_settings(root: &Path) -> Result<StoredModelSettings, String> {
    let path = settings_path(root);
    if !path.exists() {
        return Ok(StoredModelSettings {
            schema_version: SETTINGS_SCHEMA_VERSION,
            slots: ModelSlotRole::ALL.into_iter().map(empty_slot).collect(),
        });
    }
    let bytes = fs::read(&path).map_err(|error| format!("无法读取模型设置：{error}"))?;
    let settings: StoredModelSettings =
        serde_json::from_slice(&bytes).map_err(|_| "模型设置文件格式无效".to_owned())?;
    if settings.schema_version != SETTINGS_SCHEMA_VERSION {
        return Err("模型设置版本暂不受支持".to_owned());
    }
    Ok(settings)
}

fn write_settings(root: &Path, settings: &StoredModelSettings) -> Result<(), String> {
    let path = settings_path(root);
    let temporary = root.join(format!(".{SETTINGS_FILE}.tmp"));
    let bytes = serde_json::to_vec_pretty(settings)
        .map_err(|error| format!("无法编码模型设置：{error}"))?;
    fs::write(&temporary, bytes).map_err(|error| format!("无法写入模型设置：{error}"))?;
    fs::rename(&temporary, &path).map_err(|error| format!("无法提交模型设置：{error}"))
}

fn settings_path(root: &Path) -> PathBuf {
    root.join(SETTINGS_FILE)
}

fn empty_slot(role: ModelSlotRole) -> StoredModelSlot {
    StoredModelSlot {
        role,
        enabled: false,
        provider_label: String::new(),
        base_url: String::new(),
        model: String::new(),
    }
}

fn credential_entry(role: ModelSlotRole) -> Result<Entry, String> {
    Entry::new(KEYRING_SERVICE, role.as_str()).map_err(|error| format!("系统凭据库不可用：{error}"))
}

fn key_exists(role: ModelSlotRole) -> Result<bool, String> {
    match credential_entry(role)?.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(error) => Err(format!("无法读取系统凭据库状态：{error}")),
    }
}

#[cfg(target_os = "macos")]
fn secure_store_label() -> &'static str {
    "macOS Keychain"
}

#[cfg(target_os = "windows")]
fn secure_store_label() -> &'static str {
    "Windows Credential Manager"
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn secure_store_label() -> &'static str {
    "System credential store"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(role: ModelSlotRole, enabled: bool, base_url: &str) -> ModelSlotInput {
        ModelSlotInput {
            role,
            enabled,
            provider_label: if enabled { "Synthetic Provider" } else { "" }.to_owned(),
            base_url: base_url.to_owned(),
            model: if enabled { "synthetic-model" } else { "" }.to_owned(),
            api_key: None,
            clear_api_key: false,
        }
    }

    #[test]
    fn requires_one_primary_and_two_distinct_fallback_slots() {
        assert_eq!(
            serde_json::to_string(&ModelSlotRole::Primary).unwrap(),
            "\"primary\""
        );
        assert_eq!(
            serde_json::to_string(&ModelSlotRole::Fallback1).unwrap(),
            "\"fallback_1\""
        );
        assert_eq!(
            serde_json::to_string(&ModelSlotRole::Fallback2).unwrap(),
            "\"fallback_2\""
        );
        assert_eq!(
            serde_json::from_str::<ModelSlotRole>("\"fallback_1\"").unwrap(),
            ModelSlotRole::Fallback1
        );
        let valid = vec![
            input(ModelSlotRole::Primary, true, "https://models.example/v1"),
            input(ModelSlotRole::Fallback1, false, ""),
            input(ModelSlotRole::Fallback2, false, ""),
        ];
        validate_inputs(&valid).unwrap();

        let duplicate = vec![valid[0].clone(), valid[0].clone(), valid[2].clone()];
        assert!(validate_inputs(&duplicate)
            .unwrap_err()
            .contains("重复或缺失"));
    }

    #[test]
    fn rejects_plain_http_for_remote_models_but_allows_local_services() {
        assert!(chat_endpoint("http://models.example/v1").is_err());
        assert!(chat_endpoint("https://user:secret@models.example/v1").is_err());
        assert!(chat_endpoint("https://models.example/v1?token=secret").is_err());
        assert_eq!(
            chat_endpoint("http://127.0.0.1:11434/v1").unwrap().as_str(),
            "http://127.0.0.1:11434/v1/chat/completions"
        );
        assert_eq!(
            chat_endpoint("https://models.example/v1/chat/completions")
                .unwrap()
                .as_str(),
            "https://models.example/v1/chat/completions"
        );
        assert!(chat_endpoint("https://api-docs.deepseek.com/zh-cn")
            .unwrap_err()
            .contains("文档地址"));
    }

    #[test]
    fn enabled_slots_require_a_new_or_existing_api_key() {
        let enabled = vec![
            input(ModelSlotRole::Primary, true, "https://models.example/v1"),
            input(ModelSlotRole::Fallback1, false, ""),
            input(ModelSlotRole::Fallback2, false, ""),
        ];
        assert!(validate_credential_requirements(&enabled, |_| Ok(false))
            .unwrap_err()
            .contains("尚未提供 API Key"));

        let mut with_new_key = enabled.clone();
        with_new_key[0].api_key = Some("synthetic-secret".to_owned());
        validate_credential_requirements(&with_new_key, |_| Ok(false)).unwrap();
        validate_credential_requirements(&enabled, |_| Ok(true)).unwrap();

        let mut clearing = enabled;
        clearing[0].clear_api_key = true;
        assert!(validate_credential_requirements(&clearing, |_| Ok(true)).is_err());
    }

    #[test]
    fn disables_thinking_only_for_the_official_deepseek_endpoint() {
        let deepseek = Url::parse("https://api.deepseek.com/chat/completions").unwrap();
        let compatible = Url::parse("https://models.example/v1/chat/completions").unwrap();
        assert!(is_deepseek_endpoint(&deepseek));
        assert!(!is_deepseek_endpoint(&compatible));

        let request = ChatRequest {
            model: "deepseek-v4-flash",
            messages: vec![ChatMessage {
                role: "user",
                content: "Synthetic question",
            }],
            temperature: 0.2,
            max_tokens: 2_400,
            thinking: Some(ThinkingConfig { r#type: "disabled" }),
        };
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["thinking"]["type"], "disabled");
        assert_eq!(value["max_tokens"], 2_400);
    }

    #[test]
    fn reads_visible_string_and_part_answers_without_exposing_reasoning() {
        let text: ChatResponse = serde_json::from_str(
            r#"{"choices":[{"message":{"content":" Final answer ","reasoning_content":"private reasoning"},"finish_reason":"stop"}]}"#,
        )
        .unwrap();
        assert_eq!(visible_answer(&text).as_deref(), Some("Final answer"));

        let parts: ChatResponse = serde_json::from_str(
            r#"{"choices":[{"message":{"content":[{"type":"text","text":"First"},{"type":"text","text":"Second"}]},"finish_reason":"stop"}]}"#,
        )
        .unwrap();
        assert_eq!(visible_answer(&parts).as_deref(), Some("First\nSecond"));

        let reasoning_only: ChatResponse = serde_json::from_str(
            r#"{"choices":[{"message":{"content":null,"reasoning_content":"must stay private"},"finish_reason":"length"}]}"#,
        )
        .unwrap();
        assert_eq!(visible_answer(&reasoning_only), None);
        assert_eq!(
            empty_answer_message(ModelSlotRole::Primary, &reasoning_only),
            "primary 达到输出上限，但没有形成最终回答"
        );
    }

    #[test]
    fn explains_provider_http_failures_with_recovery_actions() {
        assert_eq!(
            http_failure_message(ModelSlotRole::Primary, 402),
            "primary 账户余额不足或计费未开通，请检查模型提供方账户余额（HTTP 402）"
        );
        assert!(http_failure_message(ModelSlotRole::Fallback1, 401).contains("API Key 无效"));
        assert!(http_failure_message(ModelSlotRole::Fallback2, 429).contains("使用备选模型"));
        assert_eq!(
            http_failure_message(ModelSlotRole::Primary, 418),
            "primary 返回 HTTP 418"
        );
    }
}
