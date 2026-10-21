use std::collections::HashMap;
use std::path::PathBuf;

use super::*;
use crate::source::NotInstalled;

const INSTALLED: &str = "cf295b0c31a1d0a1f9f171c978ae1b9546953c1e";
const UPSTREAM: &str = "9d1e4a2b7c3f5e6d8a9b0c1d2e3f4a5b6c7d8e9f";
const ROOT: &str = "0a0b0c0d0e0f1a1b1c1d1e1f2a2b2c2d2e2f3a3b";

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

fn tree(folder_sha: &str) -> String {
    format!(
        r#"{{"sha":"{ROOT}","tree":[
            {{"path":"skills","type":"tree","sha":"5555555555555555555555555555555555555555"}},
            {{"path":"skills/n8n-agents-official","type":"tree","sha":"{folder_sha}"}},
            {{"path":"skills/n8n-agents-official/SKILL.md","type":"blob","sha":"6666666666666666666666666666666666666666"}}
        ],"truncated":false}}"#
    )
}

fn entries() -> Vec<Entry> {
    parse_lock(LOCK).unwrap()
}

fn entry(name: &str) -> Entry {
    entries()
        .into_iter()
        .find(|entry| entry.name == name)
        .unwrap()
}

fn resolved(name: &str, trees: &HashMap<String, String>) -> Installed {
    resolve(&[entry(name)], trees).remove(0)
}

fn agents_tree(folder_sha: &str) -> HashMap<String, String> {
    HashMap::from([("n8n-io/skills".to_string(), tree(folder_sha))])
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
    let names: Vec<String> = entries().into_iter().map(|entry| entry.name).collect();
    assert_eq!(
        names,
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
fn skills_from_one_repository_share_one_tree_request() {
    assert_eq!(
        repo_key(&entry("n8n-agents-official")),
        Some("n8n-io/skills".to_string())
    );
    assert_eq!(
        repo_key(&entry("n8n-loops-official")),
        repo_key(&entry("n8n-agents-official"))
    );
    assert_eq!(repo_key(&entry("my-local")), None);
}

#[test]
fn a_pinned_ref_is_part_of_the_repository_key() {
    assert_eq!(
        repo_key(&entry("pinned")),
        Some("owner/repo#v2".to_string())
    );
}

#[test]
fn a_folder_hash_is_the_tree_entry_for_the_skill_folder() {
    assert_eq!(
        folder_hash(&tree(UPSTREAM), "skills/n8n-agents-official/SKILL.md"),
        Some(UPSTREAM.to_string())
    );
}

#[test]
fn a_root_level_skill_takes_the_hash_of_the_whole_tree() {
    assert_eq!(
        folder_hash(&tree(UPSTREAM), "SKILL.md"),
        Some(ROOT.to_string())
    );
}

#[test]
fn a_windows_style_skill_path_still_finds_its_folder() {
    assert_eq!(
        folder_hash(&tree(UPSTREAM), r"skills\n8n-agents-official\SKILL.md"),
        Some(UPSTREAM.to_string())
    );
}

#[test]
fn a_skill_whose_folder_changed_upstream_is_offered_the_new_hash() {
    let skill = resolved("n8n-agents-official", &agents_tree(UPSTREAM));
    assert_eq!(skill.version, "cf295b0");
    assert_eq!(skill.available.as_deref(), Some("9d1e4a2"));
    assert!(!skill.external);
    assert_eq!(skill.source, SourceKind::Skills);
}

#[test]
fn a_skill_whose_folder_is_unchanged_has_nothing_available() {
    let skill = resolved("n8n-agents-official", &agents_tree(INSTALLED));
    assert_eq!(skill.available, None);
    assert!(!skill.external);
}

#[test]
fn a_skill_whose_repository_did_not_answer_is_unknown_not_current() {
    let skill = resolved("n8n-agents-official", &HashMap::new());
    assert!(skill.external);
}

#[test]
fn a_skill_whose_folder_is_gone_upstream_is_unknown_not_outdated() {
    let skill = resolved("n8n-loops-official", &agents_tree(UPSTREAM));
    assert!(skill.external);
    assert_eq!(skill.available, None);
}

#[test]
fn a_skill_installed_from_a_local_path_is_listed_but_never_checked() {
    let skill = resolved("my-local", &agents_tree(UPSTREAM));
    assert!(skill.external);
}

#[test]
fn a_digits_only_hash_that_changed_is_still_offered() {
    let mut skill = entry("n8n-agents-official");
    skill.folder_hash = "1234567000000000000000000000000000000000".to_string();
    let upstream = "0234567000000000000000000000000000000000";
    let installed = resolve(&[skill], &agents_tree(upstream)).remove(0);
    assert_eq!(installed.available.as_deref(), Some("0234567"));
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
    }
}

#[test]
fn a_machine_without_the_skills_lock_file_has_no_skills_source() {
    let missing = std::env::temp_dir().join("globlin-no-such-skill-lock.json");
    let error = Skills::at(missing, None).err().unwrap();
    assert!(error.is::<NotInstalled>(), "{error}");
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
    };
    assert!(skills.update_command("n8n-agents-official").is_none());
    assert!(skills.uninstall_command("n8n-agents-official").is_none());
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
    let skills = Skills::new(None).unwrap();
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
