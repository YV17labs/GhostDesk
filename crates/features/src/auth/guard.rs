use super::strategy::TokenStrategy;

pub type AuthnGuard = nest_rs::authn::AuthnGuard<TokenStrategy>;
