use super::*;

const JSON: &str = r#"{
  "version": 1,
  "data": [
    { "packageId": "dotnetsay", "version": "2.1.4", "commands": ["dotnetsay"] },
    { "packageId": "dotnet-ef", "version": "9.0.0", "commands": ["dotnet-ef"] }
  ]
}"#;

const PASCAL_JSON: &str = r#"{
  "Version": 1,
  "Data": [
    { "PackageId": "dotnetsay", "Version": "2.1.4", "Commands": ["dotnetsay"] }
  ]
}"#;

const TABLE: &str = "Package Id      Version      Commands
-------------------------------------
dotnetsay       2.1.4        dotnetsay
dotnet-ef       9.0.0        dotnet-ef
";

fn arguments(command: &std::process::Command) -> Vec<String> {
    command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn the_json_listing_is_read_from_the_data_array() {
    let installed = parse_json(JSON);
    let names: Vec<&str> = installed.iter().map(|item| item.name.as_str()).collect();

    assert_eq!(names, vec!["dotnetsay", "dotnet-ef"]);
    assert_eq!(installed[1].version, "9.0.0");
    assert_eq!(installed[0].source, SourceKind::Dotnet);
}

#[test]
fn either_casing_of_the_json_contract_is_accepted() {
    let installed = parse_json(PASCAL_JSON);

    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].name, "dotnetsay");
    assert_eq!(installed[0].version, "2.1.4");
}

#[test]
fn an_sdk_too_old_for_json_falls_back_to_its_table() {
    let installed = parse_table(TABLE);
    let names: Vec<&str> = installed.iter().map(|item| item.name.as_str()).collect();

    assert_eq!(names, vec!["dotnetsay", "dotnet-ef"]);
    assert_eq!(installed[0].version, "2.1.4");
}

#[test]
fn the_localised_header_above_the_dashes_is_never_a_tool() {
    assert!(!parse_table(TABLE)
        .iter()
        .any(|item| item.name.contains("Package")));
}

#[test]
fn output_without_a_table_is_empty_rather_than_fatal() {
    assert!(parse_table("No tools were found.\n").is_empty());
    assert!(parse_table("").is_empty());
    assert!(parse_json("error: unrecognized option --format").is_empty());
}

#[test]
fn the_dotnet_update_command_updates_the_global_tool() {
    let dotnet = Dotnet {
        command: PathBuf::from("dotnet"),
    };

    assert_eq!(
        arguments(&dotnet.update_command("dotnetsay").unwrap()),
        vec!["tool", "update", "--global", "dotnetsay"]
    );
}

#[test]
fn the_dotnet_uninstall_command_removes_the_global_tool() {
    let dotnet = Dotnet {
        command: PathBuf::from("dotnet"),
    };

    assert_eq!(
        arguments(&dotnet.uninstall_command("dotnetsay").unwrap()),
        vec!["tool", "uninstall", "--global", "dotnetsay"]
    );
}

#[test]
#[ignore = "runs the real dotnet: cargo test -- --ignored --exact source::dotnet::tests::the_real_dotnet_listing_still_parses"]
fn the_real_dotnet_listing_still_parses() {
    let installed = Dotnet::new().unwrap().installed().unwrap();

    assert!(installed
        .iter()
        .all(|item| !item.name.is_empty() && !item.version.is_empty()));
}
