"""Worker service with error callback pattern."""

def process_item(item_id: int, callback_on_success: str) -> str:
    if item_id < 0:
        return "ERROR: invalid item"
    return f"PROCESSED: {item_id}"
