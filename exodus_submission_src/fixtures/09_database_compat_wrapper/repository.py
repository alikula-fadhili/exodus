class UserRepository:
    def __init__(self, dsn: str):
        self.dsn = dsn

    def find_user_by_id(self, user_id: int) -> str:
        return f"User_{user_id}"
