use nest_rs::authn::PrincipalIdentity;
use nest_rs::core::AmbientValue;

const DESK_OPERATOR: &str = "desk-operator";

#[derive(Clone, Debug)]
pub struct DeskOperator {
    authenticated: bool,
}

impl DeskOperator {
    pub(super) const fn authenticated() -> Self {
        Self {
            authenticated: true,
        }
    }

    pub(super) const fn anonymous() -> Self {
        Self {
            authenticated: false,
        }
    }
}

/// The guard attaches the operator to the unit it admits, so whatever runs
/// inside reads who is at the desk without being handed it.
impl AmbientValue for DeskOperator {}

impl PrincipalIdentity for DeskOperator {
    fn actor_id(&self) -> Option<String> {
        self.authenticated.then(|| DESK_OPERATOR.to_owned())
    }
}
