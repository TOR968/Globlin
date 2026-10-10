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
