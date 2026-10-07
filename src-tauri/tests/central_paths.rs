//! Central 路径与占位符替换的回归测试。
//!
//! 背景（实测证据，2026-10-04）：
//! - Legacy：用真实 token 确认成员更新 URL 必须使用 **nodeId**（10 位），
//!   用接口返回的复合 id（`{nwid}-{nodeId}`）会得到 404。
//! - New Central：用无效 token 探测响应码区分「路由存在」与「不存在」：
//!   `/status` `/network` `/network/{id}` `/network/{id}/member[/{mid}]` → 401（存在）
//!   `/orgs` `/orgs/{org}/networks` `/network/{id}/members`              → 404（不存在）
//!   即 New Central 与 Legacy 同为 REST 形状，列表/创建经 `?org-id=` 参数限定组织。
//!
//! 本测试固化该结论，防止后续重构回退。

use std::collections::HashMap;
use zt_manager_lib::central::{CentralClient, CentralMode};

fn client(mode: CentralMode, org: Option<&str>) -> CentralClient {
    CentralClient::new("TEST_TOKEN".into(), mode).with_org(org.map(|s| s.to_string()))
}

#[test]
fn legacy_paths_do_not_contain_org_placeholder() {
    let c = client(CentralMode::Legacy, Some("org-1"));
    // Legacy 路径不应残留 {orgId}，否则会发出含字面花括号的非法 URL
    assert_eq!(c.debug_path("networks", &[]), "/network");
    assert_eq!(c.debug_path("members", &[("nwid", "abc123")]), "/network/abc123/member");
    assert!(!c.debug_path("networks", &[]).contains('{'));
}

#[test]
fn new_paths_share_rest_shape_with_legacy() {
    let c = client(CentralMode::New, Some("org-42"));
    // 实测 /orgs/... 全部 404；正确形状与 Legacy 相同，组织经查询参数限定
    assert_eq!(c.debug_path("networks", &[]), "/network");
    assert_eq!(c.debug_path("network", &[("nwid", "abc123")]), "/network/abc123");
    assert_eq!(
        c.debug_path("members", &[("nwid", "abc123")]),
        "/network/abc123/member"
    );
    // member 为单数；复数 members 实测 404
    let p = c.debug_path("member", &[("nwid", "abc123"), ("mid", "6b0343add6")]);
    assert_eq!(p, "/network/abc123/member/6b0343add6");
    assert!(!p.contains('{'));
    assert!(!p.contains("org-42"));
    // New Central 无组织列表端点（实测 /orgs → 404），不得再注册该路径
    assert!(c.debug_path("orgs", &[]).is_empty());
}

#[test]
fn member_url_uses_node_id_not_composite_id() {
    // 接口返回的成员 id 是 "{nwid}-{nodeId}"；更新时只能取 nodeId 部分
    let composite = "3efa5cb78aa7c15b-6b0343add6";
    let node_id = composite
        .split_once('-')
        .map(|(_, n)| n)
        .expect("复合 id 应形如 {nwid}-{nodeId}");
    assert_eq!(node_id, "6b0343add6");
    let c = client(CentralMode::Legacy, None);
    let url = c.debug_path("member", &[("nwid", "3efa5cb78aa7c15b"), ("mid", node_id)]);
    assert_eq!(url, "/network/3efa5cb78aa7c15b/member/6b0343add6");
    // 明确断言：URL 中不得出现复合 id 的连字符形式
    assert!(!url.contains("3efa5cb78aa7c15b-"));
}

#[test]
fn custom_path_overrides_are_honoured() {
    use zt_manager_lib::central::CentralConfig;
    let mut paths = HashMap::new();
    paths.insert("networks".to_string(), "/custom/{orgId}/nets".to_string());
    let cfg = CentralConfig {
        legacy_base: "https://example.test/v1".into(),
        new_base: "https://example.test/v2".into(),
        paths,
    };
    let c = client(CentralMode::New, Some("o9")).with_config(cfg);
    assert_eq!(c.debug_path("networks", &[]), "/custom/o9/nets");
}
