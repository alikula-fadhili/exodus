//! Manual end-to-end runner for the gated migration pipeline against a single fixture directory.
//! `cargo run -p exodus-verifier --example run_gate -- <fixture_dir> <repo_root>`
use exodus_verifier::run_gated_migration;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let fixture = PathBuf::from(
        args.next()
            .expect("usage: run_gate <fixture_dir> <repo_root>"),
    );
    let repo_root = PathBuf::from(args.next().expect("repo_root required"));
    let task_id = format!("demo-{}", uuid::Uuid::new_v4());
    let run_id = format!("run-{}", uuid::Uuid::new_v4());

    let summary = run_gated_migration(&fixture, &repo_root, Some(&fixture), &task_id, &run_id)
        .await
        .expect("gate run failed");

    println!("{}", serde_json::to_string_pretty(&summary).unwrap());
}
