import stripe_legacy_sdk

def charge_card(card_token: str, amount_cents: int) -> bool:
    charge = stripe_legacy_sdk.Charge.create(amount=amount_cents, token=card_token)
    return charge.is_successful()
