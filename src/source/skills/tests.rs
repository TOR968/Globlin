use std::path::PathBuf;

use super::*;
use crate::source::NotInstalled;

const LOCK: &str = r#"{
  "version": 3,
  "skills": {
    "n8n-agents-official": {
      "source": "n8n-io/skills",
      "sourceType": "github",
      "sourceUrl": "https://github.com/n8n-io/skills.git",
      "skillPath": "skills/n8n-agents-official/SKILL.md",
      "skillFolderHash": "cf295b0c31a1d0a1f9f171c978ae1b9546953c1e",
      "installedAt": "2026-10-21T01:23:35.718Z",
      "updatedAt": "2026-10-21T01:23:35.718Z"
    },
    "n8n-loops-official": {
      "source": "n8n-io/skills",
      "sourceType": "github",
      "sourceUrl": "https://github.com/n8n-io/skills.git",
      "skillPath": "skills/n8n-loops-official/SKILL.md",
      "skillFolderHash": "1111111111111111111111111111111111111111"
    },
    "pinned": {
      "source": "owner/repo",
      "sourceType": "github",
      "ref": "v2",
      "skillPath": "SKILL.md",
      "skillFolderHash": "2222222222222222222222222222222222222222"
    },
    "my-local": {
      "source": "C:/skills/my-local",
      "sourceType": "local",
      "skillPath": "SKILL.md",
      "skillFolderHash": ""
    }
  },
  "dismissed": {},
  "lastSelectedAgents": ["codex"]
}"#;

const UP_TO_DATE: &str = "\u{1b}[38;5;145mUpdating my-local, n8n-agents-official, n8n-loops-official, pinned…\u{1b}[0m\n\n\
\r\u{1b}[38;5;102mChecking skills from source: n8n-io/skills\u{1b}[0m\u{1b}[K\n\
\r\u{1b}[38;5;102mChecking skills from source: owner/repo\u{1b}[0m\u{1b}[K\n\
\r\u{1b}[K\u{1b}[38;5;145m✓ All global skills are up to date\u{1b}[0m\n\n\
\u{1b}[38;5;102m1 skill(s) cannot be checked automatically:\u{1b}[0m\n\
  \u{1b}[38;5;145m•\u{1b}[0m my-local \u{1b}[38;5;102m(Local path)\u{1b}[0m\n";

fn entries() -> Vec<Entry> {
    parse_lock(LOCK).unwrap()
}

fn asked() -> Vec<String> {
    entries().into_iter().map(|entry| entry.name).collect()
}

fn resolved(output: &str, name: &str) -> Installed {
    let run = Run::read(output, asked());
    resolve(&entries(), Some(&run))
        .into_iter()
        .find(|skill| skill.name == name)
        .unwrap()
}

#[test]
fn the_lock_file_lives_under_xdg_state_home_when_it_is_set() {
    assert_eq!(
        lock_path(
            Some(PathBuf::from("/state")),
            Some(PathBuf::from("/home/u"))
        ),
        Some(
            PathBuf::from("/state")
                .join("skills")
                .join(".skill-lock.json")
        )
    );
}

#[test]
fn the_lock_file_lives_in_the_agents_folder_of_home_otherwise() {
    let home = PathBuf::from("/home/u");
    assert_eq!(
        lock_path(None, Some(home.clone())),
        Some(home.join(".agents").join(".skill-lock.json"))
    );
}

#[test]
fn every_lock_entry_becomes_one_skill_named_by_its_key() {
    assert_eq!(
        asked(),
        [
            "my-local",
            "n8n-agents-official",
            "n8n-loops-official",
            "pinned"
        ]
    );
}

#[test]
fn a_lock_older_than_version_three_lists_nothing() {
    assert!(parse_lock(r#"{"version":2,"skills":{"a":{}}}"#)
        .unwrap()
        .is_empty());
}

#[test]
fn a_lock_without_a_skills_map_lists_nothing() {
    assert!(parse_lock(r#"{"version":3}"#).unwrap().is_empty());
}

#[test]
fn a_lock_that_is_not_json_is_an_error() {
    assert!(parse_lock("{").is_err());
}

#[test]
fn a_skill_whose_source_the_cli_checked_is_current() {
    let skill = resolved(UP_TO_DATE, "n8n-agents-official");
    assert_eq!(skill.version, "cf295b0");
    assert_eq!(skill.available, None);
    assert!(!skill.external);
    assert_eq!(skill.source, SourceKind::Skills);
}

#[test]
fn a_skill_the_cli_could_not_check_is_unknown_not_current() {
    assert!(resolved(UP_TO_DATE, "my-local").external);
}

#[test]
fn a_skill_whose_tree_could_not_be_fetched_is_unknown_not_current() {
    let output =
        format!("{UP_TO_DATE}\n  \u{1b}[38;5;102m✗ Failed to fetch tree for owner/repo\u{1b}[0m\n");
    assert!(resolved(&output, "pinned").external);
    assert!(!resolved(&output, "n8n-agents-official").external);
}

#[test]
fn a_source_whose_check_threw_is_unknown_not_current() {
    let output =
        "Checking skills from source: owner/repo\n  ✗ Failed to check skills from owner/repo\n";
    assert!(resolved(output, "pinned").external);
}

#[test]
fn a_skill_deleted_upstream_is_unknown_not_current() {
    let output = format!(
        "{UP_TO_DATE}\nWarning: The following skills from n8n-io/skills appear to have been deleted upstream:\n  • n8n-loops-official\nSkipping deletion in non-interactive mode.\n"
    );
    assert!(resolved(&output, "n8n-loops-official").external);
    assert!(!resolved(&output, "n8n-agents-official").external);
}

#[test]
fn a_skill_the_cli_updated_during_the_check_is_current() {
    let output = "Updating pinned…\n\nChecking skills from source: owner/repo\nFound 1 global update(s)\n\nUpdating pinned…\n  ✓ Updated pinned\n\n✓ Updated 1 skill(s)\n";
    let skill = resolved(output, "pinned");
    assert_eq!(skill.available, None);
    assert!(!skill.external);
}

#[test]
fn a_skill_whose_update_failed_stays_outdated() {
    let output = "Checking skills from source: owner/repo\nFound 1 global update(s)\n\nUpdating pinned…\n  ✗ Failed to update pinned\n";
    assert_eq!(
        resolved(output, "pinned").available.as_deref(),
        Some("newer")
    );
}

#[test]
fn the_header_naming_one_skill_is_not_read_as_an_update() {
    let output = "Updating pinned…\n\nChecking skills from source: owner/repo\n✓ All global skills are up to date\n";
    assert_eq!(resolved(output, "pinned").available, None);
}

#[test]
fn a_skill_the_check_did_not_ask_about_is_unknown_even_when_its_source_was_checked() {
    let run = Run::read(UP_TO_DATE, vec!["n8n-agents-official".to_string()]);
    let skills = resolve(&entries(), Some(&run));
    let loops = skills
        .iter()
        .find(|skill| skill.name == "n8n-loops-official")
        .unwrap();
    assert!(loops.external);
}

#[test]
fn without_a_cli_run_every_skill_is_unknown() {
    assert!(resolve(&entries(), None).iter().all(|skill| skill.external));
}

#[test]
fn several_skills_on_one_bullet_line_are_all_unchecked() {
    let output = "Checking skills from source: n8n-io/skills\n  • n8n-agents-official, n8n-loops-official (Private or deleted repo)\n";
    assert!(resolved(output, "n8n-agents-official").external);
    assert!(resolved(output, "n8n-loops-official").external);
}

fn arguments(command: &std::process::Command) -> Vec<String> {
    command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

fn with_npx() -> Skills {
    Skills {
        lock: PathBuf::from("unused"),
        npx: Some(PathBuf::from("npx")),
        ignored: Vec::new(),
    }
}

#[test]
fn a_machine_without_the_skills_lock_file_has_no_skills_source() {
    let missing = std::env::temp_dir().join("globlin-no-such-skill-lock.json");
    let error = Skills::at(missing, None).err().unwrap();
    assert!(error.is::<NotInstalled>(), "{error}");
}

#[test]
fn the_check_updates_every_named_global_skill_through_npx() {
    let command = with_npx()
        .check_command(&["a".to_string(), "b".to_string()])
        .unwrap();
    assert_eq!(
        arguments(&command),
        ["--yes", "skills", "update", "a", "b", "-g", "-y"]
    );
}

#[test]
fn a_check_with_nothing_to_ask_about_runs_nothing() {
    assert!(with_npx().check_command(&[]).is_none());
}

#[test]
fn ignored_skills_and_names_that_could_inject_arguments_are_not_checked() {
    let skills = Skills {
        ignored: vec!["pinned".to_string()],
        ..with_npx()
    };
    let mut entries = entries();
    entries.push(Entry {
        name: "--all".to_string(),
        ..Entry::default()
    });
    assert_eq!(
        skills.names_to_check(&entries),
        ["my-local", "n8n-agents-official", "n8n-loops-official"]
    );
}

#[test]
fn an_update_runs_the_skills_cli_through_npx_for_one_global_skill() {
    let command = with_npx().update_command("n8n-agents-official").unwrap();
    assert_eq!(
        arguments(&command),
        [
            "--yes",
            "skills",
            "update",
            "n8n-agents-official",
            "-g",
            "-y"
        ]
    );
}

#[test]
fn an_uninstall_removes_one_global_skill_through_npx() {
    let command = with_npx().uninstall_command("n8n-agents-official").unwrap();
    assert_eq!(
        arguments(&command),
        [
            "--yes",
            "skills",
            "remove",
            "n8n-agents-official",
            "-g",
            "-y"
        ]
    );
}

#[test]
fn without_npx_a_skill_can_be_listed_but_offers_no_command() {
    let skills = Skills {
        lock: PathBuf::from("unused"),
        npx: None,
        ignored: Vec::new(),
    };
    assert!(skills.update_command("n8n-agents-official").is_none());
    assert!(skills.uninstall_command("n8n-agents-official").is_none());
    assert!(skills.check_command(&["a".to_string()]).is_none());
}

#[test]
fn a_name_that_could_inject_arguments_gets_no_command() {
    assert!(with_npx().update_command("--all").is_none());
    assert!(with_npx().uninstall_command("--all").is_none());
}

#[test]
#[ignore = "runs the real npx skills against the user's global skills: SKILL_TARGET=<installed skill> cargo test -- --ignored --exact source::skills::tests::the_real_skills_cli_updates_and_lists_without_a_prompt"]
fn the_real_skills_cli_updates_and_lists_without_a_prompt() {
    let target = std::env::var("SKILL_TARGET").unwrap();
    let skills = Skills::new(&crate::config::Config::default()).unwrap();
    let mut command = skills.update_command(&target).unwrap();

    let output =
        crate::process::output_within(&mut command, std::time::Duration::from_mins(3)).unwrap();

    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(skills
        .installed()
        .unwrap()
        .iter()
        .any(|skill| skill.name == target));
}
