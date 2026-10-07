//! ZeroTier 账号密码登录。
//!
//! 两种登录方式，自动切换（实测依据 2026-10-07）：
//! 1. New Central v2：`POST <new_base>/user/login` body `{username,password,otp?}`
//!    → `{status:"success", token, refresh_token}`；凭据错误 `failed`；2FA `otp_required`；限流 `backoff`。
//!    认证头为**裸 token**（无 Bearer 前缀，官方 SPA `headers.setAuthorization(token)`）。
//! 2. Legacy Keycloak：`POST accounts.zerotier.com/realms/zerotier/protocol/openid-connect/token`
//!    `grant_type=password&client_id=zt-central&scope=openid profile email offline_access`
//!    → `{access_token, refresh_token, expires_in}`；认证头 `Authorization: bearer <oidc>`。
//!
//! 顺序：先试 v2，凭据被拒/网络错误则回退 Keycloak；都失败才报错。
//! 会话用 DPAPI 加密落盘，过期前自动刷新。

use crate::secure;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const KC_TOKEN_URL: &str =
    "https://accounts.zerotier.com/realms/zerotier/protocol/openid-connect/token";
const KC_CLIENT_ID: &str = "zt-central";
const DEFAULT_NEW_BASE: &str = "https://central.zerotier.com/api/v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionMode {
    /// New Central（central.zerotier.com/api/v2，裸 token）
    Central,
    /// Legacy（my.zerotier.com/api/v1，Keycloak OIDC bearer）
    Keycloak,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub mode: SessionMode,
    pub email: String,
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Unix 秒；None = 未知（不做主动刷新）
    #[serde(default)]
    pub expires_at: Option<i64>,
}

impl Session {
    /// 是否接近过期（30 秒缓冲）；未知过期时间视为有效
    pub fn expiring_soon(&self) -> bool {
        match self.expires_at {
            Some(exp) => now_secs() + 30 >= exp,
            None => false,
        }
    }
}

pub enum LoginOutcome {
    Success(Session),
    /// v2 账号开启 2FA，需带 otp 重试
    NeedOtp,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn http() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(Into::into)
}

fn new_base_url(new_base: &str) -> String {
    let b = new_base.trim();
    if b.is_empty() {
        DEFAULT_NEW_BASE.to_string()
    } else {
        b.trim_end_matches('/').to_string()
    }
}

/// base64url 解码（无填充），用于解析 JWT payload
fn b64url_decode(s: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'-' => Some(62),
            b'_' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::new();
    let mut buf: u32 = 0;
    let mut bits = 0u32;
    for &c in s.as_bytes() {
        let v = val(c)?;
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xFF) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// 从 JWT 解析 exp（Unix 秒）；非 JWT 或解析失败返回 None
pub fn jwt_exp(token: &str) -> Option<i64> {
    let payload = token.split('.').nth(1)?;
    let cleaned: String = payload
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    let bytes = b64url_decode(&cleaned)?;
    let v: Value = serde_json::from_slice(&bytes).ok()?;
    v.get("exp")?.as_i64()
}

#[derive(Deserialize)]
struct V2LoginResp {
    status: String,
    #[serde(default)]
    token: String,
    #[serde(default)]
    refresh_token: String,
    #[serde(default)]
    message: String,
}

/// New Central v2 登录。Ok(None) = 凭据被拒（可回退 Keycloak）；
/// Err("__need_otp__") = 需要 OTP。
async fn login_v2(
    new_base: &str,
    email: &str,
    password: &str,
    otp: Option<&str>,
) -> Result<Option<Session>> {
    let mut body = json!({ "username": email, "password": password });
    if let Some(o) = otp {
        body["otp"] = json!(o);
    }
    let resp = http()?
        .post(format!("{}/user/login", new_base_url(new_base)))
        .json(&body)
        .send()
        .await
        .context("无法连接 New Central 登录接口")?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(anyhow!("New Central 登录接口返回 {status}: {text}"));
    }
    let r: V2LoginResp =
        serde_json::from_str(&text).with_context(|| format!("解析登录响应失败: {text}"))?;
    match r.status.as_str() {
        "success" => {
            if r.token.is_empty() {
                return Err(anyhow!("New Central 登录响应缺少 token"));
            }
            Ok(Some(Session {
                mode: SessionMode::Central,
                email: email.to_string(),
                expires_at: jwt_exp(&r.token),
                refresh_token: if r.refresh_token.is_empty() {
                    None
                } else {
                    Some(r.refresh_token)
                },
                access_token: r.token,
            }))
        }
        "otp_required" => Err(anyhow!("__need_otp__")),
        "backoff" => Err(anyhow!(
            "登录尝试过于频繁，请稍后再试{}",
            if r.message.is_empty() {
                String::new()
            } else {
                format!("：{}", r.message)
            }
        )),
        // failed 或未知状态 → 凭据可能属于 Legacy 体系
        _ => Ok(None),
    }
}

/// Keycloak 密码模式登录。Err("__bad_credentials__") = 凭据错误。
async fn login_keycloak(email: &str, password: &str) -> Result<Session> {
    let resp = http()?
        .post(KC_TOKEN_URL)
        .form(&[
            ("grant_type", "password"),
            ("client_id", KC_CLIENT_ID),
            ("username", email),
            ("password", password),
            ("scope", "openid profile email offline_access"),
        ])
        .send()
        .await
        .context("无法连接 ZeroTier 账号服务（accounts.zerotier.com）")?;
    let status = resp.status();
    let text = resp.text().await?;
    if status.as_u16() == 401 {
        return Err(anyhow!("__bad_credentials__"));
    }
    if !status.is_success() {
        let detail = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| {
                v.get("error_description")
                    .and_then(|d| d.as_str())
                    .map(String::from)
            })
            .unwrap_or(text);
        return Err(anyhow!("账号服务返回 {status}: {detail}"));
    }
    parse_keycloak_token(&text, email)
}

fn parse_keycloak_token(text: &str, email: &str) -> Result<Session> {
    let v: Value =
        serde_json::from_str(text).with_context(|| format!("解析登录响应失败: {text}"))?;
    let access = v
        .get("access_token")
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow!("账号服务响应缺少 access_token"))?
        .to_string();
    let refresh = v
        .get("refresh_token")
        .and_then(|t| t.as_str())
        .map(String::from);
    let expires_in = v.get("expires_in").and_then(|t| t.as_i64());
    Ok(Session {
        mode: SessionMode::Keycloak,
        email: email.to_string(),
        expires_at: expires_in.map(|s| now_secs() + s),
        access_token: access,
        refresh_token: refresh,
    })
}

/// 登录：先试 New Central v2，凭据被拒则回退 Keycloak。
pub async fn login(
    new_base: &str,
    email: &str,
    password: &str,
    otp: Option<&str>,
) -> Result<LoginOutcome> {
    let email = email.trim();
    if email.is_empty() || password.is_empty() {
        return Err(anyhow!("请输入邮箱和密码"));
    }
    let mut v2_err: Option<String> = None;
    match login_v2(new_base, email, password, otp).await {
        Ok(Some(s)) => return Ok(LoginOutcome::Success(s)),
        Ok(None) => {}
        Err(e) if e.to_string() == "__need_otp__" => return Ok(LoginOutcome::NeedOtp),
        Err(e) => v2_err = Some(e.to_string()),
    }
    match login_keycloak(email, password).await {
        Ok(s) => Ok(LoginOutcome::Success(s)),
        Err(e) if e.to_string() == "__bad_credentials__" => Err(match v2_err {
            Some(v2) => anyhow!("邮箱或密码错误（New Central：{v2}）"),
            None => anyhow!("邮箱或密码错误"),
        }),
        Err(e) => Err(match v2_err {
            Some(v2) => anyhow!("两种登录方式均失败：\nNew Central：{v2}\nKeycloak：{e}"),
            None => e,
        }),
    }
}

/// 刷新会话；失败返回错误（调用方应清除会话并要求重新登录）
pub async fn refresh(new_base: &str, session: &Session) -> Result<Session> {
    let refresh_token = session
        .refresh_token
        .as_deref()
        .ok_or_else(|| anyhow!("会话已过期且无刷新令牌，请重新登录"))?;
    match session.mode {
        SessionMode::Central => refresh_v2(new_base, refresh_token, session).await,
        SessionMode::Keycloak => refresh_keycloak(refresh_token, session).await,
    }
}

async fn refresh_v2(new_base: &str, refresh_token: &str, old: &Session) -> Result<Session> {
    let resp = http()?
        .post(format!("{}/user/refresh-token", new_base_url(new_base)))
        .json(&json!({ "refresh_token": refresh_token }))
        .send()
        .await
        .context("无法连接 New Central 刷新接口")?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(anyhow!("刷新令牌已失效，请重新登录 ({status})"));
    }
    let v: Value = serde_json::from_str(&text)?;
    let ok = v.get("status").and_then(|s| s.as_str()) == Some("success");
    let token = v.get("token").and_then(|t| t.as_str()).unwrap_or("");
    if !ok || token.is_empty() {
        return Err(anyhow!("刷新令牌已失效，请重新登录"));
    }
    let new_refresh = v
        .get("refresh_token")
        .and_then(|t| t.as_str())
        .map(String::from);
    Ok(Session {
        mode: old.mode,
        email: old.email.clone(),
        expires_at: jwt_exp(token),
        refresh_token: new_refresh.or_else(|| old.refresh_token.clone()),
        access_token: token.to_string(),
    })
}

async fn refresh_keycloak(refresh_token: &str, old: &Session) -> Result<Session> {
    let resp = http()?
        .post(KC_TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", KC_CLIENT_ID),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await
        .context("无法连接 ZeroTier 账号服务")?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(anyhow!("刷新令牌已失效，请重新登录 ({status})"));
    }
    let v: Value = serde_json::from_str(&text)?;
    let access = v
        .get("access_token")
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow!("刷新令牌已失效，请重新登录"))?;
    let expires_in = v.get("expires_in").and_then(|t| t.as_i64());
    Ok(Session {
        mode: old.mode,
        email: old.email.clone(),
        expires_at: expires_in.map(|s| now_secs() + s),
        refresh_token: v
            .get("refresh_token")
            .and_then(|t| t.as_str())
            .map(String::from)
            .or_else(|| old.refresh_token.clone()),
        access_token: access.to_string(),
    })
}

/// 会话落盘（DPAPI 加密）
pub fn save(path: &Path, session: &Session) -> Result<()> {
    let json = serde_json::to_vec(session)?;
    let enc = secure::protect(&json).map_err(|e| anyhow!("加密会话失败: {e}"))?;
    std::fs::write(path, enc).with_context(|| format!("写入会话文件失败: {}", path.display()))?;
    Ok(())
}

/// 读取会话；文件不存在或解密/解析失败返回 None
pub fn load(path: &Path) -> Option<Session> {
    let enc = std::fs::read(path).ok()?;
    let plain = secure::unprotect(&enc).ok()?;
    serde_json::from_slice(&plain).ok()
}

pub fn clear(path: &Path) {
    let _ = std::fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwt_exp_parses_standard_jwt() {
        // {"exp":1700000000} 的 base64url 编码（含无意义头部段）
        let payload = "eyJzdWIiOiIxIiwiZXhwIjoxNzAwMDAwMDAwfQ";
        let token = format!("eyJhbGciOiJIUzI1NiJ9.{payload}.sig");
        assert_eq!(jwt_exp(&token), Some(1700000000));
    }

    #[test]
    fn jwt_exp_rejects_garbage() {
        assert_eq!(jwt_exp("not-a-jwt"), None);
        assert_eq!(jwt_exp("a.!!!!.b"), None);
    }

    #[test]
    fn keycloak_parse_roundtrip() {
        let text = r#"{"access_token":"at","refresh_token":"rt","expires_in":60}"#;
        let s = parse_keycloak_token(text, "a@b.c").unwrap();
        assert_eq!(s.mode, SessionMode::Keycloak);
        assert_eq!(s.access_token, "at");
        assert_eq!(s.refresh_token.as_deref(), Some("rt"));
        assert!(s.expires_at.is_some());
    }

    #[test]
    fn session_disk_roundtrip() {
        let dir = std::env::temp_dir().join("ztmgr-session-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.json");
        let s = Session {
            mode: SessionMode::Central,
            email: "x@y.z".into(),
            access_token: "tok".into(),
            refresh_token: Some("r".into()),
            expires_at: Some(1700000000),
        };
        save(&path, &s).unwrap();
        let loaded = load(&path).expect("应能读回会话");
        assert_eq!(loaded.access_token, "tok");
        assert_eq!(loaded.mode, SessionMode::Central);
        clear(&path);
        assert!(load(&path).is_none());
    }
}
