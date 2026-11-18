use nest_rs::authn::PrincipalIdentity;

const DESK_OPERATOR: &str = "desk-operator";

#[derive(Clone, Debug, PrincipalIdentity)]
pub struct DeskOperator {
    #[principal(actor)]
    actor: Option<&'static str>,
}

impl DeskOperator {
    pub(super) const fn authenticated() -> Self {
        Self {
            actor: Some(DESK_OPERATOR),
        }
    }

    pub(super) const fn anonymous() -> Self {
        Self { actor: None }
    }
}
