//! What the composed app promises its clients: one endpoint, the whole tool
//! surface, the app's own identity. It boots the real graph but touches no
//! desktop, so it runs where `nestrs run test unit` does; the assertions move
//! to `e2e/` the day the supervisord stack lands in CI.

mod module;
