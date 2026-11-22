//! Android-resilient HTTP/DNS builder for `ureq`.
//!
//! On Android/Waydroid, Bionic `getaddrinfo` (`/dev/socket/dnsproxyd`) and
//! system CA paths can fail inside some containers/networks. This helper
//! configures WebPKI root certificates and, on Android, falls back to direct
//! UDP DNS A-record queries (`1.1.1.1`, `8.8.8.8`, `192.168.240.1`) whenever
//! the system resolver fails.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use ureq::http::Uri;
use ureq::unversioned::resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::NextTimeout;

pub fn webpki_tls() -> ureq::tls::TlsConfig {
    ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::WebPki)
        .build()
}

pub fn default_agent() -> ureq::Agent {
    build_agent(
        ureq::Agent::config_builder()
            .tls_config(webpki_tls())
            .build(),
    )
}

pub fn build_agent(config: ureq::config::Config) -> ureq::Agent {
    #[cfg(target_os = "android")]
    {
        ureq::Agent::with_parts(
            config,
            ureq::unversioned::transport::DefaultConnector::default(),
            AndroidDnsResolver::default(),
        )
    }
    #[cfg(not(target_os = "android"))]
    {
        config.into()
    }
}

#[derive(Debug, Default)]
pub struct AndroidDnsResolver {
    inner: DefaultResolver,
}

static DNS_CACHE: OnceLock<Mutex<HashMap<String, Vec<Ipv4Addr>>>> = OnceLock::new();

impl Resolver for AndroidDnsResolver {
    fn resolve(
        &self,
        uri: &Uri,
        config: &ureq::config::Config,
        timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, ureq::Error> {
        if let Ok(addrs) = self.inner.resolve(uri, config, timeout) {
            if !addrs.is_empty() {
                return Ok(addrs);
            }
        }
        let authority = uri.authority().ok_or(ureq::Error::HostNotFound)?;
        let scheme = uri.scheme().ok_or(ureq::Error::HostNotFound)?;
        let port = authority
            .port_u16()
            .unwrap_or_else(|| if scheme.as_str() == "https" { 443 } else { 80 });
        let host = authority.host();

        if let Ok(ip) = host.parse::<IpAddr>() {
            let mut out = self.empty();
            out.push(SocketAddr::new(ip, port));
            return Ok(out);
        }

        let ips = resolve_ipv4_cached(host).ok_or(ureq::Error::HostNotFound)?;
        let mut out = self.empty();
        for ip in ips.into_iter().take(16) {
            out.push(SocketAddr::new(IpAddr::V4(ip), port));
        }
        if out.is_empty() {
            Err(ureq::Error::HostNotFound)
        } else {
            Ok(out)
        }
    }
}

fn resolve_ipv4_cached(host: &str) -> Option<Vec<Ipv4Addr>> {
    let cache = DNS_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(g) = cache.lock() {
        if let Some(v) = g.get(host) {
            if !v.is_empty() {
                return Some(v.clone());
            }
        }
    }
    let ips = query_udp_dns_a(host)?;
    if !ips.is_empty() {
        if let Ok(mut g) = cache.lock() {
            g.insert(host.to_owned(), ips.clone());
        }
        Some(ips)
    } else {
        None
    }
}

fn query_udp_dns_a(host: &str) -> Option<Vec<Ipv4Addr>> {
    let mut pkt = Vec::with_capacity(64);
    // Transaction ID + standard recursive query (RD=1), QDCOUNT=1
    pkt.extend_from_slice(&[0x4A, 0x7C, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    for label in host.trim_end_matches('.').split('.') {
        if label.is_empty() || label.len() > 63 {
            return None;
        }
        pkt.push(label.len() as u8);
        pkt.extend_from_slice(label.as_bytes());
    }
    pkt.push(0);
    // QTYPE = A (1), QCLASS = IN (1)
    pkt.extend_from_slice(&[0x00, 0x01, 0x00, 0x01]);

    let servers = ["1.1.1.1:53", "8.8.8.8:53", "192.168.240.1:53"];
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    let _ = sock.set_read_timeout(Some(Duration::from_millis(1500)));
    let _ = sock.set_write_timeout(Some(Duration::from_millis(1500)));

    for srv in servers {
        if sock.send_to(&pkt, srv).is_err() {
            continue;
        }
        let mut buf = [0u8; 512];
        if let Ok((n, _)) = sock.recv_from(&mut buf) {
            if n >= 12 {
                let ips = parse_dns_a_records(&buf[..n]);
                if !ips.is_empty() {
                    return Some(ips);
                }
            }
        }
    }
    None
}

fn skip_dns_name(buf: &[u8], mut off: usize) -> Option<usize> {
    for _ in 0..64 {
        let b = *buf.get(off)?;
        if b == 0 {
            return Some(off + 1);
        }
        if (b & 0xC0) == 0xC0 {
            return Some(off + 2);
        }
        off += 1 + (b as usize);
    }
    None
}

fn parse_dns_a_records(buf: &[u8]) -> Vec<Ipv4Addr> {
    let mut out = Vec::new();
    if buf.len() < 12 {
        return out;
    }
    let qd = u16::from_be_bytes([buf[4], buf[5]]) as usize;
    let an = u16::from_be_bytes([buf[6], buf[7]]) as usize;
    let mut off = 12;
    for _ in 0..qd {
        let Some(next) = skip_dns_name(buf, off) else { return out };
        off = next + 4; // QTYPE + QCLASS
    }
    for _ in 0..an {
        let Some(next) = skip_dns_name(buf, off) else { break };
        off = next;
        if off + 10 > buf.len() {
            break;
        }
        let rtype = u16::from_be_bytes([buf[off], buf[off + 1]]);
        let rclass = u16::from_be_bytes([buf[off + 2], buf[off + 3]]);
        let rdlen = u16::from_be_bytes([buf[off + 8], buf[off + 9]]) as usize;
        off += 10;
        if off + rdlen > buf.len() {
            break;
        }
        if rtype == 1 && rclass == 1 && rdlen == 4 {
            out.push(Ipv4Addr::new(buf[off], buf[off + 1], buf[off + 2], buf[off + 3]));
        }
        off += rdlen;
    }
    out
}
