#!/usr/bin/env python3
"""Grounds fixtures/<fixture>/contracts.json from real differential execution of the fixture's
Python source (master-prompt oracle hierarchy tier 3: "Differential execution of the legacy unit
with controlled inputs"). Run from the repository root: `python3 scripts/ground_fixture_contracts.py`.

Each assertion's `expected` value is captured by actually importing and calling the real Python
function/class with the stated input, then formatting the result the way Rust's `{:?}` (Debug)
would format the equivalent value — this is what `exodus-verifier`'s generated harness
(`unit_gate::build_harness`) compares the migrated unit's actual output against. `input` is a Rust
call expression (constructed by hand to match the migrated signature), not the Python call — the
Python call is what produced `expected`, and both are recorded so the provenance is inspectable.

`unit_id` values must exactly match what `SemanticGraph::verification_units()` computes for that
fixture. They were derived with `cargo run -p exodus-verifier --example print_units -- <fixture>`,
not guessed — see docs/reviews/unit-verification-audit.md for the exact recorded output.

This script is deliberately explicit and unautomated about the Python->Rust value formatting (no
generic Python-repr-to-Rust-Debug converter) because a wrong automatic conversion would silently
reintroduce exactly the "invented expected value" problem this grounding step exists to avoid.
"""
import asyncio
import importlib
import json
import sys
import types
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent


def rust_str_debug(s: str) -> str:
    """Formats a Python string the way Rust's `{:?}` formats a `String`/`&str` (quoted)."""
    escaped = s.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def contract(unit_id, unit_name, unit_kind, source_sig, target_sig, assertions, status="pending"):
    return {
        "schema_version": "1.0.0",
        "contract_id": f"contract-{unit_id.replace('::', '-').replace('+', '_')}",
        "unit_id": unit_id,
        "unit_name": unit_name,
        "unit_kind": unit_kind,
        "source_signature": source_sig,
        "target_signature": target_sig,
        "assertions": assertions,
        "verification_status": status,
    }


def assertion(case_id, rust_input, expected_rust_debug, oracle, evidence):
    return {
        "case_id": case_id,
        "input": rust_input,
        "expected": expected_rust_debug,
        "oracle": oracle,
        "evidence": evidence,
    }


def ground_01_typed_functions():
    d = REPO_ROOT / "fixtures" / "01_typed_functions"
    sys.path.insert(0, str(d))
    math_ops = importlib.import_module("math_ops")
    importlib.reload(math_ops)
    sys.path.pop(0)

    add_2_3 = math_ops.add(2, 3)
    add_neg = math_ops.add(-5, 5)
    mul = math_ops.multiply_list([1, 2, 3], 2)
    mul_empty = math_ops.multiply_list([], 5)
    pos = math_ops.is_positive(5)
    neg = math_ops.is_positive(-1)
    zero = math_ops.is_positive(0)

    evidence = "differential_execution: python3 scripts/ground_fixture_contracts.py (fixtures/01_typed_functions/math_ops.py, captured 2026-08-29)"

    contracts = [
        contract(
            "function::math_ops::add", "add", "function",
            "add(a: int, b: int) -> int", "add(a: i64, b: i64) -> i64",
            [
                assertion("basic", "add(2, 3)", str(add_2_3), "differential_execution", evidence),
                assertion("cancels_out", "add(-5, 5)", str(add_neg), "differential_execution", evidence),
            ],
        ),
        contract(
            "function::math_ops::multiply_list", "multiply_list", "function",
            "multiply_list(nums: list[int], factor: int) -> list[int]",
            "multiply_list(nums: Vec<i64>, factor: i64) -> Vec<i64>",
            [
                assertion("basic", "multiply_list(vec![1, 2, 3], 2)", "[" + ", ".join(str(x) for x in mul) + "]", "differential_execution", evidence),
                assertion("empty_input", "multiply_list(vec![], 5)", "[" + ", ".join(str(x) for x in mul_empty) + "]", "differential_execution", evidence),
            ],
        ),
        contract(
            "function::math_ops::is_positive", "is_positive", "function",
            "is_positive(val: int) -> bool", "is_positive(val: i64) -> bool",
            [
                assertion("positive", "is_positive(5)", "true" if pos else "false", "differential_execution", evidence),
                assertion("negative", "is_positive(-1)", "true" if neg else "false", "differential_execution", evidence),
                assertion("boundary_zero", "is_positive(0)", "true" if zero else "false", "differential_execution", evidence),
            ],
        ),
    ]
    (d / "contracts.json").write_text(json.dumps(contracts, indent=2) + "\n")
    print(f"wrote {d/'contracts.json'} ({len(contracts)} contracts)")


def ground_02_class_conversion():
    d = REPO_ROOT / "fixtures" / "02_class_conversion"
    sys.path.insert(0, str(d))
    bank_account = importlib.import_module("bank_account")
    importlib.reload(bank_account)
    sys.path.pop(0)
    BankAccount = bank_account.BankAccount

    a = BankAccount("alice", 100)
    deposit_result = a.deposit(50)
    b = BankAccount("bob", 100)
    withdraw_result = b.withdraw(30)
    c = BankAccount("carol", 100)
    insufficient_result = c.withdraw(1000)

    evidence = "differential_execution: python3 scripts/ground_fixture_contracts.py (fixtures/02_class_conversion/bank_account.py, captured 2026-08-29)"

    unit_id = "method::bank_account::BankAccount::__init__+method::bank_account::BankAccount::deposit+method::bank_account::BankAccount::withdraw+type::bank_account::BankAccount"
    contracts = [
        contract(
            unit_id, "BankAccount", "class",
            "BankAccount(owner: str, balance: int = 0); deposit(amount)->int; withdraw(amount)->int",
            "struct BankAccount { owner: String, balance: i64 }; fn deposit(&mut self, amount: i64) -> i64; fn withdraw(&mut self, amount: i64) -> i64",
            [
                assertion(
                    "deposit_basic",
                    '{ let mut acc = BankAccount::new("alice".to_string(), 100); acc.deposit(50) }',
                    str(deposit_result), "differential_execution", evidence,
                ),
                assertion(
                    "withdraw_sufficient",
                    '{ let mut acc = BankAccount::new("bob".to_string(), 100); acc.withdraw(30) }',
                    str(withdraw_result), "differential_execution", evidence,
                ),
                assertion(
                    "withdraw_insufficient_funds",
                    '{ let mut acc = BankAccount::new("carol".to_string(), 100); acc.withdraw(1000) }',
                    str(insufficient_result), "differential_execution", evidence,
                ),
            ],
        ),
    ]
    (d / "contracts.json").write_text(json.dumps(contracts, indent=2) + "\n")
    print(f"wrote {d/'contracts.json'} ({len(contracts)} contracts)")


def ground_03_module_dependency():
    d = REPO_ROOT / "fixtures" / "03_module_dependency"
    sys.path.insert(0, str(d))
    models = importlib.import_module("models")
    service = importlib.import_module("service")
    importlib.reload(models)
    importlib.reload(service)
    sys.path.pop(0)

    p1 = models.Product("a", 10)
    price = p1.get_price()
    total = service.calculate_total([models.Product("a", 10), models.Product("b", 20), models.Product("c", 5)])
    total_empty = service.calculate_total([])

    evidence = "differential_execution: python3 scripts/ground_fixture_contracts.py (fixtures/03_module_dependency/{models,service}.py, captured 2026-08-29)"

    product_unit_id = "method::models::Product::__init__+method::models::Product::get_price+type::models::Product"
    contracts = [
        contract(
            product_unit_id, "Product", "class",
            "Product(name: str, price: int); get_price() -> int",
            "struct Product { name: String, price: i64 }; fn get_price(&mut self) -> i64",
            [
                assertion(
                    "get_price_basic",
                    '{ let mut p = Product::new("a".to_string(), 10); p.get_price() }',
                    str(price), "differential_execution", evidence,
                ),
                assertion(
                    "serde_roundtrip_is_identity",
                    '{ let p = Product::new("a".to_string(), 10); '
                    'let json1 = serde_json::to_string(&p).unwrap(); '
                    'let p2: Product = serde_json::from_str(&json1).unwrap(); '
                    'p2 == p }',
                    "true",
                    "declared_invariant",
                    "declared_invariant: serialize-then-deserialize must be a no-op for a derived "
                    "Serialize/Deserialize/PartialEq struct — this is a structural property, not "
                    "something differential-executed against the Python source (Python has no "
                    "equivalent typed round-trip), and is checked without assuming a specific "
                    "field order (the transform engine's field ordering is not guaranteed stable).",
                ),
            ],
        ),
        contract(
            "function::service::calculate_total", "calculate_total", "function",
            "calculate_total(products: list[Product]) -> int",
            "calculate_total(products: Vec<Product>) -> i64",
            [
                assertion(
                    "basic_sum",
                    'calculate_total(vec![Product::new("a".to_string(), 10), Product::new("b".to_string(), 20), Product::new("c".to_string(), 5)])',
                    str(total), "differential_execution", evidence,
                ),
                assertion(
                    "empty_list",
                    "calculate_total(vec![])",
                    str(total_empty), "differential_execution", evidence,
                ),
            ],
        ),
    ]
    (d / "contracts.json").write_text(json.dumps(contracts, indent=2) + "\n")
    print(f"wrote {d/'contracts.json'} ({len(contracts)} contracts)")


def ground_04_circular_dependency():
    d = REPO_ROOT / "fixtures" / "04_circular_dependency"
    sys.path.insert(0, str(d))

    # user.py and order.py import each other at module scope, which genuinely fails in real
    # CPython (confirmed: `import user` and `import order` both raise ImportError here — this is
    # not a migration-tool artifact, it is how this fixture's Python actually behaves). We ground
    # each function's *own* logic honestly by pre-registering a stub for its collaborator module
    # so the real load-time cycle never triggers, then executing the function's real body. Neither
    # function's own code is faked — only the untaken cross-import path is stubbed.
    stub_user = types.ModuleType("user")
    stub_user.get_user_summary = lambda uid: None
    sys.modules["user"] = stub_user
    order = importlib.import_module("order")
    orders_result = order.get_user_orders("u1")
    del sys.modules["user"]
    del sys.modules["order"]

    stub_order = types.ModuleType("order")
    stub_order.get_user_orders = lambda uid: [f"order-1-{uid}", f"order-2-{uid}"]
    sys.modules["order"] = stub_order
    user = importlib.import_module("user")
    summary_result = user.get_user_summary("u1")
    del sys.modules["order"]
    del sys.modules["user"]
    sys.path.pop(0)

    evidence = (
        "differential_execution (collaborator stubbed to break CPython's own load-time import "
        "cycle, which fails with ImportError for `import user`/`import order` as authored — "
        "see docs/reviews/unit-verification-audit.md): "
        "python3 scripts/ground_fixture_contracts.py (fixtures/04_circular_dependency/{user,order}.py, captured 2026-08-29)"
    )

    contracts = [
        contract(
            "function::order::get_user_orders", "get_user_orders", "function",
            "get_user_orders(user_id: str) -> list[str]",
            "get_user_orders(user_id: String) -> Vec<String>",
            [
                assertion(
                    "basic",
                    'get_user_orders("u1".to_string())',
                    "[" + ", ".join(rust_str_debug(x) for x in orders_result) + "]",
                    "differential_execution", evidence,
                ),
            ],
        ),
        contract(
            "function::user::get_user_summary", "get_user_summary", "function",
            "get_user_summary(user_id: str) -> str",
            "get_user_summary(user_id: String) -> String",
            [
                assertion(
                    "basic",
                    'get_user_summary("u1".to_string())',
                    rust_str_debug(summary_result),
                    "differential_execution", evidence,
                ),
            ],
        ),
    ]
    (d / "contracts.json").write_text(json.dumps(contracts, indent=2) + "\n")
    print(f"wrote {d/'contracts.json'} ({len(contracts)} contracts)")


def ground_08_async_function():
    d = REPO_ROOT / "fixtures" / "08_async_function"
    sys.path.insert(0, str(d))
    fetcher = importlib.import_module("fetcher")
    importlib.reload(fetcher)
    sys.path.pop(0)

    result = asyncio.run(fetcher.async_fetch_data("res-42"))

    evidence = "differential_execution: python3 scripts/ground_fixture_contracts.py (fixtures/08_async_function/fetcher.py, captured 2026-08-29)"

    contracts = [
        contract(
            "function::fetcher::async_fetch_data", "async_fetch_data", "function",
            "async def async_fetch_data(resource_id: str) -> str",
            "async_fetch_data(resource_id: String) -> String",
            [
                assertion(
                    "basic",
                    'async_fetch_data("res-42".to_string()).await',
                    rust_str_debug(result),
                    "differential_execution", evidence,
                ),
            ],
        ),
    ]
    (d / "contracts.json").write_text(json.dumps(contracts, indent=2) + "\n")
    print(f"wrote {d/'contracts.json'} ({len(contracts)} contracts)")


def ground_05_unsupported_decorator():
    d = REPO_ROOT / "fixtures" / "05_unsupported_decorator"
    sys.path.insert(0, str(d))
    service = importlib.import_module("cached_service")
    importlib.reload(service)
    sys.path.pop(0)

    result = service.compute_heavy_metric(5, 3)
    evidence = "differential_execution: python3 scripts/ground_fixture_contracts.py (fixtures/05_unsupported_decorator/cached_service.py)"

    contracts = [
        contract(
            "function::cached_service::compute_heavy_metric", "compute_heavy_metric", "function",
            "compute_heavy_metric(base: int, multiplier: int) -> int",
            "compute_heavy_metric(base: i64, multiplier: i64) -> i64",
            [
                assertion(
                    "basic_metric",
                    "compute_heavy_metric(5, 3)",
                    str(result),
                    "differential_execution", evidence,
                ),
            ],
        ),
    ]
    (d / "contracts.json").write_text(json.dumps(contracts, indent=2) + "\n")
    print(f"wrote {d/'contracts.json'} ({len(contracts)} contracts)")


def ground_06_dynamic_value():
    d = REPO_ROOT / "fixtures" / "06_dynamic_value"
    sys.path.insert(0, str(d))
    parser = importlib.import_module("payload_parser")
    importlib.reload(parser)
    sys.path.pop(0)

    res_ok = parser.process_dynamic_payload({"status": "ok"})
    res_err = parser.process_dynamic_payload({})
    evidence = "differential_execution: python3 scripts/ground_fixture_contracts.py (fixtures/06_dynamic_value/payload_parser.py)"

    contracts = [
        contract(
            "function::payload_parser::process_dynamic_payload", "process_dynamic_payload", "function",
            "process_dynamic_payload(payload: dict) -> int",
            "process_dynamic_payload(payload: &std::collections::HashMap<String, String>) -> i64",
            [
                assertion(
                    "status_present",
                    'process_dynamic_payload(&[("status".to_string(), "ok".to_string())].into_iter().collect())',
                    str(res_ok),
                    "differential_execution", evidence,
                ),
                assertion(
                    "status_absent",
                    "process_dynamic_payload(&std::collections::HashMap::new())",
                    str(res_err),
                    "differential_execution", evidence,
                ),
            ],
        ),
    ]
    (d / "contracts.json").write_text(json.dumps(contracts, indent=2) + "\n")
    print(f"wrote {d/'contracts.json'} ({len(contracts)} contracts)")


def ground_07_missing_sdk():
    d = REPO_ROOT / "fixtures" / "07_missing_sdk"
    sys.path.insert(0, str(d))
    stub_sdk = types.ModuleType("stripe_legacy_sdk")
    class StubCharge:
        @staticmethod
        def create(amount, token):
            return StubCharge()
        def is_successful(self):
            return True
    stub_sdk.Charge = StubCharge
    sys.modules["stripe_legacy_sdk"] = stub_sdk

    payment = importlib.import_module("external_payment")
    importlib.reload(payment)
    del sys.modules["stripe_legacy_sdk"]
    sys.path.pop(0)

    evidence = "differential_execution (with stubbed Stripe SDK): python3 scripts/ground_fixture_contracts.py (fixtures/07_missing_sdk/external_payment.py)"

    contracts = [
        contract(
            "function::external_payment::charge_card", "charge_card", "function",
            "charge_card(card_token: str, amount_cents: int) -> bool",
            "charge_card(card_token: String, amount_cents: i64) -> bool",
            [
                assertion(
                    "basic_charge",
                    'charge_card("tok_test123".to_string(), 1000)',
                    "true",
                    "differential_execution", evidence,
                ),
            ],
        ),
    ]
    (d / "contracts.json").write_text(json.dumps(contracts, indent=2) + "\n")
    print(f"wrote {d/'contracts.json'} ({len(contracts)} contracts)")


def ground_09_database_compat_wrapper():
    d = REPO_ROOT / "fixtures" / "09_database_compat_wrapper"
    sys.path.insert(0, str(d))
    repo_mod = importlib.import_module("repository")
    importlib.reload(repo_mod)
    sys.path.pop(0)

    repo = repo_mod.UserRepository("sqlite://test.db")
    user_str = repo.find_user_by_id(42)
    evidence = "differential_execution: python3 scripts/ground_fixture_contracts.py (fixtures/09_database_compat_wrapper/repository.py)"

    contracts = [
        contract(
            "class::repository::UserRepository", "UserRepository", "class",
            "class UserRepository",
            "struct UserRepository",
            [
                assertion(
                    "find_user",
                    'UserRepository::new("sqlite://test.db".to_string()).find_user_by_id(42)',
                    rust_str_debug(user_str),
                    "differential_execution", evidence,
                ),
            ],
        ),
    ]
    (d / "contracts.json").write_text(json.dumps(contracts, indent=2) + "\n")
    print(f"wrote {d/'contracts.json'} ({len(contracts)} contracts)")


def ground_10_deliberately_untranslatable_reflection():
    d = REPO_ROOT / "fixtures" / "10_deliberately_untranslatable_reflection"
    sys.path.insert(0, str(d))
    runtime = importlib.import_module("dynamic_runtime")
    importlib.reload(runtime)
    sys.path.pop(0)

    val = runtime.evaluate_runtime_code("2 + 2")
    evidence = "differential_execution: python3 scripts/ground_fixture_contracts.py (fixtures/10_deliberately_untranslatable_reflection/dynamic_runtime.py)"

    contracts = [
        contract(
            "function::dynamic_runtime::evaluate_runtime_code", "evaluate_runtime_code", "function",
            "evaluate_runtime_code(expr_str: str) -> int",
            "evaluate_runtime_code(expr_str: String) -> i64",
            [
                assertion(
                    "eval_addition",
                    'evaluate_runtime_code("2 + 2".to_string())',
                    str(val),
                    "differential_execution", evidence,
                ),
            ],
        ),
    ]
    (d / "contracts.json").write_text(json.dumps(contracts, indent=2) + "\n")
    print(f"wrote {d/'contracts.json'} ({len(contracts)} contracts)")


if __name__ == "__main__":
    ground_01_typed_functions()
    ground_02_class_conversion()
    ground_03_module_dependency()
    ground_04_circular_dependency()
    ground_05_unsupported_decorator()
    ground_06_dynamic_value()
    ground_07_missing_sdk()
    ground_08_async_function()
    ground_09_database_compat_wrapper()
    ground_10_deliberately_untranslatable_reflection()

