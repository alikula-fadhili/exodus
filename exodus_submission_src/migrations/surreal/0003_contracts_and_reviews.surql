-- Migration 0003: Behavioral Contracts and Verification Runs Schema
DEFINE TABLE OVERWRITE behavioral_contract SCHEMAFULL;
DEFINE FIELD OVERWRITE contract_id ON TABLE behavioral_contract TYPE string;
DEFINE FIELD OVERWRITE unit_id ON TABLE behavioral_contract TYPE string;
DEFINE FIELD OVERWRITE verification_status ON TABLE behavioral_contract TYPE string;
DEFINE FIELD OVERWRITE is_grounded ON TABLE behavioral_contract TYPE bool;
DEFINE FIELD OVERWRITE raw_contract_json ON TABLE behavioral_contract TYPE string;

DEFINE TABLE OVERWRITE verification_run SCHEMAFULL;
DEFINE FIELD OVERWRITE run_id ON TABLE verification_run TYPE string;
DEFINE FIELD OVERWRITE unit_id ON TABLE verification_run TYPE string;
DEFINE FIELD OVERWRITE outcome ON TABLE verification_run TYPE string;
DEFINE FIELD OVERWRITE compiler_ok ON TABLE verification_run TYPE bool;
DEFINE FIELD OVERWRITE tests_ok ON TABLE verification_run TYPE bool;
DEFINE FIELD OVERWRITE timestamp ON TABLE verification_run TYPE datetime;
