-- Migration 0002: Case Engine and Governed Knowledge Schema
DEFINE TABLE OVERWRITE migration_case SCHEMAFULL;
DEFINE FIELD OVERWRITE case_id ON TABLE migration_case TYPE string;
DEFINE FIELD OVERWRITE run_id ON TABLE migration_case TYPE string;
DEFINE FIELD OVERWRITE structural_fingerprint ON TABLE migration_case TYPE string;
DEFINE FIELD OVERWRITE status ON TABLE migration_case TYPE string;
DEFINE FIELD OVERWRITE failure_category ON TABLE migration_case TYPE string;
DEFINE FIELD OVERWRITE source_language ON TABLE migration_case TYPE string;
DEFINE FIELD OVERWRITE target_language ON TABLE migration_case TYPE string;
DEFINE FIELD OVERWRITE failure_description ON TABLE migration_case TYPE string;
DEFINE FIELD OVERWRITE applications_count ON TABLE migration_case TYPE int;
DEFINE FIELD OVERWRITE verified_success_count ON TABLE migration_case TYPE int;
DEFINE FIELD OVERWRITE raw_case_json ON TABLE migration_case TYPE string;
DEFINE FIELD OVERWRITE created_at ON TABLE migration_case TYPE datetime;

DEFINE INDEX OVERWRITE case_fp_idx ON TABLE migration_case COLUMNS structural_fingerprint;
