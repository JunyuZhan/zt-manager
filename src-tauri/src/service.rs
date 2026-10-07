use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::os::windows::process::CommandExt;
use std::process::Command;

const SERVICE: &str = "ZeroTierOneService";

fn run(cmd: &str, args: &[&str]) -> Result<String> {
    let out = Command::new(cmd)
        .args(args)
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .with_context(|| format!("执行 {cmd} 失败"))?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn query() -> Result<Value> {
    let out = run("sc", &["query", SERVICE])?;
    let state = if out.contains("RUNNING") {
        "running"
    } else if out.contains("STOPPED") {
        "stopped"
    } else if out.contains("START_PENDING") || out.contains("STOP_PENDING") {
        "pending"
    } else {
        "unknown"
    };
    let pid = out
        .lines()
        .find(|l| l.contains("PID"))
        .and_then(|l| l.split(':').nth(1))
        .and_then(|p| p.trim().parse::<u32>().ok());
    Ok(json!({ "state": state, "pid": pid }))
}

fn elevate(action: &str) -> Result<()> {
    // 通过 UAC 提权执行 sc start/stop
    let ps = format!(
        "Start-Process sc -ArgumentList '{action}','{SERVICE}' -Verb RunAs -Wait"
    );
    let out = Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps])
        .creation_flags(0x08000000)
        .output()
        .context("调用 PowerShell 提权失败")?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        if err.contains("canceled") || err.contains("取消") {
            return Err(anyhow!("已取消 UAC 提权"));
        }
        return Err(anyhow!("提权执行失败: {err}"));
    }
    Ok(())
}

/// 执行单个 sc 动作；若被拒绝访问则经 UAC 提权重试。
fn do_action(sc_action: &str) -> Result<()> {
    let out = run("sc", &[sc_action, SERVICE])?;
    // 注意：不能只判断含 "5"，否则 PID 等数字会造成误判
    let denied = out.contains("FAILED")
        || out.contains("ACCESS_DENIED")
        || out.contains("Error 5")
        || out.contains("拒绝访问");
    if denied {
        elevate(sc_action)?;
    }
    Ok(())
}

pub fn control(action: &str) -> Result<Value> {
    match action {
        "start" | "stop" => {
            do_action(action)?;
            std::thread::sleep(std::time::Duration::from_millis(800));
            query()
        }
        "restart" => {
            do_action("stop")?;
            std::thread::sleep(std::time::Duration::from_millis(1500));
            do_action("start")?;
            std::thread::sleep(std::time::Duration::from_millis(1200));
            query()
        }
        _ => Err(anyhow!("未知操作: {action}（应为 start/stop/restart）")),
    }
}
