use std::net::IpAddr;

/// What this deployment is actually serving, as one word.
///
/// Five states rather than a boolean because the operator's next move differs
/// for each, and one of them is refused at boot. Named here rather than in the
/// service so the value the trail carries and the value the check produces are
/// the same vocabulary read from one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Posture {
    Secured,
    UnauthenticatedTls,
    CleartextToken,
    LoopbackOpen,
    ExposedOpen,
}

impl Posture {
    /// How this posture is read back from a log line.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Secured => "secured",
            Self::UnauthenticatedTls => "unauthenticated_tls",
            Self::CleartextToken => "cleartext_token",
            Self::LoopbackOpen => "loopback_open",
            Self::ExposedOpen => "exposed_open",
        }
    }

    /// Which posture a bind address, a certificate and a token add up to.
    pub fn of(tls: bool, token: bool, host: &str) -> Self {
        match (tls, token) {
            (true, true) => Self::Secured,
            (true, false) => Self::UnauthenticatedTls,
            (false, true) => Self::CleartextToken,
            (false, false) if is_loopback(host) => Self::LoopbackOpen,
            (false, false) => Self::ExposedOpen,
        }
    }
}

/// A name this process cannot resolve is routable until proven otherwise: the
/// question here is whether anything but this machine can reach the port, and
/// guessing "no" is the guess that publishes a desktop.
fn is_loopback(host: &str) -> bool {
    host.parse::<IpAddr>()
        .map(|address| address.is_loopback())
        .unwrap_or_else(|_| host.eq_ignore_ascii_case("localhost"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_without_a_token_is_the_one_posture_that_is_refused() {
        assert_eq!(
            Posture::of(true, false, "127.0.0.1"),
            Posture::UnauthenticatedTls,
        );
    }

    #[test]
    fn a_routable_bind_without_a_token_is_served_and_named() {
        assert_eq!(Posture::of(false, false, "0.0.0.0"), Posture::ExposedOpen);
    }

    #[test]
    fn the_served_postures() {
        assert_eq!(Posture::of(true, true, "0.0.0.0"), Posture::Secured);
        assert_eq!(Posture::of(false, true, "0.0.0.0"), Posture::CleartextToken);
        assert_eq!(
            Posture::of(false, false, "127.0.0.1"),
            Posture::LoopbackOpen,
        );
    }

    #[test]
    fn a_token_makes_a_routable_bind_servable() {
        assert_ne!(Posture::of(false, true, "0.0.0.0"), Posture::ExposedOpen);
    }

    #[test]
    fn loopback_is_recognised_by_address_and_by_name_and_nothing_else_is() {
        assert!(is_loopback("127.0.0.1"));
        assert!(is_loopback("127.0.0.7"));
        assert!(is_loopback("::1"));
        assert!(is_loopback("localhost"));
        assert!(is_loopback("LOCALHOST"));
        assert!(!is_loopback("0.0.0.0"));
        assert!(!is_loopback("192.168.1.10"));
        assert!(!is_loopback("::"));
        assert!(
            !is_loopback("desktop.internal"),
            "a name this process cannot resolve is routable until proven otherwise",
        );
    }
}
