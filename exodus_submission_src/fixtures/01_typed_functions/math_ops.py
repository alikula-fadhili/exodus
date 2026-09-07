def add(a: int, b: int) -> int:
    """Add two integers."""
    return a + b

def multiply_list(nums: list[int], factor: int) -> list[int]:
    """Multiply list of numbers by factor."""
    result = []
    for n in nums:
        result.append(n * factor)
    return result

def is_positive(val: int) -> bool:
    if val > 0:
        return True
    else:
        return False
