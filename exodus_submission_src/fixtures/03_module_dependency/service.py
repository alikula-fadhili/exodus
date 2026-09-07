from models import Product

def calculate_total(products: list[Product]) -> int:
    total = 0
    for p in products:
        total = total + p.get_price()
    return total
