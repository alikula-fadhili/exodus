def process_dynamic_payload(payload: dict) -> int:
    """Processes dynamic heterogeneous dictionary payload."""
    if "status" in payload:
        return 200
    return 400
