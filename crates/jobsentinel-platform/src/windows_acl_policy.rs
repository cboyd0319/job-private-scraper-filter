// Defines the closed Windows trustee, access, and inheritance policy for private app storage.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PrivateObjectKind {
    Directory,
    File,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrusteeRole {
    CurrentUser,
    LocalSystem,
    BuiltinAdministrators,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AceSpec {
    pub(crate) trustee: TrusteeRole,
    pub(crate) access_mask: u32,
    pub(crate) inheritance: u32,
}

const FILE_ALL_ACCESS_MASK: u32 = 0x001f_01ff;
const CHILDREN_INHERIT: u32 = 0x3;

pub(crate) const fn private_acl_policy(kind: PrivateObjectKind) -> [AceSpec; 3] {
    let inheritance = match kind {
        PrivateObjectKind::Directory => CHILDREN_INHERIT,
        PrivateObjectKind::File => 0,
    };
    [
        AceSpec {
            trustee: TrusteeRole::CurrentUser,
            access_mask: FILE_ALL_ACCESS_MASK,
            inheritance,
        },
        AceSpec {
            trustee: TrusteeRole::LocalSystem,
            access_mask: FILE_ALL_ACCESS_MASK,
            inheritance,
        },
        AceSpec {
            trustee: TrusteeRole::BuiltinAdministrators,
            access_mask: FILE_ALL_ACCESS_MASK,
            inheritance,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_directory_acl_is_closed_and_inheritable() {
        let policy = private_acl_policy(PrivateObjectKind::Directory);
        assert_eq!(
            policy.map(|ace| ace.trustee),
            [
                TrusteeRole::CurrentUser,
                TrusteeRole::LocalSystem,
                TrusteeRole::BuiltinAdministrators,
            ]
        );
        assert!(policy
            .iter()
            .all(|ace| ace.access_mask == FILE_ALL_ACCESS_MASK));
        assert!(policy.iter().all(|ace| ace.inheritance == CHILDREN_INHERIT));
    }

    #[test]
    fn private_file_acl_is_closed_and_object_only() {
        let policy = private_acl_policy(PrivateObjectKind::File);
        assert_eq!(
            policy.map(|ace| ace.trustee),
            [
                TrusteeRole::CurrentUser,
                TrusteeRole::LocalSystem,
                TrusteeRole::BuiltinAdministrators,
            ]
        );
        assert!(policy
            .iter()
            .all(|ace| ace.access_mask == FILE_ALL_ACCESS_MASK));
        assert!(policy.iter().all(|ace| ace.inheritance == 0));
    }
}
