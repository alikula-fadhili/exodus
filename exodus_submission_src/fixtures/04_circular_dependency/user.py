from order import get_user_orders

def get_user_summary(user_id: str) -> str:
    orders = get_user_orders(user_id)
    return f"User {user_id} has {len(orders)} orders"
