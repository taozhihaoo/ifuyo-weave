//! M2.10 网络目标安全边界（§16 SSRF）：本地/内网/链路本地默认拒绝。
//!
//! - **不在 URL 字符串检查后就盲目请求**（§16）：SSRF 分类挂在自定义
//!   Resolver 上——DNS 解析结果逐地址分类后才交给连接层，rebinding
//!   TOCTOU 被闭合（连接用的就是被分类过的地址）。
//! - 分类：Loopback / Private / LinkLocal / Unresolved（域名待解析）/
//!   Public / Unknown。`allow_private` 仅在用户显式开启时放行（默认拒）。
//! - 已知残留限制（如实 §16）：分类基于 IP 拓扑，无法识别"公网 IP 但
//!   实际内网映射"的网络环境。

use std::io;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};

use weave_core::prelude::WeaveError;

fn err(code: &'static str, message: impl Into<String>) -> WeaveError {
    WeaveError::io(code, message).with_location("web::target")
}

/// 网络目标分类（§16）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TargetClass {
    Public,
    Private,
    Loopback,
    LinkLocal,
    /// 域名尚未解析（连接时由 Resolver 分类）。
    Unresolved,
    Unknown,
}

/// 字面 IP / localhost 主机名分类（域名 ⇒ Unresolved，等待 Resolver）。
pub fn classify_host(host: &str) -> TargetClass {
    let lower = host.to_lowercase();
    if lower == "localhost" || lower.ends_with(".localhost") || lower.ends_with(".local") {
        return TargetClass::Loopback;
    }
    let Ok(ip) = lower.parse::<IpAddr>() else {
        return TargetClass::Unresolved;
    };
    classify_ip(ip)
}

/// 单个 IP 分类。
pub fn classify_ip(ip: IpAddr) -> TargetClass {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            if o[0] == 127 || o == [0, 0, 0, 0] {
                TargetClass::Loopback
            } else if o[0] == 10
                || (o[0] == 172 && (16..=31).contains(&o[1]))
                || (o[0] == 192 && o[1] == 168)
            {
                TargetClass::Private
            } else if o[0] == 169 && o[1] == 254 {
                TargetClass::LinkLocal
            } else if o[0] == 100 && (64..=127).contains(&o[1]) {
                // CGNAT 100.64/10——运营商内网，按 Private 保守处理
                TargetClass::Private
            } else {
                TargetClass::Public
            }
        }
        IpAddr::V6(v6) => {
            let seg = v6.segments();
            if v6.is_loopback() {
                TargetClass::Loopback
            } else if (0xfc00..0xfe00).contains(&seg[0]) {
                // unique local fc00::/7
                TargetClass::Private
            } else if (0xfe80..0xfec0).contains(&seg[0]) {
                TargetClass::LinkLocal
            } else if let Some(mapped) = v6.to_ipv4_mapped() {
                classify_ip(IpAddr::V4(mapped))
            } else {
                TargetClass::Public
            }
        }
    }
}

/// ureq 自定义 Resolver：解析结果逐地址分类，非 Public 且未放行 ⇒ 拒绝。
/// ureq 用返回的地址建立连接 ⇒ 分类作用于真实连接目标（§16 rebinding 防护）。
pub struct GuardedResolver {
    pub allow_private: bool,
}

impl ureq::Resolver for GuardedResolver {
    fn resolve(&self, netloc: &str) -> io::Result<Vec<SocketAddr>> {
        // netloc 形如 "host:port"
        let addrs: Vec<SocketAddr> = netloc.to_socket_addrs()?.collect();
        if addrs.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no addresses resolved",
            ));
        }
        for addr in &addrs {
            let class = classify_ip(addr.ip());
            let allowed = match class {
                TargetClass::Public => true,
                TargetClass::Unknown => true,
                _ => self.allow_private,
            };
            if !allowed {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!(
                        "target {:?} {} is not allowed (local/private network blocked)",
                        class,
                        addr.ip()
                    ),
                ));
            }
        }
        Ok(addrs)
    }
}

/// 请求前主机名预检（字面 IP/localhost 立即拦截；域名由 Resolver 拦）。
pub fn precheck(host: &str, allow_private: bool) -> Result<(), WeaveError> {
    match classify_host(host) {
        TargetClass::Unresolved | TargetClass::Public | TargetClass::Unknown => Ok(()),
        _class if allow_private => Ok(()),
        class => Err(err(
            "web.targetBlocked",
            format!(
                "target is {:?} — local/private network access requires explicit allow",
                class
            ),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn literal_addresses_are_classified() {
        assert_eq!(
            classify_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
            TargetClass::Loopback
        );
        assert_eq!(
            classify_ip(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))),
            TargetClass::Loopback
        );
        assert_eq!(
            classify_ip(IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3))),
            TargetClass::Private
        );
        assert_eq!(
            classify_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))),
            TargetClass::Private
        );
        assert_eq!(
            classify_ip(IpAddr::V4(Ipv4Addr::new(172, 32, 0, 1))),
            TargetClass::Public
        );
        assert_eq!(
            classify_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))),
            TargetClass::Private
        );
        assert_eq!(
            classify_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))),
            TargetClass::LinkLocal
        );
        assert_eq!(
            classify_ip(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))),
            TargetClass::Private
        );
        assert_eq!(
            classify_ip(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))),
            TargetClass::Public
        );
        // IPv6
        assert_eq!(classify_ip("::1".parse().unwrap()), TargetClass::Loopback);
        assert_eq!(
            classify_ip("fc00::1".parse().unwrap()),
            TargetClass::Private
        );
        assert_eq!(
            classify_ip("fe80::1".parse().unwrap()),
            TargetClass::LinkLocal
        );
        assert_eq!(
            classify_ip("::ffff:192.168.0.1".parse().unwrap()),
            TargetClass::Private
        );
    }

    #[test]
    fn hostnames_and_localhost() {
        assert_eq!(classify_host("localhost"), TargetClass::Loopback);
        assert_eq!(classify_host("foo.localhost"), TargetClass::Loopback);
        assert_eq!(classify_host("nas.local"), TargetClass::Loopback);
        assert_eq!(classify_host("10.0.0.2"), TargetClass::Private);
        assert_eq!(classify_host("example.com"), TargetClass::Unresolved);
    }

    #[test]
    fn resolver_refuses_private_and_allows_when_permitted() {
        // Resolver 层拦截（本地解析即可测，无需真实外网）
        let denied = ureq::Resolver::resolve(
            &GuardedResolver {
                allow_private: false,
            },
            "127.0.0.1:80",
        );
        assert!(matches!(denied, Err(e) if e.kind() == io::ErrorKind::PermissionDenied));
        // allow_private ⇒ 同一地址放行（连接成功与否交连接层——分类不再拦）
        let allowed = ureq::Resolver::resolve(
            &GuardedResolver {
                allow_private: true,
            },
            "127.0.0.1:0",
        );
        assert!(allowed.is_ok());
    }

    #[test]
    fn precheck_blocks_local_without_allow() {
        assert!(precheck("localhost", false).is_err());
        assert!(precheck("127.0.0.1", false).is_err());
        assert!(precheck("192.168.1.10", false).is_err());
        assert!(precheck("169.254.169.254", false).is_err());
        assert!(precheck("localhost", true).is_ok());
        assert!(precheck("example.com", false).is_ok());
    }
}
