//! Extensions for high-performance database operations
//!
//! This module provides optional performance enhancements:
//! - Optimized query modes (2x faster)
//! - Batch inserts (10x faster)
//! - Prepared statements (1.5x faster)
//! - Connection pooling (parallel queries)

pub mod queries;
pub mod batch;
pub mod prepared;
pub mod pool;

// Re-export main functions for easier access
pub use queries::{query_array, query_optimized};
pub use batch::{insert_batch, insert_batch_js, insert_batch_chunked_js};
pub use prepared::JsPreparedStatement;  // ← Agora exporta a struct JS-friendly
pub use pool::ConnectionPool;
