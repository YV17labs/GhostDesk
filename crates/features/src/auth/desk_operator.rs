use nest_rs::authn::PrincipalIdentity;

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

impl PrincipalIdentity for DeskOperator {
    fn actor_id(&self) -> Option<String> {
        self.authenticated.then(|| DESK_OPERATOR.to_owned())
    }
}
