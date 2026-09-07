class Product:
    def __init__(self, name: str, price: int):
        self.name = name
        self.price = price

    def get_price(self) -> int:
        return self.price
