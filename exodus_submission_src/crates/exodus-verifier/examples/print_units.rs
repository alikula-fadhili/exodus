//! Diagnostic: prints the verification boundaries (unit_id + kind) Exodus would compute for a
//! given source directory. Used to derive exact unit_id strings when authoring
//! `fixtures/<fixture>/contracts.json` files by hand (see `docs/reviews/unit-verification-audit.md`).
use exodus_graph::SemanticGraph;
use exodus_parser::{PythonParser, SourceParser};
use std::path::PathBuf;

fn main() {
    let path = std::env::args().nth(1).expect("usage: print_units <dir>");
    let parser = PythonParser::new();
    let parsed = parser
        .parse_repository(&PathBuf::from(&path))
        .expect("parse failed");
    let graph = SemanticGraph::from_parsed_repository(&parsed);
    for boundary in graph.verification_units() {
        println!("{boundary:?}");
        println!("  unit_id = {}", {
            let mut ids = boundary.node_ids();
            ids.sort();
            ids.join("+")
        });
    }
}
