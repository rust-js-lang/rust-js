// `core::net`'s addresses, std's own bodies compiled into `@rust-js/std`
// (#1420): their constructors, accessors and tests, `From`, `==`, `Hash`
// and copies, as plain data.
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

fn kind(ip: IpAddr) -> &'static str {
    match ip {
        IpAddr::V4(_) => "v4",
        IpAddr::V6(_) => "v6",
    }
}

fn main() {
    let a = Ipv4Addr::new(127, 0, 0, 1);
    println!(
        "{:?} {} {} {}",
        a.octets(),
        a.is_loopback(),
        a.is_private(),
        a.to_bits()
    );
    let b = Ipv4Addr::from(0xc0a80001u32);
    println!("{:?} {} {}", b.octets(), b.is_private(), a == b);
    let six = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
    println!("{:?} {} {}", six.segments(), six.is_loopback(), six.to_bits());
    let ip: IpAddr = a.into();
    println!("{} {} {}", ip.is_ipv4(), ip.is_loopback(), ip == IpAddr::V4(a));
    let ip6 = IpAddr::from([0u16, 0, 0, 0, 0, 0xffff, 0xc0a8, 1]);
    println!("{} {}", ip6.is_ipv6(), ip6.to_canonical() == IpAddr::V4(b));
    let mut s = SocketAddr::new(ip, 8080);
    println!("{} {}", s.port(), s.ip() == ip);
    s.set_port(9);
    println!("{} {}", s.port(), s.is_ipv4());
    let v4 = SocketAddrV4::new(b, 443);
    println!("{} {}", v4.port(), v4.ip().octets()[3]);
    let v6 = SocketAddrV6::new(six, 53, 7, 9);
    println!(
        "{} {} {} {}",
        v6.port(),
        v6.flowinfo(),
        v6.scope_id(),
        v6.ip().segments()[0]
    );
    let mut seen = HashSet::new();
    println!(
        "{} {} {}",
        seen.insert(a),
        seen.insert(b),
        seen.insert(Ipv4Addr::new(127, 0, 0, 1))
    );
    let copy = v6;
    println!("{}", copy == v6);
    if let SocketAddr::V4(v4) = SocketAddr::new(IpAddr::V4(b), 80) {
        println!("{} {} {}", v4.port(), kind(ip), kind(ip6));
    }
}
