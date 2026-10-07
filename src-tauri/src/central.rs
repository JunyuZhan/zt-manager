//! ZeroTier Central 客户端（Legacy v1 与 New Central v2 通用）。
//!
//! 设计要点：
//! - base URL 与各资源路径均可通过 `CentralConfig` 覆盖，未提供时使用默认值。
//! - New Central 的资源路径含 `{orgId}` 占位符，请求时按当前 org 动态替换。
//! - 认证头由 `AuthStyle` 决定：token 粘贴 = `token <T>` / `Bearer`（keycloak OIDC =
//!   `bearer <T>`，小写，官方 SPA 同）/ `Raw`（v2 登录裸 token，无前缀）。
//!   未显式指定时按模式推导：Legacy = token，New = Bearer（兼容既有 token 粘贴）。
//! - Auto 模式按响应码自动探测（Legacy 200/非 401·403 命中，否则试 New）。
//! - 列表/成员响应该模块出口统一为归一化形状（见 `normalize`），前端无需区分版本。

pub mod normalize;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CentralMode {
    #[default]
    Auto,
    Legacy,
    New,
}

/// Authorization 头风格。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthStyle {
    /// `Authorization: token <T>`（Legacy API token）
    Token,
    /// `Authorization: bearer <T>`（Keycloak OIDC，官方 SPA 用小写 bearer）
    Bearer,
    /// 裸 token 无前缀（New Central v2 登录响应）
    Raw,
}

/// Central 连接的通用配置。`paths` 支持覆盖任意资源路径，便于适配版本变化。
#[derive(Debug, Clone, Deserialize)]
pub struct CentralConfig {
    #[serde(default = "default_legacy_base")]
    pub legacy_base: String,
    #[serde(default = "default_new_base")]
    pub new_base: String,
    /// 资源路径覆盖表，键如 "networks" / "network" / "members" / "orgs"。
    #[serde(default)]
    pub paths: HashMap<String, String>,
}

fn default_legacy_base() -> String {
    "https://api.zerotier.com/api/v1".into()
}
fn default_new_base() -> String {
    "https://central.zerotier.com/api/v2".into()
}

impl Default for CentralConfig {
    fn default() -> Self {
        Self {
            legacy_base: default_legacy_base(),
            new_base: default_new_base(),
            paths: HashMap::new(),
        }
    }
}

// 各资源的默认路径（Legacy / New 分别定义，New 含 {orgId} 占位符）
fn default_legacy_paths() -> HashMap<String, String> {
    HashMap::from([
        ("status".into(), "/status".into()),
        ("orgs".into(), "/org".into()),
        ("networks".into(), "/network".into()),
        ("network".into(), "/network/{nwid}".into()),
        ("members".into(), "/network/{nwid}/member".into()),
        ("member".into(), "/network/{nwid}/member/{mid}".into()),
    ])
}

fn default_new_paths() -> HashMap<String, String> {
    // 实测依据（2026-10-04，无效 token 探测响应码）：
    //   /status、/network、/network/{id}、/network/{id}/member[/{mid}] → 401（路由存在）
    //   /orgs、/orgs/{org}/networks、/network/{id}/members               → 404（不存在）
    // 即 New Central 与 Legacy 共用 REST 形状，列表/创建经 ?org-id= 查询参数限定组织。
    // 2026-10-07 从 New Central bundle 逆向补充 v2 专属端点（单数 /org，网络组创建网络）：
    //   GET  /org?permission-check=                      → {items:[{id,name}]}
    //   GET  /network-group?org-id=X                     → {items}
    //   POST /org/{orgId}/network-group {name,description}
    //   POST /network-group/{gid}/network {name,description}
    //   POST /network/{id}/member/{mid}/authorize|de-authorize|reject（无 body）
    //   DELETE /network/{id}/member body {data:[{deviceId}]}
    // 注意：键名 "orgs" 在 New 模式必须保持缺失（tests/central_paths.rs 断言）。
    HashMap::from([
        ("status".into(), "/status".into()),
        ("org".into(), "/org".into()),
        ("networks".into(), "/network".into()),
        ("network".into(), "/network/{nwid}".into()),
        ("members".into(), "/network/{nwid}/member".into()),
        ("member".into(), "/network/{nwid}/member/{mid}".into()),
        ("member_action".into(), "/network/{nwid}/member/{mid}/{action}".into()),
        ("network_groups".into(), "/network-group".into()),
        ("org_network_group".into(), "/org/{orgId}/network-group".into()),
        ("group_network".into(), "/network-group/{gid}/network".into()),
    ])
}

pub struct CentralClient {
    token: String,
    mode: CentralMode,
    pub detected: Option<CentralMode>,
    org_id: Option<String>,
    config: CentralConfig,
    /// 显式认证风格；None 时按模式推导（Legacy=Token，New=Bearer）
    auth_style: Option<AuthStyle>,
}

impl CentralClient {
    pub fn new(token: String, mode: CentralMode) -> Self {
        Self {
            token,
            mode,
            detected: None,
            org_id: None,
            config: CentralConfig::default(),
            auth_style: None,
        }
    }

    pub fn with_org(mut self, org_id: Option<String>) -> Self {
        self.org_id = org_id;
        self
    }

    pub fn with_config(mut self, config: CentralConfig) -> Self {
        self.config = config;
        self
    }

    /// 指定认证头风格（会话登录场景：Keycloak = Bearer，v2 登录 = Raw）
    pub fn with_auth_style(mut self, style: AuthStyle) -> Self {
        self.auth_style = Some(style);
        self
    }

    /// New Central（v2 REST 形状 + org 查询参数）
    pub fn is_new(&self) -> bool {
        self.resolved() == CentralMode::New
    }

    fn client() -> Result<reqwest::Client> {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(Into::into)
    }

    fn resolved(&self) -> CentralMode {
        match self.mode {
            CentralMode::Auto => self.detected.unwrap_or(CentralMode::Legacy),
            m => m,
        }
    }

    fn base(&self) -> &str {
        match self.resolved() {
            CentralMode::New => &self.config.new_base,
            _ => &self.config.legacy_base,
        }
    }

    /// 取资源路径：用户覆盖优先，否则用该模式默认值；再替换占位符。
    fn path(&self, key: &str, vars: &[(&str, &str)]) -> String {
        let defaults = match self.resolved() {
            CentralMode::New => default_new_paths(),
            _ => default_legacy_paths(),
        };
        let mut p = self
            .config
            .paths
            .get(key)
            .cloned()
            .or_else(|| defaults.get(key).cloned())
            .unwrap_or_default();
        if let Some(org) = &self.org_id {
            p = p.replace("{orgId}", org);
        }
        for (k, v) in vars {
            p = p.replace(&format!("{{{k}}}"), v);
        }
        p
    }

    /// 暴露路径计算结果，供集成测试断言占位符替换是否正确（不发起网络请求）。
    pub fn debug_path(&self, key: &str, vars: &[(&str, &str)]) -> String {
        self.path(key, vars)
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let style = self.auth_style.unwrap_or(match self.resolved() {
            CentralMode::New => AuthStyle::Bearer,
            _ => AuthStyle::Token,
        });
        match style {
            AuthStyle::Token => req.header("Authorization", format!("token {}", self.token)),
            AuthStyle::Bearer => req.header("Authorization", format!("bearer {}", self.token)),
            AuthStyle::Raw => req.header("Authorization", self.token.as_str()),
        }
    }

    pub async fn detect(&mut self) -> Result<CentralMode> {
        if self.mode != CentralMode::Auto {
            self.detected = Some(self.mode);
            return Ok(self.mode);
        }
        let legacy_path = self.path("networks", &[]);
        match Self::client()?
            .get(format!("{}{}", self.config.legacy_base, legacy_path))
            .header("Authorization", format!("token {}", self.token))
            .send()
            .await
        {
            Ok(r) if r.status().as_u16() == 200 => {
                self.detected = Some(CentralMode::Legacy);
                return Ok(CentralMode::Legacy);
            }
            Ok(r) if !matches!(r.status().as_u16(), 401 | 403) => {
                self.detected = Some(CentralMode::Legacy);
                return Ok(CentralMode::Legacy);
            }
            _ => {}
        }
        // 试 New Central：GET /status 恒存在（实测 200），无需 org-id
        let probe_path = {
            let saved = self.detected;
            self.detected = Some(CentralMode::New);
            let p = self.path("status", &[]);
            self.detected = saved;
            p
        };
        let req = Self::client()?
            .get(format!("{}{}", self.config.new_base, probe_path))
            .bearer_auth(&self.token);
        match req.send().await {
            Ok(r) if !matches!(r.status().as_u16(), 401 | 403) => {
                self.detected = Some(CentralMode::New);
                Ok(CentralMode::New)
            }
            Ok(r) => Err(anyhow!("Central 认证失败：token 无效或已过期 ({})", r.status())),
            Err(e) => Err(anyhow!("无法连接 ZeroTier Central：{e}")),
        }
    }

    async fn request(
        &self,
        method: reqwest::Method,
        key: &str,
        vars: &[(&str, &str)],
        query: &[(&str, String)],
        body: Option<Value>,
    ) -> Result<Value> {
        if self.mode == CentralMode::Auto && self.detected.is_none() {
            return Err(anyhow!("Central 模式尚未探测，请先调用 detect()"));
        }
        let path = self.path(key, vars);
        let mut req = Self::client()?.request(method, format!("{}{}", self.base(), path));
        req = self.apply_auth(req);
        if !query.is_empty() {
            req = req.query(query);
        }
        if let Some(b) = body {
            req = req.json(&b);
        }
        let resp = req.send().await.context("无法连接 ZeroTier Central")?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            // AUTH401 前缀：commands 层据此触发会话刷新后重试一次
            if status.as_u16() == 401 {
                return Err(anyhow!("AUTH401: Central API {path} 认证已过期 ({status})"));
            }
            return Err(anyhow!("Central API {path} 返回 {status}: {text}"));
        }
        if text.trim().is_empty() {
            return Ok(json!({}));
        }
        serde_json::from_str(&text).with_context(|| format!("解析 Central 响应失败: {text}"))
    }

    /// 确保 v2 组织 id：优先 settings.org_id，否则 GET /org 取第一个。
    async fn ensure_org(&self) -> Result<String> {
        if let Some(org) = &self.org_id {
            return Ok(org.clone());
        }
        let v = self
            .request(
                reqwest::Method::GET,
                "org",
                &[],
                &[("permission-check", String::new())],
                None,
            )
            .await?;
        normalize::items(&v)
            .first()
            .and_then(|o| o.get("id"))
            .and_then(|x| x.as_str())
            .map(String::from)
            .ok_or_else(|| {
                anyhow!("当前账号下未找到组织：请先在 New Central 创建组织，或在「设置」页填写 Org ID")
            })
    }

    /// 成员列表原始响应。v1 兼容：列表端点不可用时退回网络对象嵌入的 members 映射。
    async fn raw_members(&self, nwid: &str) -> Result<Value> {
        if self.is_new() {
            return self
                .request(reqwest::Method::GET, "members", &[("nwid", nwid)], &[], None)
                .await;
        }
        match self
            .request(reqwest::Method::GET, "members", &[("nwid", nwid)], &[], None)
            .await
        {
            Ok(v) => {
                if normalize::items(&v).is_empty() {
                    if let Ok(n) = self
                        .request(reqwest::Method::GET, "network", &[("nwid", nwid)], &[], None)
                        .await
                    {
                        if n.get("members").is_some() {
                            return Ok(n);
                        }
                    }
                }
                Ok(v)
            }
            Err(_) => {
                self.request(reqwest::Method::GET, "network", &[("nwid", nwid)], &[], None)
                    .await
            }
        }
    }

    /// 网络列表（归一化数组）。v2 带 stats 计数；v1 逐网补算成员计数（端点缺失时保持 null）。
    pub async fn list_networks(&self) -> Result<Value> {
        if self.is_new() {
            let org = self.ensure_org().await?;
            let v = self
                .request(
                    reqwest::Method::GET,
                    "networks",
                    &[],
                    &[("org-id", org), ("stats", "true".into())],
                    None,
                )
                .await?;
            return Ok(json!(normalize::normalize_network_list(&v, true)));
        }
        let v = self.request(reqwest::Method::GET, "networks", &[], &[], None).await?;
        let mut list = normalize::normalize_network_list(&v, false);
        for n in list.iter_mut() {
            let id = n.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
            if id.is_empty() {
                continue;
            }
            if let Ok(raw) = self.raw_members(&id).await {
                let members = normalize::normalize_members(&raw, false);
                let total = members.len();
                let auth = members.iter().filter(|m| m["authorized"] == true).count();
                let online = members.iter().filter(|m| m["online"] == true).count();
                if total > 0 {
                    n["totalMemberCount"] = json!(total);
                    n["authorizedMemberCount"] = json!(auth);
                    n["onlineMemberCount"] = json!(online);
                    n["pendingMemberCount"] = json!(total - auth);
                }
            }
        }
        Ok(json!(list))
    }

    /// 单个网络（归一化，含 v1 形状的 config）。
    pub async fn get_network(&self, nwid: &str) -> Result<Value> {
        let stats_q = vec![("stats", "true".to_string())];
        let query: &[(&str, String)] = if self.is_new() { &stats_q } else { &[] };
        let v = self
            .request(reqwest::Method::GET, "network", &[("nwid", nwid)], query, None)
            .await?;
        Ok(json!(normalize::normalize_network(&v, self.is_new())))
    }

    /// 创建网络。Legacy：`{config:{name}}`；v2：定位组织 → 确保网络组 → 组内创建。
    pub async fn create_network(&self, name: &str) -> Result<Value> {
        if !self.is_new() {
            let v = self
                .request(
                    reqwest::Method::POST,
                    "networks",
                    &[],
                    &[],
                    Some(json!({ "config": { "name": name } })),
                )
                .await?;
            return Ok(json!(normalize::normalize_network(&v, false)));
        }
        let org = self.ensure_org().await?;
        let g = self
            .request(
                reqwest::Method::GET,
                "network_groups",
                &[],
                &[("org-id", org.clone())],
                None,
            )
            .await?;
        let gid = match normalize::items(&g).first().and_then(|x| x.get("id")) {
            Some(Value::String(s)) => s.clone(),
            _ => {
                let created = self
                    .request(
                        reqwest::Method::POST,
                        "org_network_group",
                        &[("orgId", org.as_str())],
                        &[],
                        Some(json!({ "name": "Default", "description": "" })),
                    )
                    .await?;
                created
                    .get("id")
                    .and_then(|x| x.as_str())
                    .map(String::from)
                    .ok_or_else(|| anyhow!("创建网络组失败：响应缺少 id"))?
            }
        };
        let v = self
            .request(
                reqwest::Method::POST,
                "group_network",
                &[("gid", gid.as_str())],
                &[],
                Some(json!({ "name": name, "description": "" })),
            )
            .await?;
        Ok(json!(normalize::normalize_network(&v, true)))
    }

    /// 更新网络。前端按 v1 形状发（嵌套 config），v2 在此转 `{name?, description?, config}`。
    pub async fn update_network(&self, nwid: &str, body: Value) -> Result<Value> {
        let body = if self.is_new() {
            normalize::network_update_body_to_v2(&body)
        } else {
            body
        };
        let v = self
            .request(reqwest::Method::POST, "network", &[("nwid", nwid)], &[], Some(body))
            .await?;
        Ok(json!(normalize::normalize_network(&v, self.is_new())))
    }

    pub async fn delete_network(&self, nwid: &str) -> Result<Value> {
        self.request(reqwest::Method::DELETE, "network", &[("nwid", nwid)], &[], None)
            .await
    }

    /// 成员列表（归一化数组）。
    pub async fn list_members(&self, nwid: &str) -> Result<Value> {
        let raw = self.raw_members(nwid).await?;
        Ok(json!(normalize::normalize_members(&raw, self.is_new())))
    }

    /// 更新成员。兼容两种入参：`{config:{...}}`（v1 传统）与平铺 `{name, description, ipAssignments...}`。
    /// v2 的授权字段转为 authorize/de-authorize 动作端点，其余字段平铺 POST。
    pub async fn update_member(&self, nwid: &str, mid: &str, body: Value) -> Result<Value> {
        let vars = &[("nwid", nwid), ("mid", mid)];
        if self.is_new() {
            let mut last = json!({});
            if let Some(auth) = normalize::member_auth_intent(&body) {
                let action = if auth { "authorize" } else { "de-authorize" };
                last = self
                    .request(
                        reqwest::Method::POST,
                        "member_action",
                        &[("nwid", nwid), ("mid", mid), ("action", action)],
                        &[],
                        None,
                    )
                    .await?;
            }
            let flat = normalize::member_update_body_to_v2(&body);
            if !flat.as_object().map(|m| m.is_empty()).unwrap_or(true) {
                last = self
                    .request(reqwest::Method::POST, "member", vars, &[], Some(flat))
                    .await?;
            }
            return Ok(last);
        }
        let b = normalize::member_update_body_to_v1(&body);
        self.request(reqwest::Method::POST, "member", vars, &[], Some(b)).await
    }

    /// 成员授权动作：`authorize` | `deauthorize` | `reject`。
    /// v2 走专用端点；Legacy 无 reject，等价于取消授权。
    pub async fn member_action(&self, nwid: &str, mid: &str, action: &str) -> Result<Value> {
        if self.is_new() {
            let a = match action {
                "authorize" => "authorize",
                "deauthorize" => "de-authorize",
                "reject" => "reject",
                other => return Err(anyhow!("未知的成员动作: {other}")),
            };
            return self
                .request(
                    reqwest::Method::POST,
                    "member_action",
                    &[("nwid", nwid), ("mid", mid), ("action", a)],
                    &[],
                    None,
                )
                .await;
        }
        let authorized = action == "authorize";
        self.request(
            reqwest::Method::POST,
            "member",
            &[("nwid", nwid), ("mid", mid)],
            &[],
            Some(json!({ "config": { "authorized": authorized } })),
        )
        .await
    }

    /// 删除成员。Legacy：DELETE /member/{mid}；v2：DELETE /member + {data:[{deviceId}]}。
    pub async fn delete_member(&self, nwid: &str, mid: &str) -> Result<Value> {
        if self.is_new() {
            let body = json!({ "data": [{ "deviceId": mid }] });
            return self
                .request(reqwest::Method::DELETE, "members", &[("nwid", nwid)], &[], Some(body))
                .await;
        }
        self.request(
            reqwest::Method::DELETE,
            "member",
            &[("nwid", nwid), ("mid", mid)],
            &[],
            None,
        )
        .await
    }

    /// 组织列表。v2 单数 /org；Legacy 复数 /org。均返回数组。
    pub async fn list_orgs(&self) -> Result<Value> {
        let key = if self.is_new() { "org" } else { "orgs" };
        let check_q = vec![("permission-check", String::new())];
        let query: &[(&str, String)] = if self.is_new() { &check_q } else { &[] };
        let v = self.request(reqwest::Method::GET, key, &[], query, None).await?;
        Ok(json!(normalize::items(&v)))
    }
}
