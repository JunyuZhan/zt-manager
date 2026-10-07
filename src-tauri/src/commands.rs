use crate::{agent, auth, central::*, secure, service};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
    /// 账号密码登录会话（DPAPI 加密存于 session.json；None = 未登录，退回 token 粘贴模式）
    pub session: Mutex<Option<auth::Session>>,
    pub session_path: PathBuf,
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
        let session_path = path.with_file_name("session.json");
        let session = auth::load(&session_path);
        Self {
            settings: Mutex::new(settings),
            settings_path: path,
            session: Mutex::new(session),
            session_path,
        }
    }

    fn session(&self) -> Option<auth::Session> {
        self.session.lock().unwrap().clone()
    }

    fn has_session(&self) -> bool {
        self.session.lock().unwrap().is_some()
    }

    /// 会话写内存 + DPAPI 落盘
    fn save_session(&self, s: auth::Session) -> Result<(), String> {
        auth::save(&self.session_path, &s).map_err(|e| e.to_string())?;
        *self.session.lock().unwrap() = Some(s);
        Ok(())
    }

    fn clear_session(&self) {
        auth::clear(&self.session_path);
        *self.session.lock().unwrap() = None;
    }

    fn new_base(&self) -> String {
        self.settings
            .lock()
            .unwrap()
            .new_base
            .clone()
            .unwrap_or_default()
    }

    /// 主动刷新：临近过期（30 秒缓冲）时刷新；失败则清会话并要求重新登录
    async fn refresh_session_if_needed(&self) -> Result<(), String> {
        let Some(s) = self.session() else { return Ok(()) };
        if !s.expiring_soon() {
            return Ok(());
        }
        match auth::refresh(&self.new_base(), &s).await {
            Ok(ns) => self.save_session(ns),
            Err(e) => {
                self.clear_session();
                Err(format!("会话已过期，请重新登录（{e}）"))
            }
        }
    }

    /// 收到 AUTH401 后强制刷新（令牌可能被吊销/轮换）；失败清会话
    async fn force_refresh(&self) -> Result<(), String> {
        let Some(s) = self.session() else { return Ok(()) };
        match auth::refresh(&self.new_base(), &s).await {
            Ok(ns) => self.save_session(ns),
            Err(e) => {
                self.clear_session();
                Err(format!("会话已失效，请重新登录（{e}）"))
            }
        }
    }

    /// 按登录会话构造客户端；未登录时退回 token 粘贴模式。
    fn client_from_session(&self, s: &auth::Session) -> Result<CentralClient, String> {
        let settings = self.settings.lock().unwrap();
        let mut cfg = CentralConfig::default();
        match s.mode {
            auth::SessionMode::Central => {
                if let Some(b) = &settings.new_base {
                    if !b.trim().is_empty() {
                        cfg.new_base = b.trim().to_string();
                    }
                }
                // v2 裸 token + 中央 v2 API；org_id 可留空（请求时 GET /org 解析）
                Ok(CentralClient::new(s.access_token.clone(), CentralMode::New)
                    .with_auth_style(AuthStyle::Raw)
                    .with_org(settings.org_id.clone())
                    .with_config(cfg))
            }
            auth::SessionMode::Keycloak => {
                // OIDC token 仅对 my.zerotier.com 的 /api/v1 生效（官方 SPA 即如此），
                // 故此处强制覆盖 legacy_base，不读取自定义 legacy_base。
                cfg.legacy_base = "https://my.zerotier.com/api/v1".into();
                Ok(CentralClient::new(s.access_token.clone(), CentralMode::Legacy)
                    .with_auth_style(AuthStyle::Bearer)
                    .with_config(cfg))
            }
        }
    }

    /// 构造可用客户端：会话模式先按需刷新，再按会话/ token 生成。
    async fn build_client(&self) -> Result<CentralClient, String> {
        if let Some(s) = self.session() {
            self.refresh_session_if_needed().await?;
            return self.client_from_session(&s);
        }
        self.central_client()
    }

    fn central_client(&self) -> Result<CentralClient, String> {
        let s = self.settings.lock().unwrap();
        if s.token.is_empty() {
            return Err(
                "尚未登录：请在「设置」页使用 ZeroTier 账号登录，或填写 Central API Token"
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

/// 统一调用入口：构建客户端（含会话刷新）→ 探测 → 执行；
/// 401（AUTH401 标记）时强制刷新会话并重试一次（仅会话模式）。
async fn central_call<F, Fut>(state: &AppState, f: F) -> Result<Value, String>
where
    F: Fn(CentralClient) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<Value>>,
{
    let mut c = state.build_client().await?;
    let _ = c.detect().await;
    match f(c).await {
        Ok(v) => Ok(v),
        Err(e) if e.to_string().starts_with("AUTH401") && state.has_session() => {
            state.force_refresh().await?;
            let mut c2 = state.build_client().await?;
            let _ = c2.detect().await;
            f(c2).await.map_err(|e| e.to_string())
        }
        Err(e) => Err(e.to_string()),
    }
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
    let mut c = state.build_client().await?;
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

// ---------- 账号登录 ----------

/// 账号密码登录（先 New Central v2，凭据被拒自动回退 Keycloak）。
/// 返回 `{needOtp:true}` 表示需二次验证，携带 otp 再次调用即可。
#[tauri::command]
pub async fn auth_login(
    state: State<'_, AppState>,
    email: String,
    password: String,
    otp: Option<String>,
) -> Result<Value, String> {
    let outcome = auth::login(&state.new_base(), &email, &password, otp.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    match outcome {
        auth::LoginOutcome::NeedOtp => Ok(json!({ "needOtp": true })),
        auth::LoginOutcome::Success(s) => {
            let mode = match s.mode {
                auth::SessionMode::Central => "central",
                auth::SessionMode::Keycloak => "keycloak",
            };
            let email = s.email.clone();
            state.save_session(s)?;
            Ok(json!({ "mode": mode, "email": email }))
        }
    }
}

#[tauri::command]
pub fn auth_logout(state: State<AppState>) {
    state.clear_session();
}

/// 登录状态（不返回令牌本身）
#[tauri::command]
pub fn auth_status(state: State<AppState>) -> Value {
    let token_configured = !state.settings.lock().unwrap().token.is_empty();
    match state.session() {
        Some(s) => {
            let mode = match s.mode {
                auth::SessionMode::Central => "central",
                auth::SessionMode::Keycloak => "keycloak",
            };
            json!({ "loggedIn": true, "email": s.email, "mode": mode, "expiresAt": s.expires_at, "tokenConfigured": token_configured })
        }
        None => json!({ "loggedIn": false, "tokenConfigured": token_configured }),
    }
}

// ---------- Central ----------

#[tauri::command]
pub async fn central_networks(state: State<'_, AppState>) -> Result<Value, String> {
    central_call(&state, |c| async move { c.list_networks().await }).await
}

#[tauri::command]
pub async fn central_network(state: State<'_, AppState>, nwid: String) -> Result<Value, String> {
    central_call(&state, |c| {
        let nwid = nwid.clone();
        async move { c.get_network(&nwid).await }
    })
    .await
}

#[tauri::command]
pub async fn central_create_network(state: State<'_, AppState>, name: String) -> Result<Value, String> {
    central_call(&state, |c| {
        let name = name.clone();
        async move { c.create_network(&name).await }
    })
    .await
}

#[tauri::command]
pub async fn central_delete_network(state: State<'_, AppState>, nwid: String) -> Result<Value, String> {
    central_call(&state, |c| {
        let nwid = nwid.clone();
        async move { c.delete_network(&nwid).await }
    })
    .await
}

#[tauri::command]
pub async fn central_update_network(state: State<'_, AppState>, nwid: String, body: Value) -> Result<Value, String> {
    central_call(&state, |c| {
        let nwid = nwid.clone();
        let body = body.clone();
        async move { c.update_network(&nwid, body).await }
    })
    .await
}

#[tauri::command]
pub async fn central_members(state: State<'_, AppState>, nwid: String) -> Result<Value, String> {
    central_call(&state, |c| {
        let nwid = nwid.clone();
        async move { c.list_members(&nwid).await }
    })
    .await
}

#[tauri::command]
pub async fn central_update_member(state: State<'_, AppState>, nwid: String, mid: String, body: Value) -> Result<Value, String> {
    central_call(&state, |c| {
        let nwid = nwid.clone();
        let mid = mid.clone();
        let body = body.clone();
        async move { c.update_member(&nwid, &mid, body).await }
    })
    .await
}

/// 成员授权动作：authorize | deauthorize | reject
#[tauri::command]
pub async fn central_member_action(
    state: State<'_, AppState>,
    nwid: String,
    mid: String,
    action: String,
) -> Result<Value, String> {
    central_call(&state, |c| {
        let nwid = nwid.clone();
        let mid = mid.clone();
        let action = action.clone();
        async move { c.member_action(&nwid, &mid, &action).await }
    })
    .await
}

/// 从网络中删除成员设备
#[tauri::command]
pub async fn central_delete_member(
    state: State<'_, AppState>,
    nwid: String,
    mid: String,
) -> Result<Value, String> {
    central_call(&state, |c| {
        let nwid = nwid.clone();
        let mid = mid.clone();
        async move { c.delete_member(&nwid, &mid).await }
    })
    .await
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
