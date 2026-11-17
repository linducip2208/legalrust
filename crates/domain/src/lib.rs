use serde::{Deserialize, Serialize};
pub type TenantId = uuid::Uuid;
pub type UserId = uuid::Uuid;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    SuperAdmin,
    OrgOwner,
    OfficeAdmin,
    Notary,
    Ppat,
    Lawyer,
    LegalStaff,
    DocumentOfficer,
    FinanceOfficer,
    Auditor,
    Client,
    Reviewer,
}
impl Role {
    pub fn all() -> [Role; 12] {
        use Role::*;
        [
            SuperAdmin,
            OrgOwner,
            OfficeAdmin,
            Notary,
            Ppat,
            Lawyer,
            LegalStaff,
            DocumentOfficer,
            FinanceOfficer,
            Auditor,
            Client,
            Reviewer,
        ]
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Role::SuperAdmin => "super_admin",
            Role::OrgOwner => "org_owner",
            Role::OfficeAdmin => "office_admin",
            Role::Notary => "notary",
            Role::Ppat => "ppat",
            Role::Lawyer => "lawyer",
            Role::LegalStaff => "legal_staff",
            Role::DocumentOfficer => "document_officer",
            Role::FinanceOfficer => "finance_officer",
            Role::Auditor => "auditor",
            Role::Client => "client",
            Role::Reviewer => "reviewer",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    View,
    Create,
    Update,
    Delete,
    Approve,
    Reject,
    Export,
    Download,
    Share,
    Sign,
    Verify,
    Assign,
}
impl Permission {
    pub fn all() -> [Permission; 12] {
        use Permission::*;
        [
            View, Create, Update, Delete, Approve, Reject, Export, Download, Share, Sign, Verify,
            Assign,
        ]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn twelve_roles_and_permissions() {
        assert_eq!(Role::all().len(), 12);
        assert_eq!(Permission::all().len(), 12);
    }
}
