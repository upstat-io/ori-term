use crate::id::DomainId;

use super::{Domain, DomainState, SpawnConfig};

/// Mock domain for testing the trait contract.
struct MockDomain {
    id: DomainId,
    name: String,
    state: DomainState,
}

impl Domain for MockDomain {
    fn id(&self) -> DomainId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn state(&self) -> DomainState {
        self.state
    }

    fn can_spawn(&self) -> bool {
        self.state == DomainState::Attached
    }
}

#[test]
fn mock_domain_attached_can_spawn() {
    let d = MockDomain {
        id: DomainId::from_raw(1),
        name: "test".to_string(),
        state: DomainState::Attached,
    };
    assert_eq!(d.id(), DomainId::from_raw(1));
    assert_eq!(d.name(), "test");
    assert_eq!(d.state(), DomainState::Attached);
    assert!(d.can_spawn());
}

#[test]
fn mock_domain_detached_cannot_spawn() {
    let d = MockDomain {
        id: DomainId::from_raw(2),
        name: "remote".to_string(),
        state: DomainState::Detached,
    };
    assert_eq!(d.state(), DomainState::Detached);
    assert!(!d.can_spawn());
}

#[test]
fn spawn_config_defaults() {
    let cfg = SpawnConfig::default();
    assert_eq!(cfg.cols, 80);
    assert_eq!(cfg.rows, 24);
    assert!(cfg.shell.is_none());
    assert!(cfg.cwd.is_none());
    assert!(cfg.env.is_empty());
    assert_eq!(cfg.scrollback, 10_000);
}

#[test]
fn spawn_config_custom_values() {
    let cfg = SpawnConfig {
        cols: 120,
        rows: 40,
        shell: Some("/bin/zsh".to_string()),
        cwd: Some("/tmp".into()),
        env: vec![("FOO".to_string(), "bar".to_string())],
        scrollback: 50_000,
    };
    assert_eq!(cfg.cols, 120);
    assert_eq!(cfg.rows, 40);
    assert_eq!(cfg.shell.as_deref(), Some("/bin/zsh"));
    assert_eq!(cfg.cwd.as_deref().unwrap().to_str(), Some("/tmp"));
    assert_eq!(cfg.env.len(), 1);
    assert_eq!(cfg.scrollback, 50_000);
}

/// Verify the trait is object-safe (can be used as `dyn Domain`).
#[test]
fn domain_trait_is_object_safe() {
    let d: Box<dyn Domain> = Box::new(MockDomain {
        id: DomainId::from_raw(1),
        name: "obj".to_string(),
        state: DomainState::Attached,
    });
    assert_eq!(d.name(), "obj");
    assert!(d.can_spawn());
}
