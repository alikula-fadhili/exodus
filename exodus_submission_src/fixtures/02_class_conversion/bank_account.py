class BankAccount:
    """A simple bank account."""
    def __init__(self, owner: str, balance: int = 0):
        self.owner = owner
        self.balance = balance

    def deposit(self, amount: int) -> int:
        self.balance = self.balance + amount
        return self.balance

    def withdraw(self, amount: int) -> int:
        if self.balance >= amount:
            self.balance = self.balance - amount
            return self.balance
        return -1
