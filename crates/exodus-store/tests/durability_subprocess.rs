use exodus_core::{
    DeprecationEvidenceSource, DeprecationRecord, DeprecationStatus, EsgNode, EsgNodeKind,
    LanguageId,
};
use exodus_store::{GraphStore, KnowledgeStore, SurrealGraphStore, TargetLanguageCatalog};
use std::env;
use std::path::PathBuf;
use std::process::Command;

#[tokio::test]
async fn test_dual_subprocess_restart_durability() {
    let args: Vec<String> = env::args().collect();

    // Check if running in child mode
    if let Some(mode) = args.iter().find(|a| a.starts_with("--mode=")) {
        let mode_str = mode.trim_start_matches("--mode=");
        let db_path = args
            .iter()
            .find(|a| a.starts_with("--db-path="))
            .expect("Missing --db-path")
            .trim_start_matches("--db-path=");

        match mode_str {
            "writer" => {
                let mut store = SurrealGraphStore::open(PathBuf::from(db_path))
                    .await
                    .unwrap();
                assert!(store.is_durable());

                let node = EsgNode::new(
                    "urn:sym:pkg::subprocess_node",
                    LanguageId::new("python"),
                    EsgNodeKind::Function,
                    "subprocess_fn",
                );
                store.upsert_esg_node(&node).await.unwrap();

                let dep = DeprecationRecord::new(
                    "dep-subprocess-999",
                    LanguageId::new("python"),
                    "subprocess.call",
                    DeprecationStatus::Deprecated,
                    DeprecationEvidenceSource::CompilerWarning,
                );
                store.save_deprecation(&dep).await.unwrap();

                let mut spec = exodus_core::TargetLanguageSpecRecord::default_specs()
                    .into_iter()
                    .find(|spec| spec.id == "rust")
                    .unwrap();
                spec.id = "swift".to_string();
                spec.name = "Swift".to_string();
                spec.aliases = vec!["swiftlang".to_string()];
                spec.file_extension = "swift".to_string();
                spec.toolchain_profile =
                    exodus_core::ToolchainProfile::new("swift", ["swift", "build"]);
                store.upsert_language_spec(&spec).await.unwrap();
                std::process::exit(0);
            }
            "reader" => {
                let store = SurrealGraphStore::open(PathBuf::from(db_path))
                    .await
                    .unwrap();
                let node = store
                    .get_esg_node("urn:sym:pkg::subprocess_node")
                    .await
                    .unwrap()
                    .expect("Node missing in reader subprocess");
                assert_eq!(node.name, "subprocess_fn");

                let dep = store
                    .get_deprecation("dep-subprocess-999")
                    .await
                    .unwrap()
                    .expect("Deprecation missing in reader subprocess");
                assert_eq!(dep.target_symbol, "subprocess.call");

                let spec = store
                    .get_language_spec("swiftlang")
                    .await
                    .unwrap()
                    .expect("language specification missing in reader subprocess");
                assert_eq!(spec.id, "swift");
                assert_eq!(spec.toolchain_profile.check_command, ["swift", "build"]);
                std::process::exit(0);
            }
            _ => panic!("Unknown mode: {}", mode_str),
        }
    }

    // Parent orchestrator
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("surreal_db");
    let current_exe = env::current_exe().unwrap();

    // 1. Spawn writer subprocess
    let writer_status = Command::new(&current_exe)
        .arg("test_dual_subprocess_restart_durability")
        .arg("--")
        .arg("--exact")
        .arg("--nocapture")
        .arg("--mode=writer")
        .arg(format!("--db-path={}", db_path.display()))
        .status()
        .expect("Failed to execute writer subprocess");
    assert!(writer_status.success(), "Writer child process failed");

    // 2. Spawn reader subprocess
    let reader_status = Command::new(&current_exe)
        .arg("test_dual_subprocess_restart_durability")
        .arg("--")
        .arg("--exact")
        .arg("--nocapture")
        .arg("--mode=reader")
        .arg(format!("--db-path={}", db_path.display()))
        .status()
        .expect("Failed to execute reader subprocess");
    assert!(reader_status.success(), "Reader child process failed");
}
