// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptadores de infraestrutura e I/O (Transporte FTP/HTTP, Cache Hive-Parquet)

pub mod cache;
pub mod state;
pub mod storage;
pub mod transport;

pub use cache::{default_cache_dir, default_data_dir, MemoryCache};
pub use state::MemorySyncState;
pub use transport::{FileTransport, MockTransport};
