//! Foundational data structures for whole-program analysis.
//!
//! This crate provides the building blocks for constructing and querying
//! dependency graphs and call graphs, as outlined in `50_PROPOSAL_ANALYSIS_PLATFORM.md`.

pub mod dependency;
pub mod call;
pub mod query;
pub mod cache;
pub mod error;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}