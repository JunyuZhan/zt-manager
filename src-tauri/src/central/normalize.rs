//! v1/v2 响应归一化与请求体转换。
//!
//! 前端只消费统一形状，不再区分 Legacy（api/v1）与 New Central（api/v2）：
//! - Network: `{id, name, description, authorizedMemberCount, totalMemberCount,
//!   onlineMemberCount, pendingMemberCount, config}`
//!   （v2 的 `v4IpAssignmentPools` 映射为 v1 的 `ipAssignmentPools`，config 内含 name）
//! - Member: `{mid, nodeId, name, desc, authorized, status, ips, physical, mac,
//!   lastOnline, online, lastSeenIp, version, os, arch}`
//!
//! 请求体转换：前端统一按 v1 形状发（嵌套 config），调用 v2 时在此转为平铺结构。

use serde_json::{json, Map, Value};

fn empty_obj() -> Value {
    json!({})
}

fn as_str<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(|x| x.as_str())
}

fn str_or(v: &Value, key: &str) -> String {
    as_str(v, key).unwrap_or("").to_string()
}

/// 任意时间表示转 Unix 毫秒：数字（秒/毫秒自适应）、纯数字字符串、ISO-8601。
pub fn parse_time_ms(v: &Value) -> i64 {
    match v {
        Value::Number(n) => {
            let n = n.as_i64().unwrap_or(0);
            if n <= 0 {
                0
            } else if n < 100_000_000_000 {
                n * 1000 // 秒级（约 1973 年前）
            } else {
                n
            }
        }
        Value::String(s) => {
            let s = s.trim();
            if s.is_empty() {
                return 0;
            }
            if let Ok(n) = s.parse::<f64>() {
                return parse_time_ms(&json!(n));
            }
            iso_to_ms(s).unwrap_or(0)
        }
        _ => 0,
    }
}

/// ISO-8601 (YYYY-MM-DDTHH:MM:SS[.mmm][Z|±HH:MM]) 转 Unix 毫秒。手写解析，免引第三方日期库。
fn iso_to_ms(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 19 || (b[4] != b'-' || b[7] != b'-' || (b[10] != b'T' && b[10] != b' ')) {
        return None;
    }
    let num = |r: std::ops::Range<usize>| s[r].parse::<i64>().ok();
    let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, mi, sec) = (num(11..13)?, num(14..16)?, num(17..19)?);
    let mut ms = days_from_civil(y, mo, d) * 86_400_000
        + h * 3_600_000
        + mi * 60_000
        + sec * 1000;
    // 毫秒部分 .mmm
    let mut i = 19;
    if i < b.len() && b[i] == b'.' {
        i += 1;
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i > start {
            let frac = &s[start..i];
            let mut f: i64 = frac[..frac.len().min(3)].parse().ok()?;
            for _ in frac.len().min(3)..3 {
                f *= 10;
            }
            ms += f;
        }
    }
    // 时区：Z = UTC；+HH:MM / -HH:MM 需扣减
    if i < b.len() {
        match b[i] {
            b'Z' | b'z' => {}
            b'+' | b'-' => {
                let off = s.get(i + 1..i + 3)?.parse::<i64>().ok()? * 3_600_000
                    + s.get(i + 4..i + 6)?.parse::<i64>().ok()? * 60_000;
                ms += if b[i] == b'-' { off } else { -off };
            }
            _ => {}
        }
    }
    Some(ms)
}

/// days_from_civil（Howard Hinnant 算法）：公历日期转距 1970-01-01 的天数。
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// 从数组 / `{items:[...]}` / `{data:[...]}` / `{members:{mid:{...}}}` 提取对象列表。
/// map 形状会把键注入为 `nodeId`（成员 id 即 nodeId）。
pub fn items(v: &Value) -> Vec<Value> {
    if let Some(arr) = v.as_array() {
        return arr.clone();
    }
    for key in ["items", "data"] {
        if let Some(arr) = v.get(key).and_then(|x| x.as_array()) {
            return arr.clone();
        }
    }
    if let Some(map) = v.get("members").and_then(|x| x.as_object()) {
        return map
            .iter()
            .map(|(k, val)| {
                let mut val = val.clone();
                if val.is_object() {
                    let o = val.as_object_mut().unwrap();
                    o.entry("nodeId").or_insert_with(|| json!(k));
                    if let Some(nid) = o.get("nodeId").cloned() {
                        o.entry("id").or_insert(nid);
                    }
                }
                val
            })
            .collect();
    }
    vec![]
}

// ---------- Network ----------

/// v2 config 转 v1 形状：`v4IpAssignmentPools` -> `ipAssignmentPools`（保留原字段），
/// 并把网络名补进 config（前端 `n.config?.name ?? n.name` 两处都取得到）。
pub fn v2_config_to_v1(cfg: &Value, name: &str) -> Value {
    let mut c = cfg.as_object().cloned().unwrap_or_default();
    if !c.contains_key("ipAssignmentPools") {
        if let Some(p) = c.get("v4IpAssignmentPools").cloned() {
            c.insert("ipAssignmentPools".into(), p);
        }
    }
    c.entry("name").or_insert_with(|| json!(name));
    Value::Object(c)
}

fn counts_from(stats: Option<&Value>) -> (Value, Value, Value, Value) {
    let g = |k: &str| {
        stats
            .and_then(|s| s.get(k))
            .and_then(|x| x.as_i64())
            .map(Value::from)
            .unwrap_or(Value::Null)
    };
    (
        g("authorizedDevices"),
        g("totalDevices"),
        g("activeDevices"),
        g("notAuthorizedDevices"),
    )
}

/// 单个网络归一化。v2 传入原始 network 对象（含 stats），v1 传入 v1 网络对象。
pub fn normalize_network(n: &Value, v2: bool) -> Value {
    let id = as_str(n, "id").or_else(|| as_str(n, "nwid")).unwrap_or("");
    let cfg = n.get("config").cloned().unwrap_or_else(empty_obj);
    let name = as_str(n, "name")
        .map(String::from)
        .or_else(|| as_str(&cfg, "name").map(String::from))
        .unwrap_or_default();
    let description = as_str(n, "description")
        .map(String::from)
        .or_else(|| as_str(&cfg, "description").map(String::from))
        .unwrap_or_default();
    let (auth, total, online, pending) = if v2 {
        counts_from(n.get("stats"))
    } else {
        (Value::Null, Value::Null, Value::Null, Value::Null)
    };
    let config = if v2 {
        v2_config_to_v1(&cfg, &name)
    } else {
        cfg
    };
    json!({
        "id": id,
        "name": name,
        "description": description,
        "authorizedMemberCount": auth,
        "totalMemberCount": total,
        "onlineMemberCount": online,
        "pendingMemberCount": pending,
        "config": config,
    })
}

/// 网络列表归一化（数组 / `{items}` 均可）。
pub fn normalize_network_list(v: &Value, v2: bool) -> Vec<Value> {
    items(v).iter().map(|n| normalize_network(n, v2)).collect()
}

// ---------- Member ----------

fn member_node_id(m: &Value, cfg: &Value) -> String {
    if let Some(s) = as_str(m, "nodeId").or_else(|| as_str(m, "deviceId")) {
        return s.to_string();
    }
    if let Some(s) = as_str(cfg, "address") {
        return s.to_string();
    }
    let id = as_str(m, "id").unwrap_or("");
    // v1 成员 id 形如 "{nwid}-{nodeId}"，取连字符后的部分
    match id.split_once('-') {
        Some((_, tail)) if !tail.is_empty() => tail.to_string(),
        _ => id.to_string(),
    }
}

fn member_time_ms(m: &Value) -> i64 {
    for k in ["lastOnline", "lastSeen", "lastSeenTime", "lastAuthorizedTime"] {
        if let Some(v) = m.get(k) {
            let t = parse_time_ms(v);
            if t > 0 {
                return t;
            }
        }
    }
    0
}

/// 单个成员归一化（v1 = 嵌套 config 形状，v2 = oI 平铺形状）。
pub fn normalize_member(m: &Value, v2: bool) -> Value {
    let cfg = m.get("config").cloned().unwrap_or_else(empty_obj);
    let node_id = member_node_id(m, &cfg);
    let status = if v2 {
        str_or(m, "status")
            .trim()
            .trim_start_matches("MEMBER_STATUS_")
            .to_ascii_lowercase()
    } else if m
        .get("authorized")
        .or_else(|| cfg.get("authorized"))
        .and_then(|x| x.as_bool())
        .unwrap_or(false)
    {
        "authorized".into()
    } else {
        "not_authorized".into()
    };
    let authorized = status == "authorized";
    let ips: Vec<String> = {
        let mut list = Vec::new();
        let keys: &[&str] = if v2 {
            &["ipv4Assignments", "ipv6Assignments"]
        } else {
            &["ipAssignments"]
        };
        for k in keys {
            let src = if *k == "ipAssignments" {
                cfg.get("ipAssignments").or_else(|| m.get("ipAssignments"))
            } else {
                m.get(*k)
            };
            if let Some(arr) = src.and_then(|x| x.as_array()) {
                list.extend(arr.iter().filter_map(|x| x.as_str().map(String::from)));
            }
        }
        list
    };
    let last_online = member_time_ms(m);
    let online = m
        .get("online")
        .and_then(|x| x.as_bool())
        .unwrap_or_else(|| last_online > 0 && now_ms() - last_online < 5 * 60_000);
    let mac = if v2 {
        str_or(m, "macAddress")
    } else {
        str_or(m, "physicalAddress")
    };
    json!({
        "mid": node_id,
        "nodeId": node_id,
        "name": as_str(m, "name").or_else(|| as_str(&cfg, "name")).unwrap_or(""),
        "desc": as_str(m, "description").or_else(|| as_str(&cfg, "description")).unwrap_or(""),
        "authorized": authorized,
        "status": status,
        "ips": ips,
        "physical": if mac.is_empty() { str_or(m, "mac") } else { mac.clone() },
        "mac": if mac.is_empty() { str_or(m, "mac") } else { mac },
        "lastOnline": last_online,
        "online": online,
        "lastSeenIp": str_or(m, "lastSeenIp"),
        "version": as_str(m, "clientVersion").or_else(|| as_str(m, "agentVersion")).unwrap_or(""),
        "os": str_or(m, "os"),
        "arch": str_or(m, "arch"),
    })
}

/// 成员列表归一化。
pub fn normalize_members(v: &Value, v2: bool) -> Vec<Value> {
    items(v).iter().map(|m| normalize_member(m, v2)).collect()
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ---------- 请求体转换（前端按 v1 形状发，v2 转平铺） ----------

/// 网络更新体转 v2 形状：`{config:{name, ipAssignmentPools, routes, ...}}`
/// 或 `{name, description, config}` -> v2 的 `{name?, description?, config:{v4IpAssignmentPools?, ...}}`。
pub fn network_update_body_to_v2(body: &Value) -> Value {
    let mut out = Map::new();
    let cfg = body.get("config").cloned().unwrap_or_else(empty_obj);
    if let Some(v) = body.get("name").or_else(|| cfg.get("name")) {
        out.insert("name".into(), v.clone());
    }
    if let Some(v) = body.get("description").or_else(|| cfg.get("description")) {
        out.insert("description".into(), v.clone());
    }
    let mut c = cfg.as_object().cloned().unwrap_or_default();
    c.remove("name");
    c.remove("description");
    if !c.contains_key("v4IpAssignmentPools") {
        if let Some(p) = c.remove("ipAssignmentPools") {
            c.insert("v4IpAssignmentPools".into(), p);
        }
    } else {
        c.remove("ipAssignmentPools");
    }
    if !c.is_empty() {
        out.insert("config".into(), Value::Object(c));
    }
    Value::Object(out)
}

/// 从成员更新体提取授权意图（兼容 `{config:{authorized}}` 与平铺 `{authorized}`）。
pub fn member_auth_intent(body: &Value) -> Option<bool> {
    body.get("authorized")
        .or_else(|| body.get("config").and_then(|c| c.get("authorized")))
        .and_then(|x| x.as_bool())
}

/// 成员更新体转 v2 平铺形状（授权字段已由调用方先行处理，此处剔除）。
/// 输入兼容嵌套 config 与平铺 unified；字段映射 ipAssignments -> ipv4Assignments。
pub fn member_update_body_to_v2(body: &Value) -> Value {
    let cfg = body.get("config");
    let get = |k: &str| -> Option<Value> {
        body.get(k)
            .cloned()
            .or_else(|| cfg.and_then(|c| c.get(k)).cloned())
    };
    let mut out = Map::new();
    for (from, to) in [
        ("name", "name"),
        ("description", "description"),
        ("ipAssignments", "ipv4Assignments"),
        ("ipv6Assignments", "ipv6Assignments"),
        ("activeBridge", "activeBridge"),
        ("noAutoAssignIps", "noAutoAssignIps"),
        ("ssoExempt", "ssoExempt"),
        ("tags", "tags"),
    ] {
        if let Some(v) = get(from) {
            if !v.is_null() {
                out.insert(to.into(), v);
            }
        }
    }
    Value::Object(out)
}

/// 成员更新体转 v1 形状：已嵌套 config 原样透传，平铺则包一层（authorized 保留在 config 内）。
pub fn member_update_body_to_v1(body: &Value) -> Value {
    if body.get("config").map(|c| c.is_object()).unwrap_or(false) {
        body.clone()
    } else {
        json!({ "config": body.clone() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_and_numeric_times() {
        assert_eq!(parse_time_ms(&json!("2023-11-14T22:13:20Z")), 1_700_000_000_000);
        assert_eq!(parse_time_ms(&json!(1_700_000_000_000i64)), 1_700_000_000_000);
        assert_eq!(parse_time_ms(&json!(1_700_000_000i64)), 1_700_000_000_000);
        assert_eq!(parse_time_ms(&json!(0)), 0);
        assert_eq!(parse_time_ms(&json!("garbage")), 0);
    }

    #[test]
    fn v2_network_list_items_shape() {
        let raw = json!({"items": [{
            "id": "n1", "name": "MyNet", "description": "d",
            "stats": {"authorizedDevices": 2, "totalDevices": 3, "activeDevices": 1, "notAuthorizedDevices": 1},
            "config": {"v4IpAssignmentPools": [{"ipRangeStart":"10.0.0.1","ipRangeEnd":"10.0.0.254"}], "routes": [], "private": true}
        }]});
        let list = normalize_network_list(&raw, true);
        assert_eq!(list.len(), 1);
        let n = &list[0];
        assert_eq!(n["id"], "n1");
        assert_eq!(n["name"], "MyNet");
        assert_eq!(n["authorizedMemberCount"], 2);
        assert_eq!(n["totalMemberCount"], 3);
        assert_eq!(n["onlineMemberCount"], 1);
        // v2 config 映射为 v1 字段名，且 config 内含 name
        assert_eq!(n["config"]["ipAssignmentPools"][0]["ipRangeStart"], "10.0.0.1");
        assert_eq!(n["config"]["name"], "MyNet");
    }

    #[test]
    fn legacy_network_array_shape() {
        let raw = json!([{"nwid": "abc", "config": {"name": "old", "ipAssignmentPools": []}}]);
        let list = normalize_network_list(&raw, false);
        assert_eq!(list[0]["id"], "abc");
        assert_eq!(list[0]["name"], "old");
        assert!(list[0]["authorizedMemberCount"].is_null());
    }

    #[test]
    fn v2_member_flat_shape() {
        let raw = json!([{
            "deviceId": "6b0343add6", "name": "laptop", "description": "x",
            "status": "not_authorized", "ipv4Assignments": ["10.1.0.3"],
            "macAddress": "aa:bb:cc:dd:ee:ff", "lastSeenTime": 1_700_000_000_000i64,
            "lastSeenIp": "1.2.3.4", "agentVersion": "1.14.0", "os": "linux", "arch": "x64"
        }]);
        let list = normalize_members(&raw, true);
        let m = &list[0];
        assert_eq!(m["mid"], "6b0343add6");
        assert_eq!(m["authorized"], false);
        assert_eq!(m["status"], "not_authorized");
        assert_eq!(m["ips"][0], "10.1.0.3");
        assert_eq!(m["physical"], "aa:bb:cc:dd:ee:ff");
        assert_eq!(m["lastOnline"], 1_700_000_000_000i64);
        assert_eq!(m["version"], "1.14.0");
    }

    #[test]
    fn legacy_member_config_shape_and_composite_id() {
        let raw = json!({"members": {
            "6b0343add6": {"id": "3efa5cb78aa7c15b-6b0343add6", "config": {"authorized": true, "ipAssignments": ["10.0.0.9"], "name": "n"}, "physicalAddress": "11:22", "lastOnline": 1700000000000i64, "clientVersion": "1.12"}
        }});
        let list = normalize_members(&raw, false);
        let m = &list[0];
        assert_eq!(m["nodeId"], "6b0343add6");
        assert_eq!(m["authorized"], true);
        assert_eq!(m["ips"][0], "10.0.0.9");
        assert_eq!(m["name"], "n");
        assert_eq!(m["version"], "1.12");
    }

    #[test]
    fn body_transforms() {
        let v2 = network_update_body_to_v2(&json!({"config": {"name": "a", "ipAssignmentPools": [], "routes": []}}));
        assert_eq!(v2["name"], "a");
        assert!(v2["config"]["v4IpAssignmentPools"].is_array());
        assert!(v2["config"].get("ipAssignmentPools").is_none());

        assert_eq!(member_auth_intent(&json!({"config": {"authorized": true}})), Some(true));
        assert_eq!(member_auth_intent(&json!({"name": "x"})), None);

        let m2 = member_update_body_to_v2(&json!({"config": {"name": "x", "ipAssignments": ["10.0.0.1"], "authorized": true}}));
        assert_eq!(m2["name"], "x");
        assert_eq!(m2["ipv4Assignments"][0], "10.0.0.1");
        assert!(m2.get("authorized").is_none());

        let m1 = member_update_body_to_v1(&json!({"name": "x"}));
        assert_eq!(m1["config"]["name"], "x");
        let passthrough = json!({"config": {"authorized": false}});
        assert_eq!(member_update_body_to_v1(&passthrough), passthrough);
    }
}
