use reqwest::{
    Client, Url,
    dns::{Addrs, Name, Resolve, Resolving},
    redirect,
};
use std::{
    net::{IpAddr, SocketAddr},
    sync::{Arc, LazyLock},
    time::Duration,
};

pub static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .user_agent("mipmap")
        .timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(0)
        .no_proxy()
        .dns_resolver(Arc::new(PublicResolver))
        .redirect(redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 10 {
                attempt.error("too many redirects")
            } else if is_public_url(attempt.url()) {
                attempt.follow()
            } else {
                attempt.error("redirected to a private address")
            }
        }))
        .build()
        .expect("HTTP client must build")
});

struct PublicResolver;

impl Resolve for PublicResolver {
    fn resolve(&self, name: Name) -> Resolving {
        Box::pin(async move {
            let addrs: Vec<SocketAddr> = tokio::net::lookup_host((name.as_str(), 0))
                .await?
                .filter(|addr| is_public(addr.ip()))
                .collect();
            if addrs.is_empty() {
                return Err("host has no public address".into());
            }
            Ok(Box::new(addrs.into_iter()) as Addrs)
        })
    }
}

pub fn is_public_url(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some_and(|host| {
            host.trim_start_matches('[')
                .trim_end_matches(']')
                .parse()
                .map_or(true, is_public)
        })
}

pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_broadcast()
                || ip.is_documentation()
                || ip.is_multicast()
                || a == 0
                || a >= 240
                || (a == 100 && (64..128).contains(&b))
                || (a == 198 && (18..20).contains(&b))
                || (a == 192 && b == 0 && c == 0)
                || (a == 192 && b == 88 && c == 99))
        }
        IpAddr::V6(ip) => match ip.to_ipv4() {
            Some(ip) => is_public(IpAddr::V4(ip)),
            None => {
                let segments = ip.segments();
                !(ip.is_multicast()
                    || ip.is_unique_local()
                    || ip.is_unicast_link_local()
                    || segments[..2] == [0x2001, 0xdb8]
                    || (segments[0] == 0x2001 && segments[1] < 0x200)
                    || segments[0] & 0xffc0 == 0xfec0
                    || segments[..4] == [0x100, 0, 0, 0]
                    || segments[..2] == [0x64, 0xff9b]
                    || segments[0] == 0x2002)
            }
        },
    }
}
