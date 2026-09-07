//! Project Exodus Cost & Budget Engine (`exodus-cost`)
//!
//! Provides deterministic pre-flight budget and risk previews, explainable model routing,
//! atomic expenditure capping (`BudgetGuard`), and interrupted run state persistence.

pub mod budget;
pub mod estimator;
pub mod interruption;

pub use budget::{BudgetError, BudgetGuard, MICRODOLLARS_PER_USD};
pub use estimator::{
    CostEstimator, MigrationBudgetPreview, MigrationRiskBreakdown, ModelRoutingRule,
};
pub use interruption::MigrationInterrupted;
