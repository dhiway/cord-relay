// Copyright 2022 Dhiway Networks Pvt. Ltd.
// This file is part of CORD - `https://cord.network`.
// A relay node implementation based on Polkadot & Substrate.

// CORD is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// CORD is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with CORD. If not, see <https://www.gnu.org/licenses/>.

//! Expose the auto generated weight files.

pub mod block_weights;
pub mod extrinsic_weights;
pub mod paritydb_weights;
pub mod rocksdb_weights;

pub use block_weights::BlockExecutionWeight;
pub use extrinsic_weights::ExtrinsicBaseWeight;
pub use paritydb_weights::constants::ParityDbWeight;
pub use rocksdb_weights::constants::RocksDbWeight;
