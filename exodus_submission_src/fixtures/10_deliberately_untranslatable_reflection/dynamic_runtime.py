def evaluate_runtime_code(expr_str: str) -> int:
    """Dynamic eval reflection that cannot be translated statically."""
    return eval(expr_str)
