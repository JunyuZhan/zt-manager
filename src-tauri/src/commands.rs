use crate::{agent, central::*, secure, service};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{path::PathBuf, sync::Mutex};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub central_mode: CentralMode,
    #[serde(default)]
    pub org_id: Option<String>,
    /// 可选：自定义 Legacy base URL（留空用默认）
    #[serde(default)]
    pub legacy_base: Option<String>,
    /// 可选：自定义 New Central base URL（留空用默认）
    #[serde(default)]
    pub new_base: Option<String>,
}

impl Settings {
    /// 从环境变量读取 token/org（占位符式后台配置），作为 UI 未填写时的兜底。
    /// 支持：ZTM_CENTRAL_TOKEN / ZTM_CENTRAL_MODE / ZTM_CENTRAL_ORG_ID
    ///      ZTM_LEGACY_BASE / ZTM_NEW_BASE
    fn apply_env_defaults(&mut self) {
        if self.token.is_empty() {
            if let Ok(t) = std::env::var("ZTM_CENTRAL_TOKEN") {
                if !t.trim().is_empty() {
                    self.token = t.trim().to_string();
                }
            }
        }
        if self.central_mode == CentralMode::Auto {
            if let Ok(m) = std::env::var("ZTM_CENTRAL_MODE") {
                match m.trim().to_lowercase().as_str() {
                    "legacy" => self.central_mode = CentralMode::Legacy,
                    "new" => self.central_mode = CentralMode::New,
                    _ => {}
                }
            }
        }
        if self.org_id.is_none() {
            if let Ok(o) = std::env::var("ZTM_CENTRAL_ORG_ID") {
                if !o.trim().is_empty() {
                    self.org_id = Some(o.trim().to_string());
                }
            }
        }
        if self.legacy_base.is_none() {
            if let Ok(b) = std::env::var("ZTM_LEGACY_BASE") {
                if !b.trim().is_empty() {
                    self.legacy_base = Some(b.trim().to_string());
                }
            }
        }
        if self.new_base.is_none() {
            if let Ok(b) = std::env::var("ZTM_NEW_BASE") {
                if !b.trim().is_empty() {
                    self.new_base = Some(b.trim().to_string());
                }
            }
        }
    }
}

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub settings_path: PathBuf,
}

impl AppState {
    pub fn load(path: PathBuf) -> Self {
        let mut settings = std::fs::read(&path)
            .ok()
            .and_then(|raw| serde_json::from_slice::<Settings>(&raw).ok())
            .unwrap_or_default();
        // 解密 token 注入内存
        let tok_path = path.with_extension("token");
        if let Ok(enc) = std::fs::read(&tok_path) {
            if let Ok(plain) = secure::unprotect(&enc) {
                if let Ok(token) = String::from_utf8(plain) {
                    settings.token = token;
                }
            }
        }
        // 环境变量兜底（占位符式后台配置）
        settings.apply_env_defaults();
        Self { settings: Mutex::new(settings), settings_path: path }
    }

    fn central_client(&self) -> Result<CentralClient, String> {
        let s = self.settings.lock().unwrap();
        if s.token.is_empty() {
            return Err(
                "尚未配置 Central API Token：请在「设置」页填写，或设置环境变量 ZTM_CENTRAL_TOKEN"
                    .into(),
            );
        }
        let mut cfg = CentralConfig::default();
        if let Some(b) = &s.legacy_base {
            if !b.trim().is_empty() {
                cfg.legacy_base = b.trim().to_string();
            }
        }
        if let Some(b) = &s.new_base {
            if !b.trim().is_empty() {
                cfg.new_base = b.trim().to_string();
            }
        }
        Ok(CentralClient::new(s.token.clone(), s.central_mode)
            .with_org(s.org_id.clone())
            .with_config(cfg))
    }
}

async fn central_call<F, Fut>(state: &AppState, f: F) -> Result<Value, String>
where
    F: FnOnce(CentralClient) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<Value>>,
{
    let mut c = state.central_client()?;
    let _ = c.detect().await;
    f(c).await.map_err(|e| e.to_string())
}

fn to_str<T>(r: anyhow::Result<T>) -> Result<T, String> {
    r.map_err(|e| e.to_string())
}

// ---------- 设置 ----------

#[tauri::command]
pub fn settings_load(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn settings_save(state: State<AppState>, settings: Settings) -> Result<(), String> {
    let token_plain = settings.token.clone();
    let enc = if token_plain.is_empty() {
        Vec::new()
    } else {
        secure::protect(token_plain.as_bytes()).map_err(|e| e.to_string())?
    };
    // 落盘配置不含明文 token
    let mut cfg = settings.clone();
    cfg.token.clear();
    let json = serde_json::to_vec_pretty(&cfg).map_err(|e| e.to_string())?;
    std::fs::write(&state.settings_path, &json).map_err(|e| e.to_string())?;
    let tok_path = state.settings_path.with_extension("token");
    if enc.is_empty() {
        let _ = std::fs::remove_file(&tok_path);
    } else {
        std::fs::write(&tok_path, enc).map_err(|e| e.to_string())?;
    }
    *state.settings.lock().unwrap() = settings;
    Ok(())
}

#[tauri::command]
pub async fn central_detect(state: State<'_, AppState>) -> Result<String, String> {
    let mut c = state.central_client()?;
    match c.detect().await {
        Ok(CentralMode::New) => Ok("new".into()),
        Ok(_) => Ok("legacy".into()),
        Err(e) => Err(e.to_string()),
    }
}

// ---------- 本地服务 ----------

#[tauri::command]
pub fn service_query() -> Result<Value, String> {
    service::query().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn service_control(action: String) -> Result<Value, String> {
    service::control(&action).map_err(|e| e.to_string())
}

// ---------- 本地 agent ----------

#[tauri::command]
pub async fn agent_status() -> Result<Value, String> {
    to_str(agent::status().await)
}

#[tauri::command]
pub async fn agent_networks() -> Result<Vec<String>, String> {
    to_str(agent::network_list().await)
}

#[tauri::command]
pub async fn agent_network_detail(nwid: String) -> Result<Value, String> {
    to_str(agent::network_detail(&nwid).await)
}

#[tauri::command]
pub async fn agent_join(nwid: String) -> Result<Value, String> {
    to_str(agent::join(&nwid).await)
}

#[tauri::command]
pub async fn agent_leave(nwid: String) -> Result<Value, String> {
    to_str(agent::leave(&nwid).await)
}

#[tauri::command]
pub async fn agent_peers() -> Result<Value, String> {
    to_str(agent::peers().await)
}

// ---------- Central ----------

#[tauri::command]
pub async fn central_networks(state: State<'_, AppState>) -> Result<Value, String> {
    central_call(&state, |c| async move { c.list_networks().await }).await
}

#[tauri::command]
pub async fn central_network(state: State<'_, AppState>, nwid: String) -> Result<Value, String> {
    central_call(&state, |c| async move { c.get_network(&nwid).await }).await
}

#[tauri::command]
pub async fn central_create_network(state: State<'_, AppState>, name: String) -> Result<Value, String> {
    central_call(&state, |c| async move { c.create_network(&name).await }).await
}

#[tauri::command]
pub async fn central_delete_network(state: State<'_, AppState>, nwid: String) -> Result<Value, String> {
    central_call(&state, |c| async move { c.delete_network(&nwid).await }).await
}

#[tauri::command]
pub async fn central_update_network(state: State<'_, AppState>, nwid: String, body: Value) -> Result<Value, String> {
    central_call(&state, |c| async move { c.update_network(&nwid, body).await }).await
}

#[tauri::command]
pub async fn central_members(state: State<'_, AppState>, nwid: String) -> Result<Value, String> {
    central_call(&state, |c| async move { c.list_members(&nwid).await }).await
}

#[tauri::command]
pub async fn central_update_member(state: State<'_, AppState>, nwid: String, mid: String, body: Value) -> Result<Value, String> {
    central_call(&state, |c| async move { c.update_member(&nwid, &mid, body).await }).await
}

#[tauri::command]
pub async fn central_orgs(state: State<'_, AppState>) -> Result<Value, String> {
    central_call(&state, |c| async move { c.list_orgs().await }).await
}

// ---------- 本地控制器 ----------

#[tauri::command]
pub async fn controller_networks() -> Result<Value, String> {
    to_str(agent::controller_networks().await)
}

#[tauri::command]
pub async fn controller_network(nwid: String) -> Result<Value, String> {
    to_str(agent::controller_network(&nwid).await)
}

#[tauri::command]
pub async fn controller_create() -> Result<Value, String> {
    to_str(agent::controller_create().await)
}

#[tauri::command]
pub async fn controller_update(nwid: String, body: Value) -> Result<Value, String> {
    to_str(agent::controller_update(&nwid, body).await)
}

#[tauri::command]
pub async fn controller_members(nwid: String) -> Result<Value, String> {
    to_str(agent::controller_members(&nwid).await)
}

#[tauri::command]
pub async fn controller_update_member(nwid: String, mid: String, body: Value) -> Result<Value, String> {
    to_str(agent::controller_update_member(&nwid, &mid, body).await)
}

// ---------- 诊断 ----------

#[tauri::command]
pub fn ping_host(host: String) -> Result<Value, String> {
    use std::os::windows::process::CommandExt;
    let script = format!(
        "$r = Test-Connection -ComputerName '{host}' -Count 2 -ErrorAction SilentlyContinue; if ($r) {{ $avg = ($r | Measure-Object -Property Latency -Average).Average; [int]$avg }} else {{ -1 }}"
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| e.to_string())?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let ms: i64 = s.parse().unwrap_or(-1);
    Ok(serde_json::json!({ "host": host, "latencyMs": ms }))
}
