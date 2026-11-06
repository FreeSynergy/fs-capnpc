//! Generic parameter bookkeeping (counterpart of `generics.c++`), split by concern.
//!
//! Pattern: Facade — re-exports `BrandScope`, `BrandedDecl` and `ImplicitParams`.

mod brand_eval;
mod brand_lookup;
mod brand_scope;
mod branded_decl;
mod branded_type;
mod decl_expression;
mod implicit;

pub use brand_scope::BrandScope;
pub use branded_decl::BrandedDecl;
pub use implicit::ImplicitParams;
