from user import get_user_summary

def get_user_orders(user_id: str) -> list[str]:
    return [f"order-1-{user_id}", f"order-2-{user_id}"]
