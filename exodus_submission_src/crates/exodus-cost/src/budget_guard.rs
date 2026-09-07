//! Micro-cent precision budget guard and hard expenditure capping.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;

/// 1 USD = 100,000,000 micro-cents (1 micro-cent = $0.00000001).
pub const MICROCENTS_PER_USD: f64 = 100_000_000.0;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum BudgetError {
    #[error("Hard expenditure limit reached (${limit_usd:.2} USD). Execution halted.")]
    ExceededHardLimit { limit_usd: f64 },
}

/// Thread-safe budget guard that meters token usage and halts workers on hard cap breaches.
#[derive(Debug, Clone)]
pub struct BudgetGuard {
    max_budget_microcents: u64,
    spent_microcents: Arc<AtomicU64>,
    tripped: Arc<AtomicBool>,
}

impl BudgetGuard {
    pub fn new(max_budget_usd: f64) -> Self {
        let max_budget_microcents = (max_budget_usd * MICROCENTS_PER_USD).round() as u64;
        Self {
            max_budget_microcents,
            spent_microcents: Arc::new(AtomicU64::new(0)),
            tripped: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn unlimited() -> Self {
        Self {
            max_budget_microcents: u64::MAX,
            spent_microcents: Arc::new(AtomicU64::new(0)),
            tripped: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Records token consumption against specific input/output rates (in microcents per 1k tokens).
    pub fn record_usage(
        &self,
        input_tokens: u64,
        output_tokens: u64,
        cost_per_1k_in_microcents: u64,
        cost_per_1k_out_microcents: u64,
    ) -> Result<f64, BudgetError> {
        let cost = (input_tokens.saturating_mul(cost_per_1k_in_microcents) / 1000)
            + (output_tokens.saturating_mul(cost_per_1k_out_microcents) / 1000);

        let previous_spent = self.spent_microcents.fetch_add(cost, Ordering::SeqCst);
        let total_spent = previous_spent + cost;

        if total_spent >= self.max_budget_microcents {
            self.tripped.store(true, Ordering::SeqCst);
            return Err(BudgetError::ExceededHardLimit {
                limit_usd: self.limit_usd(),
            });
        }

        Ok(total_spent as f64 / MICROCENTS_PER_USD)
    }

    pub fn current_spend_usd(&self) -> f64 {
        self.spent_microcents.load(Ordering::Relaxed) as f64 / MICROCENTS_PER_USD
    }

    pub fn limit_usd(&self) -> f64 {
        if self.max_budget_microcents == u64::MAX {
            f64::INFINITY
        } else {
            self.max_budget_microcents as f64 / MICROCENTS_PER_USD
        }
    }

    pub fn is_tripped(&self) -> bool {
        self.tripped.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_guard_usage_and_hard_limit() {
        // Set $1.00 budget
        let guard = BudgetGuard::new(1.00);
        assert_eq!(guard.limit_usd(), 1.00);
        assert_eq!(guard.current_spend_usd(), 0.0);

        // Record 10,000 input tokens at $0.0015 / 1k ($150,000 microcents)
        // Record 2,000 output tokens at $0.0060 / 1k ($600,000 microcents)
        // Cost = 10 * 150_000 + 2 * 600_000 = 1,500,000 + 1,200,000 = 2,700,000 microcents ($0.027)
        let spend = guard.record_usage(10_000, 2_000, 150_000, 600_000).unwrap();
        assert!((spend - 0.027).abs() < 1e-6);

        // Record large batch that exceeds $1.00 limit
        let err = guard.record_usage(500_000, 100_000, 150_000, 600_000).unwrap_err();
        assert_eq!(err, BudgetError::ExceededHardLimit { limit_usd: 1.00 });
        assert!(guard.is_tripped());
    }
}
