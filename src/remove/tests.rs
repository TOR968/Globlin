use super::*;
use crate::config::Sources;
use crate::model::SourceKind;

fn target(name: &str) -> RemoveTarget {
    RemoveTarget {
        name: name.to_string(),
        source: SourceKind::Npm,
    }
}

#[test]
fn removing_without_any_enabled_source_fails_instead_of_reporting_success() {
    let config = Config {
        sources: Sources {
            npm: false,
            bun: false,
            pnpm: false,
            yarn: false,
            pipx: false,
            uv: false,
            scoop: false,
            cargo: false,
            go: false,
            dotnet: false,
            psgallery: false,
            gem: false,
            brew: false,
            winget: false,
            choco: false,
        },
        ..Default::default()
    };

    assert!(!run(&config, &target("prettier")));
}

#[test]
fn a_source_that_cannot_be_started_fails_instead_of_reporting_success() {
    let dir = std::env::temp_dir().join("globlin-test-remove-fake-npm");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("npm");
    std::fs::write(&path, b"not an executable").unwrap();
    let config = Config {
        npm_cmd: Some(path.clone()),
        sources: Sources {
            npm: true,
            bun: false,
            pnpm: false,
            yarn: false,
            pipx: false,
            uv: false,
            scoop: false,
            cargo: false,
            go: false,
            dotnet: false,
            psgallery: false,
            gem: false,
            brew: false,
            winget: false,
            choco: false,
        },
        ..Default::default()
    };

    let removed = run(&config, &target("prettier"));
    std::fs::remove_file(&path).ok();

    assert!(!removed);
}

struct DirectRemover {
    outcome: fn() -> crate::Result<()>,
}

impl PackageSource for DirectRemover {
    fn kind(&self) -> SourceKind {
        SourceKind::Go
    }

    fn installed(&self) -> crate::Result<Vec<crate::model::Installed>> {
        Ok(Vec::new())
    }

    fn update_command(&self, _name: &str) -> Option<std::process::Command> {
        None
    }

    fn uninstall_command(&self, _name: &str) -> Option<std::process::Command> {
        None
    }

    fn remove_directly(&self, _name: &str) -> Option<crate::Result<()>> {
        Some((self.outcome)())
    }
}

fn go_target() -> RemoveTarget {
    RemoveTarget {
        name: "golang.org/x/tools/gopls".to_string(),
        source: SourceKind::Go,
    }
}

#[test]
fn a_source_that_removes_the_package_itself_needs_no_uninstall_command() {
    let sources: Vec<Box<dyn PackageSource>> = vec![Box::new(DirectRemover { outcome: || Ok(()) })];

    assert!(remove_from(&sources, &go_target()).is_ok());
}

#[test]
fn a_direct_removal_that_fails_reports_the_source_and_the_reason() {
    let sources: Vec<Box<dyn PackageSource>> = vec![Box::new(DirectRemover {
        outcome: || Err("access denied".into()),
    })];

    let report = remove_from(&sources, &go_target()).unwrap_err();

    assert_eq!(
        report,
        "removing golang.org/x/tools/gopls via go failed: access denied
"
    );
}
