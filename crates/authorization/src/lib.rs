use legalrust_domain::{Permission, Role};
pub fn permissions(role: Role) -> &'static [Permission] {
    use Permission::*;
    match role {
        Role::SuperAdmin | Role::OrgOwner => &[
            View, Create, Update, Delete, Approve, Reject, Export, Download, Share, Sign, Verify,
            Assign,
        ],
        Role::OfficeAdmin => &[
            View, Create, Update, Approve, Export, Download, Share, Assign,
        ],
        Role::Notary | Role::Ppat => &[
            View, Create, Update, Approve, Sign, Verify, Export, Download, Share,
        ],
        Role::Lawyer => &[View, Create, Update, Export, Download, Share],
        Role::LegalStaff | Role::DocumentOfficer => {
            &[View, Create, Update, Download, Share, Verify]
        }
        Role::FinanceOfficer => &[View, Create, Update, Approve, Export, Download],
        Role::Auditor | Role::Reviewer => &[View, Export, Download],
        Role::Client => &[View, Create, Download, Share],
    }
}
pub fn can(role: Role, perm: Permission) -> bool {
    permissions(role).contains(&perm)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn least_privilege_holds() {
        assert!(!can(Role::Client, Permission::Approve));
        assert!(!can(Role::Auditor, Permission::Delete));
        assert!(can(Role::Notary, Permission::Sign));
        assert!(!can(Role::Notary, Permission::Delete));
        assert!(can(Role::SuperAdmin, Permission::Assign));
    }
}
