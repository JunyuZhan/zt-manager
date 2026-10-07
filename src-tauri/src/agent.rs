use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;

const ZT_HOME: &str = r"C:\ProgramData\ZeroTier\One";
const BASE: &str = "http://127.0.0.1:9993";

/// token 的 DPAPI 缓存路径（manifest 为 asInvoker，普通权限不能直读 ProgramData）。
fn token_cache_path() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("com.ztmanager.app").join("authtoken.dpapi")
}

fn direct_token() -> Option<String> {
    let p: PathBuf = PathBuf::from(ZT_HOME).join("authtoken.secret");
    let s = std::fs::read_to_string(p).ok()?;
    let s = s.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn cached_token() -> Option<String> {
    let enc = std::fs::read(token_cache_path()).ok()?;
    let plain = crate::secure::unprotect(&enc).ok()?;
    let s = String::from_utf8(plain).ok()?;
    let s = s.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

/// UAC 提权把 authtoken.secret 复制到用户目录，随后本进程 DPAPI 加密落盘并删除明文。
fn fetch_token_via_uac() -> Result<String> {
    let src = PathBuf::from(ZT_HOME).join("authtoken.secret");
    let cache = token_cache_path();
    let dir = cache.parent().context("缓存目录无效")?;
    std::fs::create_dir_all(dir).context("创建缓存目录失败")?;
    let tmp = dir.join("authtoken.tmp");
    // 写临时 ps1 后提权执行（避免多层引号转义），脚本失败时退出码非 0。
    let script = format!(
        "$ErrorActionPreference='Stop'\nCopy-Item -LiteralPath '{src}' -Destination '{dst}' -Force\n",
        src = src.display(),
        dst = tmp.display()
    );
    let ps1 = dir.join("fetch_token.ps1");
    std::fs::write(&ps1, script).context("写临时脚本失败")?;
    let arg = format!(
        "-NoProfile -ExecutionPolicy Bypass -File \"{}\"",
        ps1.display()
    );
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "Start-Process powershell -Verb RunAs -Wait -ArgumentList '{}'",
                arg.replace('\'', "''")
            ),
        ])
        .creation_flags(0x08000000)
        .output()
        .context("调用 PowerShell 提权失败")?;
    let _ = std::fs::remove_file(&ps1);
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow!("读取 ZeroTier token 需要管理员权限，UAC 未完成：{err}"));
    }
    let plain = std::fs::read_to_string(&tmp)
        .map_err(|_| anyhow!("提权复制 token 后仍无法读取，可能已取消 UAC"))?;
    let _ = std::fs::remove_file(&tmp);
    let plain = plain.trim().to_string();
    if plain.is_empty() {
        return Err(anyhow!("复制得到的 token 为空"));
    }
    let enc = crate::secure::protect(plain.as_bytes()).context("DPAPI 加密 token 失败")?;
    std::fs::write(&cache, enc).context("写入 token 缓存失败")?;
    Ok(plain)
}

fn invalidate_token_cache() {
    let _ = std::fs::remove_file(token_cache_path());
}

/// 优先直读（管理员运行时可用），其次 DPAPI 缓存，最后 UAC 提权复制一次。
pub fn authtoken() -> Result<String> {
    if let Some(t) = direct_token() {
        return Ok(t);
    }
    if let Some(t) = cached_token() {
        return Ok(t);
    }
    fetch_token_via_uac()
}

fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(Into::into)
}

/// 统一请求：token 失效（401/403）时清缓存重取一次（ZeroTier 重装会换 token）。
async fn request(
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
    allow_retry: bool,
) -> Result<(reqwest::StatusCode, String)> {
    let token = authtoken()?;
    let mut req = client()?
        .request(method.clone(), format!("{BASE}{path}"))
        .header("X-ZT1-Auth", token);
    if let Some(b) = &body {
        req = req.json(b);
    }
    let resp = req
        .send()
        .await
        .with_context(|| format!("无法连接本地 ZeroTier 服务 ({BASE})，服务是否在运行？"))?;
    let status = resp.status();
    let text = resp.text().await?;
    if matches!(status.as_u16(), 401 | 403) && allow_retry {
        invalidate_token_cache();
        let token = authtoken()?;
        let mut req = client()?
            .request(method, format!("{BASE}{path}"))
            .header("X-ZT1-Auth", token);
        if let Some(b) = &body {
            req = req.json(b);
        }
        let resp = req.send().await.context("重试本地 ZeroTier 服务请求失败")?;
        return Ok((resp.status(), resp.text().await?));
    }
    Ok((status, text))
}

fn parse(path: &str, status: reqwest::StatusCode, text: &str) -> Result<Value> {
    if !status.is_success() {
        return Err(anyhow!("本地 API {path} 返回 {status}: {text}"));
    }
    if text.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(text).with_context(|| format!("解析本地 API 响应失败: {text}"))
}

async fn get(path: &str) -> Result<Value> {
    let (s, t) = request(reqwest::Method::GET, path, None, true).await?;
    parse(path, s, &t)
}

async fn post(path: &str, body: Option<Value>) -> Result<Value> {
    let (s, t) = request(reqwest::Method::POST, path, body, true).await?;
    parse(path, s, &t)
}

async fn del(path: &str) -> Result<Value> {
    let (s, t) = request(reqwest::Method::DELETE, path, None, true).await?;
    if !s.is_success() {
        return Err(anyhow!("本地 API {path} 返回 {s}: {t}"));
    }
    // DELETE 的响应体不保证是 JSON（部分端点返回空或纯文本），故此容忍解析失败
    Ok(serde_json::from_str(&t).unwrap_or_else(|_| json!({})))
}

pub async fn status() -> Result<Value> {
    get("/status").await
}

pub async fn network_list() -> Result<Vec<String>> {
    // 实测（2026-10-04）：/network 返回对象数组（字段含 nwid/id/name/status…），
    // 不是字符串数组；对象的网络 ID 取 nwid（与 id 等值）。
    let v = get("/network").await?;
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| match x {
                    Value::String(s) => Some(s.clone()),
                    Value::Object(o) => o
                        .get("nwid")
                        .or_else(|| o.get("id"))
                        .and_then(|k| k.as_str())
                        .map(String::from),
                    _ => None,
                })
                .collect()
        })
        .ok_or_else(|| anyhow!("意外的 /network 响应"))
}

pub async fn network_detail(nwid: &str) -> Result<Value> {
    get(&format!("/network/{nwid}")).await
}

pub async fn join(nwid: &str) -> Result<Value> {
    post(&format!("/network/{nwid}"), None).await
}

pub async fn leave(nwid: &str) -> Result<Value> {
    del(&format!("/network/{nwid}")).await
}

pub async fn peers() -> Result<Value> {
    get("/peer").await
}

// ---- 本地控制器 ----

pub async fn controller_networks() -> Result<Value> {
    get("/controller/network").await
}

pub async fn controller_network(nwid: &str) -> Result<Value> {
    get(&format!("/controller/network/{nwid}")).await
}

pub async fn controller_create() -> Result<Value> {
    // 实测（2026-10-04）：POST /controller/network/ -> 404；
    // 必须带本机 nodeId 前缀：POST /controller/network/{nodeId}______ -> 200，控制器生成随机 nwid。
    let status = get("/status").await?;
    let node_id = status
        .get("address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("无法从 /status 取得本机 nodeId"))?;
    post(&format!("/controller/network/{node_id}______"), Some(json!({}))).await
}

pub async fn controller_update(nwid: &str, body: Value) -> Result<Value> {
    post(&format!("/controller/network/{nwid}"), Some(body)).await
}

pub async fn controller_members(nwid: &str) -> Result<Value> {
    let v = get(&format!("/controller/network/{nwid}/member")).await?;
    Ok(v)
}

pub async fn controller_member(nwid: &str, mid: &str) -> Result<Value> {
    get(&format!("/controller/network/{nwid}/member/{mid}")).await
}

pub async fn controller_update_member(nwid: &str, mid: &str, body: Value) -> Result<Value> {
    post(&format!("/controller/network/{nwid}/member/{mid}"), Some(body)).await
}
