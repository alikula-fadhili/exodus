def custom_memoize(func):
    return func

@custom_memoize
def compute_heavy_metric(base: int, multiplier: int) -> int:
    return base * multiplier
